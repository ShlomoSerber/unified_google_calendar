//! Request bodies for `events.insert` / `events.update` / `events.patch`.
//! See docs/05-sincronizacion.md section 2.3 and docs/03-modelo-de-datos.md section 4.
//!
//! `update` starts from `events.raw`, applies the draft and sends the whole object so unknown
//! fields survive. Recurring events always carry `start.timeZone` / `end.timeZone`.

use chrono::{DateTime, TimeZone};
use serde_json::{json, Map, Value};

use crate::commands::types::EventDraft;
use crate::error::AppError;
use crate::google::types::{Event, SendUpdates};

/// Fields Google owns or that must never be sent back on update.
const READ_ONLY: &[&str] = &[
    "kind",
    "etag",
    "htmlLink",
    "created",
    "updated",
    "creator",
    "iCalUID",
    "sequence",
    "hangoutLink",
];

fn rfc3339_in(ts: i64, tz: &str) -> Result<String, AppError> {
    let zone: chrono_tz::Tz = tz
        .parse()
        .map_err(|_| AppError::invalid(format!("{tz} is not a valid time zone name")))?;
    let dt = zone
        .timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| AppError::invalid("Invalid start or end time"))?;
    Ok(dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
}

/// `start`/`end` objects of a draft.
pub fn span_objects(draft: &EventDraft) -> Result<(Value, Value), AppError> {
    if draft.all_day {
        let s = draft
            .start_date
            .as_deref()
            .ok_or_else(|| AppError::invalid("An all-day event needs a start date"))?;
        let e = draft
            .end_date
            .as_deref()
            .ok_or_else(|| AppError::invalid("An all-day event needs an end date"))?;
        return Ok((json!({ "date": s }), json!({ "date": e })));
    }
    let tz = draft
        .time_zone
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| crate::config::DEFAULT_PRIMARY_TZ.to_string());
    let s = draft
        .start
        .ok_or_else(|| AppError::invalid("The event needs a start time"))?;
    let e = draft
        .end
        .ok_or_else(|| AppError::invalid("The event needs an end time"))?;
    Ok((
        json!({ "dateTime": rfc3339_in(s, &tz)?, "timeZone": tz }),
        json!({ "dateTime": rfc3339_in(e, &tz)?, "timeZone": tz }),
    ))
}

fn set_or_remove(obj: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(v) => {
            obj.insert(key.into(), v);
        }
        None => {
            obj.remove(key);
        }
    }
}

/// Merge the attendee list of a draft with the existing one so response statuses survive.
fn attendees_value(draft: &EventDraft, existing: Option<&Value>) -> Option<Value> {
    let wanted: Vec<String> = draft
        .attendees
        .iter()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();
    if wanted.is_empty() {
        return None;
    }
    let existing: Vec<Value> = existing
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for email in &wanted {
        match existing.iter().find(|a| {
            a.get("email")
                .and_then(Value::as_str)
                .map(|e| e.to_ascii_lowercase())
                .as_deref()
                == Some(email.as_str())
        }) {
            Some(a) => out.push(a.clone()),
            None => out.push(json!({ "email": email, "responseStatus": "needsAction" })),
        }
    }
    // Keep the organizer/self entries Google added even if the user's list omitted them.
    for a in existing.iter().filter(|a| {
        a.get("organizer").and_then(Value::as_bool) == Some(true)
            || a.get("self").and_then(Value::as_bool) == Some(true)
    }) {
        let email = a
            .get("email")
            .and_then(Value::as_str)
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !wanted.contains(&email) {
            out.push(a.clone());
        }
    }
    Some(Value::Array(out))
}

