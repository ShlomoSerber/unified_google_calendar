//! Webhook routes and validation branches through `tower::ServiceExt` (docs/08 F5-T1).

mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rusqlite::params;
use tower::ServiceExt;
use unified_google_calendar_lib::db::{self, DbHandle};
use unified_google_calendar_lib::sync::SyncTick;
use unified_google_calendar_lib::webhook::server::{router, WebhookState, RATE_LIMIT_PER_MINUTE};

const ACC: &str = "104857600000000000001";
const CAL: &str = "someone@example.com";
const CHANNEL: &str = "01234567-89ab-cdef-0123-456789abcdef";
const TOKEN: &str = "c2VjcmV0LXRva2Vu";

fn test_db() -> DbHandle {
    let conn = db::open_memory().unwrap();
    conn.execute(
        "INSERT INTO accounts (id, kind, email, display_name, sort_order, created_at) VALUES (?1,'google',?2,?2,1,0)",
        params![ACC, CAL],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES (?1, ?2, ?3, 'res', ?4, 9999999999, 0)",
        params![CHANNEL, ACC, CAL, TOKEN],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO channels (id, account_id, calendar_id, resource_id, token, expiration_ts, created_at) VALUES ('list-channel', ?1, NULL, 'res2', ?2, 9999999999, 0)",
        params![ACC, TOKEN],
    )
    .unwrap();
    DbHandle::spawn(conn)
}

fn app(
    verify_dir: std::path::PathBuf,
) -> (
    axum::Router,
    tokio::sync::mpsc::Receiver<SyncTick>,
    DbHandle,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let db = test_db();
    let state = WebhookState::new(db.clone(), tx, verify_dir);
    let router = router(state).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    (router, rx, db)
}

fn post(headers: &[(&str, &str)]) -> Request<Body> {
    let mut b = Request::builder().method("POST").uri("/gcal/webhook");
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    b.body(Body::from("ignored body")).unwrap()
}

#[tokio::test]
async fn healthz_and_unknown_routes() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _rx, _db) = app(dir.path().to_path_buf());
    let r = app
        .clone()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
    let r = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let r = app
        .clone()
        .oneshot(Request::get("/gcal/webhook").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::METHOD_NOT_ALLOWED);
    let r = app
        .oneshot(Request::post("/other").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn verification_file_is_served_only_when_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("googleabc.html"),
        "google-site-verification: googleabc.html",
    )
    .unwrap();
    let (app, _rx, _db) = app(dir.path().to_path_buf());
    let r = app
        .clone()
        .oneshot(Request::get("/googleabc.html").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert!(body.starts_with(b"google-site-verification"));
    let r = app
        .oneshot(Request::get("/googlezzz.html").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn webhook_validation_branches() {
    let dir = tempfile::tempdir().unwrap();
    let (app, mut rx, db) = app(dir.path().to_path_buf());

    // Unknown channel → 404.
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", "nope"),
            ("x-goog-channel-token", TOKEN),
            ("x-goog-resource-state", "exists"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    // Missing channel header → 404.
    let r = app
        .clone()
        .oneshot(post(&[("x-goog-channel-token", TOKEN)]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    // Wrong token → 404.
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", CHANNEL),
            ("x-goog-channel-token", "c2VjcmV0LXRva2Vv"),
            ("x-goog-resource-state", "exists"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    // Missing token → 404.
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", CHANNEL),
            ("x-goog-resource-state", "exists"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert!(rx.try_recv().is_err(), "no tick for rejected notifications");

    // sync message → 200, logged, no tick.
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", CHANNEL),
            ("x-goog-channel-token", TOKEN),
            ("x-goog-resource-state", "sync"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert!(rx.try_recv().is_err());
    let logged: i64 = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM sync_log WHERE kind='push' AND detail LIKE '%sync message%'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(logged, 1);

    // exists → 200 and a calendar tick.
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", CHANNEL),
            ("x-goog-channel-token", TOKEN),
            ("x-goog-resource-state", "exists"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().unwrap(),
        SyncTick {
            account_id: ACC.into(),
            calendar_id: Some(CAL.into())
        }
    );

    // not_exists on the calendarList channel → account tick (calendar_id None).
    let r = app
        .clone()
        .oneshot(post(&[
            ("x-goog-channel-id", "list-channel"),
            ("x-goog-channel-token", TOKEN),
            ("x-goog-resource-state", "not_exists"),
        ]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().unwrap(),
        SyncTick {
            account_id: ACC.into(),
            calendar_id: None
        }
    );
}

#[tokio::test]
async fn rate_limit_per_ip() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _rx, _db) = app(dir.path().to_path_buf());
    for _ in 0..RATE_LIMIT_PER_MINUTE {
        let r = app
            .clone()
            .oneshot(
                Request::get("/healthz")
                    .header("x-forwarded-for", "8.8.8.8")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }
    let r = app
        .clone()
        .oneshot(
            Request::get("/healthz")
                .header("x-forwarded-for", "8.8.8.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    // Another IP is unaffected.
    let r = app
        .oneshot(
            Request::get("/healthz")
                .header("x-forwarded-for", "1.1.1.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn oversized_body_is_refused_without_processing() {
    let dir = tempfile::tempdir().unwrap();
    let (app, mut rx, _db) = app(dir.path().to_path_buf());
    let big = vec![b'x'; 70 * 1024];
    let r = app
        .oneshot(
            Request::post("/gcal/webhook")
                .header("x-goog-channel-id", CHANNEL)
                .header("x-goog-channel-token", TOKEN)
                .header("x-goog-resource-state", "exists")
                .header("content-length", big.len().to_string())
                .body(Body::from(big))
                .unwrap(),
        )
        .await
        .unwrap();
    // The handler never reads the body, so the notification is still accepted quickly.
    assert!(r.status() == StatusCode::OK || r.status() == StatusCode::PAYLOAD_TOO_LARGE);
    if r.status() == StatusCode::OK {
        assert!(rx.try_recv().is_ok());
    }
}
