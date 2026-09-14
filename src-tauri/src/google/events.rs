//! `events` endpoints. See docs/05-sincronizacion.md sections 2.2 and 2.3 and docs/03 section 7.
//!
//! `events.list` always uses exactly `singleEvents=false&showDeleted=true&maxResults=2500` plus
//! `pageToken` or `syncToken`. `insert`, `update`, `patch` and `import` always send
//! `conferenceDataVersion=1`.

use reqwest::Method;
use serde_json::Value;

use crate::error::AppError;
use crate::google::client::Client;
use crate::google::types::{Channel, Event, EventsPage, SendUpdates, WatchRequest};

fn events_path(calendar_id: &str) -> String {
    format!("/calendars/{}/events", Client::seg(calendar_id))
}

fn event_path(calendar_id: &str, event_id: &str) -> String {
    format!(
        "/calendars/{}/events/{}",
        Client::seg(calendar_id),
        Client::seg(event_id)
    )
}

impl Client {
    /// One page of `events.list`. Full sync when `sync_token` is `None`.
    pub async fn events_page(
        &self,
        token: &str,
        calendar_id: &str,
        page_token: Option<&str>,
        sync_token: Option<&str>,
    ) -> Result<EventsPage, AppError> {
        let mut query = vec![
            ("singleEvents", "false"),
            ("showDeleted", "true"),
            ("maxResults", "2500"),
        ];
        if let Some(p) = page_token {
            query.push(("pageToken", p));
        }
        if let Some(s) = sync_token {
            query.push(("syncToken", s));
        }
        self.send(token, Method::GET, &events_path(calendar_id), &query, None)
            .await
    }

    pub async fn event_get(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<Event, AppError> {
        self.send(
            token,
            Method::GET,
            &event_path(calendar_id, event_id),
            &[],
            None,
        )
        .await
    }

    pub async fn event_insert(
        &self,
        token: &str,
        calendar_id: &str,
        body: &Event,
        send_updates: SendUpdates,
    ) -> Result<Event, AppError> {
        let v = serde_json::to_value(body)?;
        self.send(
            token,
            Method::POST,
            &events_path(calendar_id),
            &[
                ("conferenceDataVersion", "1"),
                ("sendUpdates", send_updates.as_str()),
            ],
            Some(&v),
        )
        .await
    }

    pub async fn event_update(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
        body: &Event,
        send_updates: SendUpdates,
    ) -> Result<Event, AppError> {
        let v = serde_json::to_value(body)?;
        self.send(
            token,
            Method::PUT,
            &event_path(calendar_id, event_id),
            &[
                ("conferenceDataVersion", "1"),
                ("sendUpdates", send_updates.as_str()),
            ],
            Some(&v),
        )
        .await
    }

    pub async fn event_patch(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
        body: &Value,
        send_updates: SendUpdates,
    ) -> Result<Event, AppError> {
        self.send(
            token,
            Method::PATCH,
            &event_path(calendar_id, event_id),
            &[
                ("conferenceDataVersion", "1"),
                ("sendUpdates", send_updates.as_str()),
            ],
            Some(body),
        )
        .await
    }

    pub async fn event_delete(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
        send_updates: SendUpdates,
    ) -> Result<(), AppError> {
        self.send_text(
            token,
            Method::DELETE,
            &event_path(calendar_id, event_id),
            &[("sendUpdates", send_updates.as_str())],
            None,
        )
        .await?;
        Ok(())
    }

    /// Instances of a recurring master; `original_start` filters one instance (docs/03 section 4).
    pub async fn event_instances(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
        original_start: Option<&str>,
    ) -> Result<EventsPage, AppError> {
        let mut query = vec![("maxResults", "50"), ("showDeleted", "true")];
        if let Some(o) = original_start {
            query.push(("originalStart", o));
        }
        self.send(
            token,
            Method::GET,
            &format!("{}/instances", event_path(calendar_id, event_id)),
            &query,
            None,
        )
        .await
    }

    pub async fn event_move(
        &self,
        token: &str,
        calendar_id: &str,
        event_id: &str,
        destination: &str,
    ) -> Result<Event, AppError> {
        self.send(
            token,
            Method::POST,
            &format!("{}/move", event_path(calendar_id, event_id)),
            &[("destination", destination), ("sendUpdates", "none")],
            None,
        )
        .await
    }

    pub async fn event_import(
        &self,
        token: &str,
        calendar_id: &str,
        body: &Event,
    ) -> Result<Event, AppError> {
        let v = serde_json::to_value(body)?;
        self.send(
            token,
            Method::POST,
            &format!("{}/import", events_path(calendar_id)),
            &[("conferenceDataVersion", "1")],
            Some(&v),
        )
        .await
    }

    pub async fn watch_events(
        &self,
        token: &str,
        calendar_id: &str,
        req: &WatchRequest,
    ) -> Result<Channel, AppError> {
        let body = serde_json::to_value(req)?;
        self.send(
            token,
            Method::POST,
            &format!("{}/watch", events_path(calendar_id)),
            &[],
            Some(&body),
        )
        .await
    }
}
