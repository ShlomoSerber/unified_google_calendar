//! Insert/update/delete of single events against wiremock (docs/08 F3-T1): query params
//! (`conferenceDataVersion=1`, `sendUpdates` rule) and bodies built from `events.raw`.

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
async fn update_and_delete_single_event_build_from_raw() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let occ = occ_id_of(&db, "simple1", 1_789_401_600).await;
    let mut resp = fixture("event_insert_response.json");
    resp["id"] = json!("simple1");
    resp["summary"] = json!("Renamed");
    Mock::given(method("PUT"))
        .and(path("/calendars/someone%40example.com/events/simple1"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(query_param("sendUpdates", "none"))
        .and(body_partial_json(json!({ "id": "simple1", "summary": "Renamed", "description": "Description line", "location": "Location text", "reminders": { "useDefault": true } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(resp))
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
    let mut d = draft("Renamed", 1_789_401_600, 1_789_405_200);
    d.description = Some("Description line".into());
    d.location = Some("Location text".into());
    let (detail, _) = gw::update(&ctx, &occ, &d, EditScope::All).await.unwrap();
    assert_eq!(detail.title.as_deref(), Some("Renamed"));
    let touched = gw::delete(&ctx, &detail.occurrence_id, EditScope::This)
        .await
        .unwrap();
    assert_eq!(touched.calendars.len(), 1);
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='simple1'").await,
        0
    );
}
