//! Dedicated database thread. See docs/02-arquitectura.md section 8.
//!
//! `rusqlite::Connection` is `Send` but not `Sync`, so one thread owns it and receives
//! closures over a channel. Callers await a `oneshot` for the result. Writes are therefore
//! serialized in arrival order, which is the ordering guarantee `docs/02` section 8 relies on.

use std::sync::mpsc;
use std::thread;

use rusqlite::Connection;

use crate::error::AppError;

type Job = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

#[derive(Clone)]
pub struct DbHandle {
    tx: mpsc::Sender<Job>,
}

impl DbHandle {
    /// Spawn the worker thread owning `conn`.
    pub fn spawn(mut conn: Connection) -> DbHandle {
        let (tx, rx) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("db-worker".into())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    job(&mut conn);
                }
                tracing::debug!("db worker stopped");
            })
            .expect("failed to spawn the db worker thread");
        DbHandle { tx }
    }

    /// Run `f` on the database thread and await its result.
    pub async fn call<R, F>(&self, f: F) -> Result<R, AppError>
    where
        R: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
    {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let job: Job = Box::new(move |conn| {
            let _ = reply_tx.send(f(conn));
        });
        self.tx
            .send(job)
            .map_err(|_| AppError::Db("database worker is not running".into()))?;
        reply_rx
            .await
            .map_err(|_| AppError::Db("database worker dropped the request".into()))?
    }

    /// Blocking variant for code that is not async (tests, setup).
    pub fn call_blocking<R, F>(&self, f: F) -> Result<R, AppError>
    where
        R: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
    {
        let (reply_tx, reply_rx) = mpsc::channel();
        let job: Job = Box::new(move |conn| {
            let _ = reply_tx.send(f(conn));
        });
        self.tx
            .send(job)
            .map_err(|_| AppError::Db("database worker is not running".into()))?;
        reply_rx
            .recv()
            .map_err(|_| AppError::Db("database worker dropped the request".into()))?
    }
}
