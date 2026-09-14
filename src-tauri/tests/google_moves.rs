//! Moving events between calendars and accounts (docs/08 F3-T4): the four paths of
//! docs/03 section 7.

#![allow(dead_code)]

mod common;

use std::sync::Arc;

use common::{fixture, server};
use rusqlite::params;
use serde_json::json;
use unified_google_calendar_lib::commands::google_events as gw;
use unified_google_calendar_lib::commands::types::EventDraft;
use unified_google_calendar_lib::commands::view;
use unified_google_calendar_lib::db::{self, DbHandle};
use unified_google_calendar_lib::google::Client;
use unified_google_calendar_lib::sync::ctx::test_support::{FixedToken, Recorder};
use unified_google_calendar_lib::sync::{full_sync_calendar, SyncCtx};
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ACC: &str = "104857600000000000001";
const ACC2: &str = "104857600000000000002";
const CAL: &str = "someone@example.com";

fn test_db() -> DbHandle {
    let conn = db::open_memory().unwrap();
    for (id, email) in [(ACC, CAL), (ACC2, "other@example.com")] {
        conn.execute(
            "INSERT INTO accounts (id, kind, email, display_name, sort_order, created_at) VALUES (?1,'google',?2,?2,1,0)",
            params![id, email],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible, time_zone) VALUES (?1,?2,?1,'#039be5','#000000','owner',1,1,'America/Argentina/Buenos_Aires')",
            params![email, id],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, visible) VALUES ('second@group.calendar.google.com', ?1, 'Second', '#0b8043', '#000', 'writer', 1)",
        params![ACC],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('data_window_past_days', '3650')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('data_window_future_days', '3650')",
        [],
    )
    .unwrap();
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

/// Seed the calendar from the fixture pages (so rows carry realistic `raw`), then clear mocks.
async fn seeded(s: &MockServer) -> (SyncCtx, DbHandle) {
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("pageToken", "EV_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page2.json")))
        .mount(s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page1.json")))
        .mount(s)
        .await;
    let db = test_db();
    let ctx = ctx(s, db.clone());
    full_sync_calendar(&ctx, ACC, CAL).await.unwrap();
    s.reset().await;
    (ctx, db)
}

fn draft(title: &str, start: i64, end: i64) -> EventDraft {
    EventDraft {
        account_id: ACC.into(),
        calendar_id: CAL.into(),
        title: Some(title.into()),
        description: None,
        location: None,
        all_day: false,
        start: Some(start),
        end: Some(end),
        start_date: None,
        end_date: None,
        time_zone: Some("America/Argentina/Buenos_Aires".into()),
        recurrence: vec![],
        attendees: vec![],
        reminders: None,
        color_id: None,
        add_meet: false,
        transparency: None,
        visibility: None,
    }
}

async fn occ_id_of(db: &DbHandle, event_id: &'static str, start_ts: i64) -> String {
    db.call(move |c| {
        Ok(c.query_row(
            "SELECT id FROM occurrences WHERE (event_id=?1 OR master_id=?1) AND start_ts=?2",
            params![event_id, start_ts],
            |r| r.get::<_, String>(0),
        )?)
    })
    .await
    .unwrap()
}

async fn count(db: &DbHandle, sql: &'static str) -> i64 {
    db.call(move |c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap()
}

// weekly1: Fridays 10:00 BA from 2026-09-18. Instances (UTC): 2026-09-18T13:00Z = 1789736400.
const W1: i64 = 1_789_736_400;
const W2: i64 = W1 + 7 * 86_400;
const W3: i64 = W2 + 7 * 86_400;
const W4: i64 = W3 + 7 * 86_400;

#[tokio::test]
async fn move_same_account_uses_events_move() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "simple1", 1_789_401_600).await;
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events/simple1/move"))
        .and(query_param(
            "destination",
            "second@group.calendar.google.com",
        ))
        .and(query_param("sendUpdates", "none"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(fixture("events_page1.json")["items"][0].clone()),
        )
        .expect(1)
        .mount(&s)
        .await;
    let (detail, touched) = gw::move_event(&ctx, &occ, ACC, "second@group.calendar.google.com")
        .await
        .unwrap();
    assert_eq!(detail.calendar_id, "second@group.calendar.google.com");
    assert_eq!(touched.calendars.len(), 2);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM events WHERE id='simple1' AND calendar_id='someone@example.com'"
        )
        .await,
        0
    );
    assert_eq!(count(&db, "SELECT count(*) FROM events WHERE id='simple1' AND calendar_id='second@group.calendar.google.com'").await, 1);
}

