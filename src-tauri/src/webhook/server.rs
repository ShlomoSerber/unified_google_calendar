//! Push receiver on `127.0.0.1:<webhook_port>`. See docs/05-sincronizacion.md section 3.1 and
//! docs/02-arquitectura.md sections 3.4 and 7.
//!
//! Routes: `POST /gcal/webhook`, `GET /google<token>.html`, `GET /healthz`; anything else 404.
//! The POST validates `X-Goog-Channel-ID` against `channels` and `X-Goog-Channel-Token` in
//! constant time, never reads the body (64 KB limit), rate-limits 60 requests per minute per
//! IP and answers 200 before the sync runs.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use subtle::ConstantTimeEq;
use tokio::sync::mpsc;

use crate::db::queries::{channels, settings};
use crate::db::DbHandle;
use crate::error::AppError;
use crate::sync::SyncTick;
use crate::webhook::verify_file;

pub const BODY_LIMIT: usize = 64 * 1024;
pub const RATE_LIMIT_PER_MINUTE: u32 = 60;

#[derive(Clone)]
pub struct WebhookState {
    pub db: DbHandle,
    pub ticks: mpsc::Sender<SyncTick>,
    pub verify_dir: PathBuf,
    limiter: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}

impl WebhookState {
    pub fn new(db: DbHandle, ticks: mpsc::Sender<SyncTick>, verify_dir: PathBuf) -> WebhookState {
        WebhookState {
            db,
            ticks,
            verify_dir,
            limiter: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// `true` when the client may proceed.
    fn allow(&self, ip: &str) -> bool {
        let mut map = match self.limiter.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(),
        };
        let now = Instant::now();
        let entry = map.entry(ip.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0).as_secs() >= 60 {
            *entry = (now, 0);
        }
        entry.1 += 1;
        if map.len() > 10_000 {
            map.retain(|_, (t, _)| now.duration_since(*t).as_secs() < 60);
        }
        map.get(ip)
            .map(|(_, n)| *n <= RATE_LIMIT_PER_MINUTE)
            .unwrap_or(true)
    }
}

/// Client IP: `X-Forwarded-For` added by Tailscale, else the socket peer.
fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| peer.map(|p| p.ip().to_string()))
        .unwrap_or_else(|| "unknown".into())
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Validation outcome of one notification, for logging and tests.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Channel unknown or token mismatch → 404.
    Rejected,
    /// `sync` message: acknowledged, no tick.
    SyncMessage,
    /// `exists` / `not_exists`: tick queued.
    Ticked(SyncTick),
}

pub async fn validate(state: &WebhookState, headers: &HeaderMap) -> Result<Verdict, AppError> {
    let Some(channel_id) = header(headers, "x-goog-channel-id") else {
        return Ok(Verdict::Rejected);
    };
    let id = channel_id.to_string();
    let Some(channel) = state.db.call(move |c| channels::get(c, &id)).await? else {
        return Ok(Verdict::Rejected);
    };
    let presented = header(headers, "x-goog-channel-token").unwrap_or("");
    let equal = presented.len() == channel.token.len()
        && presented.as_bytes().ct_eq(channel.token.as_bytes()).into();
    if !equal {
        return Ok(Verdict::Rejected);
    }
    let resource_state = header(headers, "x-goog-resource-state").unwrap_or("");
    match resource_state {
        "sync" => {
            let (acc, cal) = (channel.account_id.clone(), channel.calendar_id.clone());
            let _ = state
                .db
                .call(move |c| {
                    settings::log_sync(
                        c,
                        Some(&acc),
                        cal.as_deref(),
                        "push",
                        Some("channel sync message"),
                    )
                })
                .await;
            Ok(Verdict::SyncMessage)
        }
        "exists" | "not_exists" => {
            let tick = SyncTick {
                account_id: channel.account_id.clone(),
                calendar_id: channel.calendar_id.clone(),
            };
            match state.ticks.try_send(tick.clone()) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!("tick queue full; notification dropped, poll will catch up")
                }
                Err(mpsc::error::TrySendError::Closed(_)) => tracing::warn!("tick queue closed"),
            }
            Ok(Verdict::Ticked(tick))
        }
        other => {
            tracing::debug!(state = other, "unknown X-Goog-Resource-State ignored");
            Ok(Verdict::SyncMessage)
        }
    }
}

async fn webhook(
    State(state): State<WebhookState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
) -> Response {
    let headers = req.headers().clone();
    if !state.allow(&client_ip(&headers, Some(peer))) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    // The body is never read (docs/05 section 3.1 step 4).
    drop(req);
    match validate(&state, &headers).await {
        Ok(Verdict::Rejected) => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "webhook validation failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn healthz(
    State(state): State<WebhookState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if !state.allow(&client_ip(&headers, Some(peer))) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    (StatusCode::OK, "ok").into_response()
}

async fn fallback(
    State(state): State<WebhookState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
) -> Response {
    let headers = req.headers().clone();
    if !state.allow(&client_ip(&headers, Some(peer))) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    if req.method() == axum::http::Method::GET {
        if let Some(path) = verify_file::resolve(&state.verify_dir, req.uri().path()) {
            if let Ok(bytes) = tokio::fs::read(&path).await {
                return (
                    StatusCode::OK,
                    [("content-type", "text/html; charset=utf-8")],
                    Body::from(bytes),
                )
                    .into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

pub fn router(state: WebhookState) -> Router {
    Router::new()
        .route("/gcal/webhook", post(webhook))
        .route("/healthz", get(healthz))
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state)
}

/// Bind `127.0.0.1:port` and serve until the app exits.
pub async fn serve(state: WebhookState, port: u16) -> Result<(), AppError> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Network(format!("cannot bind {addr}: {e}")))?;
    tracing::info!(%addr, "webhook listening");
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| AppError::Network(format!("webhook server stopped: {e}")))
}

/// Start the server in the background. Errors (port busy) are logged, never fatal.
pub fn start(db: DbHandle, ticks: mpsc::Sender<SyncTick>, port: u16) {
    let state = WebhookState::new(db, ticks, crate::config::verify_dir());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve(state, port).await {
            tracing::error!(error = %e, "webhook server failed; push notifications are unavailable");
        }
    });
}
