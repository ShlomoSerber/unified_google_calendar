//! Write commands: create, update, delete, RSVP and move. See docs/02-arquitectura.md
//! section 3.3 and docs/03-modelo-de-datos.md sections 4 and 7.
//!
//! Local accounts are handled here on the local tables. Google accounts are routed to the
//! Google client (phase 3); until then they return an error.

use chrono::Duration;
use rusqlite::Connection;

use crate::commands::types::{CalendarKey, EditScope, EventDetail, EventDraft};
use crate::commands::view;
use crate::config::{LOCAL_ACCOUNT_ID, LOCAL_ICAL_SUFFIX};
use crate::db::queries::{calendars, events as q};
use crate::error::AppError;
use crate::recurrence::edit_scope::{self, OccurrenceRef};
use crate::recurrence::expand::{self, MasterSpec, Window};

/// Range in UTC seconds touched by a write, for `calendar:updated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Touched {
    pub from: i64,
    pub to: i64,
    pub calendars: Vec<CalendarKey>,
}

impl Touched {
    fn new(from: i64, to: i64, account_id: &str, calendar_id: &str) -> Touched {
        Touched {
            from,
            to,
            calendars: vec![CalendarKey {
                account_id: account_id.into(),
                calendar_id: calendar_id.into(),
            }],
        }
    }

    fn whole_window(window: Window, account_id: &str, calendar_id: &str) -> Touched {
        Touched::new(window.from_ts, window.to_ts, account_id, calendar_id)
    }

    fn merge(mut self, other: Touched) -> Touched {
        self.from = self.from.min(other.from);
        self.to = self.to.max(other.to);
        for k in other.calendars {
            if !self.calendars.contains(&k) {
                self.calendars.push(k);
            }
        }
        self
    }
}

pub fn is_local(account_id: &str) -> bool {
    account_id == LOCAL_ACCOUNT_ID
}

/// Validate a draft and turn it into the fields of an `events` row.
/// `existing` keeps the identity and Google-owned columns when updating.
pub fn draft_to_row(
    draft: &EventDraft,
    existing: Option<&EventRowRef<'_>>,
) -> Result<q::EventRow, AppError> {
    let title = draft
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string);
    let mut row = match existing {
        Some(e) => e.0.clone(),
        None => q::EventRow {
            account_id: draft.account_id.clone(),
            calendar_id: draft.calendar_id.clone(),
            status: "confirmed".into(),
            event_type: "default".into(),
            ..Default::default()
        },
    };
    if draft.all_day {
        let s = draft
            .start_date
            .as_deref()
            .ok_or_else(|| AppError::invalid("An all-day event needs a start date"))?;
        let e = draft
            .end_date
            .as_deref()
            .ok_or_else(|| AppError::invalid("An all-day event needs an end date"))?;
        let sd = expand::parse_date(s)
            .map_err(|_| AppError::invalid("The start date is not a valid YYYY-MM-DD date"))?;
        let ed = expand::parse_date(e)
            .map_err(|_| AppError::invalid("The end date is not a valid YYYY-MM-DD date"))?;
        if ed <= sd {
            return Err(AppError::invalid(
                "The end date must be after the start date",
            ));
        }
        row.all_day = true;
        row.start_date = Some(sd.format("%Y-%m-%d").to_string());
        row.end_date = Some(ed.format("%Y-%m-%d").to_string());
        row.start_ts = None;
        row.end_ts = None;
        row.time_zone = draft.time_zone.clone();
    } else {
        let s = draft
            .start
            .ok_or_else(|| AppError::invalid("The event needs a start time"))?;
        let e = draft
            .end
            .ok_or_else(|| AppError::invalid("The event needs an end time"))?;
        if e <= s {
            return Err(AppError::invalid(
                "The end time must be after the start time",
            ));
        }
        row.all_day = false;
        row.start_ts = Some(s);
        row.end_ts = Some(e);
        row.start_date = None;
        row.end_date = None;
        row.time_zone = Some(
            draft
                .time_zone
                .clone()
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| crate::config::DEFAULT_PRIMARY_TZ.to_string()),
        );
        if let Some(tz) = &row.time_zone {
            tz.parse::<chrono_tz::Tz>()
                .map_err(|_| AppError::invalid(format!("{tz} is not a valid time zone name")))?;
        }
    }
    row.summary = title;
    row.description = draft.description.clone().filter(|d| !d.is_empty());
    row.location = draft.location.clone().filter(|l| !l.is_empty());
    row.color_id = draft.color_id.clone().filter(|c| !c.is_empty());
    if let Some(c) = &row.color_id {
        if crate::google::colors::by_id(c).is_none() {
            return Err(AppError::invalid(
                "The event color is not one of the 11 palette colors",
            ));
        }
    }
    row.transparency = draft.transparency.clone();
    row.visibility = draft.visibility.clone();
    row.recurrence = if draft.recurrence.is_empty() {
        None
    } else {
        Some(draft.recurrence.clone())
    };
    if let Some(lines) = &row.recurrence {
        if !lines
            .iter()
            .any(|l| l.to_ascii_uppercase().starts_with("RRULE:"))
        {
            return Err(AppError::invalid("The repeat settings need an RRULE line"));
        }
        let spec = MasterSpec::from(&row);
        let probe_from = row.start_ts.unwrap_or(0);
        expand::expand_dates(
            &spec,
            Window {
                from_ts: probe_from,
                to_ts: probe_from + 86_400 * 366,
            },
            10,
        )
        .map_err(|e| AppError::invalid(format!("The repeat settings are not valid ({e})")))?;
    }
    row.attendees = serde_json::to_string(
        &draft
            .attendees
            .iter()
            .map(|e| e.trim())
            .filter(|e| !e.is_empty())
            .map(|e| serde_json::json!({ "email": e, "responseStatus": "needsAction" }))
            .collect::<Vec<_>>(),
    )?;
    row.reminders = match &draft.reminders {
        None => r#"{"useDefault":true}"#.to_string(),
        Some(list) => {
            if list.len() > 5 {
                return Err(AppError::invalid("An event can have at most 5 reminders"));
            }
            for r in list {
                if r.method != "popup" && r.method != "email" {
                    return Err(AppError::invalid("Reminder method must be popup or email"));
                }
                if !(0..=40_320).contains(&r.minutes) {
                    return Err(AppError::invalid(
                        "Reminder minutes must be between 0 and 40320",
                    ));
                }
            }
            serde_json::to_string(&serde_json::json!({ "useDefault": false, "overrides": list }))?
        }
    };
    Ok(row)
}

