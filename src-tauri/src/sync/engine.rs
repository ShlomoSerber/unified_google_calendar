//! Sync orchestration. See docs/02-arquitectura.md sections 2, 3.1, 3.4 and 8, and
//! docs/05-sincronizacion.md sections 3.4 and 6.
//!
//! One `SyncTick` per calendar (or per account when `calendar_id` is `None`) enters an mpsc
//! channel of capacity 256. The consumer deduplicates pending ticks and runs each incremental
//! under the calendar's mutex so push, poll and wake-up catch-up never overlap on one token.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use tokio::sync::{mpsc, Mutex};

use crate::commands::types::SyncStatus;
use crate::db::queries::{accounts, calendars, settings};
use crate::error::AppError;
use crate::sync::calendar_list::sync_calendar_list;
use crate::sync::ctx::SyncCtx;
use crate::sync::full::full_sync_calendar;
use crate::sync::incremental::incremental_calendar;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyncTick {
    pub account_id: String,
    pub calendar_id: Option<String>,
}

pub const TICK_CAPACITY: usize = 256;

type LockMap<K> = Mutex<HashMap<K, Arc<Mutex<()>>>>;

pub struct Engine {
    pub ctx: SyncCtx,
    locks: LockMap<(String, String)>,
    account_locks: LockMap<String>,
    tick_tx: mpsc::Sender<SyncTick>,
}

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();

impl Engine {
    pub fn new(ctx: SyncCtx) -> (Arc<Engine>, mpsc::Receiver<SyncTick>) {
        let (tick_tx, rx) = mpsc::channel(TICK_CAPACITY);
        let engine = Arc::new(Engine {
            ctx,
            locks: Mutex::new(HashMap::new()),
            account_locks: Mutex::new(HashMap::new()),
            tick_tx,
        });
        (engine, rx)
    }

    pub fn ticks(&self) -> mpsc::Sender<SyncTick> {
        self.tick_tx.clone()
    }

