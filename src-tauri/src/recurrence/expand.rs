//! Expansion of recurring masters into `occurrences`. See docs/03-modelo-de-datos.md section 3.
//!
//! The expansion runs in the master's IANA zone so a weekly 10:00 stays at 10:00 across
//! daylight-saving changes, then converts each instance to UTC. `EXDATE`, `RDATE`,
//! moved exceptions and cancelled exceptions are honoured. Results replace, in one
//! transaction, every occurrence whose `master_id` is the master.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::config::MAX_INSTANCES_PER_MASTER;
use crate::db::queries::events::{self as q, EventRow, OccurrenceRow};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub from_ts: i64,
    pub to_ts: i64,
}

impl Window {
    /// Data window of docs/03 section 3: today minus `data_window_past_days` to today plus
    /// `data_window_future_days`, aligned to UTC midnight.
    pub fn current(conn: &Connection) -> Result<Window, AppError> {
        use crate::db::queries::settings;
        let past: i64 = settings::get_or(
            conn,
            "data_window_past_days",
            crate::config::DEFAULT_DATA_WINDOW_PAST_DAYS,
        )?;
        let future: i64 = settings::get_or(
            conn,
            "data_window_future_days",
            crate::config::DEFAULT_DATA_WINDOW_FUTURE_DAYS,
        )?;
        let today = Utc::now().date_naive();
        Ok(Window {
            from_ts: date_to_ts(today - Duration::days(past)),
            to_ts: date_to_ts(today + Duration::days(future)),
        })
    }
}

/// What the expander needs from a master row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterSpec {
    pub all_day: bool,
    /// UTC start for timed masters.
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    /// `YYYY-MM-DD` for all-day masters; `end_date` is exclusive.
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// IANA zone of the master (`events.time_zone`). UTC when absent.
    pub time_zone: Option<String>,
    /// RRULE / EXDATE / RDATE lines as Google sends them.
    pub recurrence: Vec<String>,
}

impl From<&EventRow> for MasterSpec {
    fn from(e: &EventRow) -> Self {
        MasterSpec {
            all_day: e.all_day,
            start_ts: e.start_ts,
            end_ts: e.end_ts,
            start_date: e.start_date.clone(),
            end_date: e.end_date.clone(),
            time_zone: e.time_zone.clone(),
            recurrence: e.recurrence.clone().unwrap_or_default(),
        }
    }
}

/// One generated instance, in UTC seconds. For all-day instances `start_ts` is midnight UTC
/// of the date, matching the `occurrences` convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub start_ts: i64,
    pub end_ts: i64,
}

pub fn parse_date(s: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::Recurrence(format!("bad date {s}: {e}")))
}

pub fn date_to_ts(d: NaiveDate) -> i64 {
    d.and_hms_opt(0, 0, 0)
        .map(|n| n.and_utc().timestamp())
        .unwrap_or_default()
}

fn parse_tz(name: &str) -> Result<chrono_tz::Tz, AppError> {
    chrono_tz::Tz::from_str(name)
        .map_err(|_| AppError::Recurrence(format!("unknown time zone {name}")))
}

/// A parsed iCalendar content line `NAME;PARAM=V;PARAM2=V2:VALUE`.
pub struct ContentLine {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

/// Split an iCalendar content line into name, parameters and value.
pub fn split_line(line: &str) -> Option<ContentLine> {
    let colon = line.find(':')?;
    let (head, value) = line.split_at(colon);
    let value = &value[1..];
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.trim().to_ascii_uppercase(), v.trim().to_string()))
        })
        .collect();
    Some(ContentLine {
        name,
        params,
        value: value.trim().to_string(),
    })
}

/// Value of one RRULE part, e.g. `UNTIL=...`.
pub fn rrule_part<'a>(rule: &'a str, key: &str) -> Option<&'a str> {
    rule.split(';').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k.eq_ignore_ascii_case(key)).then_some(v)
    })
}

/// Rewrite an RRULE so the crate accepts it: `UNTIL` must be a UTC `Z` date-time when
/// `DTSTART` carries a zone. Date-only `UNTIL` becomes the last second of that day in UTC.
fn normalize_rrule(rule: &str, tz: chrono_tz::Tz) -> Result<String, AppError> {
    let mut parts = Vec::new();
    for p in rule.split(';').filter(|p| !p.is_empty()) {
        let Some((k, v)) = p.split_once('=') else {
            parts.push(p.to_string());
            continue;
        };
        if k.eq_ignore_ascii_case("UNTIL") {
            let ts = parse_ical_datetime(v, Some(tz), true)?;
            parts.push(format!("UNTIL={}", ts.format("%Y%m%dT%H%M%SZ")));
        } else {
            parts.push(format!("{k}={v}"));
        }
    }
    Ok(parts.join(";"))
}

