//! GNOME notifications through `notify-rust` (zbus backend). See docs/06 section 1.
//!
//! Actions: `open` shows the window and emits `window:show-event`; `join` opens the Meet link
//! in the system browser; `snooze` re-schedules 5 minutes later. `wait_for_action_async` runs
//! on a tokio task, never on the main thread.

use chrono::TimeZone;
use notify_rust::{Hint, Notification, NotificationResponse, Timeout};
use tauri::{Emitter, Manager};

use crate::commands::types::{ShowEvent, EVENT_WINDOW_SHOW_EVENT};
use crate::reminders::scheduler::{snooze, Due};

pub const APP_NAME: &str = "Unified Google Calendar";
/// Until the app icon is installed in hicolor (docs/06 section 1).
pub const ICON: &str = "x-office-calendar";

/// `"<title> in <n> min"` or `"<title> now"`.
pub fn summary_text(due: &Due, now: i64) -> String {
    let mins = (due.start_ts - now).div_euclid(60);
    if due.all_day && due.minutes == 0 {
        return format!("{} today", due.title);
    }
    if mins <= 0 {
        format!("{} now", due.title)
    } else if mins >= 120 {
        format!("{} in {} h", due.title, mins / 60)
    } else {
        format!("{} in {} min", due.title, mins)
    }
}

/// `"09:30 – 09:45 · Greelow"` in the primary zone, or `"All day · Greelow"`.
pub fn body_text(due: &Due, tz: chrono_tz::Tz) -> String {
    if due.all_day {
        return format!("All day · {}", due.account_name);
    }
    let s = tz
        .timestamp_opt(due.start_ts, 0)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default();
    let e = tz
        .timestamp_opt(due.end_ts, 0)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default();
    format!("{s} – {e} · {}", due.account_name)
}

fn primary_tz() -> chrono_tz::Tz {
    crate::db::handle()
        .and_then(|h| {
            h.call_blocking(|c| {
                crate::db::queries::settings::get_or(
                    c,
                    "primary_tz",
                    crate::config::DEFAULT_PRIMARY_TZ.to_string(),
                )
            })
        })
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

/// Show the notification and handle its actions in the background.
pub fn show(app: tauri::AppHandle, due: Due) {
    tauri::async_runtime::spawn(async move {
        let now = crate::db::now_ts();
        let tz = primary_tz();
        let mut n = Notification::new();
        n.appname(APP_NAME)
            .summary(&summary_text(&due, now))
            .body(&body_text(&due, tz))
            .icon(ICON)
            .action("open", "Open");
        if due.meet_link.is_some() {
            n.action("join", "Join");
        }
        n.action("snooze", "Snooze 5 min")
            .hint(Hint::Resident(true))
            .timeout(Timeout::Never);
        let handle = match n.show_async().await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(error = %e, key = %due.key, "notification failed");
                return;
            }
        };
        let mut chosen: Option<String> = None;
        handle
            .wait_for_action_async(|r| {
                chosen = match r {
                    NotificationResponse::Action(a) => Some(a.clone()),
                    NotificationResponse::Default => Some("open".into()),
                    _ => None,
                };
            })
            .await;
        match chosen.as_deref() {
            Some("open") => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.unminimize();
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                let _ = app.emit(
                    EVENT_WINDOW_SHOW_EVENT,
                    &ShowEvent {
                        occurrence_id: due.occurrence_id.clone(),
                    },
                );
            }
            Some("join") => {
                if let Some(link) = &due.meet_link {
                    use tauri_plugin_opener::OpenerExt;
                    if let Err(e) = app.opener().open_url(link, None::<&str>) {
                        tracing::warn!(error = %e, "could not open Meet link");
                    }
                }
            }
            Some("snooze") => snooze(&due.key, crate::db::now_ts()),
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn due(start: i64, all_day: bool, minutes: i64) -> Due {
        Due {
            key: "k".into(),
            occurrence_id: "o".into(),
            title: "Daily standup".into(),
            start_ts: start,
            end_ts: start + 900,
            all_day,
            account_name: "Greelow".into(),
            meet_link: None,
            minutes,
            fire_at: start - minutes * 60,
        }
    }

    #[test]
    fn texts() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-14T09:20:00-03:00")
            .unwrap()
            .timestamp();
        let d = due(now + 600, false, 10);
        assert_eq!(summary_text(&d, now), "Daily standup in 10 min");
        assert_eq!(summary_text(&d, now + 600), "Daily standup now");
        assert_eq!(
            summary_text(&due(now + 7200, false, 120), now),
            "Daily standup in 2 h"
        );
        assert_eq!(
            body_text(&d, chrono_tz::America::Argentina::Buenos_Aires),
            "09:30 – 09:45 · Greelow"
        );
        assert_eq!(
            body_text(&due(now, true, 0), chrono_tz::UTC),
            "All day · Greelow"
        );
        assert_eq!(summary_text(&due(now, true, 0), now), "Daily standup today");
    }
}
