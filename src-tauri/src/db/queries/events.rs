//! Queries over `events` and `occurrences`. See docs/03-modelo-de-datos.md sections 1, 3 and 4.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

/// One row of `events`. Column order matches [`EventRow::COLUMNS`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventRow {
    pub account_id: String,
    pub calendar_id: String,
    pub id: String,
    pub ical_uid: Option<String>,
    pub etag: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub color_id: Option<String>,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub all_day: bool,
    pub time_zone: Option<String>,
    pub recurrence: Option<Vec<String>>,
    pub recurring_event_id: Option<String>,
    pub original_start_ts: Option<i64>,
    pub original_start_date: Option<String>,
    pub organizer_email: Option<String>,
    pub organizer_self: bool,
    pub attendees: String,
    pub reminders: String,
    pub hangout_link: Option<String>,
    pub conference: Option<String>,
    pub html_link: Option<String>,
    pub transparency: Option<String>,
    pub visibility: Option<String>,
    pub event_type: String,
    pub guests_can_modify: bool,
    pub created_ts: Option<i64>,
    pub updated_ts: Option<i64>,
    pub raw: Option<String>,
}

impl EventRow {
    pub const COLUMNS: &'static str = "account_id, calendar_id, id, ical_uid, etag, status, summary, description, location, color_id, \
        start_ts, end_ts, start_date, end_date, all_day, time_zone, recurrence, recurring_event_id, original_start_ts, original_start_date, \
        organizer_email, organizer_self, attendees, reminders, hangout_link, conference, html_link, transparency, visibility, event_type, \
        guests_can_modify, created_ts, updated_ts, raw";

    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<EventRow> {
        let recurrence: Option<String> = r.get(16)?;
        let recurrence = recurrence.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
        Ok(EventRow {
            account_id: r.get(0)?,
            calendar_id: r.get(1)?,
            id: r.get(2)?,
            ical_uid: r.get(3)?,
            etag: r.get(4)?,
            status: r.get(5)?,
            summary: r.get(6)?,
            description: r.get(7)?,
            location: r.get(8)?,
            color_id: r.get(9)?,
            start_ts: r.get(10)?,
            end_ts: r.get(11)?,
            start_date: r.get(12)?,
            end_date: r.get(13)?,
            all_day: r.get::<_, i64>(14)? != 0,
            time_zone: r.get(15)?,
            recurrence,
            recurring_event_id: r.get(17)?,
            original_start_ts: r.get(18)?,
            original_start_date: r.get(19)?,
            organizer_email: r.get(20)?,
            organizer_self: r.get::<_, i64>(21)? != 0,
            attendees: r.get(22)?,
            reminders: r.get(23)?,
            hangout_link: r.get(24)?,
            conference: r.get(25)?,
            html_link: r.get(26)?,
            transparency: r.get(27)?,
            visibility: r.get(28)?,
            event_type: r.get(29)?,
            guests_can_modify: r.get::<_, i64>(30)? != 0,
            created_ts: r.get(31)?,
            updated_ts: r.get(32)?,
            raw: r.get(33)?,
        })
    }

    pub fn is_recurring_master(&self) -> bool {
        self.recurrence.as_ref().is_some_and(|r| !r.is_empty()) && self.recurring_event_id.is_none()
    }

    pub fn is_exception(&self) -> bool {
        self.recurring_event_id.is_some()
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == "cancelled"
    }
}

/// Insert or replace a full row. Used by sync (from Google) and by local writes.
pub fn upsert_event(conn: &Connection, e: &EventRow) -> Result<(), AppError> {
    let recurrence = match &e.recurrence {
        Some(lines) if !lines.is_empty() => Some(serde_json::to_string(lines)?),
        _ => None,
    };
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO events ({}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34)",
            EventRow::COLUMNS
        ),
        params![
            e.account_id,
            e.calendar_id,
            e.id,
            e.ical_uid,
            e.etag,
            e.status,
            e.summary,
            e.description,
            e.location,
            e.color_id,
            e.start_ts,
            e.end_ts,
            e.start_date,
            e.end_date,
            e.all_day as i64,
            e.time_zone,
            recurrence,
            e.recurring_event_id,
            e.original_start_ts,
            e.original_start_date,
            e.organizer_email,
            e.organizer_self as i64,
            e.attendees,
            e.reminders,
            e.hangout_link,
            e.conference,
            e.html_link,
            e.transparency,
            e.visibility,
            e.event_type,
            e.guests_can_modify as i64,
            e.created_ts,
            e.updated_ts,
            e.raw,
        ],
    )?;
    Ok(())
}

pub fn get_event(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
) -> Result<Option<EventRow>, AppError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {} FROM events WHERE account_id=?1 AND calendar_id=?2 AND id=?3",
                EventRow::COLUMNS
            ),
            params![account_id, calendar_id, event_id],
            EventRow::from_row,
        )
        .optional()?)
}

