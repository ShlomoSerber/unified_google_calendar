//! Guests, RSVP and Meet (docs/08 F3-T3): `patch` with the full attendee array, Meet
//! `createRequest` and polling while the conference is `pending`.

#![allow(dead_code)]

mod common;

use std::sync::Arc;

use common::{fixture, server};
use rusqlite::params;
use serde_json::json;
use unified_google_calendar_lib::commands::google_events as gw;
use unified_google_calendar_lib::commands::types::EventDraft;
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
async fn create_with_guests_and_meet_polls_until_ready() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    let mut pending = fixture("event_insert_response.json");
    pending["conferenceData"]["createRequest"]["status"]["statusCode"] = json!("pending");
    pending["hangoutLink"] = json!(null);
    Mock::given(method("POST"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(query_param("sendUpdates", "all"))
        .and(body_partial_json(json!({
            "summary": "Created from app",
            "attendees": [{ "email": "guest@example.com", "responseStatus": "needsAction" }],
            "conferenceData": { "createRequest": { "conferenceSolutionKey": { "type": "hangoutsMeet" } } },
            "start": { "dateTime": "2026-09-20T12:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(pending))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events/new1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")),
        )
        .expect(1)
        .mount(&s)
        .await;
    let mut d = draft("Created from app", 1_789_916_400, 1_789_920_000);
    d.attendees = vec!["guest@example.com".into()];
    d.add_meet = true;
    let r = gw::create(&ctx, &d).await;
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
    let (detail, touched) = r.unwrap();
    assert_eq!(detail.event_id, "new1");
    assert_eq!(
        detail.meet_link.as_deref(),
        Some("https://meet.google.com/new-meet-xyz")
    );
    assert_eq!(
        touched.from, 1_789_909_200,
        "range comes from the stored response"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='new1'").await,
        1
    );
}

#[tokio::test]
async fn rsvp_patches_the_full_attendee_array() {
    let s = server().await;
    let (ctx, db) = seeded(&s).await;
    // meet1 has attendees; the fixture organizer is self, so make a guest copy for the test.
    db.call(|c| {
        c.execute(
            "UPDATE events SET organizer_self=0, attendees='[{\"email\":\"boss@example.com\",\"organizer\":true,\"responseStatus\":\"accepted\"},{\"email\":\"someone@example.com\",\"self\":true,\"responseStatus\":\"needsAction\"}]', raw=json_set(raw, '$.attendees', json('[{\"email\":\"boss@example.com\",\"organizer\":true,\"responseStatus\":\"accepted\"},{\"email\":\"someone@example.com\",\"self\":true,\"responseStatus\":\"needsAction\"}]')) WHERE id='meet1'",
            [],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let occ = occ_id_of(&db, "meet1", 1_789_646_400).await;
    let mut resp = fixture("events_page2.json")["items"][0].clone();
    resp["attendees"] = json!([
        { "email": "boss@example.com", "organizer": true, "responseStatus": "accepted" },
        { "email": "someone@example.com", "self": true, "responseStatus": "accepted" }
    ]);
    Mock::given(method("PATCH"))
        .and(path("/calendars/someone%40example.com/events/meet1"))
        .and(query_param("sendUpdates", "all"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(json!({ "attendees": [
            { "email": "boss@example.com", "organizer": true, "responseStatus": "accepted" },
            { "email": "someone@example.com", "self": true, "responseStatus": "accepted" }
        ] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(resp))
        .expect(1)
        .mount(&s)
        .await;
    let (detail, _) = gw::rsvp(&ctx, &occ, "accepted", true).await.unwrap();
    assert_eq!(detail.my_response.as_deref(), Some("accepted"));
    assert!(gw::rsvp(&ctx, &occ, "maybe", true).await.is_err());
}
