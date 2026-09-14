//! Wake-up catch-up. See docs/02-arquitectura.md section 3.5 and docs/05 section 4.
//!
//! Listens to `org.freedesktop.login1.Manager.PrepareForSleep` on the system bus. `false`
//! means the machine just woke up: run an incremental sync of everything and check channels.

use std::future::poll_fn;
use std::pin::Pin;
use std::sync::Arc;

use zbus::export::futures_core::Stream;

use crate::db::queries::settings;
use crate::error::AppError;
use crate::sync::engine::Engine;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

async fn next_signal<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

/// What to do when the machine wakes up.
pub async fn on_wake(engine: &Engine) {
    tracing::info!("system resumed from sleep; catching up");
    let _ = engine
        .ctx
        .db
        .call(|c| settings::log_sync(c, None, None, "wake", None))
        .await;
    if let Err(e) = engine.sync_all("wake").await {
        tracing::debug!(error = %e, "wake catch-up finished with errors");
    }
    if let Err(e) = crate::sync::push::ensure_channels(&engine.ctx).await {
        tracing::debug!(error = %e, "channel check after wake failed");
    }
}

pub async fn listen(engine: Arc<Engine>) -> Result<(), AppError> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|e| AppError::Network(format!("system bus: {e}")))?;
    let proxy = Login1ManagerProxy::new(&conn)
        .await
        .map_err(|e| AppError::Network(format!("login1 proxy: {e}")))?;
    let mut stream = proxy
        .receive_prepare_for_sleep()
        .await
        .map_err(|e| AppError::Network(format!("login1 signal: {e}")))?;
    tracing::info!("listening for PrepareForSleep on login1");
    while let Some(signal) = next_signal(&mut stream).await {
        match signal.args() {
            Ok(args) if !args.start => on_wake(&engine).await,
            Ok(_) => tracing::info!("system is going to sleep"),
            Err(e) => tracing::debug!(error = %e, "bad PrepareForSleep payload"),
        }
    }
    Ok(())
}

/// Spawn the listener; a missing system bus only logs (docs/02 section 3.5 still has the poll).
pub fn start(engine: Arc<Engine>) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = listen(engine).await {
            tracing::warn!(error = %e, "sleep listener unavailable; relying on polling");
        }
    });
}
