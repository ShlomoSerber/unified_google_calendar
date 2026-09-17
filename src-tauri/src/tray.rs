//! System tray. See docs/06-integracion-gnome.md section 2.
//!
//! GNOME AppIndicator: no left click, no tooltip; everything goes through the menu. The
//! title next to the icon shows today's next timed event as `"09:30 Daily standup"`, truncated
//! to 32 characters, refreshed after every `calendar:updated` and every minute. An event stays
//! until `GRACE_SECS` after its start; when nothing is left today the title is empty until
//! tomorrow (docs/06 section 2, user decision of 2026-09-15). Every refresh alternates an
//! invisible zero-width space at the end of the title: GNOME's AppIndicator extension only
//! redraws a label whose value differs from its cache, and a value it missed (2026-09-17: the
//! label was on the bus, the panel showed nothing) stayed missing until the next event.
//! The icon is today's day of month in the primary time zone (icons/day/NN.rgba and NN.png,
//! rendered by scripts/gen-day-icons.mjs); the tray, the main window and the user's hicolor
//! theme (for GNOME's dock) get it at start and at midnight.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use chrono::{Datelike, TimeZone};
use rusqlite::{params, Connection};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Listener, Manager, Wry};

use crate::db::queries::settings;
use crate::error::AppError;

pub const TRAY_ID: &str = "main";
pub const TITLE_MAX_CHARS: usize = 32;
/// A started event keeps the title this long after its start.
pub const GRACE_SECS: i64 = 120;

static NEXT_ITEM: OnceLock<MenuItem<Wry>> = OnceLock::new();
static TICK: AtomicU32 = AtomicU32::new(0);
const ZERO_WIDTH_SPACE: char = '\u{200B}';
/// Day of month currently drawn on the icons; 0 until the first refresh.
static ICON_DAY: AtomicU32 = AtomicU32::new(0);

const DAY_ICON_SIZE: u32 = 64;
/// Raw RGBA `DAY_ICON_SIZE`² pixels, index = day of month − 1 (scripts/gen-day-icons.mjs).
static DAY_ICONS: [&[u8]; 31] = [
    include_bytes!("../icons/day/01.rgba"),
    include_bytes!("../icons/day/02.rgba"),
    include_bytes!("../icons/day/03.rgba"),
    include_bytes!("../icons/day/04.rgba"),
    include_bytes!("../icons/day/05.rgba"),
    include_bytes!("../icons/day/06.rgba"),
    include_bytes!("../icons/day/07.rgba"),
    include_bytes!("../icons/day/08.rgba"),
    include_bytes!("../icons/day/09.rgba"),
    include_bytes!("../icons/day/10.rgba"),
    include_bytes!("../icons/day/11.rgba"),
    include_bytes!("../icons/day/12.rgba"),
    include_bytes!("../icons/day/13.rgba"),
    include_bytes!("../icons/day/14.rgba"),
    include_bytes!("../icons/day/15.rgba"),
    include_bytes!("../icons/day/16.rgba"),
    include_bytes!("../icons/day/17.rgba"),
    include_bytes!("../icons/day/18.rgba"),
    include_bytes!("../icons/day/19.rgba"),
    include_bytes!("../icons/day/20.rgba"),
    include_bytes!("../icons/day/21.rgba"),
    include_bytes!("../icons/day/22.rgba"),
    include_bytes!("../icons/day/23.rgba"),
    include_bytes!("../icons/day/24.rgba"),
    include_bytes!("../icons/day/25.rgba"),
    include_bytes!("../icons/day/26.rgba"),
    include_bytes!("../icons/day/27.rgba"),
    include_bytes!("../icons/day/28.rgba"),
    include_bytes!("../icons/day/29.rgba"),
    include_bytes!("../icons/day/30.rgba"),
    include_bytes!("../icons/day/31.rgba"),
];

