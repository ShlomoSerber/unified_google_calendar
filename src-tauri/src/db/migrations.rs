//! Migrations applied in order and recorded in `PRAGMA user_version`. See docs/03 section 10.

use rusqlite::Connection;

use crate::error::AppError;

/// Embedded migrations. Never edit an applied one; add a new file instead.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("schema/0001_init.sql"))];

pub fn migrate(conn: &mut Connection) -> Result<(), AppError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        tracing::info!(version, "database migrated");
    }
    Ok(())
}

pub fn user_version(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}
