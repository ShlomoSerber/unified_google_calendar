//! Minute clock for the UI: emits `clock:minute` at every minute boundary so the now line and
//! the "Today" state move without a JS timer (docs/02 section 4: periodic work lives in Rust).

use tauri::Emitter;

pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let now = crate::db::now_ts();
            let wait = 60 - now.rem_euclid(60);
            tokio::time::sleep(std::time::Duration::from_secs(wait as u64)).await;
            let ts = crate::db::now_ts();
            if let Err(e) = app.emit(crate::commands::types::EVENT_CLOCK_MINUTE, ts) {
                tracing::debug!(error = %e, "clock event not delivered");
            }
        }
    });
}
