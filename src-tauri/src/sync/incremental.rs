//! Incremental sync of one calendar with its sync token. See docs/05-sincronizacion.md
//! section 2.2 ("Incremental") and docs/03 section 3 for the re-expansion rules.

use crate::commands::types::{CalendarKey, CalendarUpdated};
use crate::db::queries::{calendars, events as q, settings};
use crate::error::AppError;
use crate::google::types::Event;
use crate::recurrence::expand::{expand_master, materialize_simple, Window};
use crate::sync::ctx::SyncCtx;
use crate::sync::full::full_sync_calendar;
use crate::sync::map::event_to_row;

/// Range touched by a batch of changes; `None` means nothing changed.
#[derive(Debug, Default, Clone, Copy)]
pub struct TouchedRange {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub whole_window: bool,
}

impl TouchedRange {
    fn add(&mut self, s: i64, e: i64) {
        self.from = Some(self.from.map_or(s, |f| f.min(s)));
        self.to = Some(self.to.map_or(e, |t| t.max(e)));
    }
}

/// Store one page of changes and re-materialize what they touch. Runs on the db thread.
pub fn apply_changes(
    conn: &mut rusqlite::Connection,
    account_id: &str,
    calendar_id: &str,
    items: &[Event],
    calendar_tz: Option<&str>,
    window: Window,
) -> Result<TouchedRange, AppError> {
    let mut touched = TouchedRange::default();
    for ev in items.iter().filter(|e| e.id.is_some()) {
        let row = event_to_row(account_id, calendar_id, ev, calendar_tz);
        // Old occurrences of this row (before the change) also need a repaint.
        for o in q::occurrences_of_event(conn, account_id, calendar_id, &row.id)? {
            touched.add(o.start_ts, o.end_ts);
        }
        q::upsert_event(conn, &row)?;
        if row.is_recurring_master()
            || (row.is_cancelled()
                && row.recurring_event_id.is_none()
                && !q::exceptions_of(conn, account_id, calendar_id, &row.id)?.is_empty())
        {
            expand_master(conn, account_id, calendar_id, &row.id, window)?;
            touched.whole_window = true;
        } else if let Some(master_id) = &row.recurring_event_id {
            if q::get_event(conn, account_id, calendar_id, master_id)?.is_some() {
                expand_master(conn, account_id, calendar_id, master_id, window)?;
            } else {
                materialize_simple(conn, account_id, calendar_id, &row.id, window)?;
            }
            if let Some(ts) = row.original_start_ts {
                touched.add(ts, ts);
            }
            for o in q::occurrences_of_event(conn, account_id, calendar_id, &row.id)? {
                touched.add(o.start_ts, o.end_ts);
            }
        } else {
            materialize_simple(conn, account_id, calendar_id, &row.id, window)?;
            for o in q::occurrences_of_event(conn, account_id, calendar_id, &row.id)? {
                touched.add(o.start_ts, o.end_ts);
            }
        }
    }
    Ok(touched)
}

/// Incremental sync. Falls back to a full sync when there is no token or Google answers 410.
pub async fn incremental_calendar(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
) -> Result<(), AppError> {
    let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
    let calendar = ctx
        .db
        .call(move |c| calendars::get_calendar(c, &acc, &cal))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Calendar {calendar_id}")))?;
    let Some(mut sync_token) = calendar
        .sync_token
        .clone()
        .filter(|_| calendar.full_sync_done)
    else {
        return full_sync_calendar(ctx, account_id, calendar_id).await;
    };
    let token = ctx.tokens.token(account_id).await?;
    let mut page_token: Option<String> = None;
    let mut total = TouchedRange::default();
    let mut changed = 0usize;
    loop {
        let page = match ctx
            .client
            .events_page(
                &token,
                calendar_id,
                page_token.as_deref(),
                Some(&sync_token),
            )
            .await
        {
            Ok(p) => p,
            Err(AppError::Google { status: 410, .. }) => {
                tracing::info!(
                    account = account_id,
                    calendar = calendar_id,
                    "sync token gone; wiping and running a full sync"
                );
                let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
                ctx.db
                    .call(move |c| {
                        calendars::wipe_events(c, &acc, &cal)?;
                        settings::log_sync(
                            c,
                            Some(&acc),
                            Some(&cal),
                            "error",
                            Some("410 Gone: full sync"),
                        )
                    })
                    .await?;
                return full_sync_calendar(ctx, account_id, calendar_id).await;
            }
            Err(e) => return Err(e),
        };
        let tz = page
            .time_zone
            .clone()
            .or_else(|| calendar.time_zone.clone());
        let items = page.items;
        changed += items.len();
        let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
        let touched = ctx
            .db
            .call(move |c| {
                let window = Window::current(c)?;
                let tx_touched = apply_changes(c, &acc, &cal, &items, tz.as_deref(), window)?;
                Ok((tx_touched, window))
            })
            .await?;
        let (t, window) = touched;
        if t.whole_window {
            total.whole_window = true;
            total.add(window.from_ts, window.to_ts);
        }
        if let (Some(f), Some(to)) = (t.from, t.to) {
            total.add(f, to);
        }
        if let Some(st) = page.next_sync_token {
            sync_token = st;
            break;
        }
        match page.next_page_token {
            Some(p) => page_token = Some(p),
            None => {
                return Err(AppError::Network(
                    "events.list ended without nextSyncToken".into(),
                ))
            }
        }
    }
    let (acc, cal, st) = (account_id.to_string(), calendar_id.to_string(), sync_token);
    ctx.db
        .call(move |c| {
            calendars::set_sync_token(c, &acc, &cal, Some(&st), true)?;
            settings::log_sync(
                c,
                Some(&acc),
                Some(&cal),
                "incremental",
                Some(&format!("{changed} changes")),
            )
        })
        .await?;
    if let (Some(from), Some(to)) = (total.from, total.to) {
        ctx.events.calendar_updated(CalendarUpdated {
            from,
            to,
            calendar_ids: vec![CalendarKey {
                account_id: account_id.into(),
                calendar_id: calendar_id.into(),
            }],
        });
    }
    Ok(())
}