/// Parse `YYYYMMDD`, `YYYYMMDDTHHMMSS` or `YYYYMMDDTHHMMSSZ` into UTC. Date-only values map to
/// midnight (or, when `end_of_day`, to 23:59:59) in `tz`.
fn parse_ical_datetime(
    v: &str,
    tz: Option<chrono_tz::Tz>,
    end_of_day: bool,
) -> Result<DateTime<Utc>, AppError> {
    let bad = || AppError::Recurrence(format!("bad iCalendar date {v}"));
    if v.len() == 8 {
        let d = NaiveDate::parse_from_str(v, "%Y%m%d").map_err(|_| bad())?;
        let naive = if end_of_day {
            d.and_hms_opt(23, 59, 59)
        } else {
            d.and_hms_opt(0, 0, 0)
        }
        .ok_or_else(bad)?;
        return local_to_utc(naive, tz.unwrap_or(chrono_tz::UTC));
    }
    if let Some(stripped) = v.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").map_err(|_| bad())?;
        return Ok(naive.and_utc());
    }
    let naive = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").map_err(|_| bad())?;
    local_to_utc(naive, tz.unwrap_or(chrono_tz::UTC))
}

fn local_to_utc(naive: NaiveDateTime, tz: chrono_tz::Tz) -> Result<DateTime<Utc>, AppError> {
    tz.from_local_datetime(&naive)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .ok_or_else(|| AppError::Recurrence(format!("{naive} does not exist in {tz}")))
}

/// Build the iCalendar text the `rrule` crate parses, with every date normalized.
fn build_rruleset_text(
    spec: &MasterSpec,
    tz: chrono_tz::Tz,
    start_local: NaiveDateTime,
) -> Result<String, AppError> {
    let mut lines = Vec::new();
    if spec.all_day {
        lines.push(format!("DTSTART:{}", start_local.format("%Y%m%dT%H%M%SZ")));
    } else {
        lines.push(format!(
            "DTSTART;TZID={}:{}",
            tz.name(),
            start_local.format("%Y%m%dT%H%M%S")
        ));
    }
    let date_tz = if spec.all_day { chrono_tz::UTC } else { tz };
    for line in &spec.recurrence {
        let Some(ContentLine {
            name,
            params,
            value,
        }) = split_line(line)
        else {
            return Err(AppError::Recurrence(format!(
                "malformed recurrence line {line}"
            )));
        };
        match name.as_str() {
            "RRULE" => lines.push(format!("RRULE:{}", normalize_rrule(&value, date_tz)?)),
            "EXDATE" | "RDATE" => {
                let line_tz = params
                    .iter()
                    .find(|(k, _)| k == "TZID")
                    .map(|(_, v)| parse_tz(v))
                    .transpose()?
                    .unwrap_or(date_tz);
                let mut out = Vec::new();
                for v in value.split(',').filter(|v| !v.is_empty()) {
                    // A date-only value on a timed master means "the instance on that day";
                    // it is normalized to the master's local time on that date.
                    let utc = if v.len() == 8 && !spec.all_day {
                        let d = NaiveDate::parse_from_str(v, "%Y%m%d")
                            .map_err(|_| AppError::Recurrence(format!("bad date {v}")))?;
                        local_to_utc(d.and_time(start_local.time()), tz)?
                    } else {
                        parse_ical_datetime(v, Some(line_tz), false)?
                    };
                    out.push(utc.format("%Y%m%dT%H%M%SZ").to_string());
                }
                if !out.is_empty() {
                    lines.push(format!("{name}:{}", out.join(",")));
                }
            }
            "EXRULE" => tracing::warn!("EXRULE ignored (deprecated in RFC 5545)"),
            other => {
                return Err(AppError::Recurrence(format!(
                    "unsupported recurrence property {other}"
                )))
            }
        }
    }
    Ok(lines.join("\n"))
}