    async fn calendar_lock(&self, account_id: &str, calendar_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry((account_id.into(), calendar_id.into()))
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn account_lock(&self, account_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.account_locks.lock().await;
        map.entry(account_id.into())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn set_state(&self, account_id: &str, state: &str, message: Option<String>) {
        let (acc, st, msg) = (account_id.to_string(), state.to_string(), message.clone());
        let _ = self
            .ctx
            .db
            .call(move |c| accounts::set_sync_state(c, &acc, &st, msg.as_deref()))
            .await;
        self.ctx.events.sync_status(SyncStatus {
            account_id: account_id.into(),
            state: state.into(),
            message,
            at: crate::db::now_ts(),
        });
    }

    /// Incremental (or first full) sync of one calendar under its mutex.
    pub async fn sync_calendar(&self, account_id: &str, calendar_id: &str) -> Result<(), AppError> {
        let lock = self.calendar_lock(account_id, calendar_id).await;
        let _g = lock.lock().await;
        incremental_calendar(&self.ctx, account_id, calendar_id).await
    }

    /// Full sync of one calendar under its mutex.
    pub async fn full_sync(&self, account_id: &str, calendar_id: &str) -> Result<(), AppError> {
        let lock = self.calendar_lock(account_id, calendar_id).await;
        let _g = lock.lock().await;
        full_sync_calendar(&self.ctx, account_id, calendar_id).await
    }

    /// calendarList then every calendar of the account, sequentially (docs/05 section 6).
    /// Sets `sync_state` and logs errors instead of surfacing them as dialogs.
    pub async fn sync_account(&self, account_id: &str, reason: &str) -> Result<(), AppError> {
        let lock = self.account_lock(account_id).await;
        let _g = lock.lock().await;
        self.set_state(account_id, "syncing", None).await;
        let result = self.sync_account_inner(account_id).await;
        match &result {
            Ok(()) => self.set_state(account_id, "idle", None).await,
            Err(AppError::Auth(m)) if m == "invalid_grant" => {
                self.set_state(account_id, "auth_required", Some("Sign in again".into()))
                    .await
            }
            Err(e) => {
                let msg = e.user_message();
                let (acc, m2, r) = (account_id.to_string(), msg.clone(), reason.to_string());
                let _ = self
                    .ctx
                    .db
                    .call(move |c| {
                        settings::log_sync(
                            c,
                            Some(&acc),
                            None,
                            "error",
                            Some(&format!("{r}: {m2}")),
                        )
                    })
                    .await;
                self.set_state(account_id, "error", Some(msg)).await;
            }
        }
        result
    }

    async fn sync_account_inner(&self, account_id: &str) -> Result<(), AppError> {
        let outcome = sync_calendar_list(&self.ctx, account_id).await?;
        let acc = account_id.to_string();
        let cals = self
            .ctx
            .db
            .call(move |c| calendars::list_calendars_of_account(c, &acc))
            .await?;
        let mut first_error: Option<AppError> = None;
        for cal in cals {
            let r = if outcome.new_calendars.contains(&cal.id) || !cal.full_sync_done {
                self.full_sync(account_id, &cal.id).await
            } else {
                self.sync_calendar(account_id, &cal.id).await
            };
            if let Err(e) = r {
                tracing::warn!(account = account_id, calendar = %cal.id, error = %e, "calendar sync failed");
                if matches!(e, AppError::Auth(_)) {
                    return Err(e);
                }
                first_error.get_or_insert(e);
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Every Google account, in parallel between accounts.
    pub async fn sync_all(&self, reason: &str) -> Result<(), AppError> {
        let accounts = self.ctx.db.call(|c| accounts::list_accounts(c)).await?;
        let r = reason.to_string();
        let _ = self
            .ctx
            .db
            .call(move |c| settings::log_sync(c, None, None, &r, None))
            .await;
        let mut handles = Vec::new();
        for a in accounts.into_iter().filter(|a| !a.is_local()) {
            let engine = ENGINE.get().cloned();
            let reason = reason.to_string();
            handles.push(tokio::spawn(async move {
                match engine {
                    Some(e) => e.sync_account(&a.id, &reason).await,
                    None => Ok(()),
                }
            }));
        }
        let mut first_error = None;
        for h in handles {
            if let Ok(Err(e)) = h.await {
                first_error.get_or_insert(e);
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Consume ticks until the channel closes. Pending ticks for the same calendar collapse.
    pub async fn run_ticks(self: Arc<Self>, mut rx: mpsc::Receiver<SyncTick>) {
        while let Some(first) = rx.recv().await {
            let mut batch = vec![first];
            while let Ok(t) = rx.try_recv() {
                if !batch.contains(&t) {
                    batch.push(t);
                }
            }
            for tick in batch {
                let (acc, cal) = (tick.account_id.clone(), tick.calendar_id.clone());
                let _ = self
                    .ctx
                    .db
                    .call(move |c| settings::log_sync(c, Some(&acc), cal.as_deref(), "push", None))
                    .await;
                let r = match &tick.calendar_id {
                    Some(cal) => self.sync_calendar(&tick.account_id, cal).await,
                    None => self.sync_account(&tick.account_id, "push").await,
                };
                if let Err(e) = r {
                    tracing::warn!(?tick, error = %e, "tick sync failed");
                    if tick.calendar_id.is_some() {
                        self.set_state(&tick.account_id, "error", Some(e.user_message()))
                            .await;
                    }
                }
            }
        }
    }
}

/// Install the process-wide engine. Returns the existing one if already installed.
pub fn install(ctx: SyncCtx) -> (Arc<Engine>, Option<mpsc::Receiver<SyncTick>>) {
    if let Some(e) = ENGINE.get() {
        return (e.clone(), None);
    }
    let (engine, rx) = Engine::new(ctx);
    let _ = ENGINE.set(engine.clone());
    (ENGINE.get().cloned().unwrap_or(engine), Some(rx))
}

pub fn engine() -> Result<Arc<Engine>, AppError> {
    ENGINE
        .get()
        .cloned()
        .ok_or_else(|| AppError::Db("sync engine not started".into()))
}

/// `sync::start` per docs/08 section 12: install the engine, run the tick consumer and the
/// initial sync (docs/05 section 6). Push channels, polling and the sleep listener are added
/// in phase 5.
pub fn start(app: tauri::AppHandle) -> mpsc::Sender<SyncTick> {
    use crate::sync::ctx::{OAuthTokens, TauriEvents};
    let ctx = SyncCtx {
        db: crate::db::handle()
            .unwrap_or_else(|_| crate::db::init(&crate::config::db_path()).expect("database")),
        client: crate::google::Client::default(),
        tokens: Arc::new(OAuthTokens),
        events: Arc::new(TauriEvents(app.clone())),
    };
    let (engine, rx) = install(ctx);
    if let Some(rx) = rx {
        let consumer = engine.clone();
        tauri::async_runtime::spawn(async move { consumer.run_ticks(rx).await });
        let initial = engine.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = initial.sync_all("startup").await {
                tracing::warn!(error = %e, "startup sync finished with errors");
            }
            // Channels after the first sync, so every calendar has full_sync_done (docs/05 section 6).
            if let Err(e) = crate::sync::push::ensure_channels(&initial.ctx).await {
                tracing::warn!(error = %e, "startup channel setup failed");
            }
        });
        let renew_ctx = engine.ctx.clone();
        tauri::async_runtime::spawn(crate::sync::push::renew_loop(renew_ctx));
        tauri::async_runtime::spawn(crate::sync::poll::run(engine.clone()));
        crate::sync::sleep::start(engine.clone());
    }
    engine.ticks()
}

/// Contract wrappers of docs/08 section 12.
pub async fn full_sync_calendar_app(
    _app: &tauri::AppHandle,
    account_id: &str,
    calendar_id: &str,
) -> Result<(), AppError> {
    engine()?.full_sync(account_id, calendar_id).await
}

pub async fn incremental_calendar_app(
    _app: &tauri::AppHandle,
    account_id: &str,
    calendar_id: &str,
) -> Result<(), AppError> {
    engine()?.sync_calendar(account_id, calendar_id).await
}

pub async fn sync_all(_app: &tauri::AppHandle, reason: &str) -> Result<(), AppError> {
    engine()?.sync_all(reason).await
}
