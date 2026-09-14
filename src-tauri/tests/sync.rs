//! Sync engine against wiremock (docs/08 F2-T4): full sync of two pages, incremental with a
//! cancelled event and a new exception, 410 Gone that wipes and re-runs the full sync, and
//! the calendarList flow through `Engine::sync_account`.

mod common;

use std::sync::Arc;

use common::{fixture, server};
use rusqlite::params;
use unified_google_calendar_lib::db::{self, DbHandle};
use unified_google_calendar_lib::google::Client;
use unified_google_calendar_lib::sync::ctx::test_support::{FixedToken, Recorder};
use unified_google_calendar_lib::sync::engine::Engine;
use unified_google_calendar_lib::sync::{
    calendar_list, full_sync_calendar, incremental_calendar, SyncCtx,
};
use wiremock::matchers::{method, path, path_regex, query_param, query_param_is_missing};
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
        "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible, time_zone) VALUES (?1,?2,?1,'#039be5','#000000','owner',1,1,'America/Argentina/Buenos_Aires')",
        params![CAL, ACC],
    )
    .unwrap();
    // Wide window so the fixture dates (September 2026) stay inside regardless of today.
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

fn ctx(s: &MockServer, db: DbHandle, rec: Arc<Recorder>) -> SyncCtx {
    SyncCtx {
        db,
        client: Client::for_tests(s.uri()),
        tokens: Arc::new(FixedToken("tok".into())),
        events: rec,
    }
}

async fn mount_full_pages(s: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param_is_missing("pageToken"))
        .and(query_param_is_missing("syncToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page1.json")))
        .mount(s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("pageToken", "EV_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page2.json")))
        .mount(s)
        .await;
}

