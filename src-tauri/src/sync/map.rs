//! Google `Event` / `CalendarListEntry` → local rows. See docs/03-modelo-de-datos.md section 2.

use chrono::{DateTime, NaiveDate};
use serde_json::Value;

use crate::db::queries::calendars::CalendarRow;
use crate::db::queries::events::EventRow;
use crate::google::types::{CalendarListEntry, Event, EventDateTime};

pub fn parse_rfc3339(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp())
}

fn valid_date(s: &str) -> Option<String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .map(|d| d.format("%Y-%m-%d").to_string())
}

/// Start/end of an event as stored in `events`.
#[derive(Debug, Default)]
struct Span {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    all_day: bool,
    time_zone: Option<String>,
}

fn span(start: Option<&EventDateTime>, end: Option<&EventDateTime>) -> Span {
    let tz = start
        .and_then(|s| s.time_zone.clone())
        .or_else(|| end.and_then(|e| e.time_zone.clone()));
    if let Some(date) = start.and_then(|s| s.date.as_deref()).and_then(valid_date) {
        let end_date = end
            .and_then(|e| e.date.as_deref())
            .and_then(valid_date)
            .or_else(|| {
                NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok().map(|d| {
                    (d + chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string()
                })
            });
        return Span {
            start_date: Some(date),
            end_date,
            all_day: true,
            time_zone: tz,
            ..Default::default()
        };
    }
    let s = start
        .and_then(|s| s.date_time.as_deref())
        .and_then(parse_rfc3339);
    let e = end
        .and_then(|e| e.date_time.as_deref())
        .and_then(parse_rfc3339)
        .or(s);
    Span {
        start_ts: s,
        end_ts: e,
        time_zone: tz,
        ..Default::default()
    }
}

/// Map one Google event into an `events` row. `calendar_tz` fills a missing `start.timeZone`.
pub fn event_to_row(
    account_id: &str,
    calendar_id: &str,
    ev: &Event,
    calendar_tz: Option<&str>,
) -> EventRow {
    let Span {
        start_ts,
        end_ts,
        start_date,
        end_date,
        all_day,
        time_zone: tz,
    } = span(ev.start.as_ref(), ev.end.as_ref());
    let orig = span(ev.original_start_time.as_ref(), None);
    let (orig_ts, orig_date) = (orig.start_ts, orig.start_date);
    let attendees = ev
        .attendees
        .as_ref()
        .and_then(|a| serde_json::to_string(a).ok())
        .unwrap_or_else(|| "[]".into());
    let reminders = ev
        .reminders
        .as_ref()
        .filter(|r| r.is_object())
        .and_then(|r| serde_json::to_string(r).ok())
        .unwrap_or_else(|| r#"{"useDefault":true}"#.into());
    let raw = serde_json::to_string(ev).ok();
    EventRow {
        account_id: account_id.into(),
        calendar_id: calendar_id.into(),
        id: ev.id.clone().unwrap_or_default(),
        ical_uid: ev.ical_uid.clone(),
        etag: ev.etag.clone(),
        status: ev.status.clone().unwrap_or_else(|| "confirmed".into()),
        summary: ev.summary.clone(),
        description: ev.description.clone(),
        location: ev.location.clone(),
        color_id: ev.color_id.clone(),
        start_ts,
        end_ts,
        start_date,
        end_date,
        all_day,
        time_zone: tz.or_else(|| {
            if all_day {
                None
            } else {
                calendar_tz.map(str::to_string)
            }
        }),
        recurrence: ev.recurrence.clone().filter(|r| !r.is_empty()),
        recurring_event_id: ev.recurring_event_id.clone(),
        original_start_ts: orig_ts,
        original_start_date: orig_date,
        organizer_email: ev.organizer.as_ref().and_then(|o| o.email.clone()),
        organizer_self: ev
            .organizer
            .as_ref()
            .and_then(|o| o.is_self)
            .unwrap_or(false),
        attendees,
        reminders,
        hangout_link: ev.hangout_link.clone(),
        conference: ev
            .conference_data
            .as_ref()
            .filter(|c| !c.is_null())
            .and_then(|c| serde_json::to_string(c).ok()),
        html_link: ev.html_link.clone(),
        transparency: ev.transparency.clone(),
        visibility: ev.visibility.clone(),
        event_type: ev.event_type.clone().unwrap_or_else(|| "default".into()),
        guests_can_modify: ev.guests_can_modify.unwrap_or(false),
        created_ts: ev.created.as_deref().and_then(parse_rfc3339),
        updated_ts: ev.updated.as_deref().and_then(parse_rfc3339),
        raw,
    }
}

/// Map a calendarList entry. `existing` keeps `visible`, `sync_token` and `full_sync_done`.
pub fn calendar_to_row(
    account_id: &str,
    entry: &CalendarListEntry,
    existing: Option<&CalendarRow>,
    sort_order: i64,
) -> CalendarRow {
    let summary = entry
        .summary_override
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| entry.summary.clone())
        .unwrap_or_else(|| entry.id.clone());
    let default_reminders = entry
        .default_reminders
        .as_ref()
        .and_then(|r| serde_json::to_string(r).ok())
        .unwrap_or_else(|| "[]".into());
    CalendarRow {
        id: entry.id.clone(),
        account_id: account_id.into(),
        summary,
        description: entry.description.clone(),
        color_bg: entry
            .background_color
            .clone()
            .unwrap_or_else(|| "#4285f4".into()),
        color_fg: entry
            .foreground_color
            .clone()
            .unwrap_or_else(|| "#000000".into()),
        access_role: entry.access_role.clone().unwrap_or_else(|| "reader".into()),
        is_primary: entry.primary.unwrap_or(false),
        // `selected` only on first import; afterwards the app checkbox wins (docs/03 section 2).
        visible: existing
            .map(|e| e.visible)
            .unwrap_or_else(|| entry.selected.unwrap_or(false)),
        hidden_remote: entry.hidden.unwrap_or(false),
        deleted: entry.deleted.unwrap_or(false),
        time_zone: entry.time_zone.clone(),
        default_reminders,
        sync_token: existing.and_then(|e| e.sync_token.clone()),
        full_sync_done: existing.map(|e| e.full_sync_done).unwrap_or(false),
        sort_order,
    }
}

/// Convenience for tests and the write path: a `Value` into an `Event`.
pub fn event_from_value(v: &Value) -> Option<Event> {
    serde_json::from_value(v.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_timed_all_day_exception_and_cancelled() {
        let timed: Event = serde_json::from_str(r#"{"id":"a","status":"confirmed","start":{"dateTime":"2026-09-14T13:00:00-03:00","timeZone":"America/Argentina/Buenos_Aires"},"end":{"dateTime":"2026-09-14T14:00:00-03:00"},"organizer":{"email":"o@x","self":true},"attendees":[{"email":"o@x","self":true,"responseStatus":"accepted"}],"reminders":{"useDefault":false,"overrides":[{"method":"popup","minutes":5}]},"created":"2026-09-01T10:00:00.000Z","extendedProperties":{"private":{"k":"v"}}}"#).unwrap();
        let r = event_to_row("acc", "cal", &timed, Some("UTC"));
        assert_eq!(r.start_ts, Some(1_789_401_600));
        assert_eq!(r.end_ts, Some(1_789_405_200));
        assert_eq!(
            r.time_zone.as_deref(),
            Some("America/Argentina/Buenos_Aires")
        );
        assert!(r.organizer_self && !r.all_day);
        assert!(r.attendees.contains("accepted"));
        assert!(r.reminders.contains("overrides"));
        assert_eq!(r.created_ts, Some(1_788_256_800));
        assert!(r.raw.as_deref().unwrap().contains("extendedProperties"));

        let all_day: Event = serde_json::from_str(
            r#"{"id":"b","start":{"date":"2026-09-14"},"end":{"date":"2026-09-16"}}"#,
        )
        .unwrap();
        let r = event_to_row("acc", "cal", &all_day, Some("UTC"));
        assert!(r.all_day);
        assert_eq!(r.start_date.as_deref(), Some("2026-09-14"));
        assert_eq!(r.end_date.as_deref(), Some("2026-09-16"));
        assert!(r.time_zone.is_none());
        assert_eq!(r.status, "confirmed");

        let exc: Event = serde_json::from_str(r#"{"id":"m_20260925T130000Z","status":"cancelled","recurringEventId":"m","originalStartTime":{"dateTime":"2026-09-25T10:00:00-03:00"}}"#).unwrap();
        let r = event_to_row("acc", "cal", &exc, Some("UTC"));
        assert_eq!(r.recurring_event_id.as_deref(), Some("m"));
        assert_eq!(r.original_start_ts, Some(1_790_341_200));
        assert!(r.is_cancelled() && r.is_exception());

        let no_tz: Event = serde_json::from_str(r#"{"id":"c","start":{"dateTime":"2026-09-14T13:00:00Z"},"end":{"dateTime":"2026-09-14T14:00:00Z"}}"#).unwrap();
        let r = event_to_row("acc", "cal", &no_tz, Some("Europe/Madrid"));
        assert_eq!(r.time_zone.as_deref(), Some("Europe/Madrid"));
    }

    #[test]
    fn calendar_visible_only_on_first_import() {
        let entry: CalendarListEntry = serde_json::from_str(r##"{"id":"x","summary":"X","summaryOverride":"Mine","backgroundColor":"#123456","foregroundColor":"#ffffff","accessRole":"writer","selected":true,"defaultReminders":[{"method":"popup","minutes":10}]}"##).unwrap();
        let first = calendar_to_row("acc", &entry, None, 3);
        assert!(first.visible);
        assert_eq!(first.summary, "Mine");
        assert_eq!(first.sort_order, 3);
        let existing = CalendarRow {
            visible: false,
            sync_token: Some("t".into()),
            full_sync_done: true,
            ..first.clone()
        };
        let again = calendar_to_row("acc", &entry, Some(&existing), 3);
        assert!(!again.visible);
        assert_eq!(again.sync_token.as_deref(), Some("t"));
        assert!(again.full_sync_done);
    }
}
