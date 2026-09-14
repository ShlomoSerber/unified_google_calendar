//! Push channel lifecycle against wiremock (docs/08 F5-T2): creation for synced calendars and
//! the calendarList, renewal with stop of the old channel, stop on account removal, and the
//! webhook-rejection fallback to polling.

mod common;

use std::sync::Arc;

use common::{fixture, server};
use rusqlite::params;
use serde_json::json;
use unified_google_calendar_lib::db::{self, DbHandle};
use unified_google_calendar_lib::google::Client;
use unified_google_calendar_lib::sync::ctx::test_support::{FixedToken, Recorder};
use unified_google_calendar_lib::sync::push;
use unified_google_calendar_lib::sync::SyncCtx;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ACC: &str = "104857600000000000001";
const CAL: &str = "someone@example.com";

fn test_db() -> DbHandle {
    let conn = db::open_memory().unwrap();
    conn.execute(
        "INSERT INTO accounts (id, kind, email, display_name, sort_order, created_at) VALUES (?1,'google',?2,?2,1,0)",
        params![ACC, CAL],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible, full_sync_done) VALUES (?1,?2,?1,'#039be5','#000000','owner',1,1,1)",
        params![CAL, ACC],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, visible, full_sync_done) VALUES ('notyet@group.calendar.google.com', ?1, 'Not synced', '#000', '#fff', 'reader', 1, 0)",
        params![ACC],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('push_enabled', 'true')",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO settings (key, value) VALUES ('public_base_url', '\"https://pc.tail.ts.net\"')", []).unwrap();
    DbHandle::spawn(conn)
}

fn ctx(s: &MockServer, db: DbHandle) -> SyncCtx {
    SyncCtx {
        db,
        client: Client::for_tests(s.uri()),
        tokens: Arc::new(FixedToken("tok".into())),
        events: Arc::new(Recorder::default()),
    }
}

async fn channel_rows(db: &DbHandle) -> Vec<(String, Option<String>, String)> {
    db.call(|c| {
        let mut st =
            c.prepare("SELECT id, calendar_id, token FROM channels ORDER BY calendar_id")?;
        let v = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
    .await
    .unwrap()
}

fn channel_response(id_suffix: &str) -> serde_json::Value {
    let mut c = fixture("channel.json");
    c["id"] = json!(format!("ch-{id_suffix}"));
    c["resourceId"] = json!(format!("res-{id_suffix}"));
    c["expiration"] = json!(((chrono::Utc::now().timestamp() + 604_800) * 1000).to_string());
    c
}

#[tokio::test]
async fn creates_channels_for_synced_calendars_and_list() {
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events/watch"))
        .and(body_partial_json(json!({ "type": "web_hook", "address": "https://pc.tail.ts.net/gcal/webhook", "params": { "ttl": "604800" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(channel_response("events")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/users/me/calendarList/watch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(channel_response("list")))
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    let ctx = ctx(&s, db.clone());
    assert_eq!(push::ensure_channels(&ctx).await.unwrap(), 2);
    let rows = channel_rows(&db).await;
    assert_eq!(
        rows.len(),
        2,
        "no channel for the calendar without full sync"
    );
    assert!(rows.iter().any(|r| r.1.is_none()), "calendarList channel");
    assert!(
        rows.iter().all(|r| r.2.len() >= 43),
        "32 random bytes in base64url"
    );
    // Second run: nothing to do.
    assert_eq!(push::ensure_channels(&ctx).await.unwrap(), 0);
    // The token sent to Google is the one stored.
    let reqs = s.received_requests().await.unwrap();
    let watch = reqs
        .iter()
        .find(|r| r.url.path().ends_with("/events/watch"))
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&watch.body).unwrap();
    let stored = rows.iter().find(|r| r.1.is_some()).unwrap();
    assert_eq!(body["token"], stored.2);
}

#[tokio::test]
async fn renews_expiring_channels_and_stops_the_old_one() {
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events/watch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(channel_response("new-events")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/users/me/calendarList/watch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(channel_response("new-list")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/channels/stop"))
        .and(body_partial_json(
            json!({ "id": "old-events", "resourceId": "res-old" }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/channels/stop"))
        .and(body_partial_json(json!({ "id": "old-list" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    let soon = chrono::Utc::now().timestamp() + 3600;
    db.call(move |c| {
        c.execute(
            "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES ('old-events', ?1, ?2, 'res-old', 't', ?3, 0)",
            params![ACC, CAL, soon],
        )?;
        c.execute(
            "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES ('old-list', ?1, NULL, 'res-old2', 't', ?2, 0)",
            params![ACC, soon],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let ctx = ctx(&s, db.clone());
    assert_eq!(push::ensure_channels(&ctx).await.unwrap(), 2);
    let rows = channel_rows(&db).await;
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter().all(|r| r.0.starts_with("ch-new")),
        "old rows replaced: {rows:?}"
    );
}

#[tokio::test]
async fn stop_account_channels_removes_rows() {
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/channels/stop"))
        .respond_with(ResponseTemplate::new(204))
        .expect(2)
        .mount(&s)
        .await;
    let db = test_db();
    db.call(|c| {
        c.execute(
            "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES ('a', ?1, ?2, 'r', 't', 9999999999, 0)",
            params![ACC, CAL],
        )?;
        c.execute(
            "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES ('b', ?1, NULL, 'r2', 't', 9999999999, 0)",
            params![ACC],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let ctx = ctx(&s, db.clone());
    push::stop_account_channels(&ctx, ACC).await.unwrap();
    assert!(channel_rows(&db).await.is_empty());
}

#[tokio::test]
async fn webhook_rejection_disables_push() {
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/users/me/calendarList/watch"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": { "code": 400, "message": "Unauthorized WebHook callback channel: https://pc.tail.ts.net/gcal/webhook",
                       "errors": [{ "domain": "global", "reason": "push.webhookUrlUnauthorized", "message": "Unauthorized WebHook callback channel" }] }
        })))
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    let ctx = ctx(&s, db.clone());
    assert!(push::ensure_channels(&ctx).await.is_err());
    let cfg = push::config(&ctx).await.unwrap();
    assert!(!cfg.enabled);
    let err: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT value FROM settings WHERE key='push_error'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert!(err.contains("docs/09"));
    assert!(channel_rows(&db).await.is_empty());
    // With push disabled nothing is attempted any more.
    assert_eq!(push::ensure_channels(&ctx).await.unwrap(), 0);
}

#[tokio::test]
async fn test_public_url_reports_and_stores_outcome() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&s)
        .await;
    let db = test_db();
    let ctx = ctx(&s, db.clone());
    let r = push::test_public_url(&ctx, &s.uri()).await;
    assert!(r.ok, "{}", r.message);
    assert!(push::config(&ctx).await.unwrap().enabled);
    s.reset().await;
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&s)
        .await;
    let r = push::test_public_url(&ctx, &s.uri()).await;
    assert!(!r.ok);
    assert!(r.message.contains("502"));
    assert!(!push::config(&ctx).await.unwrap().enabled);
}
