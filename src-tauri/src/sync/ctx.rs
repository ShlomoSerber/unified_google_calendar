//! Dependencies of the sync engine, injectable for tests. See docs/02-arquitectura.md section 8.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::commands::types::{CalendarUpdated, SyncStatus};
use crate::db::DbHandle;
use crate::error::AppError;
use crate::google::Client;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Provides a valid access token for an account (refreshing when needed).
pub trait TokenSource: Send + Sync {
    fn token<'a>(&'a self, account_id: &'a str) -> BoxFuture<'a, Result<String, AppError>>;
}

/// Sink for the events the engine emits to the UI.
pub trait SyncEvents: Send + Sync {
    fn calendar_updated(&self, payload: CalendarUpdated);
    fn sync_status(&self, payload: SyncStatus);
}

#[derive(Clone)]
pub struct SyncCtx {
    pub db: DbHandle,
    pub client: Client,
    pub tokens: Arc<dyn TokenSource>,
    pub events: Arc<dyn SyncEvents>,
}

/// Production token source: the OAuth state of `auth`.
pub struct OAuthTokens;

impl TokenSource for OAuthTokens {
    fn token<'a>(&'a self, account_id: &'a str) -> BoxFuture<'a, Result<String, AppError>> {
        Box::pin(crate::auth::access_token(account_id))
    }
}

/// Production sink: `app.emit`.
pub struct TauriEvents(pub tauri::AppHandle);

impl SyncEvents for TauriEvents {
    fn calendar_updated(&self, payload: CalendarUpdated) {
        use tauri::Emitter;
        let _ = self
            .0
            .emit(crate::commands::types::EVENT_CALENDAR_UPDATED, &payload);
    }

    fn sync_status(&self, payload: SyncStatus) {
        use tauri::Emitter;
        let _ = self
            .0
            .emit(crate::commands::types::EVENT_SYNC_STATUS, &payload);
    }
}

/// Test doubles: a fixed token and an in-memory list of emitted events.
pub mod test_support {
    use std::sync::Mutex;

    use super::*;

    pub struct FixedToken(pub String);

    impl TokenSource for FixedToken {
        fn token<'a>(&'a self, _account_id: &'a str) -> BoxFuture<'a, Result<String, AppError>> {
            Box::pin(async move { Ok(self.0.clone()) })
        }
    }

    #[derive(Default)]
    pub struct Recorder {
        pub updates: Mutex<Vec<CalendarUpdated>>,
        pub statuses: Mutex<Vec<SyncStatus>>,
    }

    impl SyncEvents for Recorder {
        fn calendar_updated(&self, payload: CalendarUpdated) {
            self.updates.lock().map(|mut v| v.push(payload)).ok();
        }

        fn sync_status(&self, payload: SyncStatus) {
            self.statuses.lock().map(|mut v| v.push(payload)).ok();
        }
    }
}
