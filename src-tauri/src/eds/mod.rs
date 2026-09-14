//! Evolution Data Server mirror. See docs/06-integracion-gnome.md section 4.

pub mod dbus;
pub mod goa;
pub mod mirror;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Listener};

use crate::commands::types::{CalendarUpdated, EVENT_CALENDAR_UPDATED};
use crate::db::DbHandle;
use crate::error::AppError;

/// `eds::mirror_calendars` per docs/08 section 12.
pub async fn mirror_calendars(
    _app: &AppHandle,
    calendar_keys: &[(String, String)],
) -> Result<(), AppError> {
    let eds = dbus::Eds::connect().await?;
    let db = crate::db::handle()?;
    mirror::mirror_calendars(&eds, &db, calendar_keys).await
}

/// `eds::remove_account_sources` per docs/08 section 12.
pub async fn remove_account_sources(_app: &AppHandle, account_id: &str) -> Result<(), AppError> {
    let eds = dbus::Eds::connect().await?;
    let db = crate::db::handle()?;
    mirror::remove_account_sources(&eds, &db, account_id).await
}

async fn all_calendar_keys(db: &DbHandle) -> Result<Vec<(String, String)>, AppError> {
    Ok(db
        .call(|c| crate::db::queries::calendars::list_calendars(c))
        .await?
        .into_iter()
        .map(|c| (c.account_id, c.id))
        .collect())
}

async fn run_mirror(db: &DbHandle, keys: Vec<(String, String)>) {
    let eds = match dbus::Eds::connect().await {
        Ok(e) => e,
        Err(e) => {
            tracing::info!(error = %e, "eds mirror skipped");
            return;
        }
    };
    if let Err(e) = mirror::mirror_calendars(&eds, db, &keys).await {
        tracing::warn!(error = %e, "eds mirror failed");
    }
}

/// Debounced mirror after `calendar:updated` (5 s, touched calendars only) and a daily full
/// pass that moves the 30/180-day window and prunes orphan sources (docs/06 section 4.5).
pub fn start(app: AppHandle) {
    let db = match crate::db::handle() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(error = %e, "eds mirror not started");
            return;
        }
    };
    let pending: Arc<Mutex<HashSet<(String, String)>>> = Arc::new(Mutex::new(HashSet::new()));
    let scheduled = Arc::new(Mutex::new(false));
    let (p, s, db2) = (pending.clone(), scheduled.clone(), db.clone());
    app.listen(EVENT_CALENDAR_UPDATED, move |event| {
        let payload: CalendarUpdated = match serde_json::from_str(event.payload()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let all = payload.calendar_ids.is_empty();
        if let Ok(mut set) = p.lock() {
            if all {
                set.insert(("*".into(), "*".into()));
            }
            for k in payload.calendar_ids {
                set.insert((k.account_id, k.calendar_id));
            }
        }
        let already = s
            .lock()
            .map(|mut f| std::mem::replace(&mut *f, true))
            .unwrap_or(true);
        if already {
            return;
        }
        let (p2, s2, db3) = (p.clone(), s.clone(), db2.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(mirror::DEBOUNCE_SECS)).await;
            let keys: Vec<(String, String)> = p2
                .lock()
                .map(|mut set| set.drain().collect())
                .unwrap_or_default();
            if let Ok(mut f) = s2.lock() {
                *f = false;
            }
            let keys = if keys.iter().any(|(a, _)| a == "*") {
                all_calendar_keys(&db3).await.unwrap_or_default()
            } else {
                keys
            };
            run_mirror(&db3, keys).await;
        });
    });
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            let keys = all_calendar_keys(&db).await.unwrap_or_default();
            run_mirror(&db, keys).await;
            if let Ok(eds) = dbus::Eds::connect().await {
                if let Err(e) = mirror::prune_orphans(&eds, &db).await {
                    tracing::debug!(error = %e, "eds prune failed");
                }
            }
        }
    });
}
