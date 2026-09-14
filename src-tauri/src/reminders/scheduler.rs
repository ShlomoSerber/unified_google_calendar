//! Reminder scheduler. See docs/03-modelo-de-datos.md section 8 and docs/06 section 1.
//!
//! Every 60 s: occurrences starting within 24 h, `fire_at = start - minutes*60` for every
//! `popup` reminder (overrides, or the calendar defaults when `useDefault`). Reminders whose
//! `fire_at` falls in the next minute and were not fired yet are shown. Fired keys persist in
//! `settings.fired_reminders` with a 48 h expiry, so a restart never repeats them. Snoozes
//! live in memory.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::TimeZone;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::commands::view::{meet_link, parse_default_reminders, parse_reminders};
use crate::db::queries::settings;
use crate::error::AppError;

pub const TICK_SECS: u64 = 60;
pub const LOOKAHEAD_SECS: i64 = 24 * 3600;
/// A reminder missed by up to this much (e.g. after sleep) still fires.
pub const GRACE_SECS: i64 = 600;
pub const FIRED_TTL_SECS: i64 = 48 * 3600;
pub const SNOOZE_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fired {
    pub key: String,
    pub at: i64,
}

/// One reminder to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// `occurrence_id|minutes`.
    pub key: String,
    pub occurrence_id: String,
    pub title: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub all_day: bool,
    pub account_name: String,
    pub meet_link: Option<String>,
    pub minutes: i64,
    pub fire_at: i64,
}

static SNOOZED: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn snoozed() -> &'static Mutex<HashMap<String, i64>> {
    SNOOZED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Re-schedule a reminder `SNOOZE_SECS` from now (docs/06 section 1, action `snooze`).
pub fn snooze(key: &str, now: i64) {
    if let Ok(mut m) = snoozed().lock() {
        m.insert(key.to_string(), now + SNOOZE_SECS);
    }
}

pub fn snoozed_snapshot() -> HashMap<String, i64> {
    snoozed().lock().map(|m| m.clone()).unwrap_or_default()
}

fn take_snooze(key: &str) {
    if let Ok(mut m) = snoozed().lock() {
        m.remove(key);
    }
}

pub fn load_fired(conn: &Connection, now: i64) -> Result<Vec<Fired>, AppError> {
    let list: Vec<Fired> = settings::get_or(conn, "fired_reminders", Vec::new())?;
    Ok(list
        .into_iter()
        .filter(|f| now - f.at < FIRED_TTL_SECS)
        .collect())
}

pub fn mark_fired(conn: &Connection, keys: &[String], now: i64) -> Result<(), AppError> {
    let mut list = load_fired(conn, now)?;
    for k in keys {
        if !list.iter().any(|f| &f.key == k) {
            list.push(Fired {
                key: k.clone(),
                at: now,
            });
        }
    }
    settings::set(conn, "fired_reminders", &list)
}

struct Candidate {
    occurrence_id: String,
    title: Option<String>,
    start_ts: i64,
    end_ts: i64,
    all_day: bool,
    start_date: Option<String>,
    reminders: String,
    default_reminders: String,
    account_name: String,
    hangout_link: Option<String>,
    conference: Option<String>,
}

