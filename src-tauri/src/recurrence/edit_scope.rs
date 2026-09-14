//! Edit and delete scopes for recurring events on the local tables.
//! See docs/03-modelo-de-datos.md section 4 (column "Local").
//!
//! | scope       | edit                                             | delete                          |
//! |-------------|--------------------------------------------------|---------------------------------|
//! | `this`      | insert/update an exception row                    | insert a `cancelled` exception  |
//! | `following` | `UNTIL` on the master + new master from instance  | `UNTIL` on the master           |
//! | `all`       | update the master (shifted by the same delta)     | delete master and exceptions    |
//!
//! The same rules drive the Google path in `commands/events.rs`; there the exception and the
//! new master are created by the API and the resulting rows are upserted from the response.

use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::queries::events::{self as q, EventRow};
use crate::error::AppError;
use crate::recurrence::expand::{self, expand_master, materialize_simple, MasterSpec, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditScope {
    This,
    Following,
    All,
}

/// The occurrence the user acted on.
#[derive(Debug, Clone)]
pub struct OccurrenceRef {
    pub account_id: String,
    pub calendar_id: String,
    /// Row that generated the occurrence: the master or an exception.
    pub event_id: String,
    pub master_id: Option<String>,
    /// Start of the occurrence as shown (UTC seconds; midnight UTC for all-day).
    pub start_ts: i64,
}

/// Google instance id: `master_id + "_" + start in UTC as YYYYMMDDTHHMMSSZ` (or `YYYYMMDD` for all-day).
pub fn instance_id(master_id: &str, start_ts: i64, all_day: bool) -> String {
    let dt = DateTime::from_timestamp(start_ts, 0).unwrap_or_default();
    if all_day {
        format!("{master_id}_{}", dt.format("%Y%m%d"))
    } else {
        format!("{master_id}_{}", dt.format("%Y%m%dT%H%M%SZ"))
    }
}

/// `UNTIL` value for "this and following": the second before the instance, in UTC.
pub fn until_before(instance_start_ts: i64, all_day: bool) -> String {
    if all_day {
        let d =
            DateTime::from_timestamp(instance_start_ts, 0).unwrap_or_default() - Duration::days(1);
        d.format("%Y%m%d").to_string()
    } else {
        let d = DateTime::from_timestamp(instance_start_ts - 1, 0).unwrap_or_default();
        d.format("%Y%m%dT%H%M%SZ").to_string()
    }
}

/// Rewrite the RRULE lines of a master so the series ends before `instance_start_ts`.
/// `COUNT` is dropped because `UNTIL` and `COUNT` are mutually exclusive.
pub fn truncate_recurrence(lines: &[String], instance_start_ts: i64, all_day: bool) -> Vec<String> {
    let until = until_before(instance_start_ts, all_day);
    lines
        .iter()
        .map(|line| {
            if let Some(rule) = line.strip_prefix("RRULE:") {
                let mut parts: Vec<String> = rule
                    .split(';')
                    .filter(|p| !p.is_empty())
                    .filter(|p| {
                        let key = p.split('=').next().unwrap_or("");
                        !key.eq_ignore_ascii_case("UNTIL") && !key.eq_ignore_ascii_case("COUNT")
                    })
                    .map(str::to_string)
                    .collect();
                parts.push(format!("UNTIL={until}"));
                format!("RRULE:{}", parts.join(";"))
            } else {
                line.clone()
            }
        })
        .collect()
}

/// RRULE lines for the new master of "this and following": the original rule with `COUNT`
/// reduced by the instances already consumed. `UNTIL` is kept as is.
pub fn remaining_recurrence(lines: &[String], consumed: usize) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if let Some(rule) = line.strip_prefix("RRULE:") {
                let parts: Vec<String> = rule
                    .split(';')
                    .filter(|p| !p.is_empty())
                    .map(|p| match p.split_once('=') {
                        Some((k, v)) if k.eq_ignore_ascii_case("COUNT") => {
                            let n: usize = v.parse().unwrap_or(0);
                            format!("COUNT={}", n.saturating_sub(consumed).max(1))
                        }
                        _ => p.to_string(),
                    })
                    .collect();
                format!("RRULE:{}", parts.join(";"))
            } else {
                line.clone()
            }
        })
        .collect()
}

