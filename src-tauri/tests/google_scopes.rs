//! Recurrence scopes against Google (docs/08 F3-T2): `this` on the instance id with the
//! `events.instances` fallback, `following` with `UNTIL` + new master, `all` on the master.

#![allow(dead_code)]

mod common;

use std::sync::Arc;

use common::{fixture, server};
use rusqlite::params;
use serde_json::json;
use unified_google_calendar_lib::commands::google_events as gw;
use unified_google_calendar_lib::commands::types::{EditScope, EventDraft};
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
async fn scope_this_updates_the_instance_id_and_stores_the_exception() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "weekly1", W2).await;
    Mock::given(method("GET"))
        .and(path(
            "/calendars/someone%40example.com/events/weekly1_20260925T130000Z",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "id": "weekly1_20260925T130000Z" })),
        )
        .expect(1)
        .mount(&s)
        .await;
    let exception = fixture("events_incremental.json")["items"][1].clone();
    Mock::given(method("PUT"))
        .and(path("/calendars/someone%40example.com/events/weekly1_20260925T130000Z"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(json!({ "id": "weekly1_20260925T130000Z", "summary": "Weekly repeat (moved)", "start": { "dateTime": "2026-09-25T15:00:00-03:00" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(exception))
        .expect(1)
        .mount(&s)
        .await;
    let d = draft("Weekly repeat (moved)", 1_790_359_200, 1_790_361_000);
    let (detail, _) = gw::update(&ctx, &occ, &d, EditScope::This).await.unwrap();
    assert!(detail.is_exception);
    assert_eq!(detail.start, 1_790_359_200);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM events WHERE recurring_event_id='weekly1'"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1' AND start_ts=1790341200"
        )
        .await,
        0
    );
    // The PUT body must not carry a recurrence.
    let reqs = s.received_requests().await.unwrap();
    let put = reqs.iter().find(|r| r.method == "PUT").unwrap();
    let body: serde_json::Value = serde_json::from_slice(&put.body).unwrap();
    assert!(body.get("recurrence").is_none());
}

#[tokio::test]
async fn scope_this_falls_back_to_instances_when_constructed_id_is_unknown() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "weekly1", W2).await;
    Mock::given(method("GET"))
        .and(path(
            "/calendars/someone%40example.com/events/weekly1_20260925T130000Z",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(fixture("error_404.json")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/calendars/someone%40example.com/events/weekly1/instances",
        ))
        .and(query_param("originalStart", "2026-09-25T10:00:00-03:00"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "items": [{ "id": "weekly1_realid" }] })),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("DELETE"))
        .and(path(
            "/calendars/someone%40example.com/events/weekly1_realid",
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    gw::delete(&ctx, &occ, EditScope::This).await.unwrap();
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM events WHERE recurring_event_id='weekly1' AND status='cancelled'"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1' AND start_ts=1790341200"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn scope_following_truncates_master_and_inserts_new_series() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "weekly1", W3).await;
    let mut truncated = fixture("events_page1.json")["items"][2].clone();
    truncated["recurrence"] = json!(["RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261002T125959Z"]);
    Mock::given(method("PUT"))
        .and(path("/calendars/someone%40example.com/events/weekly1"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(json!({ "id": "weekly1", "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261002T125959Z"] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(truncated))
        .expect(1)
        .mount(&s)
        .await;
    let mut new_master = fixture("events_page1.json")["items"][2].clone();
    new_master["id"] = json!("weekly2");
    new_master["iCalUID"] = json!("weekly2@google.com");
    new_master["summary"] = json!("Later");
    new_master["start"] = json!({ "dateTime": "2026-10-02T11:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" });
    new_master["end"] = json!({ "dateTime": "2026-10-02T11:30:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" });
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(json!({ "summary": "Later", "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=FR"], "start": { "dateTime": "2026-10-02T11:00:00-03:00" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(new_master))
        .expect(1)
        .mount(&s)
        .await;
    let d = draft("Later", W3 + 3600, W3 + 5400);
    let (detail, _) = gw::update(&ctx, &occ, &d, EditScope::Following)
        .await
        .unwrap();
    assert_eq!(detail.event_id, "weekly2");
    assert_eq!(detail.start, W3 + 3600);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1'"
        )
        .await,
        2
    );
    assert!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly2'"
        )
        .await
            > 50
    );
    let reqs = s.received_requests().await.unwrap();
    let post = reqs.iter().find(|r| r.method == "POST").unwrap();
    let body: serde_json::Value = serde_json::from_slice(&post.body).unwrap();
    assert!(
        body.get("id").is_none() && body.get("iCalUID").is_none() && body.get("etag").is_none()
    );
}

#[tokio::test]
async fn scope_all_shifts_the_series() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "weekly1", W4).await;
    let mut shifted = fixture("events_page1.json")["items"][2].clone();
    shifted["summary"] = json!("Weekly at 11");
    shifted["start"] = json!({ "dateTime": "2026-09-18T11:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" });
    shifted["end"] = json!({ "dateTime": "2026-09-18T12:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" });
    Mock::given(method("PUT"))
        .and(path("/calendars/someone%40example.com/events/weekly1"))
        .and(body_partial_json(json!({
            "id": "weekly1", "summary": "Weekly at 11", "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=FR"],
            "start": { "dateTime": "2026-09-18T11:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" },
            "end": { "dateTime": "2026-09-18T12:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(shifted))
        .expect(1)
        .mount(&s)
        .await;
    // +1 hour and 60 minutes long, applied on the 4th instance.
    let d = draft("Weekly at 11", W4 + 3600, W4 + 7200);
    let (detail, _) = gw::update(&ctx, &occ, &d, EditScope::All).await.unwrap();
    assert_eq!(detail.title.as_deref(), Some("Weekly at 11"));
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1' AND start_ts=1789740000"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1' AND start_ts=1789736400"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn delete_following_and_all() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "weekly1", W3).await;
    let mut truncated = fixture("events_page1.json")["items"][2].clone();
    truncated["recurrence"] = json!(["RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261002T125959Z"]);
    Mock::given(method("PUT"))
        .and(path("/calendars/someone%40example.com/events/weekly1"))
        .and(body_partial_json(
            json!({ "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261002T125959Z"] }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(truncated))
        .expect(1)
        .mount(&s)
        .await;
    gw::delete(&ctx, &occ, EditScope::Following).await.unwrap();
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1'"
        )
        .await,
        2
    );
    Mock::given(method("DELETE"))
        .and(path("/calendars/someone%40example.com/events/weekly1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let first = occ_id_of(&db, "weekly1", W1).await;
    gw::delete(&ctx, &first, EditScope::All).await.unwrap();
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='weekly1'").await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE master_id='weekly1'"
        )
        .await,
        0
    );
}