/// The same icons as PNG at the pixel sizes of the hicolor directories the .deb installs
/// (tauri.conf.json `bundle.icon`), copied verbatim into the user's icon theme so the .desktop
/// icon GNOME's dock shows follows the day too. A directory's files must have its declared
/// size: GNOME Shell scales by the declared size, not by the file's pixels.
static DAY_ICONS_PNG_32: [&[u8]; 31] = [
    include_bytes!("../icons/day/01-32.png"),
    include_bytes!("../icons/day/02-32.png"),
    include_bytes!("../icons/day/03-32.png"),
    include_bytes!("../icons/day/04-32.png"),
    include_bytes!("../icons/day/05-32.png"),
    include_bytes!("../icons/day/06-32.png"),
    include_bytes!("../icons/day/07-32.png"),
    include_bytes!("../icons/day/08-32.png"),
    include_bytes!("../icons/day/09-32.png"),
    include_bytes!("../icons/day/10-32.png"),
    include_bytes!("../icons/day/11-32.png"),
    include_bytes!("../icons/day/12-32.png"),
    include_bytes!("../icons/day/13-32.png"),
    include_bytes!("../icons/day/14-32.png"),
    include_bytes!("../icons/day/15-32.png"),
    include_bytes!("../icons/day/16-32.png"),
    include_bytes!("../icons/day/17-32.png"),
    include_bytes!("../icons/day/18-32.png"),
    include_bytes!("../icons/day/19-32.png"),
    include_bytes!("../icons/day/20-32.png"),
    include_bytes!("../icons/day/21-32.png"),
    include_bytes!("../icons/day/22-32.png"),
    include_bytes!("../icons/day/23-32.png"),
    include_bytes!("../icons/day/24-32.png"),
    include_bytes!("../icons/day/25-32.png"),
    include_bytes!("../icons/day/26-32.png"),
    include_bytes!("../icons/day/27-32.png"),
    include_bytes!("../icons/day/28-32.png"),
    include_bytes!("../icons/day/29-32.png"),
    include_bytes!("../icons/day/30-32.png"),
    include_bytes!("../icons/day/31-32.png"),
];
static DAY_ICONS_PNG_128: [&[u8]; 31] = [
    include_bytes!("../icons/day/01-128.png"),
    include_bytes!("../icons/day/02-128.png"),
    include_bytes!("../icons/day/03-128.png"),
    include_bytes!("../icons/day/04-128.png"),
    include_bytes!("../icons/day/05-128.png"),
    include_bytes!("../icons/day/06-128.png"),
    include_bytes!("../icons/day/07-128.png"),
    include_bytes!("../icons/day/08-128.png"),
    include_bytes!("../icons/day/09-128.png"),
    include_bytes!("../icons/day/10-128.png"),
    include_bytes!("../icons/day/11-128.png"),
    include_bytes!("../icons/day/12-128.png"),
    include_bytes!("../icons/day/13-128.png"),
    include_bytes!("../icons/day/14-128.png"),
    include_bytes!("../icons/day/15-128.png"),
    include_bytes!("../icons/day/16-128.png"),
    include_bytes!("../icons/day/17-128.png"),
    include_bytes!("../icons/day/18-128.png"),
    include_bytes!("../icons/day/19-128.png"),
    include_bytes!("../icons/day/20-128.png"),
    include_bytes!("../icons/day/21-128.png"),
    include_bytes!("../icons/day/22-128.png"),
    include_bytes!("../icons/day/23-128.png"),
    include_bytes!("../icons/day/24-128.png"),
    include_bytes!("../icons/day/25-128.png"),
    include_bytes!("../icons/day/26-128.png"),
    include_bytes!("../icons/day/27-128.png"),
    include_bytes!("../icons/day/28-128.png"),
    include_bytes!("../icons/day/29-128.png"),
    include_bytes!("../icons/day/30-128.png"),
    include_bytes!("../icons/day/31-128.png"),
];
static DAY_ICONS_PNG_512: [&[u8]; 31] = [
    include_bytes!("../icons/day/01-512.png"),
    include_bytes!("../icons/day/02-512.png"),
    include_bytes!("../icons/day/03-512.png"),
    include_bytes!("../icons/day/04-512.png"),
    include_bytes!("../icons/day/05-512.png"),
    include_bytes!("../icons/day/06-512.png"),
    include_bytes!("../icons/day/07-512.png"),
    include_bytes!("../icons/day/08-512.png"),
    include_bytes!("../icons/day/09-512.png"),
    include_bytes!("../icons/day/10-512.png"),
    include_bytes!("../icons/day/11-512.png"),
    include_bytes!("../icons/day/12-512.png"),
    include_bytes!("../icons/day/13-512.png"),
    include_bytes!("../icons/day/14-512.png"),
    include_bytes!("../icons/day/15-512.png"),
    include_bytes!("../icons/day/16-512.png"),
    include_bytes!("../icons/day/17-512.png"),
    include_bytes!("../icons/day/18-512.png"),
    include_bytes!("../icons/day/19-512.png"),
    include_bytes!("../icons/day/20-512.png"),
    include_bytes!("../icons/day/21-512.png"),
    include_bytes!("../icons/day/22-512.png"),
    include_bytes!("../icons/day/23-512.png"),
    include_bytes!("../icons/day/24-512.png"),
    include_bytes!("../icons/day/25-512.png"),
    include_bytes!("../icons/day/26-512.png"),
    include_bytes!("../icons/day/27-512.png"),
    include_bytes!("../icons/day/28-512.png"),
    include_bytes!("../icons/day/29-512.png"),
    include_bytes!("../icons/day/30-512.png"),
    include_bytes!("../icons/day/31-512.png"),
];
/// hicolor directory → the PNG set with its pixel size (`256x256@2` holds 512 px files).
const HICOLOR_DIRS: [(&str, &[&[u8]; 31]); 4] = [
    ("32x32", &DAY_ICONS_PNG_32),
    ("128x128", &DAY_ICONS_PNG_128),
    ("256x256@2", &DAY_ICONS_PNG_512),
    ("512x512", &DAY_ICONS_PNG_512),
];
const ICON_FILE: &str = "unified-google-calendar.png";

