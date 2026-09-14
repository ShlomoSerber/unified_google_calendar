//! OAuth 2.0 with PKCE and a loopback listener. See docs/05-sincronizacion.md section 1.
//!
//! Flow of `add_account`: bind `127.0.0.1:0`, open the consent URL in the system browser, wait
//! for the redirect (5 minutes), exchange the code, read `sub`/`email` from the `id_token`,
//! store the tokens encrypted, upsert the account row. Access tokens are refreshed by
//! [`access_token`] under a per-account mutex.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::auth::pkce;
use crate::auth::token_store::{AccountTokens, TokenStore};
use crate::commands::types::AccountInfo;
use crate::db::{self, queries::accounts};
use crate::error::AppError;

pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const SCOPE: &str = "openid email https://www.googleapis.com/auth/calendar";
pub const CONSENT_TIMEOUT: Duration = Duration::from_secs(300);
/// Refresh when fewer than this many seconds remain (docs/05 section 1.3).
pub const REFRESH_MARGIN_SECS: i64 = 300;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OAuthConfig {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

impl OAuthConfig {
    pub fn load() -> Result<OAuthConfig, AppError> {
        let path = crate::config::oauth_json_path();
        let raw = std::fs::read_to_string(&path).map_err(|_| {
            AppError::Auth(format!(
                "{} is missing. Create it with your Google Cloud OAuth client (docs/09-setup-usuario.md section A)",
                path.display()
            ))
        })?;
        let cfg: OAuthConfig = serde_json::from_str(&raw)
            .map_err(|e| AppError::Auth(format!("oauth.json is malformed: {e}")))?;
        if cfg.client_id.is_empty() {
            return Err(AppError::Auth("oauth.json has an empty client_id".into()));
        }
        Ok(cfg)
    }

    pub fn exists() -> bool {
        crate::config::oauth_json_path().is_file()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

/// HTTP side of the flow, with URLs injectable for tests.
#[derive(Clone)]
pub struct OAuthClient {
    pub http: reqwest::Client,
    pub cfg: OAuthConfig,
    pub auth_url: String,
    pub token_url: String,
}

impl OAuthClient {
    pub fn new(cfg: OAuthConfig) -> OAuthClient {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!(
                "unified-google-calendar/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_default();
        OAuthClient {
            http,
            cfg,
            auth_url: AUTH_URL.into(),
            token_url: TOKEN_URL.into(),
        }
    }

    /// Consent URL of docs/05 section 1.2 step 3.
    pub fn consent_url(&self, port: u16, challenge: &str, state: &str) -> String {
        let mut url = url::Url::parse(&self.auth_url)
            .unwrap_or_else(|_| url::Url::parse(AUTH_URL).expect("static url"));
        url.query_pairs_mut()
            .append_pair("client_id", &self.cfg.client_id)
            .append_pair("redirect_uri", &format!("http://127.0.0.1:{port}"))
            .append_pair("response_type", "code")
            .append_pair("scope", SCOPE)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", state);
        url.to_string()
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> Result<TokenResponse, AppError> {
        let resp = self.http.post(&self.token_url).form(form).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if status.is_success() {
            return serde_json::from_str(&body)
                .map_err(|e| AppError::Auth(format!("token response malformed: {e}")));
        }
        let err: TokenError = serde_json::from_str(&body).unwrap_or(TokenError {
            error: format!("http {status}"),
            error_description: String::new(),
        });
        Err(match err.error.as_str() {
            "invalid_grant" => AppError::Auth("invalid_grant".into()),
            other if err.error_description.is_empty() => AppError::Auth(other.to_string()),
            other => AppError::Auth(format!("{other}: {}", err.error_description)),
        })
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        port: u16,
    ) -> Result<TokenResponse, AppError> {
        let redirect = format!("http://127.0.0.1:{port}");
        self.token_request(&[
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect),
        ])
        .await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenResponse, AppError> {
        self.token_request(&[
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .await
    }
}

/// `sub` and `email` from an id_token payload. The signature is not verified: the token comes
/// straight from the token endpoint over TLS (docs/05 section 1.2 step 6).
pub fn decode_id_token(id_token: &str) -> Result<(String, String), AppError> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::Auth("id_token has no payload".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| AppError::Auth("id_token payload is not base64url".into()))?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Auth("id_token payload is not JSON".into()))?;
    let sub = v
        .get("sub")
        .and_then(|s| s.as_str())
        .ok_or_else(|| AppError::Auth("id_token has no sub".into()))?;
    let email = v.get("email").and_then(|s| s.as_str()).unwrap_or_default();
    Ok((sub.to_string(), email.to_string()))
}

/// What the browser sent back to the loopback.
#[derive(Debug, PartialEq, Eq)]
pub enum RedirectOutcome {
    Code(String),
    /// `error` query parameter, e.g. `access_denied` or `admin_policy_enforced`.
    Error(String),
}

fn parse_query(target: &str) -> HashMap<String, String> {
    let q = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    url::form_urlencoded::parse(q.as_bytes())
        .into_owned()
        .collect()
}

const HTML_OK: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Unified Google Calendar</title></head>\
<body style=\"font-family:sans-serif;padding:2em\"><h2>Signed in.</h2><p>You can close this tab and return to Unified Google Calendar.</p></body></html>";
const HTML_BAD: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Unified Google Calendar</title></head>\
<body style=\"font-family:sans-serif;padding:2em\"><h2>Sign-in rejected.</h2><p>The state parameter did not match. Close this tab and try again from the app.</p></body></html>";

async fn respond(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Wait for one redirect on `listener`. A wrong `state` is answered with 400 and aborts.
pub async fn wait_for_redirect(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<RedirectOutcome, AppError> {
    let fut = async {
        loop {
            let (mut stream, _) = listener.accept().await?;
            let mut buf = vec![0u8; 8192];
            let n = match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf)).await
            {
                Ok(Ok(n)) => n,
                _ => continue,
            };
            let text = String::from_utf8_lossy(&buf[..n]).to_string();
            let Some(line) = text.lines().next() else {
                continue;
            };
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or("");
            let target = parts.next().unwrap_or("/");
            if method != "GET" {
                respond(&mut stream, "405 Method Not Allowed", "").await;
                continue;
            }
            // Browsers also ask for /favicon.ico; ignore anything without state.
            let params = parse_query(target);
            if !params.contains_key("state")
                && !params.contains_key("code")
                && !params.contains_key("error")
            {
                respond(&mut stream, "404 Not Found", "").await;
                continue;
            }
            if params.get("state").map(String::as_str) != Some(expected_state) {
                respond(&mut stream, "400 Bad Request", HTML_BAD).await;
                return Err(AppError::Auth(
                    "the sign-in redirect carried a wrong state; possible CSRF, aborted".into(),
                ));
            }
            if let Some(err) = params.get("error") {
                respond(&mut stream, "200 OK", HTML_OK).await;
                return Ok(RedirectOutcome::Error(err.clone()));
            }
            if let Some(code) = params.get("code") {
                respond(&mut stream, "200 OK", HTML_OK).await;
                return Ok(RedirectOutcome::Code(code.clone()));
            }
            respond(&mut stream, "400 Bad Request", HTML_BAD).await;
            return Err(AppError::Auth(
                "the sign-in redirect had neither code nor error".into(),
            ));
        }
    };
    match tokio::time::timeout(timeout, fut).await {
        Ok(r) => r,
        Err(_) => Err(AppError::Auth(
            "sign-in timed out after 5 minutes; try again".into(),
        )),
    }
}

/// Process-wide auth state: token store, HTTP client and per-account refresh locks.
pub struct AuthState {
    pub store: TokenStore,
    pub client: OAuthClient,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

static AUTH: OnceLock<AuthState> = OnceLock::new();

/// Initialise once. Needs `oauth.json`; without it the app runs local-only (docs/07 section 5).
pub fn init() -> Result<&'static AuthState, AppError> {
    if let Some(a) = AUTH.get() {
        return Ok(a);
    }
    let cfg = OAuthConfig::load()?;
    let state = AuthState {
        store: TokenStore::open_default()?,
        client: OAuthClient::new(cfg),
        locks: Mutex::new(HashMap::new()),
    };
    let _ = AUTH.set(state);
    AUTH.get()
        .ok_or_else(|| AppError::Auth("auth state not initialised".into()))
}

pub fn state() -> Result<&'static AuthState, AppError> {
    AUTH.get().ok_or_else(|| {
        AppError::Auth("Google sign-in is not configured: create oauth.json first".into())
    })
}

impl AuthState {
    async fn lock_for(&self, account_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(account_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Valid access token for `account_id`, refreshed when close to expiry. `invalid_grant`
    /// marks the account `auth_required` (docs/05 section 1.3).
    pub async fn access_token(&self, account_id: &str) -> Result<String, AppError> {
        let lock = self.lock_for(account_id).await;
        let _guard = lock.lock().await;
        let tokens = self
            .store
            .get(account_id)?
            .ok_or_else(|| AppError::Auth(format!("no stored tokens for account {account_id}")))?;
        let now = db::now_ts();
        if tokens.expires_at - now > REFRESH_MARGIN_SECS && !tokens.access_token.is_empty() {
            return Ok(tokens.access_token);
        }
        match self.client.refresh(&tokens.refresh_token).await {
            Ok(r) => {
                let new = AccountTokens {
                    refresh_token: r.refresh_token.unwrap_or(tokens.refresh_token),
                    access_token: r.access_token.clone(),
                    expires_at: now + r.expires_in,
                };
                self.store.put(account_id, new)?;
                Ok(r.access_token)
            }
            Err(AppError::Auth(m)) if m == "invalid_grant" => {
                let id = account_id.to_string();
                let _ = db::call(move |c| {
                    accounts::set_sync_state(c, &id, "auth_required", Some("Sign in again"))
                })
                .await;
                Err(AppError::Auth("invalid_grant".into()))
            }
            Err(e) => Err(e),
        }
    }
}

/// `access_token` per the module contract of docs/08 section 12.
pub async fn access_token(account_id: &str) -> Result<String, AppError> {
    state()?.access_token(account_id).await
}

/// Run the consent flow and persist the result. Blocks until the browser returns or 5 minutes pass.
pub async fn add_account(app: &tauri::AppHandle) -> Result<AccountInfo, AppError> {
    use tauri_plugin_opener::OpenerExt;
    let auth = init()?;
    let verifier = pkce::generate_verifier();
    let challenge = pkce::challenge(&verifier);
    let state_param = pkce::random_state();
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let url = auth.client.consent_url(port, &challenge, &state_param);
    tracing::info!(port, "opening consent screen");
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| AppError::Auth(format!("could not open the browser: {e}")))?;
    let outcome = wait_for_redirect(listener, &state_param, CONSENT_TIMEOUT).await?;
    let code = match outcome {
        RedirectOutcome::Code(c) => c,
        RedirectOutcome::Error(e) if e == "admin_policy_enforced" => {
            return Err(AppError::Auth(format!(
                "admin_policy_enforced: the Google Workspace admin blocked this app. Ask them to trust client ID {} in Admin console → Security → API controls → App access control",
                auth.client.cfg.client_id
            )));
        }
        RedirectOutcome::Error(e) => {
            return Err(AppError::Auth(format!(
                "Google returned {e} during sign-in"
            )))
        }
    };
    let tokens = auth.client.exchange_code(&code, &verifier, port).await?;
    let id_token = tokens
        .id_token
        .as_deref()
        .ok_or_else(|| AppError::Auth("token response has no id_token".into()))?;
    let (sub, email) = decode_id_token(id_token)?;
    let refresh = tokens
        .refresh_token
        .clone()
        .ok_or_else(|| AppError::Auth("Google did not return a refresh token; remove the app from the account's third-party access and try again".into()))?;
    auth.store.put(
        &sub,
        AccountTokens {
            refresh_token: refresh,
            access_token: tokens.access_token.clone(),
            expires_at: db::now_ts() + tokens.expires_in,
        },
    )?;
    let (sub2, email2) = (sub.clone(), email.clone());
    let info = db::call(move |c| {
        let name = if email2.is_empty() {
            sub2.clone()
        } else {
            email2.clone()
        };
        accounts::upsert_google_account(c, &sub2, &email2, &name)?;
        let row = accounts::get_account(c, &sub2)?
            .ok_or_else(|| AppError::Db("account row missing after insert".into()))?;
        Ok(AccountInfo {
            id: row.id,
            kind: row.kind,
            email: row.email,
            display_name: row.display_name,
            sort_order: row.sort_order,
            sync_state: row.sync_state,
            sync_error: row.sync_error,
            last_sync_at: row.last_sync_at,
        })
    })
    .await?;
    tracing::info!(account = %sub, "account added");
    Ok(info)
}

/// Forget an account's tokens.
pub fn remove_tokens(account_id: &str) -> Result<(), AppError> {
    state()?.store.remove(account_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn hit(port: u16, target: &str) -> String {
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        s.write_all(format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn listener_accepts_correct_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiter = tokio::spawn(async move {
            wait_for_redirect(listener, "st4te", Duration::from_secs(5)).await
        });
        let favicon = hit(port, "/favicon.ico").await;
        assert!(favicon.starts_with("HTTP/1.1 404"));
        let resp = hit(port, "/?state=st4te&code=4%2FabcDEF&scope=x").await;
        assert!(resp.starts_with("HTTP/1.1 200"));
        assert!(resp.contains("You can close this tab"));
        assert_eq!(
            waiter.await.unwrap().unwrap(),
            RedirectOutcome::Code("4/abcDEF".into())
        );
    }

    #[tokio::test]
    async fn listener_rejects_wrong_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiter = tokio::spawn(async move {
            wait_for_redirect(listener, "expected", Duration::from_secs(5)).await
        });
        let resp = hit(port, "/?state=forged&code=abc").await;
        assert!(resp.starts_with("HTTP/1.1 400"));
        let err = waiter.await.unwrap().unwrap_err();
        assert!(matches!(err, AppError::Auth(m) if m.contains("state")));
    }

    #[tokio::test]
    async fn listener_reports_google_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiter =
            tokio::spawn(
                async move { wait_for_redirect(listener, "s", Duration::from_secs(5)).await },
            );
        hit(port, "/?error=admin_policy_enforced&state=s").await;
        assert_eq!(
            waiter.await.unwrap().unwrap(),
            RedirectOutcome::Error("admin_policy_enforced".into())
        );
    }

    #[tokio::test]
    async fn listener_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let err = wait_for_redirect(listener, "s", Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn consent_url_has_every_parameter() {
        let c = OAuthClient::new(OAuthConfig {
            client_id: "cid.apps".into(),
            client_secret: "sec".into(),
        });
        let u = c.consent_url(43210, "chal", "st");
        let parsed = url::Url::parse(&u).unwrap();
        let q: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(q["client_id"], "cid.apps");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:43210");
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["scope"], SCOPE);
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["access_type"], "offline");
        assert_eq!(q["prompt"], "consent");
        assert_eq!(q["state"], "st");
    }

    #[test]
    fn id_token_decodes_sub_and_email() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"1234567890","email":"me@example.com"}"#);
        let (sub, email) = decode_id_token(&format!("hdr.{payload}.sig")).unwrap();
        assert_eq!(sub, "1234567890");
        assert_eq!(email, "me@example.com");
        assert!(decode_id_token("garbage").is_err());
    }

    #[tokio::test]
    async fn exchange_and_refresh_against_mock_token_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code_verifier=ver"))
            .and(body_string_contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "at", "expires_in": 3599, "refresh_token": "rt", "id_token": "a.b.c", "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=dead"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant", "error_description": "Token has been expired or revoked."
            })))
            .mount(&server)
            .await;
        let mut c = OAuthClient::new(OAuthConfig {
            client_id: "cid".into(),
            client_secret: "".into(),
        });
        c.token_url = format!("{}/token", server.uri());
        let t = c.exchange_code("code", "ver", 5).await.unwrap();
        assert_eq!(t.access_token, "at");
        assert_eq!(t.refresh_token.as_deref(), Some("rt"));
        let err = c.refresh("dead").await.unwrap_err();
        assert!(matches!(err, AppError::Auth(m) if m == "invalid_grant"));
    }
}
