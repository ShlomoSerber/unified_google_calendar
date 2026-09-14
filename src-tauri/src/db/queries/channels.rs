//! Queries over `channels` (push notification channels). See docs/03 section 1 and docs/05 section 3.3.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRow {
    pub id: String,
    pub account_id: String,
    /// `None` for the calendarList channel.
    pub calendar_id: Option<String>,
    pub resource_id: String,
    pub token: String,
    pub expiration_ts: i64,
    pub created_at: i64,
}

impl ChannelRow {
    pub const COLUMNS: &'static str =
        "id, account_id, calendar_id, resource_id, token, expiration_ts, created_at";

    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<ChannelRow> {
        Ok(ChannelRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            calendar_id: r.get(2)?,
            resource_id: r.get(3)?,
            token: r.get(4)?,
            expiration_ts: r.get(5)?,
            created_at: r.get(6)?,
        })
    }
}

pub fn insert(conn: &Connection, c: &ChannelRow) -> Result<(), AppError> {
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO channels ({}) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            ChannelRow::COLUMNS
        ),
        params![
            c.id,
            c.account_id,
            c.calendar_id,
            c.resource_id,
            c.token,
            c.expiration_ts,
            c.created_at
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<ChannelRow>, AppError> {
    Ok(conn
        .query_row(
            &format!("SELECT {} FROM channels WHERE id=?1", ChannelRow::COLUMNS),
            params![id],
            ChannelRow::from_row,
        )
        .optional()?)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM channels WHERE id=?1", params![id])?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<ChannelRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM channels ORDER BY account_id, calendar_id",
        ChannelRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map([], ChannelRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn list_of_account(conn: &Connection, account_id: &str) -> Result<Vec<ChannelRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM channels WHERE account_id=?1",
        ChannelRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map(params![account_id], ChannelRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Channel of one resource (calendar events, or the calendarList when `calendar_id` is None).
pub fn find_for(
    conn: &Connection,
    account_id: &str,
    calendar_id: Option<&str>,
) -> Result<Option<ChannelRow>, AppError> {
    let sql = match calendar_id {
        Some(_) => format!(
            "SELECT {} FROM channels WHERE account_id=?1 AND calendar_id=?2",
            ChannelRow::COLUMNS
        ),
        None => format!(
            "SELECT {} FROM channels WHERE account_id=?1 AND calendar_id IS NULL AND ?2 IS NULL",
            ChannelRow::COLUMNS
        ),
    };
    Ok(conn
        .query_row(&sql, params![account_id, calendar_id], ChannelRow::from_row)
        .optional()?)
}