/// Pure expansion: instances of `spec` that intersect `window`, at most `max`, ordered by start.
pub fn expand_dates(
    spec: &MasterSpec,
    window: Window,
    max: usize,
) -> Result<Vec<Instance>, AppError> {
    let (tz, start_local, duration) =
        if spec.all_day {
            let sd = parse_date(spec.start_date.as_deref().ok_or_else(|| {
                AppError::Recurrence("all-day master without start_date".into())
            })?)?;
            let ed =
                parse_date(spec.end_date.as_deref().ok_or_else(|| {
                    AppError::Recurrence("all-day master without end_date".into())
                })?)?;
            let days = (ed - sd).num_days().max(1);
            let naive = sd
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::Recurrence("bad start_date".into()))?;
            (chrono_tz::UTC, naive, days * 86_400)
        } else {
            let st = spec
                .start_ts
                .ok_or_else(|| AppError::Recurrence("timed master without start_ts".into()))?;
            let et = spec.end_ts.unwrap_or(st);
            let tz = match spec.time_zone.as_deref() {
                Some(name) if !name.is_empty() => parse_tz(name)?,
                _ => chrono_tz::UTC,
            };
            let start_local = DateTime::from_timestamp(st, 0)
                .ok_or_else(|| AppError::Recurrence("bad start_ts".into()))?
                .with_timezone(&tz)
                .naive_local();
            (tz, start_local, (et - st).max(0))
        };

    let text = build_rruleset_text(spec, tz, start_local)?;
    let set = rrule::RRuleSet::from_str(&text)
        .map_err(|e| AppError::Recurrence(format!("{e} in {text:?}")))?;

    // An instance that starts before the window but ends inside it must be kept.
    let after = Utc
        .timestamp_opt(window.from_ts - duration, 0)
        .single()
        .ok_or_else(|| AppError::Recurrence("bad window".into()))?;
    let before = Utc
        .timestamp_opt(window.to_ts, 0)
        .single()
        .ok_or_else(|| AppError::Recurrence("bad window".into()))?;
    let limit = u16::try_from(max.min(usize::from(u16::MAX))).unwrap_or(u16::MAX);
    let result = set
        .after(after.with_timezone(&rrule::Tz::UTC))
        .before(before.with_timezone(&rrule::Tz::UTC))
        .all(limit);
    if result.limited {
        tracing::warn!(limit, "recurrence expansion hit the instance limit");
    }
    let mut out: Vec<Instance> = result
        .dates
        .into_iter()
        .map(|d| {
            let s = d.with_timezone(&Utc).timestamp();
            Instance {
                start_ts: s,
                end_ts: s + duration,
            }
        })
        .filter(|i| i.end_ts >= window.from_ts && i.start_ts <= window.to_ts)
        .collect();
    out.sort_by_key(|i| i.start_ts);
    out.dedup();
    Ok(out)
}

/// Original start of an exception as a UTC timestamp (midnight UTC for all-day).
fn exception_original_ts(e: &EventRow) -> Option<i64> {
    if let Some(ts) = e.original_start_ts {
        return Some(ts);
    }
    e.original_start_date
        .as_deref()
        .and_then(|d| parse_date(d).ok())
        .map(date_to_ts)
}

/// Start/end of an exception row in UTC seconds.
fn row_span(e: &EventRow) -> Option<(i64, i64)> {
    if e.all_day {
        let s = parse_date(e.start_date.as_deref()?).ok()?;
        let end = e
            .end_date
            .as_deref()
            .and_then(|d| parse_date(d).ok())
            .unwrap_or(s + Duration::days(1));
        Some((date_to_ts(s), date_to_ts(end)))
    } else {
        let s = e.start_ts?;
        Some((s, e.end_ts.unwrap_or(s)))
    }
}

