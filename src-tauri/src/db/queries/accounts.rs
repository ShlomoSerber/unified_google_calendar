//! Queries over `accounts`. See docs/03-modelo-de-datos.md section 1.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub id: String,
    pub kind: String,
    pub email: Option<String>,
    pub display_name: String,
    pub sort_order: i64,
    pub sync_state: String,
    pub sync_error: Option<String>,
    pub last_sync_at: Option<i64>,
    pub calendar_list_sync_token: Option<String>,
    pub created_at: i64,
}

impl AccountRow {
    pub const COLUMNS: &'static str =
        "id, kind, email, display_name, sort_order, sync_state, sync_error, last_sync_at, calendar_list_sync_token, created_at";

    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<AccountRow> {
        Ok(AccountRow {
            id: r.get(0)?,
            kind: r.get(1)?,
            email: r.get(2)?,
            display_name: r.get(3)?,
            sort_order: r.get(4)?,
            sync_state: r.get(5)?,
            sync_error: r.get(6)?,
            last_sync_at: r.get(7)?,
            calendar_list_sync_token: r.get(8)?,
            created_at: r.get(9)?,
        })
    }

    pub fn is_local(&self) -> bool {
        self.kind == "local"
    }
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM accounts ORDER BY sort_order, created_at",
        AccountRow::COLUMNS
    ))?;
    let rows = stmt
        .query_map([], AccountRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_account(conn: &Connection, id: &str) -> Result<Option<AccountRow>, AppError> {
    Ok(conn
        .query_row(
            &format!("SELECT {} FROM accounts WHERE id=?1", AccountRow::COLUMNS),
            params![id],
            AccountRow::from_row,
        )
        .optional()?)
}

/// Insert a Google account or update its email/name if it already exists.
pub fn upsert_google_account(
    conn: &Connection,
    id: &str,
    email: &str,
    display_name: &str,
) -> Result<(), AppError> {
    let next_order: i64 = conn.query_row(
        "SELECT coalesce(max(sort_order), 0) + 1 FROM accounts",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO accounts (id, kind, email, display_name, sort_order, sync_state, created_at)
         VALUES (?1, 'google', ?2, ?3, ?4, 'idle', ?5)
         ON CONFLICT(id) DO UPDATE SET email=excluded.email, display_name=excluded.display_name, sync_state='idle', sync_error=NULL",
        params![id, email, display_name, next_order, crate::db::now_ts()],
    )?;
    Ok(())
}

pub fn delete_account(conn: &Connection, id: &str) -> Result<usize, AppError> {
    Ok(conn.execute(
        "DELETE FROM accounts WHERE id=?1 AND kind='google'",
        params![id],
    )?)
}

pub fn set_sync_state(
    conn: &Connection,
    id: &str,
    state: &str,
    error: Option<&str>,
) -> Result<(), AppError> {
    let last = if state == "idle" {
        Some(crate::db::now_ts())
    } else {
        None
    };
    conn.execute(
        "UPDATE accounts SET sync_state=?2, sync_error=?3, last_sync_at=coalesce(?4, last_sync_at) WHERE id=?1",
        params![id, state, error, last],
    )?;
    Ok(())
}

pub fn set_calendar_list_sync_token(
    conn: &Connection,
    id: &str,
    token: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE accounts SET calendar_list_sync_token=?2 WHERE id=?1",
        params![id, token],
    )?;
    Ok(())
}
