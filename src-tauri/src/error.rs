//! Application error type. See docs/02-arquitectura.md section 9.
//!
//! Every fallible function in the backend returns `AppError`. The `#[tauri::command]`
//! boundary converts it to a `String` with [`AppError::user_message`]. Messages never
//! contain tokens or full response bodies.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication error: {0}")]
    Auth(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("google api error {status}: {reason}")]
    Google { status: u16, reason: String },
    #[error("database error: {0}")]
    Db(String),
    #[error("recurrence error: {0}")]
    Recurrence(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    Invalid(String),
}

impl AppError {
    /// Full English sentence for the UI: what happened and what to do.
    pub fn user_message(&self) -> String {
        match self {
            AppError::Auth(m) => format!(
                "Google did not accept the account credentials ({m}). Sign in again from the sidebar."
            ),
            AppError::Network(m) => format!(
                "The app could not reach Google ({m}). Check the connection; it will retry on the next sync."
            ),
            AppError::Google { status, reason } => match *status {
                401 => "Google rejected the access token. Sign in again from the sidebar.".to_string(),
                403 => format!(
                    "Google refused the request ({reason}). Check the calendar permissions or wait a minute if the quota was exceeded."
                ),
                404 => format!("Google could not find the item ({reason}). It may have been deleted elsewhere; sync again."),
                409 => format!("Google reported a conflict ({reason}). Reload the event and try again."),
                410 => "Google discarded the sync token. The app will run a full sync of this calendar.".to_string(),
                412 => "The event changed on Google since it was loaded. Reload the event and try again.".to_string(),
                429 => "Google rate-limited the app. Wait a minute and try again.".to_string(),
                s if s >= 500 => format!("Google returned a server error ({s}). Try again in a few minutes."),
                s => format!("Google returned an error ({s}: {reason}). Try again."),
            },
            AppError::Db(m) => format!("The local database failed ({m}). Restart the app; if it persists, check the log file."),
            AppError::Recurrence(m) => format!("The recurrence rule could not be processed ({m}). Edit the repeat settings and try again."),
            AppError::NotFound(m) => format!("{m} was not found. It may have been deleted; the view will refresh."),
            AppError::Invalid(m) => format!("{m}."),
        }
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        AppError::Invalid(msg.into())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Invalid(format!("malformed JSON: {e}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // reqwest errors can embed the URL; keep only the kind of failure.
        let kind = if e.is_timeout() {
            "timeout"
        } else if e.is_connect() {
            "connection failed"
        } else if e.is_request() {
            "request failed"
        } else if e.is_body() || e.is_decode() {
            "malformed response"
        } else {
            "unknown"
        };
        AppError::Network(kind.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Db(format!("io: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_messages_are_full_sentences() {
        let errors = [
            AppError::Auth("invalid_grant".into()),
            AppError::Network("timeout".into()),
            AppError::Google { status: 404, reason: "notFound".into() },
            AppError::Google { status: 503, reason: "backendError".into() },
            AppError::Db("locked".into()),
            AppError::Recurrence("bad rule".into()),
            AppError::NotFound("The event".into()),
            AppError::Invalid("End must be after start".into()),
        ];
        for e in errors {
            let m = e.user_message();
            assert!(m.ends_with('.'), "{m}");
            assert!(m.len() > 10);
        }
    }
}