/// Expand one master into `occurrences`, replacing its previous rows. Returns the number written.
/// A cancelled master, or one without recurrence lines, ends with zero occurrences.
pub fn expand_master(
    conn: &mut Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    window: Window,
) -> Result<usize, AppError> {
    let master = q::get_event(conn, account_id, calendar_id, event_id)?
        .ok_or_else(|| AppError::NotFound(format!("Event {event_id}")))?;
    let tx = conn.transaction()?;
    q::delete_occurrences_of_event(&tx, account_id, calendar_id, event_id)?;
    if master.is_cancelled() || !master.is_recurring_master() {
        tx.commit()?;
        return Ok(0);
    }
    let spec = MasterSpec::from(&master);
    let instances = expand_dates(&spec, window, MAX_INSTANCES_PER_MASTER)?;

    let exceptions = q::exceptions_of(&tx, account_id, calendar_id, event_id)?;
    let mut by_original: HashMap<i64, &EventRow> = HashMap::new();
    for e in &exceptions {
        if let Some(ts) = exception_original_ts(e) {
            by_original.insert(ts, e);
        }
    }
    let mut consumed: Vec<&str> = Vec::new();
    let mut written = 0usize;
    for inst in &instances {
        match by_original.get(&inst.start_ts) {
            Some(exc) => {
                consumed.push(exc.id.as_str());
                if exc.is_cancelled() {
                    continue;
                }
                let Some((s, e)) = row_span(exc) else {
                    continue;
                };
                if e < window.from_ts || s > window.to_ts {
                    continue;
                }
                q::insert_occurrence(
                    &tx,
                    &OccurrenceRow {
                        id: q::occurrence_id(account_id, calendar_id, &exc.id, s, exc.all_day),
                        account_id: account_id.into(),
                        calendar_id: calendar_id.into(),
                        event_id: exc.id.clone(),
                        master_id: Some(event_id.into()),
                        start_ts: s,
                        end_ts: e,
                        all_day: exc.all_day,
                        status: exc.status.clone(),
                    },
                )?;
                written += 1;
            }
            None => {
                q::insert_occurrence(
                    &tx,
                    &OccurrenceRow {
                        id: q::occurrence_id(
                            account_id,
                            calendar_id,
                            event_id,
                            inst.start_ts,
                            master.all_day,
                        ),
                        account_id: account_id.into(),
                        calendar_id: calendar_id.into(),
                        event_id: event_id.into(),
                        master_id: Some(event_id.into()),
                        start_ts: inst.start_ts,
                        end_ts: inst.end_ts,
                        all_day: master.all_day,
                        status: master.status.clone(),
                    },
                )?;
                written += 1;
            }
        }
    }
    // Exceptions moved into the window from an original start outside it.
    for exc in exceptions
        .iter()
        .filter(|e| !consumed.contains(&e.id.as_str()) && !e.is_cancelled())
    {
        let Some((s, e)) = row_span(exc) else {
            continue;
        };
        if e < window.from_ts || s > window.to_ts {
            continue;
        }
        q::insert_occurrence(
            &tx,
            &OccurrenceRow {
                id: q::occurrence_id(account_id, calendar_id, &exc.id, s, exc.all_day),
                account_id: account_id.into(),
                calendar_id: calendar_id.into(),
                event_id: exc.id.clone(),
                master_id: Some(event_id.into()),
                start_ts: s,
                end_ts: e,
                all_day: exc.all_day,
                status: exc.status.clone(),
            },
        )?;
        written += 1;
    }
    tx.commit()?;
    Ok(written)
}