#[tokio::test]
async fn move_between_accounts_imports_then_deletes() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "simple1", 1_789_401_600).await;
    let mut imported = fixture("events_page1.json")["items"][0].clone();
    imported["id"] = json!("imported1");
    Mock::given(method("POST"))
        .and(path("/calendars/other%40example.com/events/import"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(
            json!({ "iCalUID": "simple1@google.com", "summary": "Sixty" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(imported))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/calendars/someone%40example.com/events/simple1"))
        .and(query_param("sendUpdates", "none"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let (detail, _) = gw::move_event(&ctx, &occ, ACC2, "other@example.com")
        .await
        .unwrap();
    assert_eq!(detail.account_id, ACC2);
    assert_eq!(detail.event_id, "imported1");
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='simple1'").await,
        0
    );
    let reqs = s.received_requests().await.unwrap();
    let import = reqs
        .iter()
        .find(|r| r.url.path().ends_with("/import"))
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&import.body).unwrap();
    assert!(body.get("id").is_none() && body.get("organizer").is_none());
}

#[tokio::test]
async fn move_local_to_google_and_back() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    // Local event to move.
    let mut d = draft("Local one", 1_790_000_000, 1_790_003_600);
    d.account_id = "local".into();
    d.calendar_id = "local-personal".into();
    let (local_detail, _) = db
        .call(move |c| {
            let w = unified_google_calendar_lib::recurrence::Window::current(c)?;
            unified_google_calendar_lib::commands::events::create_local(c, &d, w)
        })
        .await
        .unwrap();
    let mut created = fixture("events_page1.json")["items"][0].clone();
    created["id"] = json!("fromlocal");
    created["summary"] = json!("Local one");
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("sendUpdates", "none"))
        .and(body_partial_json(
            json!({ "summary": "Local one", "start": { "dateTime": "2026-09-21T11:13:20-03:00" } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(created))
        .expect(1)
        .mount(&s)
        .await;
    let r = gw::move_event(&ctx, &local_detail.occurrence_id, ACC, CAL).await;
    if r.is_err() {
        for req in s.received_requests().await.unwrap() {
            eprintln!(
                "REQ {} {} {}",
                req.method,
                req.url,
                String::from_utf8_lossy(&req.body)
            );
        }
    }
    let (detail, _) = r.unwrap();
    assert_eq!(detail.event_id, "fromlocal");
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE account_id='local'").await,
        0
    );
    let reqs = s.received_requests().await.unwrap();
    let post = reqs.iter().find(|r| r.method == "POST").unwrap();
    let body: serde_json::Value = serde_json::from_slice(&post.body).unwrap();
    assert!(body["iCalUID"]
        .as_str()
        .unwrap()
        .ends_with("@unified-google-calendar"));

    // And back: Google → local deletes on Google and creates a local row without guests.
    Mock::given(method("DELETE"))
        .and(path("/calendars/someone%40example.com/events/fromlocal"))
        .and(query_param("sendUpdates", "none"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let (back, _) = gw::move_event(&ctx, &detail.occurrence_id, "local", "local-personal")
        .await
        .unwrap();
    assert!(back.is_local);
    assert_eq!(back.title.as_deref(), Some("Local one"));
    assert!(back.attendees.is_empty());
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='fromlocal'").await,
        0
    );
    let v = db
        .call(|c| view::get_view(c, 1_789_000_000, 1_790_500_000, "UTC"))
        .await
        .unwrap();
    assert!(v
        .occurrences
        .iter()
        .any(|o| o.is_local && o.title.as_deref() == Some("Local one")));
}
