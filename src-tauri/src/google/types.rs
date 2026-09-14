//! Serde types for the Calendar API v3 resources the app uses. Unknown fields are kept in
//! `extra` so an `Event` round-trips to the exact JSON Google sent (`events.raw`).
//! See docs/research/google-calendar-api.md and docs/03-modelo-de-datos.md section 2.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReminderOverride {
    pub method: String,
    pub minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reminders: Option<Vec<ReminderOverride>>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListPage {
    #[serde(default)]
    pub items: Vec<CalendarListEntry>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub next_sync_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventDateTime {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "self", default, skip_serializing_if = "Option::is_none")]
    pub is_self: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizer: Option<bool>,
    #[serde(rename = "self", default, skip_serializing_if = "Option::is_none")]
    pub is_self: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_status: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "iCalUID", default, skip_serializing_if = "Option::is_none")]
    pub ical_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_start_time: Option<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizer: Option<Person>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<Person>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<Attendee>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminders: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hangout_link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conference_data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guests_can_modify: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Event {
    pub fn is_all_day(&self) -> bool {
        self.start.as_ref().is_some_and(|s| s.date.is_some())
    }

    pub fn is_cancelled(&self) -> bool {
        self.status.as_deref() == Some("cancelled")
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    #[serde(default)]
    pub items: Vec<Event>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub next_sync_token: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub default_reminders: Option<Vec<ReminderOverride>>,
}

/// Google sends int64 fields as JSON strings; accept both.
fn de_i64_lenient<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::String(s)) => s.parse().ok(),
        Some(Value::Number(n)) => n.as_i64(),
        _ => None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    #[serde(default)]
    pub resource_id: String,
    #[serde(default)]
    pub resource_uri: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    /// Unix milliseconds.
    #[serde(default, deserialize_with = "de_i64_lenient")]
    pub expiration: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub address: String,
    pub token: String,
    pub params: WatchParams,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WatchParams {
    /// Seconds, as a string (docs/05 section 3.3).
    pub ttl: String,
}

impl WatchRequest {
    pub fn web_hook(id: String, address: String, token: String, ttl_secs: u64) -> WatchRequest {
        WatchRequest {
            id,
            kind: "web_hook".into(),
            address,
            token,
            params: WatchParams {
                ttl: ttl_secs.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ColorDefinition {
    pub background: String,
    pub foreground: String,
}

/// `colors.get` resource. Not used for rendering (docs/04 section 7); kept for completeness.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Colors {
    #[serde(default)]
    pub calendar: std::collections::BTreeMap<String, ColorDefinition>,
    #[serde(default)]
    pub event: std::collections::BTreeMap<String, ColorDefinition>,
}

/// `sendUpdates` query parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendUpdates {
    All,
    ExternalOnly,
    None,
}

impl SendUpdates {
    pub fn as_str(self) -> &'static str {
        match self {
            SendUpdates::All => "all",
            SendUpdates::ExternalOnly => "externalOnly",
            SendUpdates::None => "none",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trips_unknown_fields() {
        let json = r#"{"id":"e1","iCalUID":"e1@google.com","summary":"x","start":{"dateTime":"2026-06-01T10:00:00-03:00","timeZone":"America/Argentina/Buenos_Aires"},"end":{"dateTime":"2026-06-01T11:00:00-03:00"},"attendees":[{"email":"a@b.c","self":true,"responseStatus":"accepted"}],"extendedProperties":{"private":{"k":"v"}},"sequence":3}"#;
        let e: Event = serde_json::from_str(json).unwrap();
        assert_eq!(e.ical_uid.as_deref(), Some("e1@google.com"));
        assert!(e.attendees.as_ref().unwrap()[0].is_self.unwrap());
        let back: Value = serde_json::to_value(&e).unwrap();
        assert_eq!(back["extendedProperties"]["private"]["k"], "v");
        assert_eq!(back["iCalUID"], "e1@google.com");
        assert_eq!(back["attendees"][0]["self"], true);
        assert!(
            back.get("recurrence").is_none(),
            "absent fields stay absent"
        );
    }

    #[test]
    fn channel_expiration_accepts_string_and_number() {
        let a: Channel =
            serde_json::from_str(r#"{"id":"c","resourceId":"r","expiration":"1426325213000"}"#)
                .unwrap();
        let b: Channel =
            serde_json::from_str(r#"{"id":"c","resourceId":"r","expiration":1426325213000}"#)
                .unwrap();
        assert_eq!(a.expiration, Some(1_426_325_213_000));
        assert_eq!(a.expiration, b.expiration);
    }

    #[test]
    fn watch_request_shape() {
        let w = WatchRequest::web_hook(
            "id".into(),
            "https://x/gcal/webhook".into(),
            "tok".into(),
            604_800,
        );
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v["type"], "web_hook");
        assert_eq!(v["params"]["ttl"], "604800");
    }
}
