//! iCalendar (RFC 5545) parser for calendars subscribed by URL. See docs/99-decisiones.md
//! ("Calendarios iCal por URL"). Covers what Google's secret address exports: VEVENT with
//! DTSTART/DTEND (UTC, TZID or DATE), RRULE/EXDATE/RDATE, RECURRENCE-ID, STATUS, SUMMARY,
//! DESCRIPTION, LOCATION, ORGANIZER, ATTENDEE, TRANSP, SEQUENCE, CREATED, LAST-MODIFIED.

use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::json;

use crate::db::queries::events::EventRow;

/// One content line: `NAME;PARAM=V:VALUE`, unfolded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

impl Line {
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// Unfold (RFC 5545 section 3.1) and split into lines.
pub fn unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t')) {
            if let Some(last) = out.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        if !line.is_empty() {
            out.push(line.to_string());
        }
    }
    out
}

/// Parse one content line. Quoted parameter values may contain `:` and `;`.
pub fn parse_line(line: &str) -> Option<Line> {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();
    let mut key: Option<String> = None;
    let mut phase = 0; // 0 name, 1 param key, 2 param value
    for (i, c) in line.char_indices() {
        match (phase, c) {
            (_, '"') => in_quotes = !in_quotes,
            (0, ';') => {
                name = std::mem::take(&mut current);
                phase = 1;
            }
            (0, ':') => {
                name = std::mem::take(&mut current);
                return Some(Line {
                    name: name.to_ascii_uppercase(),
                    params,
                    value: line[i + 1..].to_string(),
                });
            }
            (1, '=') => {
                key = Some(std::mem::take(&mut current));
                phase = 2;
            }
            (2, ';') if !in_quotes => {
                params.push((
                    key.take().unwrap_or_default().to_ascii_uppercase(),
                    std::mem::take(&mut current),
                ));
                phase = 1;
            }
            (2, ':') if !in_quotes => {
                params.push((
                    key.take().unwrap_or_default().to_ascii_uppercase(),
                    std::mem::take(&mut current),
                ));
                return Some(Line {
                    name: name.to_ascii_uppercase(),
                    params,
                    value: line[i + 1..].to_string(),
                });
            }
            _ => current.push(c),
        }
    }
    None
}

/// Unescape a TEXT value (`\n`, `\,`, `\;`, `\\`).
pub fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A parsed VEVENT.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VEvent {
    pub lines: Vec<Line>,
}

impl VEvent {
    pub fn get(&self, name: &str) -> Option<&Line> {
        self.lines.iter().find(|l| l.name == name)
    }
    pub fn all(&self, name: &str) -> Vec<&Line> {
        self.lines.iter().filter(|l| l.name == name).collect()
    }
    pub fn text(&self, name: &str) -> Option<String> {
        self.get(name)
            .map(|l| unescape(&l.value))
            .filter(|s| !s.is_empty())
    }
}

/// Calendar-level information plus its events.
#[derive(Debug, Default)]
pub struct Calendar {
    pub name: Option<String>,
    pub time_zone: Option<String>,
    pub events: Vec<VEvent>,
}

pub fn parse(text: &str) -> Calendar {
    let mut cal = Calendar::default();
    let mut current: Option<VEvent> = None;
    let mut depth_other = 0usize;
    for raw in unfold(text) {
        let Some(line) = parse_line(&raw) else {
            continue;
        };
        match (line.name.as_str(), line.value.as_str()) {
            ("BEGIN", "VEVENT") => current = Some(VEvent::default()),
            ("END", "VEVENT") => {
                if let Some(ev) = current.take() {
                    cal.events.push(ev);
                }
            }
            ("BEGIN", _) if current.is_some() => depth_other += 1,
            ("END", _) if current.is_some() && depth_other > 0 => depth_other -= 1,
            ("X-WR-CALNAME", v) if current.is_none() => cal.name = Some(unescape(v)),
            ("X-WR-TIMEZONE", v) if current.is_none() => cal.time_zone = Some(v.to_string()),
            _ => {
                if let Some(ev) = current.as_mut() {
                    // Skip VALARM and other nested components.
                    if depth_other == 0 {
                        ev.lines.push(line);
                    }
                }
            }
        }
    }
    cal
}