async fn count(db: &DbHandle, sql: &'static str) -> i64 {
    db.call(move |c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap()
}

async fn occurrence_starts(db: &DbHandle, event_id: &'static str) -> Vec<i64> {
    db.call(move |c| {
        let mut st = c.prepare(
            "SELECT start_ts FROM occurrences WHERE event_id=?1 OR master_id=?1 ORDER BY start_ts",
        )?;
        let v = st
            .query_map(params![event_id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn full_sync_two_pages_stores_events_and_materializes() {
    let s = server().await;
    mount_full_pages(&s).await;
    let db = test_db();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec.clone());
    full_sync_calendar(&ctx, ACC, CAL).await.unwrap();

    assert_eq!(
        count(&db, "SELECT count(*) FROM events").await,
        5,
        "all items including the cancelled one are stored"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE status='cancelled'").await,
        1
    );
    let (token, done): (String, i64) = db
        .call(|c| Ok(c.query_row("SELECT sync_token, full_sync_done FROM calendars WHERE account_id='104857600000000000001'", [], |r| Ok((r.get(0)?, r.get(1)?)))?))
        .await
        .unwrap();
    assert_eq!(token, "EV_SYNC_1");
    assert_eq!(done, 1);
    // simple1, allday1, meet1 → one occurrence each; weekly1 → many; gone1 → none.
    assert_eq!(occurrence_starts(&db, "simple1").await, vec![1_789_401_600]);
    assert_eq!(occurrence_starts(&db, "allday1").await.len(), 1);
    assert_eq!(occurrence_starts(&db, "meet1").await.len(), 1);
    assert!(occurrence_starts(&db, "weekly1").await.len() > 100);
    assert!(occurrence_starts(&db, "gone1").await.is_empty());
    let raw: String = db
        .call(|c| {
            Ok(
                c.query_row("SELECT raw FROM events WHERE id='simple1'", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert!(raw.contains("\"htmlLink\""));
    assert_eq!(
        count(&db, "SELECT count(*) FROM sync_log WHERE kind='full'").await,
        1
    );
    let updates = rec.updates.lock().unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].calendar_ids[0].calendar_id, CAL);
}

#[tokio::test]
async fn incremental_applies_cancellation_and_new_exception() {
    let s = server().await;
    mount_full_pages(&s).await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("syncToken", "EV_SYNC_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_incremental.json")))
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec.clone());
    // No token yet → incremental falls back to full.
    incremental_calendar(&ctx, ACC, CAL).await.unwrap();
    let before = occurrence_starts(&db, "weekly1").await;
    assert!(
        before.contains(&1_790_341_200),
        "2026-09-25 10:00 BA instance exists before the exception"
    );

    incremental_calendar(&ctx, ACC, CAL).await.unwrap();
    assert!(
        occurrence_starts(&db, "simple1").await.is_empty(),
        "cancelled simple event loses its occurrence"
    );
    let status: String = db
        .call(|c| {
            Ok(
                c.query_row("SELECT status FROM events WHERE id='simple1'", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(status, "cancelled");
    let after = occurrence_starts(&db, "weekly1").await;
    assert!(
        !after.contains(&1_790_341_200),
        "original instance replaced"
    );
    assert!(
        after.contains(&1_790_359_200),
        "moved exception at 15:00 BA appears"
    );
    assert_eq!(after.len(), before.len());
    let token: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT sync_token FROM calendars WHERE account_id='104857600000000000001'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(token, "EV_SYNC_2");
    let updates = rec.updates.lock().unwrap();
    assert_eq!(updates.len(), 2);
    let inc = &updates[1];
    assert!(
        inc.from <= 1_789_401_600 && inc.to >= 1_790_361_000,
        "range covers the cancelled event and the moved exception"
    );
}

#[tokio::test]
async fn gone_410_wipes_and_runs_full_sync() {
    let s = server().await;
    mount_full_pages(&s).await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("syncToken", "stale"))
        .respond_with(ResponseTemplate::new(410).set_body_json(fixture("error_410.json")))
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    db.call(|c| {
        c.execute("UPDATE calendars SET sync_token='stale', full_sync_done=1 WHERE account_id=?1", params![ACC])?;
        // A stale local row that Google no longer has: must disappear with the wipe.
        c.execute(
            "INSERT INTO events (account_id, calendar_id, id, status, all_day) VALUES (?1, ?2, 'stale-row', 'confirmed', 0)",
            params![ACC, CAL],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec.clone());
    incremental_calendar(&ctx, ACC, CAL).await.unwrap();
    assert_eq!(
        count(&db, "SELECT count(*) FROM events WHERE id='stale-row'").await,
        0
    );
    assert_eq!(count(&db, "SELECT count(*) FROM events").await, 5);
    let token: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT sync_token FROM calendars WHERE account_id='104857600000000000001'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(token, "EV_SYNC_1");
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_log WHERE kind='error' AND detail LIKE '410%'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn calendar_list_sync_upserts_marks_deleted_and_keeps_visibility() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page1.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param("pageToken", "CL_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page2.json")))
        .mount(&s)
        .await;
    let db = test_db();
    // Pre-existing rows: the primary (hidden by the user) and the one Google now reports deleted.
    db.call(|c| {
        c.execute("UPDATE calendars SET visible=0 WHERE id=?1", params![CAL])?;
        c.execute(
            "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role) VALUES ('old@group.calendar.google.com', ?1, 'Old', '#000', '#fff', 'owner')",
            params![ACC],
        )?;
        c.execute(
            "INSERT INTO events (account_id, calendar_id, id, status, all_day) VALUES (?1, 'old@group.calendar.google.com', 'e', 'confirmed', 0)",
            params![ACC],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec);
    let outcome = calendar_list::sync_calendar_list(&ctx, ACC).await.unwrap();
    assert_eq!(
        outcome.new_calendars,
        vec![
            "abc123@group.calendar.google.com",
            "en.ar#holiday@group.v.calendar.google.com"
        ]
    );
    assert_eq!(
        outcome.deleted_calendars,
        vec!["old@group.calendar.google.com"]
    );
    let rows: Vec<(String, i64, i64, String)> = db
        .call(|c| {
            let mut st =
                c.prepare("SELECT id, visible, deleted, access_role FROM calendars ORDER BY id")?;
            let v = st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(v)
        })
        .await
        .unwrap();
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|r| r.0 != "local-personal")
        .collect();
    assert_eq!(rows.len(), 4);
    let primary = rows.iter().find(|r| r.0 == CAL).unwrap();
    assert_eq!(primary.1, 0, "user's checkbox survives the resync");
    let fixtures = rows
        .iter()
        .find(|r| r.0 == "abc123@group.calendar.google.com")
        .unwrap();
    assert_eq!(fixtures.1, 0, "selected=false on first import");
    let holidays = rows.iter().find(|r| r.0.starts_with("en.ar")).unwrap();
    assert_eq!((holidays.1, holidays.3.as_str()), (1, "reader"));
    let old = rows.iter().find(|r| r.0.starts_with("old@")).unwrap();
    assert_eq!(old.2, 1);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM events WHERE calendar_id='old@group.calendar.google.com'"
        )
        .await,
        0
    );
    let token: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT calendar_list_sync_token FROM accounts WHERE kind='google'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(token, "CL_SYNC_1");
}

#[tokio::test]
async fn engine_sync_account_runs_list_then_every_calendar() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page1.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param("pageToken", "CL_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page2.json")))
        .mount(&s)
        .await;
    // Every calendar's events: one page with a sync token.
    Mock::given(method("GET"))
        .and(path_regex(r"^/calendars/[^/]+/events$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page2.json")))
        .expect(3)
        .mount(&s)
        .await;
    let db = test_db();
    let rec = Arc::new(Recorder::default());
    let (engine, _rx) = Engine::new(ctx(&s, db.clone(), rec.clone()));
    engine.sync_account(ACC, "test").await.unwrap();
    assert_eq!(count(&db, "SELECT count(*) FROM calendars WHERE deleted=0 AND full_sync_done=1 AND account_id='104857600000000000001'").await, 3);
    assert_eq!(count(&db, "SELECT count(*) FROM events").await, 6);
    let (state, last): (String, Option<i64>) = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT sync_state, last_sync_at FROM accounts WHERE kind='google'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(state, "idle");
    assert!(last.is_some());
    {
        let statuses = rec.statuses.lock().unwrap();
        assert_eq!(statuses.first().map(|s| s.state.as_str()), Some("syncing"));
        assert_eq!(statuses.last().map(|s| s.state.as_str()), Some("idle"));
    }

    // A network failure on the second run marks the account `error` without panicking.
    s.reset().await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .respond_with(ResponseTemplate::new(503).set_body_string("{}"))
        .mount(&s)
        .await;
    assert!(engine.sync_account(ACC, "test").await.is_err());
    let state: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT sync_state FROM accounts WHERE kind='google'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(state, "error");
}

#[tokio::test]
async fn poll_interval_follows_push_setting_and_window_refresh_prunes() {
    use unified_google_calendar_lib::sync::poll;
    let s = server().await;
    let db = test_db();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec);
    assert_eq!(poll::interval_for(&ctx).await, poll::POLL_WITHOUT_PUSH);
    db.call(|c| {
        c.execute("INSERT INTO settings (key, value) VALUES ('push_enabled', 'true')", [])?;
        // An occurrence far outside any window, and a stale one to be rebuilt.
        c.execute(
            "INSERT INTO events (account_id, calendar_id, id, status, start_ts, end_ts, all_day) VALUES (?1, ?2, 'old', 'confirmed', 100, 200, 0)",
            params![ACC, CAL],
        )?;
        c.execute(
            "INSERT INTO occurrences (id, account_id, calendar_id, event_id, start_ts, end_ts, all_day, status) VALUES ('x', ?1, ?2, 'old', 100, 200, 0, 'confirmed')",
            params![ACC, CAL],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(poll::interval_for(&ctx).await, poll::POLL_WITH_PUSH);
    let w = poll::refresh_window(&ctx).await.unwrap();
    assert!(w.from_ts > 200);
    assert_eq!(
        count(&db, "SELECT count(*) FROM occurrences WHERE id='x'").await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_log WHERE detail='data window refreshed'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn ical_subscription_is_added_synced_and_skipped_when_unchanged() {
    use unified_google_calendar_lib::sync::ical;
    use wiremock::matchers::header;
    let s = server().await;
    let feed = "BEGIN:VCALENDAR\r\nX-WR-CALNAME:RappiCard\r\nX-WR-TIMEZONE:America/Mexico_City\r\nBEGIN:VEVENT\r\nUID:one@google.com\r\nDTSTART;TZID=America/Mexico_City:20260914T090000\r\nDTEND;TZID=America/Mexico_City:20260914T093000\r\nSUMMARY:Standup\r\nATTENDEE;PARTSTAT=ACCEPTED:mailto:me@rappicard.mx\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    Mock::given(method("GET"))
        .and(path("/private/basic.ics"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/private/basic.ics"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_string(feed),
        )
        .expect(1)
        .mount(&s)
        .await;
    let db = test_db();
    let rec = Arc::new(Recorder::default());
    let ctx = ctx(&s, db.clone(), rec.clone());
    // The URL goes to the encrypted token store; keep it in a temp dir.
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_DATA_HOME", tmp.path());
    let url = format!("{}/private/basic.ics", s.uri());
    assert!(ical::add_ical_account(&ctx, "  ", &url, None)
        .await
        .is_err());
    assert!(
        ical::add_ical_account(&ctx, "RappiCard", "http://example.com/x.ics", None)
            .await
            .is_err(),
        "https required off loopback"
    );
    let id = ical::add_ical_account(&ctx, "RappiCard", &url, Some("me@rappicard.mx"))
        .await
        .unwrap();
    assert_eq!(
        count(&db, "SELECT count(*) FROM accounts WHERE kind='ical'").await,
        1
    );
    let (summary, role): (String, String) = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT summary, access_role FROM calendars WHERE id='ical'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .await
        .unwrap();
    assert_eq!((summary.as_str(), role.as_str()), ("RappiCard", "reader"));
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM occurrences WHERE calendar_id='ical'"
        )
        .await,
        1
    );
    let att: String = db
        .call(|c| {
            Ok(c.query_row(
                "SELECT attendees FROM events WHERE id='one@google.com'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert!(att.contains("\"self\":true"));
    assert!(
        !std::fs::read(tmp.path().join("unified-google-calendar/tokens.bin"))
            .unwrap()
            .windows(7)
            .any(|w| w == b"private"),
        "url encrypted"
    );
    // Second sync: the server answers 304 and nothing changes.
    ical::sync_ical_account(&ctx, &id).await.unwrap();
    assert_eq!(rec.updates.lock().unwrap().len(), 1);
}