/// Build the body for insert (`raw = None`) or update (`raw = Some(events.raw)`).
/// `recurrence` overrides the draft's lines when given (used by the scope logic).
pub fn from_draft(
    draft: &EventDraft,
    raw: Option<&str>,
    recurrence: Option<&[String]>,
) -> Result<Event, AppError> {
    let mut obj: Map<String, Value> = match raw {
        Some(r) => serde_json::from_str::<Value>(r)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default(),
        None => Map::new(),
    };
    for k in READ_ONLY {
        obj.remove(*k);
    }
    let title = draft
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| Value::String(t.into()));
    set_or_remove(&mut obj, "summary", title);
    set_or_remove(
        &mut obj,
        "description",
        draft
            .description
            .clone()
            .filter(|d| !d.is_empty())
            .map(Value::String),
    );
    set_or_remove(
        &mut obj,
        "location",
        draft
            .location
            .clone()
            .filter(|l| !l.is_empty())
            .map(Value::String),
    );
    set_or_remove(
        &mut obj,
        "colorId",
        draft
            .color_id
            .clone()
            .filter(|c| !c.is_empty())
            .map(Value::String),
    );
    set_or_remove(
        &mut obj,
        "transparency",
        draft.transparency.clone().map(Value::String),
    );
    set_or_remove(
        &mut obj,
        "visibility",
        draft.visibility.clone().map(Value::String),
    );
    let (start, end) = span_objects(draft)?;
    obj.insert("start".into(), start);
    obj.insert("end".into(), end);
    let lines: Vec<String> = match recurrence {
        Some(r) => r.to_vec(),
        None => draft.recurrence.clone(),
    };
    set_or_remove(
        &mut obj,
        "recurrence",
        if lines.is_empty() {
            None
        } else {
            Some(json!(lines))
        },
    );
    let existing_attendees = obj.get("attendees").cloned();
    set_or_remove(
        &mut obj,
        "attendees",
        attendees_value(draft, existing_attendees.as_ref()),
    );
    let reminders = match &draft.reminders {
        None => json!({ "useDefault": true }),
        Some(list) => json!({ "useDefault": false, "overrides": list }),
    };
    obj.insert("reminders".into(), reminders);
    if draft.add_meet
        && obj
            .get("conferenceData")
            .and_then(|c| c.get("entryPoints"))
            .is_none()
    {
        obj.insert(
            "conferenceData".into(),
            json!({ "createRequest": { "requestId": uuid::Uuid::new_v4().to_string(), "conferenceSolutionKey": { "type": "hangoutsMeet" } } }),
        );
    }
    serde_json::from_value(Value::Object(obj))
        .map_err(|e| AppError::invalid(format!("could not build the event body ({e})")))
}

/// Body for the truncated master of "this and following" / delete following: `raw` with its
/// `recurrence` replaced.
pub fn with_recurrence(raw: &str, recurrence: &[String]) -> Result<Event, AppError> {
    let mut obj: Map<String, Value> = serde_json::from_str::<Value>(raw)?
        .as_object()
        .cloned()
        .unwrap_or_default();
    for k in READ_ONLY {
        obj.remove(*k);
    }
    obj.insert("recurrence".into(), json!(recurrence));
    serde_json::from_value(Value::Object(obj))
        .map_err(|e| AppError::invalid(format!("could not build the event body ({e})")))
}

/// Body for `events.import` in another account: `raw` without ids and links, keeping `iCalUID`.
pub fn for_import(raw: &str, ical_uid: &str) -> Result<Event, AppError> {
    let mut obj: Map<String, Value> = serde_json::from_str::<Value>(raw)?
        .as_object()
        .cloned()
        .unwrap_or_default();
    for k in [
        "kind",
        "etag",
        "id",
        "htmlLink",
        "created",
        "updated",
        "creator",
        "organizer",
        "sequence",
        "hangoutLink",
        "recurringEventId",
        "originalStartTime",
    ] {
        obj.remove(k);
    }
    obj.insert("iCalUID".into(), Value::String(ical_uid.into()));
    serde_json::from_value(Value::Object(obj))
        .map_err(|e| AppError::invalid(format!("could not build the import body ({e})")))
}

