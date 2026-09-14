//! Sync of calendars subscribed by secret iCal URL. See docs/99-decisiones.md
//! ("Calendarios iCal por URL"). Read-only: the feed replaces the calendar's events.

use sha2::{Digest, Sha256};

use crate::auth::TokenStore;
use crate::commands::types::{CalendarKey, CalendarUpdated};
use crate::db::queries::{accounts, calendars, events as q, settings};
use crate::error::AppError;
use crate::recurrence::expand::{rebuild_calendar, Window};
use crate::sync::ics;
use crate::sync::SyncCtx;

/// Calendar id used for the single calendar of an iCal account.
pub const CALENDAR_ID: &str = "ical";

pub fn validate_url(url: &str) -> Result<(), AppError> {
    let parsed = url::Url::parse(url)
        .map_err(|_| AppError::invalid("The iCal address is not a valid URL"))?;
    // Plain http is accepted only for loopback (tests and local mirrors).
    let loopback = matches!(parsed.host_str(), Some("127.0.0.1") | Some("localhost"));
    if parsed.scheme() != "https" && !loopback {
        return Err(AppError::invalid(
            "The iCal address must start with https://",
        ));
    }
    Ok(())
}

/// Download the feed. Returns `None` when the server answered 304 for `etag`.
pub async fn fetch(
    ctx: &SyncCtx,
    url: &str,
    etag: Option<&str>,
) -> Result<Option<(String, Option<String>)>, AppError> {
    let mut req = ctx.client.http.get(url);
    if let Some(e) = etag {
        req = req.header("If-None-Match", e);
    }
    let resp = req.send().await?;
    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(AppError::Network(format!(
            "the iCal address answered {}",
            resp.status()
        )));
    }
    let new_etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = resp.text().await?;
    if !body.contains("BEGIN:VCALENDAR") {
        return Err(AppError::Invalid(
            "The address did not return an iCalendar file".into(),
        ));
    }
    Ok(Some((body, new_etag)))
}

/// Replace the calendar's events with the feed's, re-materialize, and report the change.
/// Returns the number of events stored.
pub fn apply(
    conn: &mut rusqlite::Connection,
    account_id: &str,
    text: &str,
    self_email: Option<&str>,
) -> Result<usize, AppError> {
    let cal = ics::parse(text);
    let existing = calendars::get_calendar(conn, account_id, CALENDAR_ID)?;
    let account = accounts::get_account(conn, account_id)?
        .ok_or_else(|| AppError::NotFound("The account".into()))?;
    // Feeds often name the calendar after the e-mail address; the user's label reads better.
    let summary = cal
        .name
        .clone()
        .filter(|n| !n.is_empty() && !n.contains('@'))
        .unwrap_or_else(|| account.display_name.clone());
    let row = calendars::CalendarRow {
        id: CALENDAR_ID.into(),
        account_id: account_id.into(),
        summary,
        description: None,
        color_bg: existing
            .as_ref()
            .map(|e| e.color_bg.clone())
            .unwrap_or_else(|| "#7986cb".into()),
        color_fg: existing
            .as_ref()
            .map(|e| e.color_fg.clone())
            .unwrap_or_else(|| "#ffffff".into()),
        access_role: "reader".into(),
        is_primary: true,
        visible: existing.as_ref().map(|e| e.visible).unwrap_or(true),
        hidden_remote: false,
        deleted: false,
        time_zone: cal.time_zone.clone(),
        default_reminders: existing
            .as_ref()
            .map(|e| e.default_reminders.clone())
            .unwrap_or_else(|| "[]".into()),
        sync_token: None,
        full_sync_done: true,
        sort_order: 0,
    };
    calendars::upsert_calendar(conn, &row)?;
    let rows: Vec<q::EventRow> = cal
        .events
        .iter()
        .filter_map(|e| {
            ics::to_row(
                account_id,
                CALENDAR_ID,
                e,
                cal.time_zone.as_deref(),
                self_email,
            )
        })
        .collect();
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM events WHERE account_id=?1 AND calendar_id=?2",
        rusqlite::params![account_id, CALENDAR_ID],
    )?;
    for r in &rows {
        q::upsert_event(&tx, r)?;
    }
    tx.commit()?;
    let window = Window::current(conn)?;
    rebuild_calendar(conn, account_id, CALENDAR_ID, window)?;
    calendars::set_sync_token(conn, account_id, CALENDAR_ID, None, true)?;
    settings::log_sync(
        conn,
        Some(account_id),
        Some(CALENDAR_ID),
        "incremental",
        Some(&format!("ical {} events", rows.len())),
    )?;
    Ok(rows.len())
}