/// Number of instances of `master` that start strictly before `instance_start_ts`.
pub fn instances_before(master: &EventRow, instance_start_ts: i64) -> Result<usize, AppError> {
    let spec = MasterSpec::from(master);
    let from = if master.all_day {
        master
            .start_date
            .as_deref()
            .map(expand::parse_date)
            .transpose()?
            .map(expand::date_to_ts)
            .unwrap_or(0)
    } else {
        master.start_ts.unwrap_or(0)
    };
    if instance_start_ts <= from {
        return Ok(0);
    }
    let inst = expand::expand_dates(
        &spec,
        Window {
            from_ts: from,
            to_ts: instance_start_ts - 1,
        },
        crate::config::MAX_INSTANCES_PER_MASTER,
    )?;
    Ok(inst
        .iter()
        .filter(|i| i.start_ts < instance_start_ts)
        .count())
}

/// Copy the user-editable fields of `draft` onto `target`.
pub fn apply_fields(target: &mut EventRow, draft: &EventRow) {
    target.summary = draft.summary.clone();
    target.description = draft.description.clone();
    target.location = draft.location.clone();
    target.color_id = draft.color_id.clone();
    target.start_ts = draft.start_ts;
    target.end_ts = draft.end_ts;
    target.start_date = draft.start_date.clone();
    target.end_date = draft.end_date.clone();
    target.all_day = draft.all_day;
    target.time_zone = draft.time_zone.clone();
    target.attendees = draft.attendees.clone();
    target.reminders = draft.reminders.clone();
    target.transparency = draft.transparency.clone();
    target.visibility = draft.visibility.clone();
    target.hangout_link = draft.hangout_link.clone();
    target.conference = draft.conference.clone();
    target.updated_ts = Some(crate::db::now_ts());
}

fn draft_span(draft: &EventRow) -> Result<(i64, i64), AppError> {
    if draft.all_day {
        let s = expand::parse_date(
            draft
                .start_date
                .as_deref()
                .ok_or_else(|| AppError::invalid("Start date is required"))?,
        )?;
        let e = expand::parse_date(
            draft
                .end_date
                .as_deref()
                .ok_or_else(|| AppError::invalid("End date is required"))?,
        )?;
        Ok((expand::date_to_ts(s), expand::date_to_ts(e)))
    } else {
        let s = draft
            .start_ts
            .ok_or_else(|| AppError::invalid("Start time is required"))?;
        let e = draft
            .end_ts
            .ok_or_else(|| AppError::invalid("End time is required"))?;
        Ok((s, e))
    }
}

/// Shift a row's start/end by `delta` seconds keeping its all-day/timed shape, and set its
/// duration to `duration` seconds.
fn shift_row(row: &mut EventRow, delta: i64, duration: i64) -> Result<(), AppError> {
    if row.all_day {
        let s = expand::parse_date(
            row.start_date
                .as_deref()
                .ok_or_else(|| AppError::invalid("Start date is required"))?,
        )?;
        let ns = s + Duration::seconds(delta);
        let ne = ns + Duration::seconds(duration.max(86_400));
        row.start_date = Some(ns.format("%Y-%m-%d").to_string());
        row.end_date = Some(ne.format("%Y-%m-%d").to_string());
    } else {
        let s = row
            .start_ts
            .ok_or_else(|| AppError::invalid("Start time is required"))?;
        row.start_ts = Some(s + delta);
        row.end_ts = Some(s + delta + duration);
    }
    Ok(())
}