/// Reminders due in the next minute (plus the grace period), excluding fired ones.
pub fn compute_due(
    conn: &Connection,
    now: i64,
    snoozes: &HashMap<String, i64>,
) -> Result<Vec<Due>, AppError> {
    let primary_tz: String = settings::get_or(
        conn,
        "primary_tz",
        crate::config::DEFAULT_PRIMARY_TZ.to_string(),
    )?;
    let tz: chrono_tz::Tz = primary_tz.parse().unwrap_or(chrono_tz::UTC);
    let fired = load_fired(conn, now)?;
    let mut stmt = conn.prepare(
        "SELECT o.id, e.summary, o.start_ts, o.end_ts, o.all_day, e.start_date, e.reminders, c.default_reminders, a.display_name, e.hangout_link, e.conference \
         FROM occurrences o \
         JOIN events e ON e.account_id=o.account_id AND e.calendar_id=o.calendar_id AND e.id=o.event_id \
         JOIN calendars c ON c.account_id=o.account_id AND c.id=o.calendar_id \
         JOIN accounts a ON a.id=o.account_id \
         WHERE o.status != 'cancelled' AND c.deleted=0 AND o.start_ts BETWEEN ?1 AND ?2",
    )?;
    let rows = stmt
        .query_map(
            params![
                now - GRACE_SECS - 40_320 * 60,
                now + LOOKAHEAD_SECS + 86_400
            ],
            |r| {
                Ok(Candidate {
                    occurrence_id: r.get(0)?,
                    title: r.get(1)?,
                    start_ts: r.get(2)?,
                    end_ts: r.get(3)?,
                    all_day: r.get::<_, i64>(4)? != 0,
                    start_date: r.get(5)?,
                    reminders: r.get(6)?,
                    default_reminders: r.get(7)?,
                    account_name: r.get(8)?,
                    hangout_link: r.get(9)?,
                    conference: r.get(10)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut due = Vec::new();
    for c in rows {
        let (use_default, overrides) = parse_reminders(&c.reminders);
        let list = if use_default {
            parse_default_reminders(&c.default_reminders)
        } else {
            overrides
        };
        // All-day reminders count from local midnight of the start date (docs/03 section 8).
        let base = if c.all_day {
            c.start_date
                .as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .and_then(|n| tz.from_local_datetime(&n).earliest())
                .map(|d| d.timestamp())
                .unwrap_or(c.start_ts)
        } else {
            c.start_ts
        };
        for r in list.into_iter().filter(|r| r.method == "popup") {
            let key = format!("{}|{}", c.occurrence_id, r.minutes);
            let fire_at = match snoozes.get(&key) {
                Some(t) => *t,
                None => base - r.minutes * 60,
            };
            let in_window = fire_at <= now + TICK_SECS as i64 && fire_at > now - GRACE_SECS;
            let already = snoozes.get(&key).is_none() && fired.iter().any(|f| f.key == key);
            if in_window && !already {
                due.push(Due {
                    key,
                    occurrence_id: c.occurrence_id.clone(),
                    title: c.title.clone().unwrap_or_else(|| "(No title)".into()),
                    start_ts: c.start_ts,
                    end_ts: c.end_ts,
                    all_day: c.all_day,
                    account_name: c.account_name.clone(),
                    meet_link: meet_link(c.hangout_link.as_deref(), c.conference.as_deref()),
                    minutes: r.minutes,
                    fire_at,
                });
            }
        }
    }
    due.sort_by_key(|d| (d.fire_at, d.key.clone()));
    Ok(due)
}

/// One scheduler tick: compute, mark as fired, return what to show.
pub fn tick(conn: &Connection, now: i64) -> Result<Vec<Due>, AppError> {
    let snoozes = snoozed_snapshot();
    let due = compute_due(conn, now, &snoozes)?;
    if !due.is_empty() {
        let keys: Vec<String> = due.iter().map(|d| d.key.clone()).collect();
        mark_fired(conn, &keys, now)?;
        for k in &keys {
            take_snooze(k);
        }
    }
    Ok(due)
}

/// `reminders::start` per docs/08 section 12.
pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(TICK_SECS));
        loop {
            interval.tick().await;
            let now = crate::db::now_ts();
            let due = match crate::db::call(move |c| tick(c, now)).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "reminder tick failed");
                    continue;
                }
            };
            for d in due {
                crate::reminders::notify::show(app.clone(), d);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::events::EventRow;
    use crate::recurrence::{expand::Window, materialize_simple};

    fn seed(conn: &mut Connection, id: &str, start: i64, reminders: &str, meet: bool) {
        let e = EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: id.into(),
            status: "confirmed".into(),
            summary: Some(format!("Event {id}")),
            start_ts: Some(start),
            end_ts: Some(start + 1800),
            all_day: false,
            attendees: "[]".into(),
            reminders: reminders.into(),
            hangout_link: if meet {
                Some("https://meet.google.com/x".into())
            } else {
                None
            },
            event_type: "default".into(),
            ..Default::default()
        };
        crate::db::queries::events::upsert_event(conn, &e).unwrap();
        materialize_simple(
            conn,
            "local",
            "local-personal",
            id,
            Window {
                from_ts: 0,
                to_ts: i64::MAX / 2,
            },
        )
        .unwrap();
    }

    #[test]
    fn fires_once_uses_defaults_and_honours_snooze() {
        let mut conn = crate::db::open_memory().unwrap();
        let now = 1_789_400_000;
        // Default reminder (10 min) → fires when start is 10 minutes away.
        seed(&mut conn, "a", now + 600, r#"{"useDefault":true}"#, true);
        // Explicit 30 and 5 minute overrides on an event 30 minutes away: only the 30 fires now.
        seed(
            &mut conn,
            "b",
            now + 1800,
            r#"{"useDefault":false,"overrides":[{"method":"popup","minutes":30},{"method":"popup","minutes":5},{"method":"email","minutes":30}]}"#,
            false,
        );
        // Too far away.
        seed(&mut conn, "c", now + 7200, r#"{"useDefault":true}"#, false);
        let due = tick(&conn, now).unwrap();
        let keys: Vec<&str> = due.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys[0].starts_with("local|local-personal|a|") && keys[0].ends_with("|10"));
        assert!(keys[1].ends_with("|30"));
        assert_eq!(due[0].title, "Event a");
        assert_eq!(
            due[0].meet_link.as_deref(),
            Some("https://meet.google.com/x")
        );
        assert_eq!(due[0].account_name, "This computer");
        // Same minute again: nothing (persisted).
        assert!(tick(&conn, now + 10).unwrap().is_empty());
        // Snooze the first one: fires again 5 minutes later.
        let key = due[0].key.clone();
        snooze(&key, now);
        assert!(tick(&conn, now + 100).unwrap().is_empty());
        let again = tick(&conn, now + SNOOZE_SECS).unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].key, key);
        // 25 minutes later the 5-minute reminder of b fires.
        let later = tick(&conn, now + 1500).unwrap();
        assert_eq!(later.len(), 1);
        assert!(later[0].key.ends_with("|5"));
        // Fired list expires after 48 h.
        assert!(load_fired(&conn, now + 1500 + FIRED_TTL_SECS + 1)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn all_day_reminder_counts_from_local_midnight() {
        let mut conn = crate::db::open_memory().unwrap();
        // 2026-09-15 local midnight in Buenos Aires = 03:00Z.
        let e = EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: "ad".into(),
            status: "confirmed".into(),
            summary: Some("Holiday".into()),
            all_day: true,
            start_date: Some("2026-09-15".into()),
            end_date: Some("2026-09-16".into()),
            attendees: "[]".into(),
            reminders: r#"{"useDefault":false,"overrides":[{"method":"popup","minutes":60}]}"#
                .into(),
            event_type: "default".into(),
            ..Default::default()
        };
        crate::db::queries::events::upsert_event(&conn, &e).unwrap();
        materialize_simple(
            &mut conn,
            "local",
            "local-personal",
            "ad",
            Window {
                from_ts: 0,
                to_ts: i64::MAX / 2,
            },
        )
        .unwrap();
        let midnight_ba = chrono::DateTime::parse_from_rfc3339("2026-09-15T00:00:00-03:00")
            .unwrap()
            .timestamp();
        let due = compute_due(&conn, midnight_ba - 3600, &HashMap::new()).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].fire_at, midnight_ba - 3600);
        assert!(compute_due(&conn, midnight_ba - 7200, &HashMap::new())
            .unwrap()
            .is_empty());
    }
}
