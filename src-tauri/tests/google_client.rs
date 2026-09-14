//! Calendar API client against wiremock (docs/08 F2-T3): pagination, nextSyncToken, 410,
//! backoff on 429 and 403 usageLimits, query parameters of every write.

mod common;

use common::{fixture, server};
use serde_json::json;
use unified_google_calendar_lib::error::AppError;
use unified_google_calendar_lib::google::types::{Event, EventDateTime, SendUpdates, WatchRequest};
use unified_google_calendar_lib::google::Client;
use wiremock::matchers::{
    body_json, body_partial_json, header, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn calendar_list_paginates_and_returns_sync_token() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param("showHidden", "true"))
        .and(query_param("showDeleted", "true"))
        .and(query_param("maxResults", "250"))
        .and(query_param_is_missing("pageToken"))
        .and(header("authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page1.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .and(query_param("pageToken", "CL_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("calendar_list_page2.json")))
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let p1 = c.calendar_list_page("tok", None, None).await.unwrap();
    assert_eq!(p1.items.len(), 2);
    assert_eq!(p1.next_page_token.as_deref(), Some("CL_PAGE_2"));
    assert!(p1.next_sync_token.is_none());
    assert_eq!(p1.items[0].background_color.as_deref(), Some("#039be5"));
    assert_eq!(p1.items[0].primary, Some(true));
    let p2 = c
        .calendar_list_page("tok", p1.next_page_token.as_deref(), None)
        .await
        .unwrap();
    assert_eq!(p2.next_sync_token.as_deref(), Some("CL_SYNC_1"));
    assert_eq!(p2.items[1].deleted, Some(true));
    assert_eq!(p2.items[0].access_role.as_deref(), Some("reader"));
}

#[tokio::test]
async fn events_list_uses_fixed_params_and_pages_to_sync_token() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("singleEvents", "false"))
        .and(query_param("showDeleted", "true"))
        .and(query_param("maxResults", "2500"))
        .and(query_param_is_missing("timeMin"))
        .and(query_param_is_missing("timeMax"))
        .and(query_param_is_missing("orderBy"))
        .and(query_param_is_missing("pageToken"))
        .and(query_param_is_missing("syncToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page1.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("pageToken", "EV_PAGE_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page2.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/someone%40example.com/events"))
        .and(query_param("syncToken", "EV_SYNC_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_incremental.json")))
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let p1 = c
        .events_page("tok", "someone@example.com", None, None)
        .await
        .unwrap();
    assert_eq!(p1.items.len(), 3);
    assert_eq!(p1.next_page_token.as_deref(), Some("EV_PAGE_2"));
    assert!(p1.items[1].is_all_day());
    assert_eq!(
        p1.items[2].recurrence.as_ref().unwrap()[0],
        "RRULE:FREQ=WEEKLY;BYDAY=FR"
    );
    let p2 = c
        .events_page(
            "tok",
            "someone@example.com",
            p1.next_page_token.as_deref(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(p2.next_sync_token.as_deref(), Some("EV_SYNC_1"));
    assert!(p2.items[1].is_cancelled());
    assert_eq!(
        p2.items[0].hangout_link.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    let inc = c
        .events_page("tok", "someone@example.com", None, Some("EV_SYNC_1"))
        .await
        .unwrap();
    assert_eq!(inc.next_sync_token.as_deref(), Some("EV_SYNC_2"));
    assert_eq!(inc.items[1].recurring_event_id.as_deref(), Some("weekly1"));
    assert_eq!(
        inc.items[1]
            .original_start_time
            .as_ref()
            .unwrap()
            .date_time
            .as_deref(),
        Some("2026-09-25T10:00:00-03:00")
    );
}

#[tokio::test]
async fn gone_410_is_reported_as_google_error() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .and(query_param("syncToken", "stale"))
        .respond_with(ResponseTemplate::new(410).set_body_json(fixture("error_410.json")))
        .expect(1)
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let err = c
        .events_page("tok", "primary", None, Some("stale"))
        .await
        .unwrap_err();
    match err {
        AppError::Google { status, reason } => {
            assert_eq!(status, 410);
            assert_eq!(reason, "fullSyncRequired");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn backoff_retries_429_then_succeeds() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(429).set_body_json(fixture("error_429.json")))
        .up_to_n_times(2)
        .expect(2)
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_page2.json")))
        .expect(1)
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let p = c.events_page("tok", "primary", None, None).await.unwrap();
    assert_eq!(p.next_sync_token.as_deref(), Some("EV_SYNC_1"));
}

#[tokio::test]
async fn backoff_gives_up_after_five_attempts() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(403).set_body_json(fixture("error_403_quota.json")))
        .expect(5)
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let err = c
        .events_page("tok", "primary", None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Google { status: 403, .. }));
    assert!(err.user_message().contains("quota"));
}

