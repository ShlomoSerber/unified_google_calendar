//! Tauri builder, plugins and setup. See docs/02-arquitectura.md sections 2 and 3.1.

pub mod auth;
pub mod commands;
pub mod config;
pub mod db;
pub mod eds;
pub mod error;
pub mod google;
pub mod recurrence;
pub mod reminders;
pub mod sync;
pub mod tray;
pub mod webhook;

use tauri::Manager;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Log to `logs/app.log` with daily rotation and 7 files kept (docs/02 section 9).
/// Returns the guard that flushes the non-blocking writer; it must live as long as the app.
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = config::logs_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create log dir {}: {e}", dir.display());
        return None;
    }
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("app")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_env("UGC_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(writer);
    let stderr_layer = if cfg!(debug_assertions) {
        Some(fmt::layer().with_writer(std::io::stderr))
    } else {
        None
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .init();
    Some(guard)
}

pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = init_logging();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "app started");

    let app = tauri::Builder::default()
        // Must be the first plugin (docs/06 section 3).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!("second instance launched; showing existing window");
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(commands::handler())
        .setup(|_app| {
            db::init(&config::db_path())?;
            match auth::oauth::init() {
                Ok(_) => tracing::info!("oauth configured"),
                Err(e) => {
                    tracing::warn!(error = %e, "Google sign-in unavailable until oauth.json exists")
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            tracing::error!(error = %e, "error while building the Tauri application");
            std::process::exit(1);
        }
    };
    app.run(|_app, event| {
        // Closing the last window must not exit; "Quit" from the tray calls `app.exit(0)`,
        // which arrives with `code: Some(0)` and must go through (docs/06 section 3).
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