/// Wrapper so `draft_to_row` can take an optional existing row by reference.
pub struct EventRowRef<'a>(pub &'a q::EventRow);

fn ensure_local_calendar(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
) -> Result<(), AppError> {
    let cal = calendars::get_calendar(conn, account_id, calendar_id)?
        .ok_or_else(|| AppError::NotFound("The calendar".into()))?;
    if !cal.can_write() {
        return Err(AppError::invalid(
            "You do not have permission to write to this calendar",
        ));
    }
    Ok(())
}

fn first_occurrence_id(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    near: Option<i64>,
) -> Result<String, AppError> {
    let occ = q::occurrences_of_event(conn, account_id, calendar_id, event_id)?;
    let pick = match near {
        Some(t) => occ.iter().min_by_key(|o| (o.start_ts - t).abs()),
        None => occ.first(),
    };
    pick.map(|o| o.id.clone())
        .ok_or_else(|| AppError::Invalid("The event is outside the data window".into()))
}

/// Create a local event. Returns the detail of its first occurrence and the touched range.
pub fn create_local(
    conn: &mut Connection,
    draft: &EventDraft,
    window: Window,
) -> Result<(EventDetail, Touched), AppError> {
    ensure_local_calendar(conn, &draft.account_id, &draft.calendar_id)?;
    let mut row = draft_to_row(draft, None)?;
    row.id = uuid::Uuid::new_v4().to_string();
    row.ical_uid = Some(format!("{}{}", row.id, LOCAL_ICAL_SUFFIX));
    row.created_ts = Some(crate::db::now_ts());
    row.updated_ts = row.created_ts;
    q::upsert_event(conn, &row)?;
    expand::materialize_simple(conn, &row.account_id, &row.calendar_id, &row.id, window)?;
    let occ_id = first_occurrence_id(conn, &row.account_id, &row.calendar_id, &row.id, None)?;
    let detail = view::get_event(conn, &occ_id)?;
    let touched = if row.is_recurring_master() {
        Touched::whole_window(window, &row.account_id, &row.calendar_id)
    } else {
        Touched::new(detail.start, detail.end, &row.account_id, &row.calendar_id)
    };
    Ok((detail, touched))
}

