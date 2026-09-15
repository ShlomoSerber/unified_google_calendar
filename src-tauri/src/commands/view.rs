//! Read commands: `get_view` and `get_event`. See docs/02-arquitectura.md section 3.2 and
//! docs/03-modelo-de-datos.md sections 5, 6 and 9.

use std::collections::HashMap;

use chrono::DateTime;
use rusqlite::{params, Connection};
use serde_json::Value;

use crate::commands::types::{
    AlsoIn, AttendeeInfo, CalendarVisibility, ConferencePhone, ConflictInfo, EventDetail, Reminder,
    ViewOccurrence, ViewPayload,
};
use crate::db::queries::{accounts, calendars, events as q};
use crate::error::AppError;
use crate::google::colors;
use crate::recurrence::describe;

/// One occurrence joined with the columns the view needs.
#[derive(Debug, Clone)]
struct JoinedOccurrence {
    id: String,
    event_id: String,
    calendar_id: String,
    account_id: String,
    account_sort: i64,
    is_local: bool,
    title: Option<String>,
    start: i64,
    end: i64,
    all_day: bool,
    status: String,
    ical_uid: Option<String>,
    color_id: Option<String>,
    calendar_bg: String,
    calendar_fg: String,
    attendees: String,
    organizer_self: bool,
    hangout_link: Option<String>,
    conference: Option<String>,
    is_recurring: bool,
    transparency: Option<String>,
    location: Option<String>,
}

const JOIN_SQL: &str = "SELECT o.id, o.event_id, o.calendar_id, o.account_id, a.sort_order, a.kind, e.summary, o.start_ts, o.end_ts, o.all_day, \
    o.status, e.ical_uid, e.color_id, c.color_bg, c.color_fg, e.attendees, e.organizer_self, e.hangout_link, e.conference, \
    (o.master_id IS NOT NULL), e.transparency, e.location \
    FROM occurrences o \
    JOIN events e ON e.account_id = o.account_id AND e.calendar_id = o.calendar_id AND e.id = o.event_id \
    JOIN calendars c ON c.account_id = o.account_id AND c.id = o.calendar_id \
    JOIN accounts a ON a.id = o.account_id \
    WHERE o.end_ts > ?1 AND o.start_ts < ?2 AND o.status != 'cancelled' AND c.deleted = 0";

fn joined_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<JoinedOccurrence> {
    Ok(JoinedOccurrence {
        id: r.get(0)?,
        event_id: r.get(1)?,
        calendar_id: r.get(2)?,
        account_id: r.get(3)?,
        account_sort: r.get(4)?,
        is_local: r.get::<_, String>(5)? == "local",
        title: r.get(6)?,
        start: r.get(7)?,
        end: r.get(8)?,
        all_day: r.get::<_, i64>(9)? != 0,
        status: r.get(10)?,
        ical_uid: r.get(11)?,
        color_id: r.get(12)?,
        calendar_bg: r.get(13)?,
        calendar_fg: r.get(14)?,
        attendees: r.get(15)?,
        organizer_self: r.get::<_, i64>(16)? != 0,
        hangout_link: r.get(17)?,
        conference: r.get(18)?,
        is_recurring: r.get::<_, i64>(19)? != 0,
        transparency: r.get(20)?,
        location: r.get(21)?,
    })
}

