//! System tray. See docs/06-integracion-gnome.md section 2.
//!
//! GNOME AppIndicator: no left click, no tooltip; everything goes through the menu. The
//! title next to the icon shows the next timed event within 12 h as `"09:30 Daily standup"`,
//! truncated to 32 characters, refreshed after every `calendar:updated` and every minute.

use std::sync::OnceLock;

use chrono::TimeZone;
use rusqlite::{params, Connection};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Listener, Wry};

use crate::db::queries::settings;
use crate::error::AppError;

pub const TRAY_ID: &str = "main";
pub const TITLE_MAX_CHARS: usize = 32;
pub const LOOKAHEAD_SECS: i64 = 12 * 3600;

static NEXT_ITEM: OnceLock<MenuItem<Wry>> = OnceLock::new();

/// Labels for the tray: `(title next to the icon, menu line)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextEvent {
    pub title: String,
    pub menu: String,
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Next timed occurrence of a visible calendar starting within 12 h.
pub fn next_event(conn: &Connection, now: i64) -> Result<Option<NextEvent>, AppError> {
    let primary_tz: String = settings::get_or(
        conn,
        "primary_tz",
        crate::config::DEFAULT_PRIMARY_TZ.to_string(),
    )?;
    let tz: chrono_tz::Tz = primary_tz.parse().unwrap_or(chrono_tz::UTC);
    let row = conn
        .query_row(
            "SELECT o.start_ts, e.summary FROM occurrences o \
             JOIN events e ON e.account_id=o.account_id AND e.calendar_id=o.calendar_id AND e.id=o.event_id \
             JOIN calendars c ON c.account_id=o.account_id AND c.id=o.calendar_id \
             WHERE o.all_day=0 AND o.status != 'cancelled' AND c.visible=1 AND c.deleted=0 \
               AND o.start_ts >= ?1 AND o.start_ts <= ?2 \
             ORDER BY o.start_ts LIMIT 1",
            params![now, now + LOOKAHEAD_SECS],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .ok();
    Ok(row.map(|(start, summary)| {
        let time = tz
            .timestamp_opt(start, 0)
            .single()
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_default();
        let name = summary.unwrap_or_else(|| "(No title)".into());
        NextEvent {
            title: truncate(&format!("{time} {name}"), TITLE_MAX_CHARS),
            menu: format!("Next: {name} · {time}"),
        }
    }))
}

/// `tray::build` per docs/08 section 12.
pub fn build(app: &AppHandle) -> Result<(), AppError> {
    let err = |e: tauri::Error| AppError::Invalid(format!("tray: {e}"));
    let next =
        MenuItem::with_id(app, "next", "No upcoming events", false, None::<&str>).map_err(err)?;
    let open = MenuItem::with_id(
        app,
        "open",
        "Open Unified Google Calendar",
        true,
        None::<&str>,
    )
    .map_err(err)?;
    let sync = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>).map_err(err)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(err)?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(err)?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(err)?;
    let menu = Menu::with_items(app, &[&next, &sep1, &open, &sync, &sep2, &quit]).map_err(err)?;
    let _ = NEXT_ITEM.set(next);
    let mut builder =
        TrayIconBuilder::with_id(TRAY_ID)
            .menu(&menu)
            .on_menu_event(|app, ev| match ev.id().as_ref() {
                "open" => crate::show_main_window(app),
                "sync" => {
                    if let Ok(engine) = crate::sync::engine::engine() {
                        tauri::async_runtime::spawn(async move {
                            let _ = engine.sync_all("manual").await;
                        });
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app).map_err(err)?;
    Ok(())
}

/// `tray::set_next_event` per docs/08 section 12: title next to the icon and the menu line.
pub fn set_next_event(app: &AppHandle, label: Option<String>) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(label.clone());
    }
    if let Some(item) = NEXT_ITEM.get() {
        let _ = item.set_text(
            label
                .map(|l| format!("Next: {l}"))
                .unwrap_or_else(|| "No upcoming events".into()),
        );
    }
}

async fn refresh(app: &AppHandle) {
    let now = crate::db::now_ts();
    match crate::db::call(move |c| next_event(c, now)).await {
        Ok(Some(n)) => {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_title(Some(n.title.clone()));
            }
            if let Some(item) = NEXT_ITEM.get() {
                let _ = item.set_text(&n.menu);
            }
        }
        Ok(None) => set_next_event(app, None),
        Err(e) => tracing::debug!(error = %e, "tray refresh failed"),
    }
}

/// Refresh every minute and after each `calendar:updated`.
pub fn start(app: AppHandle) {
    let handle = app.clone();
    app.listen(crate::commands::types::EVENT_CALENDAR_UPDATED, move |_| {
        let h = handle.clone();
        tauri::async_runtime::spawn(async move { refresh(&h).await });
    });
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            refresh(&app).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::events::EventRow;
    use crate::recurrence::{expand::Window, materialize_simple};

    #[test]
    fn next_event_label_and_truncation() {
        let mut conn = crate::db::open_memory().unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-14T09:00:00-03:00")
            .unwrap()
            .timestamp();
        let mk = |id: &str, start: i64, title: &str, all_day: bool| EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: id.into(),
            status: "confirmed".into(),
            summary: Some(title.into()),
            start_ts: if all_day { None } else { Some(start) },
            end_ts: if all_day { None } else { Some(start + 900) },
            start_date: if all_day {
                Some("2026-09-14".into())
            } else {
                None
            },
            end_date: if all_day {
                Some("2026-09-15".into())
            } else {
                None
            },
            all_day,
            attendees: "[]".into(),
            reminders: "{}".into(),
            event_type: "default".into(),
            ..Default::default()
        };
        let w = Window {
            from_ts: 0,
            to_ts: i64::MAX / 2,
        };
        for e in [
            mk("allday", now, "All day thing", true),
            mk("past", now - 600, "Past", false),
            mk("far", now + 13 * 3600, "Far", false),
            mk(
                "next",
                now + 1800,
                "Daily standup with a very long name indeed",
                false,
            ),
            mk("later", now + 3600, "Later", false),
        ] {
            crate::db::queries::events::upsert_event(&conn, &e).unwrap();
            materialize_simple(&mut conn, "local", "local-personal", &e.id, w).unwrap();
        }
        let n = next_event(&conn, now).unwrap().unwrap();
        assert_eq!(n.title, "09:30 Daily standup with a very…");
        assert_eq!(n.title.chars().count(), 32);
        assert_eq!(
            n.menu,
            "Next: Daily standup with a very long name indeed · 09:30"
        );
        crate::db::queries::calendars::set_visible(&conn, "local", "local-personal", false)
            .unwrap();
        assert!(next_event(&conn, now).unwrap().is_none());
        assert_eq!(truncate("short", 32), "short");
    }
}