fn occurrence_ref(
    conn: &Connection,
    occurrence_id: &str,
) -> Result<(OccurrenceRef, q::OccurrenceRow), AppError> {
    let occ = q::get_occurrence(conn, occurrence_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    Ok((
        OccurrenceRef {
            account_id: occ.account_id.clone(),
            calendar_id: occ.calendar_id.clone(),
            event_id: occ.event_id.clone(),
            master_id: occ.master_id.clone(),
            start_ts: occ.start_ts,
        },
        occ,
    ))
}

/// Update a local event with scope. Returns the detail of the edited occurrence.
pub fn update_local(
    conn: &mut Connection,
    occurrence_id: &str,
    draft: &EventDraft,
    scope: EditScope,
    window: Window,
) -> Result<(EventDetail, Touched), AppError> {
    let (occ_ref, occ) = occurrence_ref(conn, occurrence_id)?;
    ensure_local_calendar(conn, &occ.account_id, &occ.calendar_id)?;
    let existing = q::get_event(conn, &occ.account_id, &occ.calendar_id, &occ.event_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    let row = draft_to_row(draft, Some(&EventRowRef(&existing)))?;
    let old_span = Touched::new(occ.start_ts, occ.end_ts, &occ.account_id, &occ.calendar_id);
    let new_id = edit_scope::apply_edit(conn, scope, &occ_ref, &row, window)?;
    let near = row.start_ts.or_else(|| {
        row.start_date
            .as_deref()
            .and_then(|d| expand::parse_date(d).ok())
            .map(expand::date_to_ts)
    });
    let occ_id = first_occurrence_id(conn, &occ.account_id, &occ.calendar_id, &new_id, near)?;
    let detail = view::get_event(conn, &occ_id)?;
    let touched = match scope {
        EditScope::This if occ.master_id.is_none() && !row.is_recurring_master() => old_span.merge(
            Touched::new(detail.start, detail.end, &occ.account_id, &occ.calendar_id),
        ),
        _ => Touched::whole_window(window, &occ.account_id, &occ.calendar_id),
    };
    Ok((detail, touched))
}

/// Delete a local event with scope.
pub fn delete_local(
    conn: &mut Connection,
    occurrence_id: &str,
    scope: EditScope,
    window: Window,
) -> Result<Touched, AppError> {
    let (occ_ref, occ) = occurrence_ref(conn, occurrence_id)?;
    ensure_local_calendar(conn, &occ.account_id, &occ.calendar_id)?;
    edit_scope::apply_delete(conn, scope, &occ_ref, window)?;
    Ok(match scope {
        EditScope::This => {
            Touched::new(occ.start_ts, occ.end_ts, &occ.account_id, &occ.calendar_id)
        }
        _ => Touched::whole_window(window, &occ.account_id, &occ.calendar_id),
    })
}

/// Move a local event (the whole series) to another local calendar. Docs/03 section 7.
pub fn move_local_to_local(
    conn: &mut Connection,
    occurrence_id: &str,
    target_account_id: &str,
    target_calendar_id: &str,
    window: Window,
) -> Result<(EventDetail, Touched), AppError> {
    let (_, occ) = occurrence_ref(conn, occurrence_id)?;
    if !is_local(&occ.account_id) || !is_local(target_account_id) {
        return Err(AppError::invalid(
            "Only local to local moves are available in this build",
        ));
    }
    ensure_local_calendar(conn, target_account_id, target_calendar_id)?;
    let master_id = occ
        .master_id
        .clone()
        .unwrap_or_else(|| occ.event_id.clone());
    let master = q::get_event(conn, &occ.account_id, &occ.calendar_id, &master_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    if occ.calendar_id == target_calendar_id {
        let detail = view::get_event(conn, occurrence_id)?;
        return Ok((
            detail,
            Touched::new(occ.start_ts, occ.end_ts, &occ.account_id, &occ.calendar_id),
        ));
    }
    let exceptions = q::exceptions_of(conn, &occ.account_id, &occ.calendar_id, &master_id)?;
    let tx = conn.transaction()?;
    q::delete_event(&tx, &occ.account_id, &occ.calendar_id, &master_id)?;
    let mut moved = master.clone();
    moved.calendar_id = target_calendar_id.into();
    moved.updated_ts = Some(crate::db::now_ts());
    q::upsert_event(&tx, &moved)?;
    for mut e in exceptions {
        e.calendar_id = target_calendar_id.into();
        q::upsert_event(&tx, &e)?;
    }
    tx.commit()?;
    expand::materialize_simple(
        conn,
        target_account_id,
        target_calendar_id,
        &master_id,
        window,
    )?;
    let occ_id = first_occurrence_id(
        conn,
        target_account_id,
        target_calendar_id,
        &master_id,
        Some(occ.start_ts),
    )?;
    let detail = view::get_event(conn, &occ_id)?;
    let touched = Touched::whole_window(window, &occ.account_id, &occ.calendar_id).merge(
        Touched::whole_window(window, target_account_id, target_calendar_id),
    );
    Ok((detail, touched))
}

/// Duration helper used by the Google path too.
pub fn span_of(row: &q::EventRow) -> Option<(i64, i64)> {
    if row.all_day {
        let s = expand::parse_date(row.start_date.as_deref()?).ok()?;
        let e = row
            .end_date
            .as_deref()
            .and_then(|d| expand::parse_date(d).ok())
            .unwrap_or(s + Duration::days(1));
        Some((expand::date_to_ts(s), expand::date_to_ts(e)))
    } else {
        Some((row.start_ts?, row.end_ts?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::types::Reminder;
    use crate::commands::view::tests::{ts, WIN};

    fn draft(title: &str, start: &str, end: &str) -> EventDraft {
        EventDraft {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            title: Some(title.into()),
            description: None,
            location: None,
            all_day: false,
            start: Some(ts(start)),
            end: Some(ts(end)),
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

    #[test]
    fn create_validates_and_materializes() {
        let mut conn = crate::db::open_memory().unwrap();
        let (detail, touched) = create_local(
            &mut conn,
            &draft("Dentist", "2026-06-01T12:00:00Z", "2026-06-01T13:00:00Z"),
            WIN,
        )
        .unwrap();
        assert_eq!(detail.title.as_deref(), Some("Dentist"));
        assert!(detail.is_local && detail.can_edit && detail.can_delete);
        assert_eq!(touched.from, ts("2026-06-01T12:00:00Z"));
        assert_eq!(touched.calendars[0].calendar_id, "local-personal");
        let v = view::get_view(
            &conn,
            ts("2026-06-01T00:00:00Z"),
            ts("2026-06-02T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 1);

        let bad = draft("x", "2026-06-01T13:00:00Z", "2026-06-01T12:00:00Z");
        let err = create_local(&mut conn, &bad, WIN).unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
        assert!(err.user_message().contains("end time must be after"));

        let mut untitled = draft("   ", "2026-06-02T12:00:00Z", "2026-06-02T13:00:00Z");
        untitled.reminders = Some(vec![Reminder {
            method: "popup".into(),
            minutes: 5,
        }]);
        let (d, _) = create_local(&mut conn, &untitled, WIN).unwrap();
        assert!(d.title.is_none());
        assert_eq!(
            d.reminders,
            vec![Reminder {
                method: "popup".into(),
                minutes: 5
            }]
        );

        let mut bad_rule = draft("r", "2026-06-03T12:00:00Z", "2026-06-03T13:00:00Z");
        bad_rule.recurrence = vec!["RRULE:FREQ=NOPE".into()];
        assert!(create_local(&mut conn, &bad_rule, WIN).is_err());
        bad_rule.recurrence = vec!["EXDATE:20260603T120000Z".into()];
        assert!(create_local(&mut conn, &bad_rule, WIN).is_err());

        let mut bad_tz = draft("t", "2026-06-03T12:00:00Z", "2026-06-03T13:00:00Z");
        bad_tz.time_zone = Some("Mars/Olympus".into());
        assert!(create_local(&mut conn, &bad_tz, WIN).is_err());

        let mut bad_color = draft("c", "2026-06-03T12:00:00Z", "2026-06-03T13:00:00Z");
        bad_color.color_id = Some("12".into());
        assert!(create_local(&mut conn, &bad_color, WIN).is_err());

        let mut google = draft("g", "2026-06-03T12:00:00Z", "2026-06-03T13:00:00Z");
        google.account_id = "nope".into();
        assert!(matches!(
            create_local(&mut conn, &google, WIN),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn all_day_create_and_update() {
        let mut conn = crate::db::open_memory().unwrap();
        let mut d = draft("Trip", "2026-06-01T00:00:00Z", "2026-06-01T00:00:00Z");
        d.all_day = true;
        d.start = None;
        d.end = None;
        d.start_date = Some("2026-06-01".into());
        d.end_date = Some("2026-06-01".into());
        assert!(create_local(&mut conn, &d, WIN).is_err());
        d.end_date = Some("2026-06-03".into());
        let (detail, _) = create_local(&mut conn, &d, WIN).unwrap();
        assert!(detail.all_day);
        assert_eq!(detail.end - detail.start, 2 * 86_400);
        d.title = Some("Longer trip".into());
        d.end_date = Some("2026-06-04".into());
        let (updated, _) =
            update_local(&mut conn, &detail.occurrence_id, &d, EditScope::This, WIN).unwrap();
        assert_eq!(updated.title.as_deref(), Some("Longer trip"));
        assert_eq!(updated.end - updated.start, 3 * 86_400);
    }

    #[test]
    fn recurring_update_and_delete_by_scope() {
        let mut conn = crate::db::open_memory().unwrap();
        let mut d = draft("Weekly", "2026-03-02T13:00:00Z", "2026-03-02T14:00:00Z");
        d.recurrence = vec!["RRULE:FREQ=WEEKLY;COUNT=4".into()];
        let (detail, touched) = create_local(&mut conn, &d, WIN).unwrap();
        assert_eq!(
            detail.recurrence_text.as_deref(),
            Some("Weekly on Monday, 4 times")
        );
        assert_eq!(touched.from, WIN.from_ts);
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 4);
        let second = v.occurrences[1].id.clone();

        // this: rename only the second instance.
        let mut e = draft("Renamed", "2026-03-09T13:00:00Z", "2026-03-09T14:00:00Z");
        e.recurrence = vec![];
        let (det, _) = update_local(&mut conn, &second, &e, EditScope::This, WIN).unwrap();
        assert_eq!(det.title.as_deref(), Some("Renamed"));
        assert!(det.is_exception && det.is_recurring);
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(
            v.occurrences
                .iter()
                .filter(|o| o.title.as_deref() == Some("Renamed"))
                .count(),
            1
        );

        // following from the third: new time.
        let third = v.occurrences[2].id.clone();
        let f = draft("Later", "2026-03-16T15:00:00Z", "2026-03-16T16:00:00Z");
        let (det, _) = update_local(&mut conn, &third, &f, EditScope::Following, WIN).unwrap();
        assert_eq!(det.start, ts("2026-03-16T15:00:00Z"));
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 4);
        assert_eq!(
            v.occurrences
                .iter()
                .filter(|o| o.title.as_deref() == Some("Later"))
                .count(),
            2
        );

        // delete all of the new series.
        let last = v.occurrences[3].id.clone();
        delete_local(&mut conn, &last, EditScope::All, WIN).unwrap();
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 2);

        // delete this on the first of the old series.
        let first = v.occurrences[0].id.clone();
        delete_local(&mut conn, &first, EditScope::This, WIN).unwrap();
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 1);
        assert!(delete_local(&mut conn, "missing", EditScope::This, WIN).is_err());
    }

    #[test]
    fn move_between_local_calendars() {
        let mut conn = crate::db::open_memory().unwrap();
        calendars::insert_local_calendar(&conn, "local-work", "Work", "#039be5").unwrap();
        let mut d = draft("Weekly", "2026-03-02T13:00:00Z", "2026-03-02T14:00:00Z");
        d.recurrence = vec!["RRULE:FREQ=WEEKLY;COUNT=3".into()];
        let (detail, _) = create_local(&mut conn, &d, WIN).unwrap();
        let (moved, touched) =
            move_local_to_local(&mut conn, &detail.occurrence_id, "local", "local-work", WIN)
                .unwrap();
        assert_eq!(moved.calendar_id, "local-work");
        assert_eq!(moved.color_bg, "#039be5");
        assert_eq!(touched.calendars.len(), 2);
        let v = view::get_view(
            &conn,
            ts("2026-03-01T00:00:00Z"),
            ts("2026-04-01T00:00:00Z"),
            "UTC",
        )
        .unwrap();
        assert_eq!(v.occurrences.len(), 3);
        assert!(v.occurrences.iter().all(|o| o.calendar_id == "local-work"));
        assert!(
            move_local_to_local(&mut conn, &moved.occurrence_id, "google", "primary", WIN).is_err()
        );
    }
}
