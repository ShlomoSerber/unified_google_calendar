//! Creates the `UGC Fixtures` calendar and the test events of docs/design/fixture-events.md
//! in a Google account already added to the app, using the app's own stored tokens.
//! Usage: cargo run --example seed_fixtures -- <account_email> [--delete]
//! `--delete` removes the fixture calendar instead. Never touches other calendars.
//! Palette measurement (docs/04 section 7): `--calendar-color <1..24|get>` reads or sets the
//! fixture calendar's `calendarList` colorId; the colour events (one per event colorId) are
//! part of the regular seed, two weeks after the reference week.

use chrono::{Datelike, Duration, NaiveDate, TimeZone};
use serde_json::json;
use unified_google_calendar_lib::google::types::{Event, SendUpdates};
use unified_google_calendar_lib::google::Client;
use unified_google_calendar_lib::{auth, config, db};

const NAME: &str = "UGC Fixtures";
const TZ: &str = "America/Argentina/Buenos_Aires";

fn dt(monday: NaiveDate, day: i64, h: u32, m: u32) -> String {
    let tz: chrono_tz::Tz = TZ.parse().unwrap();
    let d = monday + Duration::days(day);
    tz.from_local_datetime(&d.and_hms_opt(h, m, 0).unwrap())
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn timed(
    monday: NaiveDate,
    day: i64,
    s: (u32, u32),
    e: (u32, u32),
    title: &str,
) -> serde_json::Value {
    json!({ "summary": title, "start": { "dateTime": dt(monday, day, s.0, s.1), "timeZone": TZ }, "end": { "dateTime": dt(monday, day, e.0, e.1), "timeZone": TZ } })
}

fn all_day(monday: NaiveDate, day: i64, title: &str) -> serde_json::Value {
    let d = monday + Duration::days(day);
    json!({ "summary": title, "start": { "date": d.format("%Y-%m-%d").to_string() }, "end": { "date": (d + Duration::days(1)).format("%Y-%m-%d").to_string() } })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let email = args
        .get(1)
        .expect("usage: seed_fixtures <account_email> [--delete]")
        .clone();
    let delete = args.iter().any(|a| a == "--delete");
    db::init(&config::db_path()).expect("db");
    auth::oauth::init().expect("oauth.json");
    let account = db::call(move |c| {
        Ok(db::queries::accounts::list_accounts(c)?
            .into_iter()
            .find(|a| a.email.as_deref() == Some(&email)))
    })
    .await
    .expect("db")
    .expect("account not found in the app");
    let token = auth::access_token(&account.id).await.expect("token");
    let client = Client::default();

    // Find an existing fixtures calendar.
    let page = client
        .calendar_list_page(&token, None, None)
        .await
        .expect("calendarList");
    let existing = page
        .items
        .iter()
        .find(|c| c.summary.as_deref() == Some(NAME))
        .map(|c| c.id.clone());
    if delete {
        match existing {
            Some(id) => {
                client.calendar_delete(&token, &id).await.expect("delete");
                println!("deleted {NAME} ({id})");
            }
            None => println!("no {NAME} calendar"),
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--calendar-color") {
        let id = existing.expect("no fixtures calendar");
        let path = format!("/users/me/calendarList/{}", Client::seg(&id));
        let want = args.get(i + 1).map(String::as_str).unwrap_or("get");
        if want != "get" {
            let _: serde_json::Value = client
                .send(
                    &token,
                    reqwest::Method::PATCH,
                    &path,
                    &[],
                    Some(&json!({ "colorId": want })),
                )
                .await
                .expect("calendarList.patch");
        }
        let entry: serde_json::Value = client
            .send(&token, reqwest::Method::GET, &path, &[], None)
            .await
            .expect("calendarList.get");
        println!(
            "colorId={} backgroundColor={}",
            entry["colorId"].as_str().unwrap_or(""),
            entry["backgroundColor"].as_str().unwrap_or("")
        );
        return;
    }
    let cal_id = match existing {
        Some(id) => {
            println!("{NAME} already exists ({id}); adding missing events only");
            id
        }
        None => {
            let created = client
                .calendar_insert(&token, NAME, TZ)
                .await
                .expect("calendars.insert");
            let id = created["id"].as_str().expect("id").to_string();
            println!("created {NAME} ({id})");
            id
        }
    };
    let have = client
        .events_page(&token, &cal_id, None, None)
        .await
        .expect("events");
    let titles: Vec<String> = have
        .items
        .iter()
        .filter_map(|e| e.summary.clone())
        .collect();

    let today = chrono::Utc::now()
        .with_timezone(&TZ.parse::<chrono_tz::Tz>().unwrap())
        .date_naive();
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let next_monday = monday + Duration::days(7);
    let self_email = account.email.clone().unwrap();
    let guests: Vec<String> = db::call(|c| {
        Ok(db::queries::accounts::list_accounts(c)?
            .into_iter()
            .filter(|a| a.kind == "google")
            .filter_map(|a| a.email)
            .collect())
    })
    .await
    .unwrap();
    let other_guest = guests.iter().find(|g| **g != self_email).cloned();

    let mut events = vec![
        all_day(monday, 0, "All-day one"),
        timed(monday, 0, (9, 0), (9, 15), "Fifteen"),
        timed(monday, 0, (10, 0), (10, 30), "Thirty"),
        timed(monday, 0, (11, 0), (11, 45), "Forty-five"),
        timed(monday, 0, (13, 0), (14, 0), "Sixty"),
        timed(monday, 0, (15, 0), (16, 30), "Ninety"),
        all_day(monday, 1, "All-day two A"),
        all_day(monday, 1, "All-day two B"),
        timed(monday, 1, (10, 0), (11, 0), "Overlap A"),
        timed(monday, 1, (10, 30), (11, 30), "Overlap B"),
        timed(monday, 2, (14, 0), (15, 0), "Triple A"),
        timed(monday, 2, (14, 15), (15, 15), "Triple B"),
        timed(monday, 2, (14, 30), (15, 30), "Triple C"),
        timed(monday, 5, (10, 0), (11, 0), "Weekend"),
    ];
    let mut meet = timed(monday, 3, (9, 0), (10, 0), "With Meet");
    meet["description"] = json!("Description line");
    meet["location"] = json!("Location text");
    meet["conferenceData"] = json!({ "createRequest": { "requestId": uuid::Uuid::new_v4().to_string(), "conferenceSolutionKey": { "type": "hangoutsMeet" } } });
    events.push(meet);
    let mut with_guests = timed(monday, 3, (11, 0), (12, 0), "With guests");
    let mut attendees =
        vec![json!({ "email": self_email, "organizer": true, "responseStatus": "accepted" })];
    if let Some(g) = &other_guest {
        attendees.push(json!({ "email": g, "responseStatus": "accepted" }));
    }
    attendees
        .push(json!({ "email": "guest.pending@example.com", "responseStatus": "needsAction" }));
    with_guests["attendees"] = json!(attendees);
    events.push(with_guests);
    let mut weekly = timed(monday, 4, (10, 0), (10, 30), "Weekly repeat");
    weekly["recurrence"] = json!(["RRULE:FREQ=WEEKLY"]);
    events.push(weekly);
    let mut tentative = timed(monday, 4, (12, 0), (13, 0), "Tentative");
    tentative["attendees"] =
        json!([{ "email": self_email, "self": true, "responseStatus": "tentative" }]);
    events.push(tentative);
    let mut declined = timed(monday, 4, (14, 0), (15, 0), "Declined");
    declined["attendees"] =
        json!([{ "email": self_email, "self": true, "responseStatus": "declined" }]);
    events.push(declined);
    for i in 0..5u32 {
        events.push(timed(
            next_monday,
            0,
            (8 + i, 0),
            (8 + i, 30),
            &format!("Month {}", i + 1),
        ));
    }
    // One event per event colorId, two weeks after the reference week (palette measurement).
    let color_monday = monday + Duration::days(14);
    for id in 1..=11u32 {
        let mut e = timed(
            color_monday,
            0,
            (7 + id, 0),
            (7 + id, 45),
            &format!("Color {id}"),
        );
        e["colorId"] = json!(id.to_string());
        events.push(e);
    }

    for ev in events {
        let title = ev["summary"].as_str().unwrap().to_string();
        if titles.contains(&title) {
            continue;
        }
        let body: Event = serde_json::from_value(ev).unwrap();
        match client
            .event_insert(&token, &cal_id, &body, SendUpdates::None)
            .await
        {
            Ok(e) => println!("created {title} ({})", e.id.unwrap_or_default()),
            Err(e) => println!("FAILED {title}: {e}"),
        }
    }
    println!("done; week of {monday}");
}