/// Sync one iCal account: fetch, skip if unchanged, apply.
pub async fn sync_ical_account(ctx: &SyncCtx, account_id: &str) -> Result<(), AppError> {
    let store = TokenStore::open_default()?;
    let url = store
        .get_ical_url(account_id)?
        .ok_or_else(|| AppError::Auth("no iCal address stored for this account".into()))?;
    let acc = account_id.to_string();
    let (etag_key, hash_key) = (
        format!("ical_etag:{account_id}"),
        format!("ical_hash:{account_id}"),
    );
    let (k1, k2) = (etag_key.clone(), hash_key.clone());
    let (etag, old_hash): (Option<String>, Option<String>) = ctx
        .db
        .call(move |c| Ok((settings::get(c, &k1)?, settings::get(c, &k2)?)))
        .await?;
    let Some((body, new_etag)) = fetch(ctx, &url, etag.as_deref()).await? else {
        tracing::debug!(account = account_id, "ical feed not modified");
        return Ok(());
    };
    let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
    if old_hash.as_deref() == Some(hash.as_str()) {
        return Ok(());
    }
    let email = ctx
        .db
        .call({
            let a = acc.clone();
            move |c| Ok(accounts::get_account(c, &a)?.and_then(|r| r.email))
        })
        .await?;
    let n = ctx
        .db
        .call(move |c| {
            let n = apply(c, &acc, &body, email.as_deref())?;
            settings::set(c, &hash_key, &hash)?;
            match &new_etag {
                Some(e) => settings::set(c, &etag_key, e)?,
                None => settings::delete(c, &etag_key)?,
            }
            Ok((n, Window::current(c)?))
        })
        .await?;
    let (count, window) = n;
    tracing::info!(account = account_id, events = count, "ical feed applied");
    ctx.events.calendar_updated(CalendarUpdated {
        from: window.from_ts,
        to: window.to_ts,
        calendar_ids: vec![CalendarKey {
            account_id: account_id.into(),
            calendar_id: CALENDAR_ID.into(),
        }],
    });
    Ok(())
}

/// Create the account, store the URL encrypted and run the first sync.
pub async fn add_ical_account(
    ctx: &SyncCtx,
    name: &str,
    url: &str,
    email: Option<&str>,
) -> Result<String, AppError> {
    validate_url(url)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::invalid("Give the calendar a name"));
    }
    // Probe first so a wrong address never leaves a half-created account.
    let (body, etag) = fetch(ctx, url, None)
        .await?
        .ok_or_else(|| AppError::Network("empty answer from the iCal address".into()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let store = TokenStore::open_default()?;
    store.put_ical_url(&id, url)?;
    let (id2, name2, email2) = (id.clone(), name.to_string(), email.map(str::to_string));
    ctx.db
        .call(move |c| {
            accounts::insert_ical_account(c, &id2, &name2, email2.as_deref())?;
            apply(c, &id2, &body, email2.as_deref())?;
            settings::set(
                c,
                &format!("ical_hash:{id2}"),
                &format!("{:x}", Sha256::digest(body.as_bytes())),
            )?;
            if let Some(e) = &etag {
                settings::set(c, &format!("ical_etag:{id2}"), e)?;
            }
            Ok(())
        })
        .await?;
    let w = ctx.db.call(|c| Window::current(c)).await?;
    ctx.events.calendar_updated(CalendarUpdated {
        from: w.from_ts,
        to: w.to_ts,
        calendar_ids: vec![CalendarKey {
            account_id: id.clone(),
            calendar_id: CALENDAR_ID.into(),
        }],
    });
    tracing::info!(account = %id, "ical calendar added");
    Ok(id)
}