#[tokio::test]
async fn non_retryable_errors_are_not_retried() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(fixture("error_404.json")))
        .expect(1)
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let err = c.event_get("tok", "primary", "missing").await.unwrap_err();
    assert!(matches!(err, AppError::Google { status: 404, .. }));
}

#[tokio::test]
async fn insert_update_patch_delete_send_the_right_params_and_bodies() {
    let s = server().await;
    let body = Event {
        summary: Some("Created from app".into()),
        start: Some(EventDateTime {
            date_time: Some("2026-09-20T10:00:00-03:00".into()),
            time_zone: Some("America/Argentina/Buenos_Aires".into()),
            ..Default::default()
        }),
        end: Some(EventDateTime {
            date_time: Some("2026-09-20T11:00:00-03:00".into()),
            time_zone: Some("America/Argentina/Buenos_Aires".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    Mock::given(method("POST"))
        .and(path("/calendars/primary/events"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(query_param("sendUpdates", "none"))
        .and(body_json(json!({
            "summary": "Created from app",
            "start": { "dateTime": "2026-09-20T10:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" },
            "end": { "dateTime": "2026-09-20T11:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("PUT"))
        .and(path("/calendars/primary/events/new1"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(query_param("sendUpdates", "all"))
        .and(body_partial_json(json!({ "summary": "Created from app" })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/calendars/primary/events/new1"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(query_param("sendUpdates", "all"))
        .and(body_json(json!({ "attendees": [{ "email": "a@b.c", "self": true, "responseStatus": "accepted" }] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/calendars/primary/events/new1"))
        .and(query_param("sendUpdates", "none"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/calendars/primary/events/new1/move"))
        .and(query_param(
            "destination",
            "other@group.calendar.google.com",
        ))
        .and(query_param("sendUpdates", "none"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/calendars/primary/events/import"))
        .and(query_param("conferenceDataVersion", "1"))
        .and(body_partial_json(json!({ "iCalUID": "new1@google.com" })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("event_insert_response.json")),
        )
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events/weekly1/instances"))
        .and(query_param("originalStart", "2026-09-25T10:00:00-03:00"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("events_incremental.json")))
        .expect(1)
        .mount(&s)
        .await;

    let c = Client::for_tests(s.uri());
    let created = c
        .event_insert("tok", "primary", &body, SendUpdates::None)
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("new1"));
    assert_eq!(
        created.hangout_link.as_deref(),
        Some("https://meet.google.com/new-meet-xyz")
    );
    c.event_update("tok", "primary", "new1", &created, SendUpdates::All)
        .await
        .unwrap();
    c.event_patch(
        "tok",
        "primary",
        "new1",
        &json!({ "attendees": [{ "email": "a@b.c", "self": true, "responseStatus": "accepted" }] }),
        SendUpdates::All,
    )
    .await
    .unwrap();
    c.event_delete("tok", "primary", "new1", SendUpdates::None)
        .await
        .unwrap();
    c.event_move("tok", "primary", "new1", "other@group.calendar.google.com")
        .await
        .unwrap();
    let mut import = created.clone();
    import.id = None;
    c.event_import("tok", "primary", &import).await.unwrap();
    let inst = c
        .event_instances(
            "tok",
            "primary",
            "weekly1",
            Some("2026-09-25T10:00:00-03:00"),
        )
        .await
        .unwrap();
    assert_eq!(inst.items.len(), 2);
}

#[tokio::test]
async fn watch_and_stop() {
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/calendars/en.ar%23holiday%40group.v.calendar.google.com/events/watch"))
        .and(body_json(json!({
            "id": "01234567-89ab-cdef-0123-456789abcdef", "type": "web_hook",
            "address": "https://pc.tail.ts.net/gcal/webhook", "token": "c2VjcmV0LXRva2Vu", "params": { "ttl": "604800" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("channel.json")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/users/me/calendarList/watch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("channel.json")))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/channels/stop"))
        .and(body_json(
            json!({ "id": "01234567-89ab-cdef-0123-456789abcdef", "resourceId": "o3hgv1538sdjfh" }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let c = Client::for_tests(s.uri());
    let req = WatchRequest::web_hook(
        "01234567-89ab-cdef-0123-456789abcdef".into(),
        "https://pc.tail.ts.net/gcal/webhook".into(),
        "c2VjcmV0LXRva2Vu".into(),
        604_800,
    );
    let ch = c
        .watch_events("tok", "en.ar#holiday@group.v.calendar.google.com", &req)
        .await
        .unwrap();
    assert_eq!(ch.resource_id, "o3hgv1538sdjfh");
    assert_eq!(ch.expiration, Some(1_789_604_800_000));
    c.watch_calendar_list("tok", &req).await.unwrap();
    c.channel_stop("tok", &ch.id, &ch.resource_id)
        .await
        .unwrap();
}
