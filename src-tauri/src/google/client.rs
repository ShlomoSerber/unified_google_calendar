//! HTTP client for the Calendar API. See docs/05-sincronizacion.md section 2.2 (backoff) and
//! docs/02-arquitectura.md section 1. Only `google/` builds Google URLs.
//!
//! Retries `403 usageLimits`, `429` and `5xx` with `min(2^n + jitter, 64 s)` for at most 5
//! attempts. Every other status maps to `AppError::Google { status, reason }`.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::AppError;

pub const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";
pub const MAX_ATTEMPTS: u32 = 5;
pub const MAX_BACKOFF: Duration = Duration::from_secs(64);

#[derive(Clone)]
pub struct Client {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    /// Unit of the exponential backoff (1 s in production, milliseconds in tests).
    pub(crate) backoff_unit: Duration,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new(DEFAULT_BASE_URL.to_string())
    }
}

impl Client {
    pub fn new(base_url: String) -> Client {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent(concat!(
                "unified-google-calendar/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_default();
        Client {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            backoff_unit: Duration::from_secs(1),
        }
    }

    /// Client for tests: base URL of a mock server and millisecond backoff.
    pub fn for_tests(base_url: String) -> Client {
        let mut c = Client::new(base_url);
        c.backoff_unit = Duration::from_millis(1);
        c
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Percent-encode a path segment such as a calendar id (`#` and `@` must be escaped).
    pub fn seg(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes())
            .collect::<String>()
            .replace('+', "%20")
    }

    fn backoff(&self, attempt: u32) -> Duration {
        let exp = self
            .backoff_unit
            .saturating_mul(2u32.saturating_pow(attempt));
        let jitter = self.backoff_unit.mul_f64(rand::random::<f64>());
        let cap = if self.backoff_unit >= Duration::from_secs(1) {
            MAX_BACKOFF
        } else {
            self.backoff_unit * 64
        };
        (exp + jitter).min(cap)
    }

    /// Send a request with bearer auth, retrying transient failures. `body` is JSON when given.
    pub async fn send<T: DeserializeOwned>(
        &self,
        token: &str,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<T, AppError> {
        let text = self.send_text(token, method, path, query, body).await?;
        if text.trim().is_empty() {
            // `T` for empty responses is `()` via serde_json::Value::Null handling below.
            return serde_json::from_str("null")
                .map_err(|e| AppError::Network(format!("empty response could not map: {e}")));
        }
        serde_json::from_str(&text)
            .map_err(|e| AppError::Network(format!("malformed response: {e}")))
    }

    /// Like [`send`] but returns the raw body text.
    pub async fn send_text(
        &self,
        token: &str,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<String, AppError> {
        let url = self.url(path);
        let mut attempt = 0u32;
        loop {
            let mut req = self
                .http
                .request(method.clone(), &url)
                .bearer_auth(token)
                .query(query);
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) if attempt + 1 < MAX_ATTEMPTS && (e.is_connect() || e.is_timeout()) => {
                    attempt += 1;
                    tokio::time::sleep(self.backoff(attempt)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.is_success() {
                return Ok(text);
            }
            let reason = error_reason(&text);
            let retryable = status == StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
                || (status == StatusCode::FORBIDDEN && is_quota_reason(&reason));
            if retryable && attempt + 1 < MAX_ATTEMPTS {
                attempt += 1;
                let wait = self.backoff(attempt);
                tracing::warn!(%status, %reason, attempt, ?wait, "google request retry");
                tokio::time::sleep(wait).await;
                continue;
            }
            tracing::debug!(%status, %reason, %path, "google request failed");
            return Err(AppError::Google {
                status: status.as_u16(),
                reason,
            });
        }
    }
}

fn is_quota_reason(reason: &str) -> bool {
    matches!(
        reason,
        "rateLimitExceeded"
            | "userRateLimitExceeded"
            | "quotaExceeded"
            | "dailyLimitExceeded"
            | "usageLimits"
    )
}

/// `error.errors[0].reason` or `error.status` or `error.message` of a Google error body.
pub fn error_reason(body: &str) -> String {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return body.chars().take(80).collect(),
    };
    let err = v.get("error").cloned().unwrap_or(Value::Null);
    if let Some(r) = err
        .get("errors")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("reason"))
        .and_then(Value::as_str)
    {
        return r.to_string();
    }
    if let Some(s) = err.get("status").and_then(Value::as_str) {
        return s.to_string();
    }
    err.get("message")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_extraction() {
        assert_eq!(
            error_reason(r#"{"error":{"errors":[{"reason":"rateLimitExceeded"}],"code":403}}"#),
            "rateLimitExceeded"
        );
        assert_eq!(
            error_reason(
                r#"{"error":{"code":410,"message":"Sync token is no longer valid","status":"GONE"}}"#
            ),
            "GONE"
        );
        assert_eq!(error_reason("not json"), "not json");
    }

    #[test]
    fn segment_encoding() {
        assert_eq!(
            Client::seg("en.ar#holiday@group.v.calendar.google.com"),
            "en.ar%23holiday%40group.v.calendar.google.com"
        );
        assert_eq!(Client::seg("a b"), "a%20b");
    }
}