/// Shift `start`/`end` of `raw` by `delta` seconds and set the duration (edit "all").
pub fn shifted(
    raw: &str,
    delta: i64,
    duration: i64,
    all_day: bool,
) -> Result<Map<String, Value>, AppError> {
    let mut obj: Map<String, Value> = serde_json::from_str::<Value>(raw)?
        .as_object()
        .cloned()
        .unwrap_or_default();
    let start = obj.get("start").cloned().unwrap_or(Value::Null);
    if all_day {
        let d = start
            .get("date")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid("The series has no start date"))?;
        let sd = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .map_err(|_| AppError::invalid("Bad start date"))?;
        let ns = sd + chrono::Duration::seconds(delta);
        let ne = ns + chrono::Duration::seconds(duration.max(86_400));
        obj.insert(
            "start".into(),
            json!({ "date": ns.format("%Y-%m-%d").to_string() }),
        );
        obj.insert(
            "end".into(),
            json!({ "date": ne.format("%Y-%m-%d").to_string() }),
        );
    } else {
        let dt = start
            .get("dateTime")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid("The series has no start time"))?;
        let tz = start
            .get("timeZone")
            .and_then(Value::as_str)
            .unwrap_or(crate::config::DEFAULT_PRIMARY_TZ)
            .to_string();
        let s = DateTime::parse_from_rfc3339(dt)
            .map_err(|_| AppError::invalid("Bad start time"))?
            .timestamp();
        obj.insert(
            "start".into(),
            json!({ "dateTime": rfc3339_in(s + delta, &tz)?, "timeZone": tz }),
        );
        obj.insert(
            "end".into(),
            json!({ "dateTime": rfc3339_in(s + delta + duration, &tz)?, "timeZone": tz }),
        );
    }
    Ok(obj)
}

/// `sendUpdates` rule of docs/05 section 2.3: `all` when the event has guests and the user is
/// the organizer, `none` otherwise.
pub fn send_updates_for(has_attendees: bool, organizer_self: bool) -> SendUpdates {
    if has_attendees && organizer_self {
        SendUpdates::All
    } else {
        SendUpdates::None
    }
}

