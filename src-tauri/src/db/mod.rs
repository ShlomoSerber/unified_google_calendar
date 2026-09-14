//! SQLite access. See docs/02-arquitectura.md section 8 and docs/03-modelo-de-datos.md.
//!
//! One connection lives on a dedicated thread ([`worker`]). Every query in the app goes
//! through [`call`], which forwards to the process-wide handle set by [`init`].

pub mod migrations;
pub mod queries;
pub mod worker;

use std::path::Path;
use std::sync::OnceLock;

use rusqlite::Connection;

use crate::error::AppError;
pub use migrations::migrate;
pub use worker::DbHandle;

static HANDLE: OnceLock<DbHandle> = OnceLock::new();

/// Open (or create) the database file with the pragmas of docs/03 and run migrations.
pub fn open(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// In-memory database with the same pragmas and schema, for tests.
pub fn open_memory() -> Result<Connection, AppError> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<(), AppError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

/// Open the database at `path`, start the worker and register it as the global handle.
/// Calling it twice returns the existing handle.
pub fn init(path: &Path) -> Result<DbHandle, AppError> {
    if let Some(h) = HANDLE.get() {
        return Ok(h.clone());
    }
    let conn = open(path)?;
    let handle = DbHandle::spawn(conn);
    let _ = HANDLE.set(handle.clone());
    Ok(handle)
}

/// Register an already-open connection as the global handle (tests).
pub fn init_with(conn: Connection) -> DbHandle {
    if let Some(h) = HANDLE.get() {
        return h.clone();
    }
    let handle = DbHandle::spawn(conn);
    let _ = HANDLE.set(handle.clone());
    handle
}

pub fn handle() -> Result<DbHandle, AppError> {
    HANDLE
        .get()
        .cloned()
        .ok_or_else(|| AppError::Db("database not initialised".into()))
}

/// Run `f` on the database thread. Signature from docs/08 section 12.
pub async fn call<R, F>(f: F) -> Result<R, AppError>
where
    R: Send + 'static,
    F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
{
    handle()?.call(f).await
}

/// Current Unix time in seconds.
pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_schema_and_local_account() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let conn = open(&path).unwrap();
        assert_eq!(migrations::user_version(&conn).unwrap(), 1);
        let kind: String = conn
            .query_row("SELECT kind FROM accounts WHERE id = 'local'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kind, "local");
        let (color, reminders): (String, String) = conn
            .query_row(
                "SELECT color_bg, default_reminders FROM calendars WHERE account_id='local' AND id='local-personal'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(color, "#f4511e");
        assert_eq!(reminders, r#"[{"method":"popup","minutes":10}]"#);
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn migrating_twice_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        {
            let _ = open(&path).unwrap();
        }
        let mut conn = open(&path).unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(migrations::user_version(&conn).unwrap(), 1);
        let n: i64 = conn.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn worker_runs_closures_in_order() {
        let conn = open_memory().unwrap();
        let h = DbHandle::spawn(conn);
        h.call(|c| {
            c.execute("INSERT INTO settings (key, value) VALUES ('a', '1')", [])?;
            Ok(())
        })
        .await
        .unwrap();
        let v: String = h
            .call(|c| Ok(c.query_row("SELECT value FROM settings WHERE key='a'", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(v, "1");
        let err = h.call(|_| Err::<(), _>(AppError::Db("boom".into()))).await.unwrap_err();
        assert!(matches!(err, AppError::Db(_)));
    }
}
