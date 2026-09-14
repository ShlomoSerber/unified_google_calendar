//! Full sync of one calendar. See docs/05-sincronizacion.md section 2.2 ("Full sync").

use crate::commands::types::{CalendarKey, CalendarUpdated};
use crate::db::queries::{calendars, events as q, settings};
use crate::error::AppError;
use crate::recurrence::expand::{rebuild_calendar, Window};
use crate::sync::ctx::SyncCtx;
use crate::sync::map::event_to_row;

/// Page through `events.list` without a sync token, store every item, then materialize the
/// window and save `nextSyncToken`.
pub async fn full_sync_calendar(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
) -> Result<(), AppError> {
    let token = ctx.tokens.token(account_id).await?;
    let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
    let calendar_tz = ctx
        .db
        .call(move |c| Ok(calendars::get_calendar(c, &acc, &cal)?.and_then(|r| r.time_zone)))
        .await?;
    let mut page_token: Option<String> = None;
    let mut pages = 0usize;
    let sync_token = loop {
        let page = ctx
            .client
            .events_page(&token, calendar_id, page_token.as_deref(), None)
            .await?;
        pages += 1;
        let tz = page.time_zone.clone().or_else(|| calendar_tz.clone());
        let rows: Vec<q::EventRow> = page
            .items
            .iter()
            .filter(|e| e.id.is_some())
            .map(|e| event_to_row(account_id, calendar_id, e, tz.as_deref()))
            .collect();
        let n = rows.len();
        ctx.db
            .call(move |c| {
                let tx = c.transaction()?;
                for r in &rows {
                    q::upsert_event(&tx, r)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        tracing::debug!(
            account = account_id,
            calendar = calendar_id,
            page = pages,
            items = n,
            "full sync page stored"
        );
        if let Some(st) = page.next_sync_token {
            break st;
        }
        match page.next_page_token {
            Some(p) => page_token = Some(p),
            None => {
                return Err(AppError::Network(
                    "events.list ended without nextSyncToken".into(),
                ))
            }
        }
    };
    let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
    let window = ctx
        .db
        .call(move |c| {
            let window = Window::current(c)?;
            rebuild_calendar(c, &acc, &cal, window)?;
            calendars::set_sync_token(c, &acc, &cal, Some(&sync_token), true)?;
            settings::log_sync(
                c,
                Some(&acc),
                Some(&cal),
                "full",
                Some(&format!("{pages} pages")),
            )?;
            Ok(window)
        })
        .await?;
    ctx.events.calendar_updated(CalendarUpdated {
        from: window.from_ts,
        to: window.to_ts,
        calendar_ids: vec![CalendarKey {
            account_id: account_id.into(),
            calendar_id: calendar_id.into(),
        }],
    });
    Ok(())
}
