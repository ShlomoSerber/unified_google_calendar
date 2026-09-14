//! Live check of the Evolution Data Server mirror (docs/08 F6-T3). Needs a GNOME session bus
//! with evolution-source-registry running, so it is `#[ignore]` by default:
//! `cargo test --test eds_live -- --ignored`. It creates and removes one `ugc-*` source.

use rusqlite::params;
use unified_google_calendar_lib::db::{self, DbHandle};
use unified_google_calendar_lib::eds::dbus::Eds;
use unified_google_calendar_lib::eds::mirror;

#[tokio::test]
#[ignore]
async fn mirror_creates_source_and_objects_then_removes_them() {
    let conn = db::open_memory().unwrap();
    conn.execute(
        "INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, visible) VALUES ('local-eds-test', 'local', 'EDS test', '#0b8043', '#fff', 'owner', 1)",
        [],
    )
    .unwrap();
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO events (account_id, calendar_id, id, status, summary, start_ts, end_ts, all_day, updated_ts) VALUES ('local', 'local-eds-test', 'e1', 'confirmed', 'UGC mirror test', ?1, ?2, 0, ?3)",
        params![now + 3600, now + 7200, now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO occurrences (id, account_id, calendar_id, event_id, start_ts, end_ts, all_day, status) VALUES ('local|local-eds-test|e1|x', 'local', 'local-eds-test', 'e1', ?1, ?2, 0, 'confirmed')",
        params![now + 3600, now + 7200],
    )
    .unwrap();
    let db = DbHandle::spawn(conn);
    let eds = Eds::connect().await.unwrap();
    let key = ("local".to_string(), "local-eds-test".to_string());
    mirror::mirror_calendars(&eds, &db, std::slice::from_ref(&key))
        .await
        .unwrap();
    let uid = mirror::source_uid("local", "local-eds-test");
    let sources = eds.sources().await.unwrap();
    let src = sources.get(&uid).expect("source created");
    assert!(src.data.contains("IncludeMe=false"));
    assert!(src.data.contains("DisplayName=EDS test"));
    let cal = eds.open_calendar(&uid).await.unwrap();
    let objects = cal.objects("#t").await.unwrap();
    cal.close().await;
    assert_eq!(objects.len(), 1, "{objects:?}");
    assert!(objects[0].contains("SUMMARY:UGC mirror test"));
    assert!(!objects[0].contains("VALARM"));
    // Second pass is a no-op and idempotent.
    mirror::mirror_calendars(&eds, &db, std::slice::from_ref(&key))
        .await
        .unwrap();
    // Cleanup: remove the source.
    eds.remove_source(&src.path).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(!eds.sources().await.unwrap().contains_key(&uid));
}
