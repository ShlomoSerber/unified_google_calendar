//! Queries over `calendars`. See docs/03-modelo-de-datos.md sections 1 and 2.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CalendarRow {
    pub id: String,
    pub account_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub color_bg: String,
    pub color_fg: String,
    pub access_role: String,
    pub is_primary: bool,
    pub visible: bool,
    pub hidden_remote: bool,
    pub deleted: bool,
    pub time_zone: Option<String>,
    pub default_reminders: String,
    pub sync_token: Option<String>,
    pub full_sync_done: bool,
    pub sort_order: i64,
}

impl CalendarRow {
    pub const COLUMNS: &'static str = "id, account_id, summary, description, color_bg, color_fg, access_role, is_primary, visible, hidden_remote, \
        deleted, time_zone, default_reminders, sync_token, full_sync_done, sort_order";

    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<CalendarRow> {
        Ok(CalendarRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            summary: r.get(2)?,
            description: r.get(3)?,
            color_bg: r.get(4)?,
            color_fg: r.get(5)?,
            access_role: r.get(6)?,
            is_primary: r.get::<_, i64>(7)? != 0,
            visible: r.get::<_, i64>(8)? != 0,
            hidden_remote: r.get::<_, i64>(9)? != 0,
            deleted: r.get::<_, i64>(10)? != 0,
            time_zone: r.get(11)?,
            default_reminders: r.get(12)?,
            sync_token: r.get(13)?,
            full_sync_done: r.get::<_, i64>(14)? != 0,
            sort_order: r.get(15)?,
        })
    }

    pub fn can_write(&self) -> bool {
        self.access_role == "owner" || self.access_role == "writer"
    }
}

pub fn list_calendars(conn: &Connection) -> Result<Vec<CalendarRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM calendars WHERE deleted=0 ORDER BY account_id, is_primary DESC, sort_order, summary",
        CalendarRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map([], CalendarRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn list_calendars_of_account(
    conn: &Connection,
    account_id: &str,
) -> Result<Vec<CalendarRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM calendars WHERE account_id=?1 AND deleted=0 ORDER BY is_primary DESC, sort_order, summary",
        CalendarRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map(params![account_id], CalendarRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_calendar(
    conn: &Connection,
    account_id: &str,
    id: &str,
) -> Result<Option<CalendarRow>, AppError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {} FROM calendars WHERE account_id=?1 AND id=?2",
                CalendarRow::COLUMNS
            ),
            params![account_id, id],
            CalendarRow::from_row,
        )
        .optional()?)
}

/// Insert a calendar or update the remote-owned columns. `visible` is set only on first import
/// (docs/03 section 2: after that the user's checkbox in the app wins).
pub fn upsert_calendar(conn: &Connection, c: &CalendarRow) -> Result<(), AppError> {
    conn.execute(
        &format!(
            "INSERT INTO calendars ({}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
             ON CONFLICT(account_id, id) DO UPDATE SET summary=excluded.summary, description=excluded.description,
               color_bg=excluded.color_bg, color_fg=excluded.color_fg, access_role=excluded.access_role,
               is_primary=excluded.is_primary, hidden_remote=excluded.hidden_remote, deleted=excluded.deleted,
               time_zone=excluded.time_zone, default_reminders=excluded.default_reminders, sort_order=excluded.sort_order",
            CalendarRow::COLUMNS
        ),
        params![
            c.id,
            c.account_id,
            c.summary,
            c.description,
            c.color_bg,
            c.color_fg,
            c.access_role,
            c.is_primary as i64,
            c.visible as i64,
            c.hidden_remote as i64,
            c.deleted as i64,
            c.time_zone,
            c.default_reminders,
            c.sync_token,
            c.full_sync_done as i64,
            c.sort_order,
        ],
    )?;
    Ok(())
}

pub fn set_visible(
    conn: &Connection,
    account_id: &str,
    id: &str,
    visible: bool,
) -> Result<usize, AppError> {
    Ok(conn.execute(
        "UPDATE calendars SET visible=?3 WHERE account_id=?1 AND id=?2",
        params![account_id, id, visible as i64],
    )?)
}

pub fn set_sync_token(
    conn: &Connection,
    account_id: &str,
    id: &str,
    token: Option<&str>,
    full_sync_done: bool,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE calendars SET sync_token=?3, full_sync_done=?4 WHERE account_id=?1 AND id=?2",
        params![account_id, id, token, full_sync_done as i64],
    )?;
    Ok(())
}

/// Mark a calendar deleted and drop its events and occurrences.
pub fn mark_deleted(conn: &Connection, account_id: &str, id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM events WHERE account_id=?1 AND calendar_id=?2",
        params![account_id, id],
    )?;
    conn.execute(
        "UPDATE calendars SET deleted=1, sync_token=NULL, full_sync_done=0 WHERE account_id=?1 AND id=?2",
        params![account_id, id],
    )?;
    Ok(())
}

/// Drop all events of a calendar and its sync token (410 Gone handling, docs/05 section 2.2).
pub fn wipe_events(conn: &Connection, account_id: &str, id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM events WHERE account_id=?1 AND calendar_id=?2",
        params![account_id, id],
    )?;
    set_sync_token(conn, account_id, id, None, false)
}

/// Create a local calendar (only `local-personal` exists in version 1, but the helper keeps the
/// insert in one place).
pub fn insert_local_calendar(
    conn: &Connection,
    id: &str,
    summary: &str,
    color_bg: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible, default_reminders)
         VALUES (?1, 'local', ?2, ?3, '#ffffff', 'owner', 0, 1, '[{\"method\":\"popup\",\"minutes\":10}]')",
        params![id, summary, color_bg],
    )?;
    Ok(())
}