/// Apply an edit on the local tables. `draft` carries the desired field values for the
/// occurrence the user edited (its new start/end included). Returns the id of the row that
/// now represents that occurrence.
pub fn apply_edit(
    conn: &mut Connection,
    scope: EditScope,
    occ: &OccurrenceRef,
    draft: &EventRow,
    window: Window,
) -> Result<String, AppError> {
    let (account_id, calendar_id) = (occ.account_id.as_str(), occ.calendar_id.as_str());
    let row = q::get_event(conn, account_id, calendar_id, &occ.event_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    let master_id = match occ
        .master_id
        .clone()
        .or_else(|| row.recurring_event_id.clone())
    {
        Some(m) => m,
        None => {
            // Not recurring: every scope edits the row itself.
            let mut updated = row;
            apply_fields(&mut updated, draft);
            if updated.is_recurring_master() != draft.is_recurring_master()
                || draft.recurrence.is_some()
            {
                updated.recurrence = draft.recurrence.clone();
            }
            q::upsert_event(conn, &updated)?;
            materialize_simple(conn, account_id, calendar_id, &updated.id, window)?;
            return Ok(updated.id);
        }
    };
    let master = q::get_event(conn, account_id, calendar_id, &master_id)?
        .ok_or_else(|| AppError::NotFound("The recurring event".into()))?;

    match scope {
        EditScope::This => {
            let mut exc = if row.is_exception() {
                row
            } else {
                let mut e = master.clone();
                e.id = instance_id(&master_id, occ.start_ts, master.all_day);
                e.ical_uid = master.ical_uid.clone();
                e.recurring_event_id = Some(master_id.clone());
                if master.all_day {
                    e.original_start_date = Some(
                        DateTime::from_timestamp(occ.start_ts, 0)
                            .unwrap_or_default()
                            .format("%Y-%m-%d")
                            .to_string(),
                    );
                } else {
                    e.original_start_ts = Some(occ.start_ts);
                }
                e.recurrence = None;
                e
            };
            apply_fields(&mut exc, draft);
            exc.recurrence = None;
            q::upsert_event(conn, &exc)?;
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
            Ok(exc.id)
        }
        EditScope::Following => {
            let first_start = if master.all_day {
                master
                    .start_date
                    .as_deref()
                    .map(expand::parse_date)
                    .transpose()?
                    .map(expand::date_to_ts)
                    .unwrap_or(0)
            } else {
                master.start_ts.unwrap_or(0)
            };
            if occ.start_ts <= first_start {
                return apply_edit(conn, EditScope::All, occ, draft, window);
            }
            let consumed = instances_before(&master, occ.start_ts)?;
            let lines = master.recurrence.clone().unwrap_or_default();
            let tx = conn.transaction()?;
            // 1. Truncate the master and drop exceptions at or after the instance.
            let mut truncated = master.clone();
            truncated.recurrence = Some(truncate_recurrence(&lines, occ.start_ts, master.all_day));
            truncated.updated_ts = Some(crate::db::now_ts());
            q::upsert_event(&tx, &truncated)?;
            for exc in q::exceptions_of(&tx, account_id, calendar_id, &master_id)? {
                let orig = exc.original_start_ts.or_else(|| {
                    exc.original_start_date
                        .as_deref()
                        .and_then(|d| expand::parse_date(d).ok())
                        .map(expand::date_to_ts)
                });
                if orig.is_some_and(|o| o >= occ.start_ts) {
                    q::delete_event(&tx, account_id, calendar_id, &exc.id)?;
                }
            }
            // 2. New master starting at the edited instance.
            let mut new_master = master.clone();
            new_master.id = uuid::Uuid::new_v4().to_string();
            new_master.ical_uid = Some(format!(
                "{}{}",
                new_master.id,
                crate::config::LOCAL_ICAL_SUFFIX
            ));
            new_master.recurring_event_id = None;
            new_master.original_start_ts = None;
            new_master.original_start_date = None;
            apply_fields(&mut new_master, draft);
            new_master.recurrence = Some(match &draft.recurrence {
                Some(r) if !r.is_empty() => r.clone(),
                _ => remaining_recurrence(&lines, consumed),
            });
            new_master.created_ts = Some(crate::db::now_ts());
            q::upsert_event(&tx, &new_master)?;
            tx.commit()?;
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
            expand_master(conn, account_id, calendar_id, &new_master.id, window)?;
            Ok(new_master.id)
        }
        EditScope::All => {
            let (draft_start, draft_end) = draft_span(draft)?;
            let delta = draft_start - occ.start_ts;
            let mut updated = master.clone();
            let keep_recurrence = updated.recurrence.clone();
            apply_fields(&mut updated, draft);
            // Start/end come from the master shifted by the same delta the user applied.
            updated.all_day = master.all_day;
            updated.start_ts = master.start_ts;
            updated.end_ts = master.end_ts;
            updated.start_date = master.start_date.clone();
            updated.end_date = master.end_date.clone();
            if draft.all_day != master.all_day {
                // Changing the all-day shape applies the draft span to the first instance.
                updated.all_day = draft.all_day;
                updated.start_ts = draft.start_ts;
                updated.end_ts = draft.end_ts;
                updated.start_date = draft.start_date.clone();
                updated.end_date = draft.end_date.clone();
            } else {
                shift_row(&mut updated, delta, draft_end - draft_start)?;
            }
            updated.recurrence = match &draft.recurrence {
                Some(r) if !r.is_empty() => Some(r.clone()),
                _ => keep_recurrence,
            };
            q::upsert_event(conn, &updated)?;
            if delta != 0 || draft.all_day != master.all_day {
                // Exceptions no longer line up with the series; Google drops them too.
                for exc in q::exceptions_of(conn, account_id, calendar_id, &master_id)? {
                    q::delete_event(conn, account_id, calendar_id, &exc.id)?;
                }
            }
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
            Ok(master_id)
        }
    }
}

/// Delete with scope on the local tables.
pub fn apply_delete(
    conn: &mut Connection,
    scope: EditScope,
    occ: &OccurrenceRef,
    window: Window,
) -> Result<(), AppError> {
    let (account_id, calendar_id) = (occ.account_id.as_str(), occ.calendar_id.as_str());
    let row = q::get_event(conn, account_id, calendar_id, &occ.event_id)?
        .ok_or_else(|| AppError::NotFound("The event".into()))?;
    let Some(master_id) = occ
        .master_id
        .clone()
        .or_else(|| row.recurring_event_id.clone())
    else {
        q::delete_event(conn, account_id, calendar_id, &row.id)?;
        return Ok(());
    };
    let master = q::get_event(conn, account_id, calendar_id, &master_id)?
        .ok_or_else(|| AppError::NotFound("The recurring event".into()))?;
    match scope {
        EditScope::This => {
            let mut exc = if row.is_exception() {
                row
            } else {
                let mut e = master.clone();
                e.id = instance_id(&master_id, occ.start_ts, master.all_day);
                e.recurring_event_id = Some(master_id.clone());
                if master.all_day {
                    e.original_start_date = Some(
                        DateTime::from_timestamp(occ.start_ts, 0)
                            .unwrap_or_default()
                            .format("%Y-%m-%d")
                            .to_string(),
                    );
                } else {
                    e.original_start_ts = Some(occ.start_ts);
                }
                e.recurrence = None;
                e
            };
            exc.status = "cancelled".into();
            exc.updated_ts = Some(crate::db::now_ts());
            q::upsert_event(conn, &exc)?;
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
        }
        EditScope::Following => {
            let first_start = if master.all_day {
                master
                    .start_date
                    .as_deref()
                    .map(expand::parse_date)
                    .transpose()?
                    .map(expand::date_to_ts)
                    .unwrap_or(0)
            } else {
                master.start_ts.unwrap_or(0)
            };
            if occ.start_ts <= first_start {
                return apply_delete(conn, EditScope::All, occ, window);
            }
            let lines = master.recurrence.clone().unwrap_or_default();
            let mut truncated = master.clone();
            truncated.recurrence = Some(truncate_recurrence(&lines, occ.start_ts, master.all_day));
            truncated.updated_ts = Some(crate::db::now_ts());
            q::upsert_event(conn, &truncated)?;
            for exc in q::exceptions_of(conn, account_id, calendar_id, &master_id)? {
                let orig = exc.original_start_ts.or_else(|| {
                    exc.original_start_date
                        .as_deref()
                        .and_then(|d| expand::parse_date(d).ok())
                        .map(expand::date_to_ts)
                });
                if orig.is_some_and(|o| o >= occ.start_ts) {
                    q::delete_event(conn, account_id, calendar_id, &exc.id)?;
                }
            }
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
        }
        EditScope::All => {
            q::delete_event(conn, account_id, calendar_id, &master_id)?;
        }
    }
    Ok(())
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> i64 {
        DateTime::parse_from_rfc3339(s).unwrap().timestamp()
    }
    const WIN: Window = Window {
        from_ts: 1_735_689_600,
        to_ts: 1_798_761_600,
    };

    /// Weekly master, 10 instances, Mondays 10:00 Buenos Aires from 2026-03-02.
    fn seed(conn: &mut Connection) -> EventRow {
        let m = EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: "m1".into(),
            ical_uid: Some("m1@unified-google-calendar".into()),
            status: "confirmed".into(),
            summary: Some("Weekly".into()),
            start_ts: Some(ts("2026-03-02T10:00:00-03:00")),
            end_ts: Some(ts("2026-03-02T11:00:00-03:00")),
            all_day: false,
            time_zone: Some("America/Argentina/Buenos_Aires".into()),
            recurrence: Some(vec!["RRULE:FREQ=WEEKLY;COUNT=10".into()]),
            attendees: "[]".into(),
            reminders: r#"{"useDefault":true}"#.into(),
            event_type: "default".into(),
            ..Default::default()
        };
        q::upsert_event(conn, &m).unwrap();
        expand_master(conn, "local", "local-personal", "m1", WIN).unwrap();
        m
    }

    fn all_occurrences(conn: &Connection) -> Vec<q::OccurrenceRow> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM occurrences ORDER BY start_ts",
                q::OccurrenceRow::COLUMNS
            ))
            .unwrap();
        stmt.query_map([], q::OccurrenceRow::from_row)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn occ_ref(start: &str) -> OccurrenceRef {
        OccurrenceRef {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            event_id: "m1".into(),
            master_id: Some("m1".into()),
            start_ts: ts(start),
        }
    }

    fn draft_from(m: &EventRow, start: &str, end: &str, title: &str) -> EventRow {
        EventRow {
            summary: Some(title.into()),
            start_ts: Some(ts(start)),
            end_ts: Some(ts(end)),
            recurrence: None,
            ..m.clone()
        }
    }

    #[test]
    fn seed_has_ten_instances() {
        let mut conn = crate::db::open_memory().unwrap();
        seed(&mut conn);
        assert_eq!(all_occurrences(&conn).len(), 10);
    }

    #[test]
    fn edit_this_creates_exception_with_google_style_id() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        // Fourth instance (2026-03-23 13:00Z) moved to 15:00Z and renamed.
        let draft = draft_from(&m, "2026-03-23T15:00:00Z", "2026-03-23T16:00:00Z", "Moved");
        let id = apply_edit(
            &mut conn,
            EditScope::This,
            &occ_ref("2026-03-23T13:00:00Z"),
            &draft,
            WIN,
        )
        .unwrap();
        assert_eq!(id, "m1_20260323T130000Z");
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 10);
        let moved = occ.iter().find(|o| o.event_id == id).unwrap();
        assert_eq!(moved.start_ts, ts("2026-03-23T15:00:00Z"));
        assert!(occ.iter().all(|o| o.start_ts != ts("2026-03-23T13:00:00Z")));
        let exc = q::get_event(&conn, "local", "local-personal", &id)
            .unwrap()
            .unwrap();
        assert_eq!(exc.summary.as_deref(), Some("Moved"));
        assert_eq!(exc.original_start_ts, Some(ts("2026-03-23T13:00:00Z")));
        assert_eq!(exc.recurring_event_id.as_deref(), Some("m1"));
        // Master untouched.
        let master = q::get_event(&conn, "local", "local-personal", "m1")
            .unwrap()
            .unwrap();
        assert_eq!(master.summary.as_deref(), Some("Weekly"));
    }

    #[test]
    fn edit_following_splits_series_and_recomputes_count() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        // From the 4th instance on: 14:00Z, title "Later".
        let draft = draft_from(&m, "2026-03-23T14:00:00Z", "2026-03-23T15:00:00Z", "Later");
        let new_id = apply_edit(
            &mut conn,
            EditScope::Following,
            &occ_ref("2026-03-23T13:00:00Z"),
            &draft,
            WIN,
        )
        .unwrap();
        let master = q::get_event(&conn, "local", "local-personal", "m1")
            .unwrap()
            .unwrap();
        assert_eq!(
            master.recurrence.unwrap(),
            vec!["RRULE:FREQ=WEEKLY;UNTIL=20260323T125959Z"]
        );
        let new_master = q::get_event(&conn, "local", "local-personal", &new_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            new_master.recurrence.unwrap(),
            vec!["RRULE:FREQ=WEEKLY;COUNT=7"]
        );
        assert_eq!(new_master.summary.as_deref(), Some("Later"));
        assert_eq!(new_master.start_ts, Some(ts("2026-03-23T14:00:00Z")));
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 10);
        let old: Vec<_> = occ
            .iter()
            .filter(|o| o.master_id.as_deref() == Some("m1"))
            .collect();
        let new: Vec<_> = occ
            .iter()
            .filter(|o| o.master_id.as_deref() == Some(new_id.as_str()))
            .collect();
        assert_eq!(old.len(), 3);
        assert_eq!(new.len(), 7);
        assert_eq!(new[0].start_ts, ts("2026-03-23T14:00:00Z"));
        assert_eq!(new[6].start_ts, ts("2026-05-04T14:00:00Z"));
    }

    #[test]
    fn edit_following_on_first_instance_behaves_as_all() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        let draft = draft_from(&m, "2026-03-02T13:00:00Z", "2026-03-02T14:30:00Z", "Longer");
        let id = apply_edit(
            &mut conn,
            EditScope::Following,
            &occ_ref("2026-03-02T13:00:00Z"),
            &draft,
            WIN,
        )
        .unwrap();
        assert_eq!(id, "m1");
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 10);
        assert!(occ.iter().all(|o| o.end_ts - o.start_ts == 5400));
    }

    #[test]
    fn edit_all_shifts_every_instance_and_drops_exceptions() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        // First make an exception, then edit all from the 3rd instance moving it +1h.
        let exc_draft = draft_from(&m, "2026-03-09T18:00:00Z", "2026-03-09T19:00:00Z", "Exc");
        apply_edit(
            &mut conn,
            EditScope::This,
            &occ_ref("2026-03-09T13:00:00Z"),
            &exc_draft,
            WIN,
        )
        .unwrap();
        let draft = draft_from(
            &m,
            "2026-03-16T14:00:00Z",
            "2026-03-16T14:30:00Z",
            "All renamed",
        );
        let id = apply_edit(
            &mut conn,
            EditScope::All,
            &occ_ref("2026-03-16T13:00:00Z"),
            &draft,
            WIN,
        )
        .unwrap();
        assert_eq!(id, "m1");
        let master = q::get_event(&conn, "local", "local-personal", "m1")
            .unwrap()
            .unwrap();
        assert_eq!(master.summary.as_deref(), Some("All renamed"));
        assert_eq!(master.start_ts, Some(ts("2026-03-02T14:00:00Z")));
        assert_eq!(master.end_ts, Some(ts("2026-03-02T14:30:00Z")));
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 10);
        assert!(occ
            .iter()
            .all(|o| o.event_id == "m1" && o.end_ts - o.start_ts == 1800));
        assert_eq!(occ[0].start_ts, ts("2026-03-02T14:00:00Z"));
        assert!(q::exceptions_of(&conn, "local", "local-personal", "m1")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn edit_all_without_time_change_keeps_exceptions() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        let exc_draft = draft_from(&m, "2026-03-09T18:00:00Z", "2026-03-09T19:00:00Z", "Exc");
        apply_edit(
            &mut conn,
            EditScope::This,
            &occ_ref("2026-03-09T13:00:00Z"),
            &exc_draft,
            WIN,
        )
        .unwrap();
        let draft = draft_from(
            &m,
            "2026-03-16T13:00:00Z",
            "2026-03-16T14:00:00Z",
            "Renamed only",
        );
        apply_edit(
            &mut conn,
            EditScope::All,
            &occ_ref("2026-03-16T13:00:00Z"),
            &draft,
            WIN,
        )
        .unwrap();
        assert_eq!(
            q::exceptions_of(&conn, "local", "local-personal", "m1")
                .unwrap()
                .len(),
            1
        );
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 10);
        assert!(occ.iter().any(|o| o.start_ts == ts("2026-03-09T18:00:00Z")));
    }

    #[test]
    fn delete_this_inserts_cancelled_exception() {
        let mut conn = crate::db::open_memory().unwrap();
        seed(&mut conn);
        apply_delete(
            &mut conn,
            EditScope::This,
            &occ_ref("2026-03-23T13:00:00Z"),
            WIN,
        )
        .unwrap();
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 9);
        assert!(occ.iter().all(|o| o.start_ts != ts("2026-03-23T13:00:00Z")));
        let exc = q::get_event(&conn, "local", "local-personal", "m1_20260323T130000Z")
            .unwrap()
            .unwrap();
        assert_eq!(exc.status, "cancelled");
    }

    #[test]
    fn delete_following_truncates_with_until() {
        let mut conn = crate::db::open_memory().unwrap();
        seed(&mut conn);
        apply_delete(
            &mut conn,
            EditScope::Following,
            &occ_ref("2026-03-23T13:00:00Z"),
            WIN,
        )
        .unwrap();
        let occ = all_occurrences(&conn);
        assert_eq!(occ.len(), 3);
        assert_eq!(occ[2].start_ts, ts("2026-03-16T13:00:00Z"));
        let master = q::get_event(&conn, "local", "local-personal", "m1")
            .unwrap()
            .unwrap();
        assert_eq!(
            master.recurrence.unwrap(),
            vec!["RRULE:FREQ=WEEKLY;UNTIL=20260323T125959Z"]
        );
    }

    #[test]
    fn delete_all_removes_master_exceptions_and_occurrences() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed(&mut conn);
        let exc_draft = draft_from(&m, "2026-03-09T18:00:00Z", "2026-03-09T19:00:00Z", "Exc");
        apply_edit(
            &mut conn,
            EditScope::This,
            &occ_ref("2026-03-09T13:00:00Z"),
            &exc_draft,
            WIN,
        )
        .unwrap();
        apply_delete(
            &mut conn,
            EditScope::All,
            &occ_ref("2026-03-16T13:00:00Z"),
            WIN,
        )
        .unwrap();
        assert!(all_occurrences(&conn).is_empty());
        let n: i64 = conn
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn all_day_helpers() {
        assert_eq!(
            instance_id("m", ts("2026-03-23T00:00:00Z"), true),
            "m_20260323"
        );
        assert_eq!(until_before(ts("2026-03-23T00:00:00Z"), true), "20260322");
        assert_eq!(
            truncate_recurrence(
                &["RRULE:FREQ=DAILY;COUNT=5".into(), "EXDATE:20260301".into()],
                ts("2026-03-23T13:00:00Z"),
                false
            ),
            vec!["RRULE:FREQ=DAILY;UNTIL=20260323T125959Z", "EXDATE:20260301"]
        );
        assert_eq!(
            remaining_recurrence(&["RRULE:FREQ=DAILY;COUNT=5;INTERVAL=2".into()], 3),
            vec!["RRULE:FREQ=DAILY;COUNT=2;INTERVAL=2"]
        );
    }
}