/// Writes today's PNGs into the user's hicolor theme (config::user_hicolor_dir). Errors are
/// logged: the tray and window icons work without it.
fn write_theme_icon(day: u32) {
    write_theme_icon_into(&crate::config::user_hicolor_dir(), day);
}

/// `write_theme_icon` with the theme directory as a parameter. After the PNGs it bumps the
/// mtime of `root`: GNOME Shell and GTK only rescan a theme when its root directory changes,
/// and overwriting a file in place leaves that mtime alone, so without this the dock kept
/// yesterday's number until the session restarted (user report of 2026-09-16).
fn write_theme_icon_into(root: &std::path::Path, day: u32) {
    let idx = day.clamp(1, 31) as usize - 1;
    for (size, icons) in HICOLOR_DIRS {
        let dir = root.join(size).join("apps");
        let result = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(dir.join(ICON_FILE), icons[idx]));
        if let Err(e) = result {
            tracing::debug!(error = %e, path = %dir.display(), "day icon not written to the icon theme");
        }
    }
    let touched =
        std::fs::File::open(root).and_then(|f| f.set_modified(std::time::SystemTime::now()));
    if let Err(e) = touched {
        tracing::debug!(error = %e, path = %root.display(), "icon theme directory not touched");
    }
}

/// The icon for a day of month (1–31); out-of-range days fall back to the 1st.
pub fn day_icon(day: u32) -> Image<'static> {
    let idx = day.clamp(1, 31) as usize - 1;
    Image::new(DAY_ICONS[idx], DAY_ICON_SIZE, DAY_ICON_SIZE)
}

/// Today's day of month in the primary time zone (settings `primary_tz`).
fn today_day(conn: &Connection, now: i64) -> Result<u32, AppError> {
    let tz: String = settings::get_or(
        conn,
        "primary_tz",
        crate::config::DEFAULT_PRIMARY_TZ.to_string(),
    )?;
    let tz: chrono_tz::Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    Ok(tz
        .timestamp_opt(now, 0)
        .single()
        .map(|d| d.day())
        .unwrap_or(1))
}