/// A DTSTART/DTEND/RECURRENCE-ID value resolved to UTC seconds or a date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    Timed { ts: i64, tz: Option<String> },
    Date(String),
}

fn zone(name: &str) -> Option<chrono_tz::Tz> {
    name.trim_matches('"').parse().ok()
}

/// Resolve a date/date-time value. `default_tz` applies to floating values.
pub fn resolve(line: &Line, default_tz: Option<&str>) -> Option<When> {
    let v = line.value.trim();
    let is_date = line
        .param("VALUE")
        .is_some_and(|p| p.eq_ignore_ascii_case("DATE"))
        || (v.len() == 8 && v.chars().all(|c| c.is_ascii_digit()));
    if is_date {
        let d = NaiveDate::parse_from_str(v, "%Y%m%d").ok()?;
        return Some(When::Date(d.format("%Y-%m-%d").to_string()));
    }
    if let Some(z) = v.strip_suffix('Z') {
        let n = NaiveDateTime::parse_from_str(z, "%Y%m%dT%H%M%S").ok()?;
        return Some(When::Timed {
            ts: n.and_utc().timestamp(),
            tz: None,
        });
    }
    let n = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    let tz_name = line
        .param("TZID")
        .map(|t| t.trim_matches('"').to_string())
        .or_else(|| default_tz.map(str::to_string));
    let tz = tz_name.as_deref().and_then(zone).unwrap_or(chrono_tz::UTC);
    let ts = tz.from_local_datetime(&n).earliest()?.timestamp();
    Some(When::Timed {
        ts,
        tz: Some(tz.name().to_string()),
    })
}

fn stamp(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%Y%m%dT%H%M%SZ").to_string())
        .unwrap_or_default()
}

/// Recurrence lines as Google's API would send them: `RRULE:`, `EXDATE...`, `RDATE...`.
fn recurrence_lines(ev: &VEvent) -> Vec<String> {
    let mut out = Vec::new();
    for l in &ev.lines {
        if l.name == "RRULE" || l.name == "EXDATE" || l.name == "RDATE" {
            let params: String = l.params.iter().map(|(k, v)| format!(";{k}={v}")).collect();
            out.push(format!("{}{}:{}", l.name, params, l.value));
        }
    }
    out
}

fn mailto(v: &str) -> String {
    v.strip_prefix("mailto:")
        .or_else(|| v.strip_prefix("MAILTO:"))
        .unwrap_or(v)
        .to_string()
}

fn response_status(partstat: Option<&str>) -> &'static str {
    match partstat.map(|p| p.to_ascii_uppercase()).as_deref() {
        Some("ACCEPTED") => "accepted",
        Some("DECLINED") => "declined",
        Some("TENTATIVE") => "tentative",
        _ => "needsAction",
    }
}