/// Materialize a single event: one occurrence if it is not cancelled and intersects the window.
/// Recurring masters and exceptions are routed to [`expand_master`] of the master.
pub fn materialize_simple(
    conn: &mut Connection,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    window: Window,
) -> Result<(), AppError> {
    let row = q::get_event(conn, account_id, calendar_id, event_id)?
        .ok_or_else(|| AppError::NotFound(format!("Event {event_id}")))?;
    if row.is_recurring_master() {
        expand_master(conn, account_id, calendar_id, event_id, window)?;
        return Ok(());
    }
    if let Some(master_id) = row.recurring_event_id.clone() {
        if q::get_event(conn, account_id, calendar_id, &master_id)?.is_some() {
            expand_master(conn, account_id, calendar_id, &master_id, window)?;
            return Ok(());
        }
        // Orphan exception (master not synced): treat it as a simple event.
    }
    let tx = conn.transaction()?;
    q::delete_occurrences_of_event(&tx, account_id, calendar_id, event_id)?;
    if !row.is_cancelled() {
        if let Some((s, e)) = row_span(&row) {
            if e >= window.from_ts && s <= window.to_ts {
                q::insert_occurrence(
                    &tx,
                    &OccurrenceRow {
                        id: q::occurrence_id(account_id, calendar_id, event_id, s, row.all_day),
                        account_id: account_id.into(),
                        calendar_id: calendar_id.into(),
                        event_id: event_id.into(),
                        master_id: row.recurring_event_id.clone(),
                        start_ts: s,
                        end_ts: e,
                        all_day: row.all_day,
                        status: row.status.clone(),
                    },
                )?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Re-materialize every event of a calendar (after a full sync or a window move).
pub fn rebuild_calendar(
    conn: &mut Connection,
    account_id: &str,
    calendar_id: &str,
    window: Window,
) -> Result<(), AppError> {
    for id in q::masters_of_calendar(conn, account_id, calendar_id)? {
        if let Err(e) = expand_master(conn, account_id, calendar_id, &id, window) {
            tracing::warn!(event = %id, error = %e, "skipping master that failed to expand");
        }
    }
    for id in q::simple_events_of_calendar(conn, account_id, calendar_id)? {
        materialize_simple(conn, account_id, calendar_id, &id, window)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> i64 {
        DateTime::parse_from_rfc3339(s).unwrap().timestamp()
    }

    fn spec_timed(start: &str, end: &str, tz: &str, lines: &[&str]) -> MasterSpec {
        MasterSpec {
            all_day: false,
            start_ts: Some(ts(start)),
            end_ts: Some(ts(end)),
            start_date: None,
            end_date: None,
            time_zone: Some(tz.into()),
            recurrence: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn spec_allday(start: &str, end: &str, lines: &[&str]) -> MasterSpec {
        MasterSpec {
            all_day: true,
            start_ts: None,
            end_ts: None,
            start_date: Some(start.into()),
            end_date: Some(end.into()),
            time_zone: None,
            recurrence: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    const WIN: Window = Window {
        from_ts: 1_735_689_600,
        to_ts: 1_798_761_600,
    }; // 2025-01-01 .. 2027-01-01

    fn starts(spec: &MasterSpec) -> Vec<String> {
        expand_dates(spec, WIN, 5000)
            .unwrap()
            .into_iter()
            .map(|i| {
                DateTime::from_timestamp(i.start_ts, 0)
                    .unwrap()
                    .to_rfc3339()
            })
            .collect()
    }

    #[test]
    fn daily_five_times() {
        // 10:00 Buenos Aires = 13:00Z (no DST in Argentina).
        let s = spec_timed(
            "2026-03-02T10:00:00-03:00",
            "2026-03-02T10:30:00-03:00",
            "America/Argentina/Buenos_Aires",
            &["RRULE:FREQ=DAILY;COUNT=5"],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-03-02T13:00:00+00:00",
                "2026-03-03T13:00:00+00:00",
                "2026-03-04T13:00:00+00:00",
                "2026-03-05T13:00:00+00:00",
                "2026-03-06T13:00:00+00:00"
            ]
        );
        let inst = expand_dates(&s, WIN, 5000).unwrap();
        assert_eq!(inst[0].end_ts - inst[0].start_ts, 1800);
    }

    #[test]
    fn weekly_mon_wed_until() {
        let s = spec_timed(
            "2026-03-02T09:00:00-03:00",
            "2026-03-02T10:00:00-03:00",
            "America/Argentina/Buenos_Aires",
            &["RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20260318T115959Z"],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-03-02T12:00:00+00:00",
                "2026-03-04T12:00:00+00:00",
                "2026-03-09T12:00:00+00:00",
                "2026-03-11T12:00:00+00:00",
                "2026-03-16T12:00:00+00:00"
            ]
        );
    }

    #[test]
    fn monthly_on_31st_skips_short_months() {
        let s = spec_timed(
            "2026-01-31T12:00:00Z",
            "2026-01-31T13:00:00Z",
            "UTC",
            &["RRULE:FREQ=MONTHLY;BYMONTHDAY=31;COUNT=4"],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-01-31T12:00:00+00:00",
                "2026-03-31T12:00:00+00:00",
                "2026-05-31T12:00:00+00:00",
                "2026-07-31T12:00:00+00:00"
            ]
        );
    }

    #[test]
    fn yearly() {
        let s = spec_timed(
            "2025-06-15T15:00:00Z",
            "2025-06-15T16:00:00Z",
            "UTC",
            &["RRULE:FREQ=YEARLY"],
        );
        assert_eq!(
            starts(&s),
            vec!["2025-06-15T15:00:00+00:00", "2026-06-15T15:00:00+00:00"]
        );
    }

    #[test]
    fn every_two_weeks_with_exdate() {
        let s = spec_timed(
            "2026-04-06T10:00:00-03:00",
            "2026-04-06T11:00:00-03:00",
            "America/Argentina/Buenos_Aires",
            &[
                "RRULE:FREQ=WEEKLY;INTERVAL=2;COUNT=4",
                "EXDATE;TZID=America/Argentina/Buenos_Aires:20260420T100000",
            ],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-04-06T13:00:00+00:00",
                "2026-05-04T13:00:00+00:00",
                "2026-05-18T13:00:00+00:00"
            ]
        );
    }

    #[test]
    fn rdate_adds_an_extra_instance() {
        let s = spec_timed(
            "2026-04-06T10:00:00-03:00",
            "2026-04-06T11:00:00-03:00",
            "America/Argentina/Buenos_Aires",
            &["RRULE:FREQ=WEEKLY;COUNT=2", "RDATE:20260410T130000Z"],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-04-06T13:00:00+00:00",
                "2026-04-10T13:00:00+00:00",
                "2026-04-13T13:00:00+00:00"
            ]
        );
    }

    #[test]
    fn all_day_two_days_weekly() {
        let s = spec_allday("2026-05-04", "2026-05-06", &["RRULE:FREQ=WEEKLY;COUNT=2"]);
        let inst = expand_dates(&s, WIN, 5000).unwrap();
        assert_eq!(inst.len(), 2);
        assert_eq!(inst[0].start_ts, ts("2026-05-04T00:00:00Z"));
        assert_eq!(inst[0].end_ts, ts("2026-05-06T00:00:00Z"));
        assert_eq!(inst[1].start_ts, ts("2026-05-11T00:00:00Z"));
    }

    #[test]
    fn all_day_with_date_until_and_date_exdate() {
        let s = spec_allday(
            "2026-05-04",
            "2026-05-05",
            &[
                "RRULE:FREQ=DAILY;UNTIL=20260507",
                "EXDATE;VALUE=DATE:20260506",
            ],
        );
        assert_eq!(
            starts(&s),
            vec![
                "2026-05-04T00:00:00+00:00",
                "2026-05-05T00:00:00+00:00",
                "2026-05-07T00:00:00+00:00"
            ]
        );
    }

    #[test]
    fn buenos_aires_weekly_keeps_local_hour_while_mexico_city_offset_is_fixed() {
        // Weekly 10:00 Buenos Aires from late September through November. Argentina has no DST;
        // every instance is 13:00Z and 07:00 in Mexico City (which dropped DST in 2022).
        let s = spec_timed(
            "2026-09-28T10:00:00-03:00",
            "2026-09-28T11:00:00-03:00",
            "America/Argentina/Buenos_Aires",
            &["RRULE:FREQ=WEEKLY;COUNT=8"],
        );
        let inst = expand_dates(&s, WIN, 5000).unwrap();
        assert_eq!(inst.len(), 8);
        for i in &inst {
            let utc = DateTime::from_timestamp(i.start_ts, 0).unwrap();
            assert_eq!(utc.format("%H:%M").to_string(), "13:00");
            let ba = utc.with_timezone(&chrono_tz::America::Argentina::Buenos_Aires);
            assert_eq!(ba.format("%H:%M").to_string(), "10:00");
            let mx = utc.with_timezone(&chrono_tz::America::Mexico_City);
            assert_eq!(mx.format("%H:%M").to_string(), "07:00");
        }
    }

    #[test]
    fn weekly_in_dst_zone_keeps_local_hour_and_shifts_utc() {
        // New York leaves DST on 2026-11-01: 10:00 local is 14:00Z before and 15:00Z after.
        let s = spec_timed(
            "2026-10-26T10:00:00-04:00",
            "2026-10-26T11:00:00-04:00",
            "America/New_York",
            &["RRULE:FREQ=WEEKLY;COUNT=2"],
        );
        assert_eq!(
            starts(&s),
            vec!["2026-10-26T14:00:00+00:00", "2026-11-02T15:00:00+00:00"]
        );
    }

    #[test]
    fn instance_limit_is_respected() {
        let s = spec_timed(
            "2025-01-01T12:00:00Z",
            "2025-01-01T12:30:00Z",
            "UTC",
            &["RRULE:FREQ=HOURLY"],
        );
        let inst = expand_dates(&s, WIN, 5000).unwrap();
        assert_eq!(inst.len(), 5000);
    }

    #[test]
    fn instance_starting_before_window_but_ending_inside_is_kept() {
        let s = spec_timed(
            "2024-12-31T23:00:00Z",
            "2025-01-01T01:00:00Z",
            "UTC",
            &["RRULE:FREQ=YEARLY;COUNT=1"],
        );
        let inst = expand_dates(&s, WIN, 5000).unwrap();
        assert_eq!(inst.len(), 1);
    }

    // ---- database level: exceptions -------------------------------------------------

    fn seed_master(conn: &Connection) -> EventRow {
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
            recurrence: Some(vec!["RRULE:FREQ=WEEKLY;COUNT=4".into()]),
            attendees: "[]".into(),
            reminders: r#"{"useDefault":true}"#.into(),
            event_type: "default".into(),
            ..Default::default()
        };
        q::upsert_event(conn, &m).unwrap();
        m
    }

    #[test]
    fn moved_and_cancelled_exceptions() {
        let mut conn = crate::db::open_memory().unwrap();
        let m = seed_master(&conn);
        // Second instance moved two hours later; third cancelled.
        let moved = EventRow {
            id: "m1_20260309T130000Z".into(),
            recurring_event_id: Some("m1".into()),
            original_start_ts: Some(ts("2026-03-09T13:00:00Z")),
            start_ts: Some(ts("2026-03-09T15:00:00Z")),
            end_ts: Some(ts("2026-03-09T16:00:00Z")),
            recurrence: None,
            ..m.clone()
        };
        let cancelled = EventRow {
            id: "m1_20260316T130000Z".into(),
            recurring_event_id: Some("m1".into()),
            original_start_ts: Some(ts("2026-03-16T13:00:00Z")),
            status: "cancelled".into(),
            recurrence: None,
            ..m.clone()
        };
        q::upsert_event(&conn, &moved).unwrap();
        q::upsert_event(&conn, &cancelled).unwrap();

        let n = expand_master(&mut conn, "local", "local-personal", "m1", WIN).unwrap();
        assert_eq!(n, 3);
        let occ = q::occurrences_of_event(&conn, "local", "local-personal", "m1").unwrap();
        let got: Vec<(String, i64)> = occ
            .iter()
            .map(|o| (o.event_id.clone(), o.start_ts))
            .collect();
        assert_eq!(
            got,
            vec![
                ("m1".to_string(), ts("2026-03-02T13:00:00Z")),
                (
                    "m1_20260309T130000Z".to_string(),
                    ts("2026-03-09T15:00:00Z")
                ),
                ("m1".to_string(), ts("2026-03-23T13:00:00Z")),
            ]
        );
        assert!(occ.iter().all(|o| o.master_id.as_deref() == Some("m1")));

        // Re-expanding replaces, never duplicates.
        expand_master(&mut conn, "local", "local-personal", "m1", WIN).unwrap();
        assert_eq!(
            q::occurrences_of_event(&conn, "local", "local-personal", "m1")
                .unwrap()
                .len(),
            3
        );

        // Cancelling the master removes everything.
        q::upsert_event(
            &conn,
            &EventRow {
                status: "cancelled".into(),
                ..m
            },
        )
        .unwrap();
        assert_eq!(
            expand_master(&mut conn, "local", "local-personal", "m1", WIN).unwrap(),
            0
        );
        assert!(
            q::occurrences_of_event(&conn, "local", "local-personal", "m1")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn materialize_simple_event_and_cancelled() {
        let mut conn = crate::db::open_memory().unwrap();
        let e = EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: "s1".into(),
            status: "confirmed".into(),
            start_ts: Some(ts("2026-06-01T12:00:00Z")),
            end_ts: Some(ts("2026-06-01T13:00:00Z")),
            all_day: false,
            attendees: "[]".into(),
            reminders: "{}".into(),
            event_type: "default".into(),
            ..Default::default()
        };
        q::upsert_event(&conn, &e).unwrap();
        materialize_simple(&mut conn, "local", "local-personal", "s1", WIN).unwrap();
        let occ = q::occurrences_of_event(&conn, "local", "local-personal", "s1").unwrap();
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].id, "local|local-personal|s1|1780315200");
        assert!(occ[0].master_id.is_none());
        q::upsert_event(
            &conn,
            &EventRow {
                status: "cancelled".into(),
                ..e
            },
        )
        .unwrap();
        materialize_simple(&mut conn, "local", "local-personal", "s1", WIN).unwrap();
        assert!(
            q::occurrences_of_event(&conn, "local", "local-personal", "s1")
                .unwrap()
                .is_empty()
        );
    }
}