/// Delete an event row. Occurrences cascade. Exceptions of a master are deleted explicitly
/// because the schema has no foreign key from exception to master.
pub fn delete_event(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
) -> Result<usize, AppError> {
    conn.execute(
        "DELETE FROM events WHERE account_id=?1 AND calendar_id=?2 AND recurring_event_id=?3",
        params![account_id, calendar_id, event_id],
    )?;
    Ok(conn.execute(
        "DELETE FROM events WHERE account_id=?1 AND calendar_id=?2 AND id=?3",
        params![account_id, calendar_id, event_id],
    )?)
}

/// Exceptions (rows with `recurring_event_id = master_id`) of one master.
pub fn exceptions_of(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    master_id: &str,
) -> Result<Vec<EventRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM events WHERE account_id=?1 AND calendar_id=?2 AND recurring_event_id=?3",
        EventRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map(
            params![account_id, calendar_id, master_id],
            EventRow::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// All recurring masters of a calendar (for re-expansion after a full sync).
pub fn masters_of_calendar(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM events WHERE account_id=?1 AND calendar_id=?2 AND recurrence IS NOT NULL AND recurring_event_id IS NULL",
    )?;
    let ids = stmt
        .query_map(params![account_id, calendar_id], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

/// All non-recurring, non-exception events of a calendar.
pub fn simple_events_of_calendar(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM events WHERE account_id=?1 AND calendar_id=?2 AND recurrence IS NULL AND recurring_event_id IS NULL",
    )?;
    let ids = stmt
        .query_map(params![account_id, calendar_id], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

/// One row of `occurrences`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceRow {
    pub id: String,
    pub account_id: String,
    pub calendar_id: String,
    pub event_id: String,
    pub master_id: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
    pub all_day: bool,
    pub status: String,
}

impl OccurrenceRow {
    pub const COLUMNS: &'static str =
        "id, account_id, calendar_id, event_id, master_id, start_ts, end_ts, all_day, status";

    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<OccurrenceRow> {
        Ok(OccurrenceRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            calendar_id: r.get(2)?,
            event_id: r.get(3)?,
            master_id: r.get(4)?,
            start_ts: r.get(5)?,
            end_ts: r.get(6)?,
            all_day: r.get::<_, i64>(7)? != 0,
            status: r.get(8)?,
        })
    }
}

pub fn insert_occurrence(conn: &Connection, o: &OccurrenceRow) -> Result<(), AppError> {
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO occurrences ({}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            OccurrenceRow::COLUMNS
        ),
        params![
            o.id,
            o.account_id,
            o.calendar_id,
            o.event_id,
            o.master_id,
            o.start_ts,
            o.end_ts,
            o.all_day as i64,
            o.status
        ],
    )?;
    Ok(())
}

pub fn get_occurrence(conn: &Connection, id: &str) -> Result<Option<OccurrenceRow>, AppError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {} FROM occurrences WHERE id=?1",
                OccurrenceRow::COLUMNS
            ),
            params![id],
            OccurrenceRow::from_row,
        )
        .optional()?)
}

/// Occurrences generated by a master or by the event itself, ordered by start.
pub fn occurrences_of_event(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
) -> Result<Vec<OccurrenceRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM occurrences WHERE account_id=?1 AND calendar_id=?2 AND (event_id=?3 OR master_id=?3) ORDER BY start_ts",
        OccurrenceRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map(
            params![account_id, calendar_id, event_id],
            OccurrenceRow::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Remove every occurrence that a master (and its exceptions) or a simple event produced.
pub fn delete_occurrences_of_event(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
) -> Result<usize, AppError> {
    Ok(conn.execute(
        "DELETE FROM occurrences WHERE account_id=?1 AND calendar_id=?2 AND (event_id=?3 OR master_id=?3)",
        params![account_id, calendar_id, event_id],
    )?)
}

/// Delete occurrences outside the data window (docs/03 section 3).
pub fn delete_occurrences_outside(
    conn: &Connection,
    from_ts: i64,
    to_ts: i64,
) -> Result<usize, AppError> {
    Ok(conn.execute(
        "DELETE FROM occurrences WHERE end_ts < ?1 OR start_ts > ?2",
        params![from_ts, to_ts],
    )?)
}

/// Occurrence id per docs/03 section 1: `account_id|calendar_id|event_id|start_ts` or start date for all-day.
pub fn occurrence_id(
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    start_ts: i64,
    all_day: bool,
) -> String {
    if all_day {
        let d = chrono::DateTime::from_timestamp(start_ts, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| start_ts.to_string());
        format!("{account_id}|{calendar_id}|{event_id}|{d}")
    } else {
        format!("{account_id}|{calendar_id}|{event_id}|{start_ts}")
    }
}

/// Split an occurrence id back into `(account_id, calendar_id, event_id)`.
pub fn parse_occurrence_id(id: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = id.splitn(4, '|').collect();
    if parts.len() != 4 {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}
