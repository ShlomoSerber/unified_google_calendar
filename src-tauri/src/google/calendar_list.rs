//! `calendarList` endpoints. See docs/05-sincronizacion.md section 2.1.

use reqwest::Method;
use serde_json::json;

use crate::error::AppError;
use crate::google::client::Client;
use crate::google::types::{CalendarListEntry, CalendarListPage, Channel, WatchRequest};

impl Client {
    /// One page of `calendarList.list` with the fixed parameters of docs/05 section 2.1.
    pub async fn calendar_list_page(
        &self,
        token: &str,
        page_token: Option<&str>,
        sync_token: Option<&str>,
    ) -> Result<CalendarListPage, AppError> {
        let mut query = vec![
            ("showHidden", "true"),
            ("showDeleted", "true"),
            ("maxResults", "250"),
        ];
        if let Some(p) = page_token {
            query.push(("pageToken", p));
        }
        if let Some(s) = sync_token {
            query.push(("syncToken", s));
        }
        self.send(token, Method::GET, "/users/me/calendarList", &query, None)
            .await
    }

    /// `calendarList.insert`: subscribe to an existing calendar (holidays, docs/03 section 1).
    pub async fn calendar_list_insert(
        &self,
        token: &str,
        calendar_id: &str,
    ) -> Result<CalendarListEntry, AppError> {
        self.send(
            token,
            Method::POST,
            "/users/me/calendarList",
            &[],
            Some(&json!({ "id": calendar_id })),
        )
        .await
    }

    /// `calendars.insert`: create a secondary calendar (used by the fixture seeding tool).
    pub async fn calendar_insert(
        &self,
        token: &str,
        summary: &str,
        time_zone: &str,
    ) -> Result<serde_json::Value, AppError> {
        self.send(
            token,
            Method::POST,
            "/calendars",
            &[],
            Some(&json!({ "summary": summary, "timeZone": time_zone })),
        )
        .await
    }

    /// `calendars.delete`: remove a secondary calendar owned by the account.
    pub async fn calendar_delete(&self, token: &str, calendar_id: &str) -> Result<(), AppError> {
        self.send_text(
            token,
            Method::DELETE,
            &format!("/calendars/{}", Client::seg(calendar_id)),
            &[],
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn watch_calendar_list(
        &self,
        token: &str,
        req: &WatchRequest,
    ) -> Result<Channel, AppError> {
        let body = serde_json::to_value(req)?;
        self.send(
            token,
            Method::POST,
            "/users/me/calendarList/watch",
            &[],
            Some(&body),
        )
        .await
    }
}