fn load_occurrences(
    conn: &Connection,
    from: i64,
    to: i64,
    only_visible: bool,
) -> Result<Vec<JoinedOccurrence>, AppError> {
    let sql = if only_visible {
        format!("{JOIN_SQL} AND c.visible = 1")
    } else {
        JOIN_SQL.to_string()
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![from, to], joined_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn parse_attendees(json: &str) -> Vec<AttendeeInfo> {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(json) else {
        return vec![];
    };
    items
        .iter()
        .filter_map(|a| {
            let email = a.get("email")?.as_str()?.to_string();
            Some(AttendeeInfo {
                email,
                display_name: a
                    .get("displayName")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                response_status: a
                    .get("responseStatus")
                    .and_then(Value::as_str)
                    .unwrap_or("needsAction")
                    .to_string(),
                organizer: a.get("organizer").and_then(Value::as_bool).unwrap_or(false),
                is_self: a.get("self").and_then(Value::as_bool).unwrap_or(false),
                optional: a.get("optional").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

/// The user's own response: the attendee Google flags as `self`, or, on a copy held by another
/// calendar of the same account (where Google omits the flag), the attendee with the account's
/// e-mail address.
fn my_response(attendees: &[AttendeeInfo], account_email: Option<&str>) -> Option<String> {
    attendees
        .iter()
        .find(|a| a.is_self)
        .or_else(|| {
            let email = account_email?;
            attendees.iter().find(|a| a.email.eq_ignore_ascii_case(email))
        })
        .map(|a| a.response_status.clone())
}

/// Meet link: `hangoutLink`, else the first `video` entry point of `conferenceData`.
pub fn meet_link(hangout_link: Option<&str>, conference: Option<&str>) -> Option<String> {
    if let Some(l) = hangout_link.filter(|l| !l.is_empty()) {
        return Some(l.to_string());
    }
    let conf: Value = serde_json::from_str(conference?).ok()?;
    let entries = conf.get("entryPoints")?.as_array()?;
    entries
        .iter()
        .find(|e| e.get("entryPointType").and_then(Value::as_str) == Some("video"))
        .and_then(|e| e.get("uri").and_then(Value::as_str))
        .map(str::to_string)
}

/// Dial-in entry of `conferenceData` (the popup's "Join by phone" row, docs/99 F7-T3).
fn conference_phone(conference: Option<&str>) -> Option<ConferencePhone> {
    let conf: Value = serde_json::from_str(conference?).ok()?;
    let entries = conf.get("entryPoints")?.as_array()?;
    let phone = entries
        .iter()
        .find(|e| e.get("entryPointType").and_then(Value::as_str) == Some("phone"))?;
    let more = entries
        .iter()
        .find(|e| e.get("entryPointType").and_then(Value::as_str) == Some("more"))
        .and_then(|e| e.get("uri").and_then(Value::as_str))
        .map(str::to_string);
    let number = phone
        .get("label")
        .and_then(Value::as_str)
        .or_else(|| phone.get("uri").and_then(Value::as_str))?;
    // Google's popup shows "(AR) +54 11 3986-3700 PIN: 224 472 849 1492#".
    let label = match phone.get("regionCode").and_then(Value::as_str) {
        Some(r) => format!("({r}) {number}"),
        None => number.to_string(),
    };
    Some(ConferencePhone {
        label,
        uri: phone.get("uri").and_then(Value::as_str).map(str::to_string),
        pin: phone.get("pin").and_then(Value::as_str).map(group_pin),
        more_url: more,
    })
}

/// "2244728491492" → "224 472 849 1492" (groups of three, the rest at the end), as Google prints it.
fn group_pin(pin: &str) -> String {
    let digits: Vec<char> = pin.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() <= 4 {
        return pin.to_string();
    }
    let mut out = String::new();
    let full = (digits.len() - 1) / 3;
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && i % 3 == 0 && i / 3 <= full && digits.len() - i > 3 {
            out.push(' ');
        }
        out.push(*c);
    }
    out
}

fn conference_label(conference: Option<&str>) -> Option<String> {
    let conf: Value = serde_json::from_str(conference?).ok()?;
    conf.get("conferenceSolution")?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

/// Dedup key per docs/03 section 5: `(ical_uid, start_ts)`. Rows without `ical_uid` are unique.
fn dedup_key(o: &JoinedOccurrence) -> Option<(String, i64)> {
    o.ical_uid.as_ref().map(|u| (u.clone(), o.start))
}

/// A shown occurrence and the hidden copies of the same event in other accounts.
type DedupGroup = (JoinedOccurrence, Vec<JoinedOccurrence>);

/// Deduplicate copies of the same event across accounts. Returns the winners with their
/// `also_in` lists, plus a map from every occurrence id to the winner id.
fn deduplicate(rows: Vec<JoinedOccurrence>) -> (Vec<DedupGroup>, HashMap<String, String>) {
    let mut groups: Vec<Vec<JoinedOccurrence>> = Vec::new();
    let mut index: HashMap<(String, i64), usize> = HashMap::new();
    for o in rows {
        match dedup_key(&o) {
            Some(k) => match index.get(&k) {
                Some(&i) => groups[i].push(o),
                None => {
                    index.insert(k, groups.len());
                    groups.push(vec![o]);
                }
            },
            None => groups.push(vec![o]),
        }
    }
    let mut out = Vec::with_capacity(groups.len());
    let mut winners = HashMap::new();
    for mut g in groups {
        // Winner: organizer_self first, then the account with the lowest sort_order.
        g.sort_by(|a, b| {
            b.organizer_self
                .cmp(&a.organizer_self)
                .then(a.account_sort.cmp(&b.account_sort))
        });
        let winner = g.remove(0);
        for o in &g {
            winners.insert(o.id.clone(), winner.id.clone());
        }
        winners.insert(winner.id.clone(), winner.id.clone());
        out.push((winner, g));
    }
    (out, winners)
}

fn counts_for_conflicts(o: &JoinedOccurrence) -> bool {
    !o.all_day && o.status != "cancelled" && o.transparency.as_deref() != Some("transparent")
}

/// First conflict of each occurrence by start order (docs/03 section 6).
fn conflicts(items: &[JoinedOccurrence]) -> HashMap<String, String> {
    let mut sorted: Vec<&JoinedOccurrence> =
        items.iter().filter(|o| counts_for_conflicts(o)).collect();
    sorted.sort_by_key(|o| (o.start, -(o.end - o.start)));
    let mut out = HashMap::new();
    for (i, a) in sorted.iter().enumerate() {
        for b in sorted.iter().skip(i + 1) {
            if b.start >= a.end {
                break;
            }
            if b.account_id != a.account_id && b.start < a.end && a.start < b.end {
                out.entry(a.id.clone()).or_insert_with(|| b.id.clone());
                out.entry(b.id.clone()).or_insert_with(|| a.id.clone());
            }
        }
    }
    out
}

/// `get_view`. `tz` is accepted for the IPC contract; all-day placement happens in the UI.
pub fn get_view(conn: &Connection, from: i64, to: i64, _tz: &str) -> Result<ViewPayload, AppError> {
    if to <= from {
        return Err(AppError::invalid(
            "The view range end must be after its start",
        ));
    }
    let rows = load_occurrences(conn, from, to, true)?;
    let emails: std::collections::HashMap<String, String> = accounts::list_accounts(conn)?
        .into_iter()
        .filter_map(|a| Some((a.id, a.email?)))
        .collect();
    let (groups, _) = deduplicate(rows);
    let winners: Vec<JoinedOccurrence> = groups.iter().map(|(w, _)| w.clone()).collect();
    let conflict_map = conflicts(&winners);
    let mut occurrences: Vec<ViewOccurrence> = groups
        .into_iter()
        .map(|(o, others)| {
            let attendees = parse_attendees(&o.attendees);
            let my = my_response(&attendees, emails.get(&o.account_id).map(String::as_str));
            ViewOccurrence {
                id: o.id.clone(),
                event_id: o.event_id,
                calendar_id: o.calendar_id,
                account_id: o.account_id,
                title: o.title,
                start: o.start,
                end: o.end,
                all_day: o.all_day,
                color_bg: colors::resolve_bg(o.color_id.as_deref(), &o.calendar_bg).to_string(),
                color_fg: o.calendar_fg,
                color_id: o.color_id,
                status: o.status,
                my_response: my,
                is_recurring: o.is_recurring,
                has_meet: meet_link(o.hangout_link.as_deref(), o.conference.as_deref()).is_some(),
                attendee_count: attendees.len() as i64,
                conflict_with: conflict_map.get(&o.id).cloned(),
                also_in: others.into_iter().map(|x| x.account_id).collect(),
                is_local: o.is_local,
                transparency: o.transparency,
                location: o.location,
            }
        })
        .collect();
    occurrences.sort_by_key(|o| (o.start, -(o.end - o.start), o.id.clone()));

    let calendars = calendars::list_calendars(conn)?
        .into_iter()
        .map(|c| CalendarVisibility {
            account_id: c.account_id,
            id: c.id,
            visible: c.visible,
        })
        .collect();
    Ok(ViewPayload {
        from,
        to,
        occurrences,
        calendars,
    })
}

pub fn parse_reminders(json: &str) -> (bool, Vec<Reminder>) {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let use_default = v.get("useDefault").and_then(Value::as_bool).unwrap_or(true);
    let overrides = v
        .get("overrides")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    Some(Reminder {
                        method: r.get("method")?.as_str()?.to_string(),
                        minutes: r.get("minutes")?.as_i64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (use_default, overrides)
}

pub fn parse_default_reminders(json: &str) -> Vec<Reminder> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    Some(Reminder {
                        method: r.get("method")?.as_str()?.to_string(),
                        minutes: r.get("minutes")?.as_i64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `get_event`: everything the popup shows for one occurrence.
pub fn get_event(conn: &Connection, occurrence_id: &str) -> Result<EventDetail, AppError> {
    let occ = q::get_occurrence(conn, occurrence_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    let row = q::get_event(conn, &occ.account_id, &occ.calendar_id, &occ.event_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    let calendar = calendars::get_calendar(conn, &occ.account_id, &occ.calendar_id)?
        .ok_or_else(|| AppError::NotFound("The calendar".into()))?;
    let account = accounts::get_account(conn, &occ.account_id)?
        .ok_or_else(|| AppError::NotFound("The account".into()))?;
    let master = match &occ.master_id {
        Some(m) if m != &row.id => q::get_event(conn, &occ.account_id, &occ.calendar_id, m)?,
        _ => None,
    };
    let recurrence = master
        .as_ref()
        .and_then(|m| m.recurrence.clone())
        .or_else(|| row.recurrence.clone())
        .unwrap_or_default();
    let series_start = master.as_ref().unwrap_or(&row);
    let start_local_date = if series_start.all_day {
        series_start
            .start_date
            .as_deref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    } else {
        let tz = series_start
            .time_zone
            .as_deref()
            .and_then(|t| t.parse::<chrono_tz::Tz>().ok())
            .unwrap_or(chrono_tz::UTC);
        series_start
            .start_ts
            .and_then(|s| DateTime::from_timestamp(s, 0))
            .map(|d| d.with_timezone(&tz).date_naive())
    };
    let recurrence_text = if recurrence.is_empty() {
        None
    } else {
        start_local_date
            .and_then(|d| describe::describe(&recurrence, d))
            .or(Some("Custom".into()))
    };

    let attendees = parse_attendees(&row.attendees);
    let organizer = attendees.iter().find(|a| a.organizer).cloned().or_else(|| {
        row.organizer_email.as_ref().map(|e| AttendeeInfo {
            email: e.clone(),
            display_name: None,
            response_status: "accepted".into(),
            organizer: true,
            is_self: row.organizer_self,
            optional: false,
        })
    });
    let my = my_response(&attendees, account.email.as_deref());
    let (use_default, overrides) = parse_reminders(&row.reminders);
    let reminders = if use_default {
        parse_default_reminders(&calendar.default_reminders)
    } else {
        overrides
    };

    // also_in and conflict_with: recompute over the occurrences overlapping this one.
    let window_rows = load_occurrences(conn, occ.start_ts - 1, occ.end_ts + 1, false)?;
    let mut also_in = Vec::new();
    if let Some(uid) = &row.ical_uid {
        for other in window_rows.iter().filter(|o| {
            o.id != occ.id && o.ical_uid.as_deref() == Some(uid) && o.start == occ.start_ts
        }) {
            let name = accounts::get_account(conn, &other.account_id)?
                .map(|a| a.display_name)
                .unwrap_or_else(|| other.account_id.clone());
            also_in.push(AlsoIn {
                account_id: other.account_id.clone(),
                account_name: name,
                occurrence_id: other.id.clone(),
            });
        }
    }
    let (groups, winner_of) = deduplicate(window_rows);
    let winners: Vec<JoinedOccurrence> = groups.iter().map(|(w, _)| w.clone()).collect();
    let conflict_map = conflicts(&winners);
    let my_winner = winner_of
        .get(&occ.id)
        .cloned()
        .unwrap_or_else(|| occ.id.clone());
    let conflict_with = conflict_map
        .get(&my_winner)
        .and_then(|cid| winners.iter().find(|w| &w.id == cid))
        .map(|w| ConflictInfo {
            occurrence_id: w.id.clone(),
            title: w.title.clone(),
            start: w.start,
            end: w.end,
            account_name: accounts::get_account(conn, &w.account_id)
                .ok()
                .flatten()
                .map(|a| a.display_name)
                .unwrap_or_default(),
        });

    let is_local = account.is_local();
    let writable = calendar.can_write();
    let has_self_attendee = attendees.iter().any(|a| a.is_self);
    let can_edit = writable
        && (is_local || row.organizer_self || attendees.is_empty() || row.guests_can_modify);
    let can_delete = writable;
    let can_rsvp = writable && has_self_attendee && !row.organizer_self;

    Ok(EventDetail {
        occurrence_id: occ.id.clone(),
        event_id: row.id.clone(),
        master_id: occ.master_id.clone(),
        calendar_id: row.calendar_id.clone(),
        account_id: row.account_id.clone(),
        account_name: account.display_name.clone(),
        account_email: account.email.clone(),
        calendar_name: calendar.summary.clone(),
        is_local,
        title: row.summary.clone(),
        description: row.description.clone(),
        location: row.location.clone(),
        start: occ.start_ts,
        end: occ.end_ts,
        all_day: occ.all_day,
        time_zone: row.time_zone.clone(),
        status: row.status.clone(),
        color_bg: colors::resolve_bg(row.color_id.as_deref(), &calendar.color_bg).to_string(),
        color_fg: calendar.color_fg.clone(),
        color_id: row.color_id.clone(),
        transparency: row.transparency.clone(),
        visibility: row.visibility.clone(),
        is_recurring: occ.master_id.is_some(),
        is_exception: row.is_exception(),
        recurrence_text,
        recurrence,
        meet_link: meet_link(row.hangout_link.as_deref(), row.conference.as_deref()),
        conference_label: conference_label(row.conference.as_deref()),
        conference_phone: conference_phone(row.conference.as_deref()),
        html_link: row.html_link.clone(),
        attendees,
        organizer,
        my_response: my,
        use_default_reminders: use_default,
        reminders,
        also_in,
        conflict_with,
        can_edit,
        can_delete,
        can_rsvp,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db::queries::events::EventRow;
    use crate::recurrence::{expand::Window, materialize_simple};

    pub fn ts(s: &str) -> i64 {
        DateTime::parse_from_rfc3339(s).unwrap().timestamp()
    }
    pub const WIN: Window = Window {
        from_ts: 1_735_689_600,
        to_ts: 1_798_761_600,
    };

    pub fn add_google_account(conn: &Connection, id: &str, email: &str, sort: i64) {
        conn.execute(
            "INSERT INTO accounts (id, kind, email, display_name, sort_order, created_at) VALUES (?1,'google',?2,?2,?3,0)",
            params![id, email, sort],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible) VALUES ('primary',?1,?2,'#039be5','#000000','owner',1,1)",
            params![id, email],
        )
        .unwrap();
    }

    pub fn event(account: &str, calendar: &str, id: &str, start: &str, end: &str) -> EventRow {
        EventRow {
            account_id: account.into(),
            calendar_id: calendar.into(),
            id: id.into(),
            ical_uid: Some(format!("{id}@google.com")),
            status: "confirmed".into(),
            summary: Some(id.to_string()),
            start_ts: Some(ts(start)),
            end_ts: Some(ts(end)),
            all_day: false,
            attendees: "[]".into(),
            reminders: r#"{"useDefault":true}"#.into(),
            event_type: "default".into(),
            ..Default::default()
        }
    }

    pub fn put(conn: &mut Connection, e: &EventRow) {
        q::upsert_event(conn, e).unwrap();
        materialize_simple(conn, &e.account_id, &e.calendar_id, &e.id, WIN).unwrap();
    }

    fn setup() -> Connection {
        let mut conn = crate::db::open_memory().unwrap();
        add_google_account(&conn, "acc1", "one@example.com", 1);
        add_google_account(&conn, "acc2", "two@example.com", 2);
        // Same invitation in two accounts (same iCalUID, different ids), organized by acc2.
        let mut a = event(
            "acc1",
            "primary",
            "inv-a",
            "2026-06-01T12:00:00Z",
            "2026-06-01T13:00:00Z",
        );
        a.ical_uid = Some("shared@google.com".into());
        a.attendees = r#"[{"email":"two@example.com","organizer":true,"responseStatus":"accepted"},{"email":"one@example.com","self":true,"responseStatus":"tentative"}]"#.into();
        let mut b = event(
            "acc2",
            "primary",
            "inv-b",
            "2026-06-01T12:00:00Z",
            "2026-06-01T13:00:00Z",
        );
        b.ical_uid = Some("shared@google.com".into());
        b.organizer_self = true;
        b.attendees = r#"[{"email":"two@example.com","organizer":true,"self":true,"responseStatus":"accepted"},{"email":"one@example.com","responseStatus":"tentative"}]"#.into();
        put(&mut conn, &a);
        put(&mut conn, &b);
        // A real conflict: local event overlapping the invitation.
        put(
            &mut conn,
            &event(
                "local",
                "local-personal",
                "dentist",
                "2026-06-01T12:30:00Z",
                "2026-06-01T13:30:00Z",
            ),
        );
        // A transparent event overlapping everything: never a conflict.
        let mut t = event(
            "acc1",
            "primary",
            "free",
            "2026-06-01T12:00:00Z",
            "2026-06-01T14:00:00Z",
        );
        t.transparency = Some("transparent".into());
        put(&mut conn, &t);
        // An all-day event the same day: never a conflict.
        let mut ad = event(
            "acc2",
            "primary",
            "allday",
            "2026-06-01T00:00:00Z",
            "2026-06-02T00:00:00Z",
        );
        ad.all_day = true;
        ad.start_ts = None;
        ad.end_ts = None;
        ad.start_date = Some("2026-06-01".into());
        ad.end_date = Some("2026-06-02".into());
        put(&mut conn, &ad);
        conn
    }

    #[test]
    fn view_dedups_conflicts_and_orders() {
        let conn = setup();
        let v = get_view(
            &conn,
            ts("2026-06-01T00:00:00Z"),
            ts("2026-06-02T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        let ids: Vec<&str> = v.occurrences.iter().map(|o| o.event_id.as_str()).collect();
        // Order: start asc, duration desc. all-day first (midnight), then free (2h), inv-b (1h), dentist.
        assert_eq!(ids, vec!["allday", "free", "inv-b", "dentist"]);
        let inv = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "inv-b")
            .unwrap();
        assert_eq!(inv.account_id, "acc2", "organizer_self copy wins");
        assert_eq!(inv.also_in, vec!["acc1".to_string()]);
        assert_eq!(inv.my_response.as_deref(), Some("accepted"));
        assert_eq!(inv.attendee_count, 2);
        let dentist = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "dentist")
            .unwrap();
        assert_eq!(inv.conflict_with.as_deref(), Some(dentist.id.as_str()));
        assert_eq!(dentist.conflict_with.as_deref(), Some(inv.id.as_str()));
        let free = v.occurrences.iter().find(|o| o.event_id == "free").unwrap();
        assert!(free.conflict_with.is_none());
        let allday = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "allday")
            .unwrap();
        assert!(allday.conflict_with.is_none());
        assert_eq!(v.calendars.len(), 3);
        assert_eq!(dentist.color_bg, "#f4511e");
        assert!(dentist.is_local);
    }

    #[test]
    fn view_hides_invisible_calendars_and_maps_color_id() {
        let mut conn = setup();
        calendars::set_visible(&conn, "acc1", "primary", false).unwrap();
        let mut colored = event(
            "acc2",
            "primary",
            "colored",
            "2026-06-01T15:00:00Z",
            "2026-06-01T16:00:00Z",
        );
        colored.color_id = Some("10".into());
        put(&mut conn, &colored);
        let v = get_view(
            &conn,
            ts("2026-06-01T00:00:00Z"),
            ts("2026-06-02T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert!(v.occurrences.iter().all(|o| o.account_id != "acc1"));
        // The invitation is now shown from acc2 with nothing in also_in.
        let inv = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "inv-b")
            .unwrap();
        assert!(inv.also_in.is_empty());
        let c = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "colored")
            .unwrap();
        assert_eq!(c.color_bg, "#0b8043");
        assert_eq!(c.color_id.as_deref(), Some("10"));
        assert!(get_view(&conn, 10, 5, "UTC").is_err());
    }

    #[test]
    fn event_detail_has_everything_the_popup_needs() {
        let conn = setup();
        let v = get_view(
            &conn,
            ts("2026-06-01T00:00:00Z"),
            ts("2026-06-02T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        let inv = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "inv-b")
            .unwrap();
        let d = get_event(&conn, &inv.id).unwrap();
        assert_eq!(d.account_name, "two@example.com");
        assert_eq!(d.calendar_name, "two@example.com");
        assert_eq!(d.attendees.len(), 2);
        assert!(d.organizer.as_ref().unwrap().is_self);
        assert_eq!(d.also_in.len(), 1);
        assert_eq!(d.also_in[0].account_name, "one@example.com");
        let c = d.conflict_with.unwrap();
        assert_eq!(c.title.as_deref(), Some("dentist"));
        assert_eq!(c.account_name, "This computer");
        assert!(d.can_edit && d.can_delete && !d.can_rsvp);
        assert!(d.recurrence_text.is_none());
        assert_eq!(d.reminders, vec![]);

        // The invited copy can RSVP but not edit.
        let other = get_event(&conn, &d.also_in[0].occurrence_id).unwrap();
        assert!(other.can_rsvp && !other.can_edit);
        assert_eq!(other.my_response.as_deref(), Some("tentative"));

        let dentist = v
            .occurrences
            .iter()
            .find(|o| o.event_id == "dentist")
            .unwrap();
        let dd = get_event(&conn, &dentist.id).unwrap();
        assert!(dd.is_local);
        assert_eq!(
            dd.reminders,
            vec![Reminder {
                method: "popup".into(),
                minutes: 10
            }]
        );
        assert!(dd.use_default_reminders);
        assert!(get_event(&conn, "nope").is_err());
    }

    #[test]
    fn event_detail_recurrence_text_and_meet() {
        let mut conn = crate::db::open_memory().unwrap();
        let mut m = event(
            "local",
            "local-personal",
            "m1",
            "2026-03-02T13:00:00Z",
            "2026-03-02T14:00:00Z",
        );
        m.time_zone = Some("America/Argentina/Buenos_Aires".into());
        m.recurrence = Some(vec!["RRULE:FREQ=WEEKLY;COUNT=3".into()]);
        m.conference = Some(r#"{"conferenceSolution":{"name":"Google Meet"},"entryPoints":[{"entryPointType":"video","uri":"https://meet.google.com/x"}]}"#.into());
        m.reminders =
            r#"{"useDefault":false,"overrides":[{"method":"popup","minutes":30}]}"#.into();
        put(&mut conn, &m);
        let v = get_view(
            &conn,
            ts("2026-03-09T00:00:00Z"),
            ts("2026-03-10T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 1);
        assert!(v.occurrences[0].has_meet && v.occurrences[0].is_recurring);
        let d = get_event(&conn, &v.occurrences[0].id).unwrap();
        assert_eq!(
            d.recurrence_text.as_deref(),
            Some("Weekly on Monday, 3 times")
        );
        assert_eq!(d.meet_link.as_deref(), Some("https://meet.google.com/x"));
        assert_eq!(d.conference_label.as_deref(), Some("Google Meet"));
        assert_eq!(
            d.reminders,
            vec![Reminder {
                method: "popup".into(),
                minutes: 30
            }]
        );
        assert!(!d.use_default_reminders);
        assert_eq!(d.master_id.as_deref(), Some("m1"));
        assert_eq!(d.start, ts("2026-03-09T13:00:00Z"));
    }
}