/// Map a VEVENT to an `events` row. `self_email` marks the attendee entry that is "me".
pub fn to_row(
    account_id: &str,
    calendar_id: &str,
    ev: &VEvent,
    default_tz: Option<&str>,
    self_email: Option<&str>,
) -> Option<EventRow> {
    let uid = ev.text("UID")?;
    let start = resolve(ev.get("DTSTART")?, default_tz)?;
    let end = ev.get("DTEND").and_then(|l| resolve(l, default_tz));
    let recurrence_id = ev.get("RECURRENCE-ID").and_then(|l| resolve(l, default_tz));
    let (id, recurring_event_id, original_start_ts, original_start_date) = match &recurrence_id {
        Some(When::Timed { ts, .. }) => (
            format!("{uid}_{}", stamp(*ts)),
            Some(uid.clone()),
            Some(*ts),
            None,
        ),
        Some(When::Date(d)) => (
            format!("{uid}_{}", d.replace('-', "")),
            Some(uid.clone()),
            None,
            Some(d.clone()),
        ),
        None => (uid.clone(), None, None, None),
    };
    let status = match ev.text("STATUS").map(|s| s.to_ascii_uppercase()).as_deref() {
        Some("CANCELLED") => "cancelled",
        Some("TENTATIVE") => "tentative",
        _ => "confirmed",
    };
    let mut row = EventRow {
        account_id: account_id.into(),
        calendar_id: calendar_id.into(),
        id,
        ical_uid: Some(uid.clone()),
        etag: ev.text("SEQUENCE"),
        status: status.into(),
        summary: ev.text("SUMMARY"),
        description: ev.text("DESCRIPTION"),
        location: ev.text("LOCATION"),
        recurring_event_id,
        original_start_ts,
        original_start_date,
        transparency: ev.text("TRANSP").map(|t| {
            if t.eq_ignore_ascii_case("TRANSPARENT") {
                "transparent".into()
            } else {
                "opaque".into()
            }
        }),
        event_type: "default".into(),
        reminders: r#"{"useDefault":false}"#.into(),
        attendees: "[]".into(),
        ..Default::default()
    };
    match (&start, &end) {
        (When::Date(s), e) => {
            row.all_day = true;
            row.start_date = Some(s.clone());
            row.end_date = Some(match e {
                Some(When::Date(d)) => d.clone(),
                _ => NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| {
                    (d + chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string()
                })?,
            });
        }
        (When::Timed { ts, tz }, e) => {
            row.all_day = false;
            row.start_ts = Some(*ts);
            row.end_ts = Some(match e {
                Some(When::Timed { ts: et, .. }) => *et,
                _ => *ts,
            });
            row.time_zone = tz.clone().or_else(|| default_tz.map(str::to_string));
        }
    }
    let rec = recurrence_lines(ev);
    if !rec.is_empty() && row.recurring_event_id.is_none() {
        row.recurrence = Some(rec);
    }
    if let Some(org) = ev.get("ORGANIZER") {
        let email = mailto(&org.value);
        row.organizer_self = self_email.is_some_and(|me| me.eq_ignore_ascii_case(&email));
        row.organizer_email = Some(email);
    }
    let attendees: Vec<serde_json::Value> = ev
        .all("ATTENDEE")
        .iter()
        .map(|a| {
            let email = mailto(&a.value);
            let is_self = self_email.is_some_and(|me| me.eq_ignore_ascii_case(&email));
            let mut o =
                json!({ "email": email, "responseStatus": response_status(a.param("PARTSTAT")) });
            if let Some(cn) = a.param("CN") {
                o["displayName"] = json!(cn.trim_matches('"'));
            }
            if a.param("ROLE")
                .is_some_and(|r| r.eq_ignore_ascii_case("OPT-PARTICIPANT"))
            {
                o["optional"] = json!(true);
            }
            if is_self {
                o["self"] = json!(true);
            }
            if row
                .organizer_email
                .as_deref()
                .is_some_and(|oe| oe.eq_ignore_ascii_case(&mailto(&a.value)))
            {
                o["organizer"] = json!(true);
            }
            o
        })
        .collect();
    row.attendees = serde_json::to_string(&attendees).unwrap_or_else(|_| "[]".into());
    row.created_ts = ev
        .get("CREATED")
        .and_then(|l| resolve(l, None))
        .and_then(|w| {
            if let When::Timed { ts, .. } = w {
                Some(ts)
            } else {
                None
            }
        });
    row.updated_ts = ev
        .get("LAST-MODIFIED")
        .and_then(|l| resolve(l, None))
        .and_then(|w| {
            if let When::Timed { ts, .. } = w {
                Some(ts)
            } else {
                None
            }
        });
    // Google Meet links exported as X-GOOGLE-CONFERENCE; feeds written by hand (an Apps
    // Script) may carry the link in LOCATION or DESCRIPTION instead (docs/99, 2026-09-15).
    row.hangout_link = ev
        .text("X-GOOGLE-CONFERENCE")
        .filter(|l| l.starts_with("https://"))
        .or_else(|| row.location.as_deref().and_then(meet_link_in))
        .or_else(|| row.description.as_deref().and_then(meet_link_in));
    Some(row)
}