/// `attendees` array for an RSVP: the full array from `raw` with the `self` entry changed.
pub fn rsvp_attendees(raw: &str, status: &str) -> Result<Value, AppError> {
    let v: Value = serde_json::from_str(raw)?;
    let mut list = v
        .get("attendees")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| AppError::invalid("This event has no guest list to respond to"))?;
    let mut found = false;
    for a in list.iter_mut() {
        if a.get("self").and_then(Value::as_bool) == Some(true) {
            if let Some(o) = a.as_object_mut() {
                o.insert("responseStatus".into(), Value::String(status.into()));
                found = true;
            }
        }
    }
    if !found {
        return Err(AppError::invalid("You are not a guest of this event"));
    }
    Ok(Value::Array(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> EventDraft {
        EventDraft {
            account_id: "acc".into(),
            calendar_id: "primary".into(),
            title: Some("Sync".into()),
            description: Some("desc".into()),
            location: None,
            all_day: false,
            start: Some(1_789_401_600),
            end: Some(1_789_405_200),
            start_date: None,
            end_date: None,
            time_zone: Some("America/Argentina/Buenos_Aires".into()),
            recurrence: vec!["RRULE:FREQ=WEEKLY".into()],
            attendees: vec!["Guest@Example.com".into()],
            reminders: None,
            color_id: Some("10".into()),
            add_meet: true,
            transparency: None,
            visibility: None,
        }
    }

    #[test]
    fn insert_body_from_draft() {
        let e = from_draft(&draft(), None, None).unwrap();
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["summary"], "Sync");
        assert_eq!(v["start"]["dateTime"], "2026-09-14T13:00:00-03:00");
        assert_eq!(v["start"]["timeZone"], "America/Argentina/Buenos_Aires");
        assert_eq!(v["end"]["dateTime"], "2026-09-14T14:00:00-03:00");
        assert_eq!(v["recurrence"][0], "RRULE:FREQ=WEEKLY");
        assert_eq!(v["attendees"][0]["email"], "guest@example.com");
        assert_eq!(v["attendees"][0]["responseStatus"], "needsAction");
        assert_eq!(v["reminders"]["useDefault"], true);
        assert_eq!(v["colorId"], "10");
        assert_eq!(
            v["conferenceData"]["createRequest"]["conferenceSolutionKey"]["type"],
            "hangoutsMeet"
        );
        assert!(
            v["conferenceData"]["createRequest"]["requestId"]
                .as_str()
                .unwrap()
                .len()
                > 10
        );
        assert!(v.get("id").is_none());
    }

    #[test]
    fn update_body_keeps_unknown_fields_and_statuses() {
        let raw = r#"{"kind":"calendar#event","id":"e1","etag":"x","iCalUID":"e1@google.com","summary":"Old","extendedProperties":{"private":{"k":"v"}},"attendees":[{"email":"guest@example.com","responseStatus":"accepted"},{"email":"me@example.com","self":true,"organizer":true,"responseStatus":"accepted"}],"conferenceData":{"entryPoints":[{"entryPointType":"video","uri":"https://meet/x"}]},"start":{"dateTime":"2026-09-14T10:00:00-03:00"},"end":{"dateTime":"2026-09-14T11:00:00-03:00"},"hangoutLink":"https://meet/x","sequence":2}"#;
        let mut d = draft();
        d.recurrence = vec![];
        d.add_meet = false;
        let e = from_draft(&d, Some(raw), None).unwrap();
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["id"], "e1");
        assert_eq!(v["extendedProperties"]["private"]["k"], "v");
        assert!(
            v.get("etag").is_none()
                && v.get("iCalUID").is_none()
                && v.get("hangoutLink").is_none()
                && v.get("sequence").is_none()
        );
        assert_eq!(v["summary"], "Sync");
        assert!(v.get("recurrence").is_none());
        let att = v["attendees"].as_array().unwrap();
        assert_eq!(att.len(), 2);
        assert_eq!(
            att[0]["responseStatus"], "accepted",
            "existing guest keeps status"
        );
        assert_eq!(att[1]["self"], true, "organizer entry preserved");
        assert!(
            v["conferenceData"]["entryPoints"].is_array(),
            "existing Meet kept"
        );
    }

    #[test]
    fn all_day_and_reminders() {
        let mut d = draft();
        d.all_day = true;
        d.start_date = Some("2026-09-14".into());
        d.end_date = Some("2026-09-15".into());
        d.reminders = Some(vec![crate::commands::types::Reminder {
            method: "popup".into(),
            minutes: 30,
        }]);
        d.attendees = vec![];
        d.add_meet = false;
        let v = serde_json::to_value(from_draft(&d, None, None).unwrap()).unwrap();
        assert_eq!(v["start"]["date"], "2026-09-14");
        assert_eq!(v["end"]["date"], "2026-09-15");
        assert_eq!(v["reminders"]["overrides"][0]["minutes"], 30);
        assert!(v.get("attendees").is_none());
    }

    #[test]
    fn helpers() {
        let raw = r#"{"id":"m","etag":"e","recurrence":["RRULE:FREQ=WEEKLY;COUNT=10"],"start":{"dateTime":"2026-09-14T10:00:00-03:00","timeZone":"America/Argentina/Buenos_Aires"},"end":{"dateTime":"2026-09-14T11:00:00-03:00","timeZone":"America/Argentina/Buenos_Aires"},"attendees":[{"email":"a","self":true,"responseStatus":"needsAction"},{"email":"b"}],"iCalUID":"m@google.com"}"#;
        let t = serde_json::to_value(
            with_recurrence(raw, &["RRULE:FREQ=WEEKLY;UNTIL=20260921T125959Z".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            t["recurrence"][0],
            "RRULE:FREQ=WEEKLY;UNTIL=20260921T125959Z"
        );
        assert!(t.get("etag").is_none());
        let imp = serde_json::to_value(for_import(raw, "m@google.com").unwrap()).unwrap();
        assert!(imp.get("id").is_none());
        assert_eq!(imp["iCalUID"], "m@google.com");
        let sh = shifted(raw, 3600, 1800, false).unwrap();
        assert_eq!(sh["start"]["dateTime"], "2026-09-14T11:00:00-03:00");
        assert_eq!(sh["end"]["dateTime"], "2026-09-14T11:30:00-03:00");
        let r = rsvp_attendees(raw, "accepted").unwrap();
        assert_eq!(r[0]["responseStatus"], "accepted");
        assert!(r[1].get("responseStatus").is_none());
        assert!(rsvp_attendees(r#"{"attendees":[{"email":"b"}]}"#, "accepted").is_err());
        assert_eq!(send_updates_for(true, true), SendUpdates::All);
        assert_eq!(send_updates_for(true, false), SendUpdates::None);
        assert_eq!(send_updates_for(false, true), SendUpdates::None);
    }
}
