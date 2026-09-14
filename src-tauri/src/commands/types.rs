//! IPC types. See docs/02-arquitectura.md section 5 and docs/03-modelo-de-datos.md section 9.
//!
//! Mirrored by hand in `src/types/ipc.ts`. The test at the bottom writes one JSON sample per
//! type to `src/types/fixtures/<Type>.json`; a vitest test imports them with the TS types so a
//! drift between the two sides fails `npm run typecheck`.

use serde::{Deserialize, Serialize};

pub use crate::recurrence::EditScope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    /// `local` | `google`
    pub kind: String,
    pub email: Option<String>,
    pub display_name: String,
    pub sort_order: i64,
    /// `idle` | `syncing` | `error` | `auth_required`
    pub sync_state: String,
    pub sync_error: Option<String>,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    /// `popup` | `email`
    pub method: String,
    pub minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarInfo {
    pub id: String,
    pub account_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub color_bg: String,
    pub color_fg: String,
    /// `owner` | `writer` | `reader` | `freeBusyReader`
    pub access_role: String,
    pub is_primary: bool,
    pub visible: bool,
    pub hidden_remote: bool,
    pub time_zone: Option<String>,
    pub default_reminders: Vec<Reminder>,
    pub sort_order: i64,
    pub is_local: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarVisibility {
    pub account_id: String,
    pub id: String,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewOccurrence {
    pub id: String,
    pub event_id: String,
    pub calendar_id: String,
    pub account_id: String,
    pub title: Option<String>,
    /// UTC seconds. For all-day: midnight UTC of the start date; `end` is exclusive.
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub color_bg: String,
    pub color_fg: String,
    /// `colorId` of the event when set, so the UI can pick the measured palette tokens.
    pub color_id: Option<String>,
    /// `confirmed` | `tentative`
    pub status: String,
    /// `accepted` | `declined` | `tentative` | `needsAction`, or null when not invited.
    pub my_response: Option<String>,
    pub is_recurring: bool,
    pub has_meet: bool,
    pub attendee_count: i64,
    pub conflict_with: Option<String>,
    pub also_in: Vec<String>,
    pub is_local: bool,
    /// `opaque` | `transparent`
    pub transparency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewPayload {
    pub from: i64,
    pub to: i64,
    pub occurrences: Vec<ViewOccurrence>,
    pub calendars: Vec<CalendarVisibility>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttendeeInfo {
    pub email: String,
    pub display_name: Option<String>,
    /// `needsAction` | `declined` | `tentative` | `accepted`
    pub response_status: String,
    pub organizer: bool,
    #[serde(rename = "self")]
    pub is_self: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlsoIn {
    pub account_id: String,
    pub account_name: String,
    pub occurrence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub occurrence_id: String,
    pub title: Option<String>,
    pub start: i64,
    pub end: i64,
    pub account_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDetail {
    pub occurrence_id: String,
    pub event_id: String,
    pub master_id: Option<String>,
    pub calendar_id: String,
    pub account_id: String,
    pub account_name: String,
    pub account_email: Option<String>,
    pub calendar_name: String,
    pub is_local: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub time_zone: Option<String>,
    /// `confirmed` | `tentative`
    pub status: String,
    pub color_bg: String,
    pub color_fg: String,
    pub color_id: Option<String>,
    pub transparency: Option<String>,
    pub visibility: Option<String>,
    pub is_recurring: bool,
    pub is_exception: bool,
    /// Human readable, e.g. "Weekly on Monday". Null when not recurring.
    pub recurrence_text: Option<String>,
    /// Raw RRULE/EXDATE/RDATE lines of the master.
    pub recurrence: Vec<String>,
    pub meet_link: Option<String>,
    pub conference_label: Option<String>,
    pub html_link: Option<String>,
    pub attendees: Vec<AttendeeInfo>,
    pub organizer: Option<AttendeeInfo>,
    pub my_response: Option<String>,
    pub use_default_reminders: bool,
    /// Effective reminders: the overrides, or the calendar defaults when `use_default_reminders`.
    pub reminders: Vec<Reminder>,
    pub also_in: Vec<AlsoIn>,
    pub conflict_with: Option<ConflictInfo>,
    pub can_edit: bool,
    pub can_delete: bool,
    pub can_rsvp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDraft {
    pub account_id: String,
    pub calendar_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub all_day: bool,
    /// UTC seconds when not all-day.
    pub start: Option<i64>,
    pub end: Option<i64>,
    /// `YYYY-MM-DD` when all-day; `end_date` exclusive as in Google.
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// IANA zone. Required for recurring events; defaults to the primary zone.
    pub time_zone: Option<String>,
    /// RRULE/EXDATE/RDATE lines. Empty for a single event.
    pub recurrence: Vec<String>,
    /// Guest e-mails.
    pub attendees: Vec<String>,
    /// Null keeps the calendar defaults.
    pub reminders: Option<Vec<Reminder>>,
    pub color_id: Option<String>,
    pub add_meet: bool,
    pub transparency: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorEntry {
    pub id: String,
    pub name: String,
    pub bg: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorPalette {
    pub events: Vec<ColorEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarKey {
    pub account_id: String,
    pub calendar_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub primary_tz: String,
    pub secondary_tz: Option<String>,
    /// 1 = Monday.
    pub week_start: u8,
    /// `24` | `12`
    pub hour_format: String,
    /// `system` only in version 1.
    pub theme: String,
    pub default_calendar: Option<CalendarKey>,
    pub data_window_past_days: i64,
    pub data_window_future_days: i64,
    pub webhook_port: u16,
    pub public_base_url: Option<String>,
    pub push_enabled: bool,
    pub holidays_account: Option<String>,
    /// Message of the last push setup failure, for the Settings screen.
    pub push_error: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            primary_tz: crate::config::DEFAULT_PRIMARY_TZ.into(),
            secondary_tz: Some(crate::config::DEFAULT_SECONDARY_TZ.into()),
            week_start: 1,
            hour_format: "24".into(),
            theme: "system".into(),
            default_calendar: None,
            data_window_past_days: crate::config::DEFAULT_DATA_WINDOW_PAST_DAYS,
            data_window_future_days: crate::config::DEFAULT_DATA_WINDOW_FUTURE_DAYS,
            webhook_port: crate::config::DEFAULT_WEBHOOK_PORT,
            public_base_url: None,
            push_enabled: false,
            holidays_account: None,
            push_error: None,
        }
    }
}

// ---- events emitted from Rust (docs/02 section 5) --------------------------------------------

pub const EVENT_CALENDAR_UPDATED: &str = "calendar:updated";
pub const EVENT_SYNC_STATUS: &str = "sync:status";
pub const EVENT_ACCOUNT_CHANGED: &str = "account:changed";
pub const EVENT_WINDOW_SHOW_EVENT: &str = "window:show-event";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarUpdated {
    pub from: i64,
    pub to: i64,
    pub calendar_ids: Vec<CalendarKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub account_id: String,
    pub state: String,
    pub message: Option<String>,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowEvent {
    pub occurrence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushTestResult {
    pub ok: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/types/fixtures")
    }

    fn write<T: Serialize>(name: &str, value: &T) {
        let dir = fixtures_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let json = serde_json::to_string_pretty(value).unwrap() + "\n";
        std::fs::write(dir.join(format!("{name}.json")), json).unwrap();
    }

    fn attendee(email: &str, status: &str, is_self: bool, organizer: bool) -> AttendeeInfo {
        AttendeeInfo {
            email: email.into(),
            display_name: Some(email.split('@').next().unwrap().into()),
            response_status: status.into(),
            organizer,
            is_self,
            optional: false,
        }
    }

    /// Regenerates `src/types/fixtures/*.json`. Run `cargo test` after changing any type.
    #[test]
    fn write_fixtures() {
        write(
            "AccountInfo",
            &vec![
                AccountInfo {
                    id: "local".into(),
                    kind: "local".into(),
                    email: None,
                    display_name: "This computer".into(),
                    sort_order: 0,
                    sync_state: "idle".into(),
                    sync_error: None,
                    last_sync_at: None,
                },
                AccountInfo {
                    id: "104857600000000000001".into(),
                    kind: "google".into(),
                    email: Some("someone@example.com".into()),
                    display_name: "someone@example.com".into(),
                    sort_order: 1,
                    sync_state: "error".into(),
                    sync_error: Some("The app could not reach Google (timeout).".into()),
                    last_sync_at: Some(1_789_000_000),
                },
            ],
        );
        write(
            "CalendarInfo",
            &vec![CalendarInfo {
                id: "primary".into(),
                account_id: "104857600000000000001".into(),
                summary: "someone@example.com".into(),
                description: None,
                color_bg: "#039be5".into(),
                color_fg: "#000000".into(),
                access_role: "owner".into(),
                is_primary: true,
                visible: true,
                hidden_remote: false,
                time_zone: Some("America/Argentina/Buenos_Aires".into()),
                default_reminders: vec![Reminder {
                    method: "popup".into(),
                    minutes: 10,
                }],
                sort_order: 0,
                is_local: false,
            }],
        );
        write(
            "ViewPayload",
            &ViewPayload {
                from: 1_789_000_000,
                to: 1_789_604_800,
                occurrences: vec![ViewOccurrence {
                    id: "104857600000000000001|primary|abc|1789012345".into(),
                    event_id: "abc".into(),
                    calendar_id: "primary".into(),
                    account_id: "104857600000000000001".into(),
                    title: Some("Daily standup".into()),
                    start: 1_789_012_345,
                    end: 1_789_013_245,
                    all_day: false,
                    color_bg: "#0b8043".into(),
                    color_fg: "#ffffff".into(),
                    color_id: Some("10".into()),
                    status: "confirmed".into(),
                    my_response: Some("accepted".into()),
                    is_recurring: true,
                    has_meet: true,
                    attendee_count: 3,
                    conflict_with: None,
                    also_in: vec![],
                    is_local: false,
                    transparency: Some("opaque".into()),
                }],
                calendars: vec![CalendarVisibility {
                    account_id: "104857600000000000001".into(),
                    id: "primary".into(),
                    visible: true,
                }],
            },
        );
        write(
            "EventDetail",
            &EventDetail {
                occurrence_id: "104857600000000000001|primary|abc|1789012345".into(),
                event_id: "abc".into(),
                master_id: Some("abc".into()),
                calendar_id: "primary".into(),
                account_id: "104857600000000000001".into(),
                account_name: "someone@example.com".into(),
                account_email: Some("someone@example.com".into()),
                calendar_name: "someone@example.com".into(),
                is_local: false,
                title: Some("Daily standup".into()),
                description: Some("Description line".into()),
                location: Some("Location text".into()),
                start: 1_789_012_345,
                end: 1_789_013_245,
                all_day: false,
                time_zone: Some("America/Argentina/Buenos_Aires".into()),
                status: "confirmed".into(),
                color_bg: "#0b8043".into(),
                color_fg: "#ffffff".into(),
                color_id: Some("10".into()),
                transparency: Some("opaque".into()),
                visibility: Some("default".into()),
                is_recurring: true,
                is_exception: false,
                recurrence_text: Some("Weekly on Monday".into()),
                recurrence: vec!["RRULE:FREQ=WEEKLY;BYDAY=MO".into()],
                meet_link: Some("https://meet.google.com/abc-defg-hij".into()),
                conference_label: Some("Google Meet".into()),
                html_link: Some("https://www.google.com/calendar/event?eid=abc".into()),
                attendees: vec![
                    attendee("someone@example.com", "accepted", true, true),
                    attendee("guest@example.com", "needsAction", false, false),
                ],
                organizer: Some(attendee("someone@example.com", "accepted", true, true)),
                my_response: Some("accepted".into()),
                use_default_reminders: true,
                reminders: vec![Reminder {
                    method: "popup".into(),
                    minutes: 10,
                }],
                also_in: vec![AlsoIn {
                    account_id: "104857600000000000002".into(),
                    account_name: "other@example.com".into(),
                    occurrence_id: "104857600000000000002|primary|xyz|1789012345".into(),
                }],
                conflict_with: Some(ConflictInfo {
                    occurrence_id: "local|local-personal|uuid|1789012000".into(),
                    title: Some("Dentist".into()),
                    start: 1_789_012_000,
                    end: 1_789_015_600,
                    account_name: "This computer".into(),
                }),
                can_edit: true,
                can_delete: true,
                can_rsvp: true,
            },
        );
        write(
            "EventDraft",
            &EventDraft {
                account_id: "local".into(),
                calendar_id: "local-personal".into(),
                title: Some("Dentist".into()),
                description: None,
                location: None,
                all_day: false,
                start: Some(1_789_012_000),
                end: Some(1_789_015_600),
                start_date: None,
                end_date: None,
                time_zone: Some("America/Argentina/Buenos_Aires".into()),
                recurrence: vec![],
                attendees: vec![],
                reminders: None,
                color_id: None,
                add_meet: false,
                transparency: None,
                visibility: None,
            },
        );
        write(
            "ColorPalette",
            &ColorPalette {
                events: crate::google::colors::EVENT_PALETTE
                    .iter()
                    .map(|c| ColorEntry {
                        id: c.id.into(),
                        name: c.name.into(),
                        bg: c.bg.into(),
                    })
                    .collect(),
            },
        );
        write("Settings", &Settings::default());
        write(
            "CalendarUpdated",
            &CalendarUpdated {
                from: 1_789_000_000,
                to: 1_789_604_800,
                calendar_ids: vec![CalendarKey {
                    account_id: "local".into(),
                    calendar_id: "local-personal".into(),
                }],
            },
        );
        write(
            "SyncStatus",
            &SyncStatus {
                account_id: "104857600000000000001".into(),
                state: "syncing".into(),
                message: None,
                at: 1_789_000_000,
            },
        );
        write(
            "ShowEvent",
            &ShowEvent {
                occurrence_id: "local|local-personal|uuid|1789012000".into(),
            },
        );
        write(
            "PushTestResult",
            &PushTestResult {
                ok: false,
                message: "GET https://pc.tail.ts.net/healthz failed: timeout".into(),
            },
        );
        write(
            "EditScope",
            &vec![EditScope::This, EditScope::Following, EditScope::All],
        );
    }
}
