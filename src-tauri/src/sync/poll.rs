//! Safety-net polling and the daily data-window move. See docs/05-sincronizacion.md
//! section 4 and docs/03-modelo-de-datos.md section 3.
//!
//! Interval: 10 minutes with push enabled, 60 seconds without. Once a day the window is
//! recomputed, new occurrences materialized and the ones that fell out deleted.

use std::time::Duration;

use crate::db::queries::{calendars, events as q, settings};
use crate::error::AppError;
use crate::recurrence::expand::{rebuild_calendar, Window};
use crate::sync::engine::Engine;
use crate::sync::SyncCtx;
use std::sync::Arc;

pub const POLL_WITH_PUSH: Duration = Duration::from_secs(600);
pub const POLL_WITHOUT_PUSH: Duration = Duration::from_secs(60);
pub const WINDOW_REFRESH: Duration = Duration::from_secs(24 * 3600);

pub async fn interval_for(ctx: &SyncCtx) -> Duration {
    let enabled = ctx
        .db
        .call(|c| settings::get_or(c, "push_enabled", false))
        .await
        .unwrap_or(false);
    if enabled {
        POLL_WITH_PUSH
    } else {
        POLL_WITHOUT_PUSH
    }
}

/// Recompute the window and re-materialize every calendar (docs/03 section 3).
pub async fn refresh_window(ctx: &SyncCtx) -> Result<Window, AppError> {
    ctx.db
        .call(|c| {
            let window = Window::current(c)?;
            for cal in calendars::list_calendars(c)? {
                rebuild_calendar(c, &cal.account_id, &cal.id, window)?;
            }
            q::delete_occurrences_outside(c, window.from_ts, window.to_ts)?;
            settings::log_sync(c, None, None, "incremental", Some("data window refreshed"))?;
            Ok(window)
        })
        .await
}

/// Runs forever: incremental of everything at the configured interval, window refresh daily.
pub async fn run(engine: Arc<Engine>) {
    let mut last_window = tokio::time::Instant::now();
    loop {
        let wait = interval_for(&engine.ctx).await;
        tokio::time::sleep(wait).await;
        let (acc, reason) = (None::<String>, "poll");
        let _ = engine
            .ctx
            .db
            .call(move |c| settings::log_sync(c, acc.as_deref(), None, reason, None))
            .await;
        if let Err(e) = engine.sync_all("poll").await {
            tracing::debug!(error = %e, "poll finished with errors");
        }
        if last_window.elapsed() >= WINDOW_REFRESH {
            last_window = tokio::time::Instant::now();
            match refresh_window(&engine.ctx).await {
                Ok(w) => {
                    engine
                        .ctx
                        .events
                        .calendar_updated(crate::commands::types::CalendarUpdated {
                            from: w.from_ts,
                            to: w.to_ts,
                            calendar_ids: vec![],
                        });
                }
                Err(e) => tracing::warn!(error = %e, "window refresh failed"),
            }
        }
    }
}