/// Draws `day` on the tray and the main window unless it is already there.
fn apply_day_icon(app: &AppHandle, day: u32) {
    if ICON_DAY.swap(day, Ordering::SeqCst) == day {
        return;
    }
    let icon = day_icon(day);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon.clone()));
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_icon(icon);
    }
    write_theme_icon(day);
}

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
    // Only today's events in the primary zone: the window ends at the next local midnight.
    let end_of_today = tz
        .timestamp_opt(now, 0)
        .single()
        .and_then(|d| (d.date_naive() + chrono::Duration::days(1)).and_hms_opt(0, 0, 0))
        .and_then(|m| tz.from_local_datetime(&m).earliest())
        .map(|m| m.timestamp())
        .unwrap_or(now + 86_400);
    let row = conn
        .query_row(
            "SELECT o.start_ts, e.summary FROM occurrences o \
             JOIN events e ON e.account_id=o.account_id AND e.calendar_id=o.calendar_id AND e.id=o.event_id \
             JOIN calendars c ON c.account_id=o.account_id AND c.id=o.calendar_id \
             WHERE o.all_day=0 AND o.status != 'cancelled' AND c.visible=1 AND c.deleted=0 \
               AND o.start_ts > ?1 AND o.start_ts < ?2 \
             ORDER BY o.start_ts LIMIT 1",
            params![now - GRACE_SECS, end_of_today],
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
    let now = crate::db::now_ts();
    let day = crate::db::handle()
        .and_then(|h| h.call_blocking(move |c| today_day(c, now)))
        .unwrap_or(1);
    builder = builder.icon(day_icon(day));
    builder.build(app).map_err(err)?;
    ICON_DAY.store(day, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_icon(day_icon(day));
    }
    write_theme_icon(day);
    Ok(())
}

/// The title of refresh number `tick`: the same text, with a zero-width space on odd ticks so
/// that consecutive values never compare equal (see the module comment).
pub fn tick_title(title: &str, tick: u32) -> String {
    if tick % 2 == 1 {
        format!("{title}{ZERO_WIDTH_SPACE}")
    } else {
        title.to_string()
    }
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
    match crate::db::call(move |c| today_day(c, now)).await {
        Ok(day) => apply_day_icon(app, day),
        Err(e) => tracing::debug!(error = %e, "day icon refresh failed"),
    }
    let tick = TICK.fetch_add(1, Ordering::SeqCst);
    match crate::db::call(move |c| next_event(c, now)).await {
        Ok(Some(n)) => {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_title(Some(tick_title(&n.title, tick)));
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
        refresh(&app).await;
        // GNOME's AppIndicator extension misses the first label at start and paints the next
        // distinct value (docs/06 section 2): send it after 5 s instead of after a minute.
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
    fn theme_icon_rewrite_bumps_the_theme_root_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("hicolor");
        write_theme_icon_into(&root, 15);
        let file = root.join("128x128").join("apps").join(ICON_FILE);
        assert_eq!(std::fs::read(&file).unwrap(), DAY_ICONS_PNG_128[14]);

        // Pretend the last write was long ago, as at midnight after a day of use.
        let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::open(&root)
            .unwrap()
            .set_modified(past)
            .unwrap();
        write_theme_icon_into(&root, 16);
        assert_eq!(std::fs::read(&file).unwrap(), DAY_ICONS_PNG_128[15]);
        let mtime = std::fs::metadata(&root).unwrap().modified().unwrap();
        assert!(
            mtime > past,
            "the theme root must change so GNOME Shell rescans it"
        );
    }

    #[test]
    fn tick_title_alternates_an_invisible_suffix() {
        let a = tick_title("10:30 Daily Team", 0);
        let b = tick_title("10:30 Daily Team", 1);
        assert_eq!(a, "10:30 Daily Team");
        assert_ne!(a, b);
        assert_eq!(b.trim_end_matches(ZERO_WIDTH_SPACE), a);
        assert_eq!(tick_title("10:30 Daily Team", 2), a);
    }

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
            mk("tomorrow", now + 16 * 3600, "Tomorrow", false),
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
        // A started event holds the title for GRACE_SECS, then the next one takes over.
        let started = next_event(&conn, now + 1800 + GRACE_SECS - 1)
            .unwrap()
            .unwrap();
        assert!(started.title.starts_with("09:30 "));
        let after = next_event(&conn, now + 1800 + GRACE_SECS).unwrap().unwrap();
        assert_eq!(after.title, "10:00 Later");
        // After today's last event nothing shows, even with an event tomorrow.
        assert!(next_event(&conn, now + 3600 + GRACE_SECS)
            .unwrap()
            .is_none());
        crate::db::queries::calendars::set_visible(&conn, "local", "local-personal", false)
            .unwrap();
        assert!(next_event(&conn, now).unwrap().is_none());
        assert_eq!(truncate("short", 32), "short");
    }
}