/// First `https://meet.google.com/...` link inside free text.
pub fn meet_link_in(text: &str) -> Option<String> {
    let start = text.find("https://meet.google.com/")?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | ')' | ',' | ';'))
        .unwrap_or(rest.len());
    Some(rest[..end].trim_end_matches('.').to_string())
}

#[cfg(test)]
mod meet_tests {
    #[test]
    fn meet_link_from_text() {
        assert_eq!(
            super::meet_link_in("Join: https://meet.google.com/abc-defg-hij, then dial in."),
            Some("https://meet.google.com/abc-defg-hij".into())
        );
        assert_eq!(super::meet_link_in("Room 1"), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nPRODID:-//Google Inc//Google Calendar 70.9054//EN\r\nVERSION:2.0\r\nX-WR-CALNAME:RappiCard\r\nX-WR-TIMEZONE:America/Mexico_City\r\nBEGIN:VTIMEZONE\r\nTZID:America/Mexico_City\r\nEND:VTIMEZONE\r\nBEGIN:VEVENT\r\nDTSTART;TZID=America/Mexico_City:20260914T090000\r\nDTEND;TZID=America/Mexico_City:20260914T093000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE\r\nEXDATE;TZID=America/Mexico_City:20260916T090000\r\nDTSTAMP:20260914T120000Z\r\nORGANIZER;CN=Boss:mailto:boss@rappicard.mx\r\nUID:abc123@google.com\r\nATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED;CN=Shlomo\r\n  Serber;X-NUM-GUESTS=0:mailto:shlomo.serber.ext@rappicard.mx\r\nATTENDEE;ROLE=OPT-PARTICIPANT;PARTSTAT=NEEDS-ACTION:mailto:x@rappicard.mx\r\nCREATED:20260901T100000Z\r\nDESCRIPTION:Line one\\nLine two\\, with comma\r\nLAST-MODIFIED:20260910T100000Z\r\nLOCATION:Room 1\r\nSEQUENCE:2\r\nSTATUS:CONFIRMED\r\nSUMMARY:Daily sync\r\nTRANSP:OPAQUE\r\nX-GOOGLE-CONFERENCE:https://meet.google.com/abc-defg-hij\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-P0DT0H10M0S\r\nEND:VALARM\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260920\r\nDTEND;VALUE=DATE:20260921\r\nUID:allday@google.com\r\nSUMMARY:Off\r\nTRANSP:TRANSPARENT\r\nSTATUS:CONFIRMED\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nDTSTART;TZID=America/Mexico_City:20260921T100000\r\nDTEND;TZID=America/Mexico_City:20260921T103000\r\nRECURRENCE-ID;TZID=America/Mexico_City:20260921T090000\r\nUID:abc123@google.com\r\nSUMMARY:Daily sync (moved)\r\nSTATUS:CONFIRMED\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nDTSTART:20260922T150000Z\r\nDTEND:20260922T160000Z\r\nUID:utc@google.com\r\nSUMMARY:Cancelled one\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    #[test]
    fn parses_lines_with_quoted_params() {
        let l = parse_line("ATTENDEE;CN=\"Doe, John\";PARTSTAT=ACCEPTED:mailto:john@x.y").unwrap();
        assert_eq!(l.name, "ATTENDEE");
        assert_eq!(l.param("CN"), Some("Doe, John"));
        assert_eq!(l.value, "mailto:john@x.y");
        assert_eq!(unescape(r"a\nb\,c\;d\\e"), "a\nb,c;d\\e");
    }

    #[test]
    fn google_secret_feed_maps_to_rows() {
        let cal = parse(SAMPLE);
        assert_eq!(cal.name.as_deref(), Some("RappiCard"));
        assert_eq!(cal.time_zone.as_deref(), Some("America/Mexico_City"));
        assert_eq!(cal.events.len(), 4);
        let me = Some("shlomo.serber.ext@rappicard.mx");
        let rows: Vec<EventRow> = cal
            .events
            .iter()
            .filter_map(|e| to_row("acc", "cal", e, cal.time_zone.as_deref(), me))
            .collect();
        assert_eq!(rows.len(), 4);

        let master = &rows[0];
        assert_eq!(master.id, "abc123@google.com");
        // 09:00 Mexico City (UTC-6, no DST) = 15:00Z.
        assert_eq!(
            master.start_ts,
            Some(1_789_398_000 - 3600 * 3 + 3600 * 6 - 3600 * 3)
        );
        assert_eq!(master.end_ts.unwrap() - master.start_ts.unwrap(), 1800);
        assert_eq!(master.time_zone.as_deref(), Some("America/Mexico_City"));
        assert_eq!(
            master.recurrence.as_ref().unwrap(),
            &vec![
                "RRULE:FREQ=WEEKLY;BYDAY=MO,WE".to_string(),
                "EXDATE;TZID=America/Mexico_City:20260916T090000".to_string()
            ]
        );
        assert_eq!(
            master.description.as_deref(),
            Some("Line one\nLine two, with comma")
        );
        assert_eq!(master.organizer_email.as_deref(), Some("boss@rappicard.mx"));
        assert!(!master.organizer_self);
        let att: serde_json::Value = serde_json::from_str(&master.attendees).unwrap();
        assert_eq!(att[0]["email"], "shlomo.serber.ext@rappicard.mx");
        assert_eq!(att[0]["self"], true);
        assert_eq!(att[0]["responseStatus"], "accepted");
        assert_eq!(att[0]["displayName"], "Shlomo Serber");
        assert_eq!(att[1]["optional"], true);
        assert_eq!(
            master.hangout_link.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
        assert_eq!(master.updated_ts, Some(1_789_034_400));
        assert!(!master.attendees.contains("VALARM"));

        let allday = &rows[1];
        assert!(allday.all_day);
        assert_eq!(allday.start_date.as_deref(), Some("2026-09-20"));
        assert_eq!(allday.end_date.as_deref(), Some("2026-09-21"));
        assert_eq!(allday.transparency.as_deref(), Some("transparent"));

        let exc = &rows[2];
        assert_eq!(exc.recurring_event_id.as_deref(), Some("abc123@google.com"));
        assert_eq!(exc.id, "abc123@google.com_20260921T150000Z");
        assert_eq!(exc.original_start_ts, Some(1_790_002_800));
        assert!(exc.recurrence.is_none());

        let cancelled = &rows[3];
        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(cancelled.start_ts, Some(1_790_089_200));
        assert_eq!(cancelled.time_zone.as_deref(), Some("America/Mexico_City"));
    }

    #[test]
    fn expands_through_the_recurrence_engine() {
        let cal = parse(SAMPLE);
        let mut conn = crate::db::open_memory().unwrap();
        conn.execute("INSERT INTO accounts (id, kind, display_name, created_at) VALUES ('acc','ical','RappiCard',0)", []).unwrap();
        conn.execute("INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role) VALUES ('cal','acc','RappiCard','#000','#fff','reader')", []).unwrap();
        for e in &cal.events {
            let row = to_row("acc", "cal", e, cal.time_zone.as_deref(), None).unwrap();
            crate::db::queries::events::upsert_event(&conn, &row).unwrap();
        }
        let w = crate::recurrence::expand::Window {
            from_ts: 1_789_000_000,
            to_ts: 1_791_000_000,
        };
        crate::recurrence::expand::rebuild_calendar(&mut conn, "acc", "cal", w).unwrap();
        let starts: Vec<i64> = conn
            .prepare("SELECT start_ts FROM occurrences WHERE master_id='abc123@google.com' ORDER BY start_ts")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        // Mon 14 (15:00Z), Wed 16 excluded, Mon 21 moved to 16:00Z, Wed 23, ...
        assert_eq!(starts[0], 1_789_398_000);
        assert!(
            !starts.contains(&(1_789_398_000 + 2 * 86_400)),
            "EXDATE honoured"
        );
        assert!(
            starts.contains(&1_790_006_400),
            "moved exception at 10:00 Mexico City"
        );
        assert!(
            !starts.contains(&1_790_002_800),
            "original instance replaced"
        );
    }
}
