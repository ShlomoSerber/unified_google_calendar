//! Queries over `settings` (JSON values by key). See docs/03-modelo-de-datos.md section 1.

use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::AppError;

pub fn get_raw(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key=?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

pub fn get<T: DeserializeOwned>(conn: &Connection, key: &str) -> Result<Option<T>, AppError> {
    match get_raw(conn, key)? {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

pub fn get_or<T: DeserializeOwned>(
    conn: &Connection,
    key: &str,
    default: T,
) -> Result<T, AppError> {
    Ok(get(conn, key)?.unwrap_or(default))
}

pub fn set<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string(value)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, json],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, key: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM settings WHERE key=?1", params![key])?;
    Ok(())
}

pub fn log_sync(
    conn: &Connection,
    account_id: Option<&str>,
    calendar_id: Option<&str>,
    kind: &str,
    detail: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO sync_log (at, account_id, calendar_id, kind, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![crate::db::now_ts(), account_id, calendar_id, kind, detail],
    )?;
    // Keep the log bounded.
    conn.execute(
        "DELETE FROM sync_log WHERE id < (SELECT max(id) FROM sync_log) - 5000",
        [],
    )?;
    Ok(())
}
