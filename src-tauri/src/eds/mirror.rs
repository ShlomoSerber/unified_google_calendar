//! Mirror of the app's occurrences into Evolution Data Server so GNOME Shell's calendar
//! panel shows them. See docs/06-integracion-gnome.md sections 4.2, 4.4 and 4.5.
//!
//! One local EDS source per calendar (`ugc-<sha1(account|calendar)>`, `ugc-local` for the
//! personal calendar), `[Alarms] IncludeMe=false`, VEVENTs in UTC without VALARM, one per
//! occurrence, window today-30 .. today+180 days. Diff by comparing the generated text.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use sha1::{Digest, Sha1};

use crate::config::{LOCAL_ACCOUNT_ID, LOCAL_CALENDAR_ID};
use crate::db::queries::calendars;
use crate::db::DbHandle;
use crate::eds::dbus::Eds;
use crate::error::AppError;

pub const PAST_DAYS: i64 = 30;
pub const FUTURE_DAYS: i64 = 180;
pub const DEBOUNCE_SECS: u64 = 5;

pub fn source_uid(account_id: &str, calendar_id: &str) -> String {
    if account_id == LOCAL_ACCOUNT_ID && calendar_id == LOCAL_CALENDAR_ID {
        return "ugc-local".into();
    }
    let digest = Sha1::digest(format!("{account_id}|{calendar_id}").as_bytes());
    format!(
        "ugc-{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

pub fn display_name(summary: &str, email: Option<&str>, is_local: bool) -> String {
    if is_local {
        "Personal (Unified Google Calendar)".into()
    } else {
        match email {
            Some(e) => format!("{summary} · {e}"),
            None => summary.to_string(),
        }
    }
}

/// Key-file of docs/06 section 4.2.
pub fn keyfile(display: &str, color: &str) -> String {
    format!(
        "[Data Source]\nDisplayName={display}\nEnabled=true\nParent=local-stub\n\n[Calendar]\nBackendName=local\nColor={color}\nSelected=true\nOrder=0\n\n[Alarms]\nIncludeMe=false\nForEveryEvent=false\n\n[Offline]\nStaySynchronized=true\n"
    )
}

fn ical_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn fmt_utc(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .unwrap_or_default()
        .format("%Y%m%dT%H%M%SZ")
        .to_string()
}

fn fmt_date(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .unwrap_or_default()
        .format("%Y%m%d")
        .to_string()
}

/// One occurrence to mirror.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorItem {
    pub uid: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub all_day: bool,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub updated_ts: i64,
}

/// VEVENT of docs/06 section 4.4: UTC, no VALARM, no RRULE.
pub fn vevent(item: &MirrorItem) -> String {
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}", item.uid),
        format!("DTSTAMP:{}", fmt_utc(item.updated_ts)),
    ];
    if item.all_day {
        lines.push(format!("DTSTART;VALUE=DATE:{}", fmt_date(item.start_ts)));
        lines.push(format!("DTEND;VALUE=DATE:{}", fmt_date(item.end_ts)));
    } else {
        lines.push(format!("DTSTART:{}", fmt_utc(item.start_ts)));
        lines.push(format!("DTEND:{}", fmt_utc(item.end_ts)));
    }
    lines.push(format!(
        "SUMMARY:{}",
        ical_escape(item.summary.as_deref().unwrap_or("(No title)"))
    ));
    if let Some(l) = item.location.as_deref().filter(|l| !l.is_empty()) {
        lines.push(format!("LOCATION:{}", ical_escape(l)));
    }
    lines.push(format!("LAST-MODIFIED:{}", fmt_utc(item.updated_ts)));
    lines.push("SEQUENCE:0".into());
    lines.push("END:VEVENT".into());
    lines
        .iter()
        .map(|l| fold(l))
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n"
}

/// Fold a content line at 72 octet-safe characters (RFC 5545 section 3.1).
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for c in line.chars() {
        if count + c.len_utf8() > 72 {
            out.push_str("\r\n ");
            count = 1;
        }
        out.push(c);
        count += c.len_utf8();
    }
    out
}

/// `UID` property of a VEVENT text (lines are unfolded first: EDS folds at 75 octets).
pub fn uid_of(vevent: &str) -> Option<String> {
    crate::sync::ics::unfold(vevent)
        .into_iter()
        .find_map(|l| l.strip_prefix("UID:").map(str::to_string))
}

/// Comparable form of a VEVENT: the lines that matter, unfolded and normalized.
fn normalized(vevent: &str) -> String {
    crate::sync::ics::unfold(vevent)
        .into_iter()
        .filter(|l| {
            let key = l.split([':', ';']).next().unwrap_or("");
            matches!(
                key,
                "UID" | "DTSTART" | "DTEND" | "SUMMARY" | "LOCATION" | "LAST-MODIFIED"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What to create, modify and remove to make `existing` equal `desired`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub create: Vec<String>,
    pub modify: Vec<String>,
    pub remove: Vec<String>,
}

pub fn diff(existing: &[String], desired: &[MirrorItem]) -> Diff {
    let mut have: HashMap<String, String> = HashMap::new();
    for e in existing {
        if let Some(uid) = uid_of(e) {
            have.insert(uid, normalized(e));
        }
    }
    let mut out = Diff::default();
    let mut seen = HashSet::new();
    for item in desired {
        let text = vevent(item);
        seen.insert(item.uid.clone());
        match have.get(&item.uid) {
            None => out.create.push(text),
            Some(old) if *old != normalized(&text) => out.modify.push(text),
            Some(_) => {}
        }
    }
    out.remove = have
        .keys()
        .filter(|u| !seen.contains(*u))
        .cloned()
        .collect();
    out.remove.sort();
    out
}

/// Mirror window in UTC seconds.
pub fn window(now: DateTime<Utc>) -> (i64, i64) {
    let today = now.date_naive();
    let from = (today - Duration::days(PAST_DAYS))
        .and_hms_opt(0, 0, 0)
        .map(|n| n.and_utc().timestamp())
        .unwrap_or(0);
    let to = (today + Duration::days(FUTURE_DAYS))
        .and_hms_opt(0, 0, 0)
        .map(|n| n.and_utc().timestamp())
        .unwrap_or(0);
    (from, to)
}

/// Occurrences of one calendar inside the mirror window.
pub fn items_for(
    conn: &Connection,
    account_id: &str,
    calendar_id: &str,
    from_ts: i64,
    to_ts: i64,
) -> Result<Vec<MirrorItem>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT o.id, o.start_ts, o.end_ts, o.all_day, e.summary, e.location, coalesce(e.updated_ts, e.created_ts, 0) \
         FROM occurrences o JOIN events e ON e.account_id=o.account_id AND e.calendar_id=o.calendar_id AND e.id=o.event_id \
         WHERE o.account_id=?1 AND o.calendar_id=?2 AND o.status != 'cancelled' AND o.end_ts > ?3 AND o.start_ts < ?4 \
         ORDER BY o.start_ts",
    )?;
    let rows = stmt
        .query_map(params![account_id, calendar_id, from_ts, to_ts], |r| {
            let occ_id: String = r.get(0)?;
            Ok(MirrorItem {
                uid: format!(
                    "ugc-{}",
                    occ_id.replace('|', "-").replace(['@', '#', ' '], "_")
                ),
                start_ts: r.get(1)?,
                end_ts: r.get(2)?,
                all_day: r.get::<_, i64>(3)? != 0,
                summary: r.get(4)?,
                location: r.get(5)?,
                updated_ts: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Ensure the EDS source of a calendar exists with the right name and color.
async fn ensure_source(eds: &Eds, uid: &str, display: &str, color: &str) -> Result<(), AppError> {
    let sources = eds.sources().await?;
    let wanted = keyfile(display, color);
    match sources.get(uid) {
        None => {
            eds.create_source(uid, &wanted).await?;
            // The registry announces the source asynchronously; the calendar factory only
            // sees it afterwards.
            for _ in 0..20 {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if eds.sources().await?.contains_key(uid) {
                    break;
                }
            }
            Ok(())
        }
        Some(info) => {
            let has_name = info.data.contains(&format!("DisplayName={display}"));
            let has_color = info.data.contains(&format!("Color={color}"));
            if has_name && has_color {
                Ok(())
            } else {
                eds.write_source(&info.path, &wanted).await?;
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    let fresh = eds.sources().await?;
                    if fresh
                        .get(uid)
                        .is_some_and(|i| i.data.contains(&format!("DisplayName={display}")))
                    {
                        break;
                    }
                }
                Ok(())
            }
        }
    }
}

/// Mirror the given calendars (docs/08 section 12 `mirror_calendars`).
pub async fn mirror_calendars(
    eds: &Eds,
    db: &DbHandle,
    calendar_keys: &[(String, String)],
) -> Result<(), AppError> {
    let (from, to) = window(Utc::now());
    for (account_id, calendar_id) in calendar_keys {
        let (acc, cal) = (account_id.clone(), calendar_id.clone());
        let data = db
            .call(move |c| {
                let Some(row) = calendars::get_calendar(c, &acc, &cal)? else {
                    return Ok(None);
                };
                if row.deleted {
                    return Ok(None);
                }
                let account = crate::db::queries::accounts::get_account(c, &acc)?;
                let items = items_for(c, &acc, &cal, from, to)?;
                Ok(Some((row, account.and_then(|a| a.email), items)))
            })
            .await?;
        let Some((row, email, items)) = data else {
            continue;
        };
        let uid = source_uid(account_id, calendar_id);
        let is_local_personal = account_id == LOCAL_ACCOUNT_ID && calendar_id == LOCAL_CALENDAR_ID;
        let display = display_name(&row.summary, email.as_deref(), is_local_personal);
        ensure_source(eds, &uid, &display, &row.color_bg).await?;
        let mut calendar = eds.open_calendar(&uid).await;
        for _ in 0..12 {
            if calendar.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            calendar = eds.open_calendar(&uid).await;
        }
        let calendar = calendar?;
        let result = async {
            let existing = calendar.objects("#t").await?;
            let d = diff(&existing, &items);
            calendar.create(&d.create).await?;
            calendar.modify(&d.modify).await?;
            calendar.remove(&d.remove).await?;
            tracing::debug!(calendar = %uid, created = d.create.len(), modified = d.modify.len(), removed = d.remove.len(), "eds mirrored");
            Ok::<(), AppError>(())
        }
        .await;
        calendar.close().await;
        result?;
    }
    Ok(())
}

/// Remove the sources of an account (docs/08 section 12 `remove_account_sources`). Call it
/// before the account rows are deleted, so the calendar list is still known.
pub async fn remove_account_sources(
    eds: &Eds,
    db: &DbHandle,
    account_id: &str,
) -> Result<(), AppError> {
    let acc = account_id.to_string();
    let cals = db
        .call(move |c| calendars::list_calendars_of_account(c, &acc))
        .await?;
    let wanted: HashSet<String> = cals.iter().map(|c| source_uid(account_id, &c.id)).collect();
    let sources = eds.sources().await?;
    for (uid, info) in sources {
        if wanted.contains(&uid) && info.removable {
            eds.remove_source(&info.path).await?;
        }
    }
    Ok(())
}

/// Remove `ugc-*` sources whose calendar no longer exists in the app.
pub async fn prune_orphans(eds: &Eds, db: &DbHandle) -> Result<(), AppError> {
    let cals = db.call(|c| calendars::list_calendars(c)).await?;
    let wanted: HashSet<String> = cals
        .iter()
        .map(|c| source_uid(&c.account_id, &c.id))
        .collect();
    for (uid, info) in eds.sources().await? {
        if uid.starts_with("ugc-") && !wanted.contains(&uid) && info.removable {
            tracing::info!(%uid, "removing orphan eds source");
            eds.remove_source(&info.path).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uids_and_keyfile() {
        assert_eq!(source_uid("local", "local-personal"), "ugc-local");
        let u = source_uid("acc", "cal@group.calendar.google.com");
        assert!(u.starts_with("ugc-") && u.len() == 4 + 40);
        assert_eq!(u, source_uid("acc", "cal@group.calendar.google.com"));
        let k = keyfile("Work · a@b.c", "#0b8043");
        assert!(k.contains("[Alarms]\nIncludeMe=false"));
        assert!(k.contains("Color=#0b8043"));
        assert!(k.contains("BackendName=local"));
        assert_eq!(
            display_name("Personal", None, true),
            "Personal (Unified Google Calendar)"
        );
        assert_eq!(display_name("Work", Some("a@b.c"), false), "Work · a@b.c");
    }

    #[test]
    fn vevent_and_diff() {
        let a = MirrorItem {
            uid: "ugc-a".into(),
            start_ts: 1_789_401_600,
            end_ts: 1_789_405_200,
            all_day: false,
            summary: Some("Sixty; go, now".into()),
            location: None,
            updated_ts: 1_789_000_000,
        };
        let b = MirrorItem {
            uid: "ugc-b".into(),
            start_ts: 1_789_344_000,
            end_ts: 1_789_430_400,
            all_day: true,
            summary: None,
            location: Some("Here".into()),
            updated_ts: 1_789_000_000,
        };
        let va = vevent(&a);
        assert!(va.contains("DTSTART:20260914T160000Z\r\nDTEND:20260914T170000Z"));
        assert!(va.contains("SUMMARY:Sixty\\; go\\, now"));
        assert!(!va.contains("VALARM") && !va.contains("RRULE"));
        let vb = vevent(&b);
        assert!(vb.contains("DTSTART;VALUE=DATE:20260914\r\nDTEND;VALUE=DATE:20260915"));
        assert!(vb.contains("SUMMARY:(No title)") && vb.contains("LOCATION:Here"));

        // Nothing exists: create both.
        let d = diff(&[], &[a.clone(), b.clone()]);
        assert_eq!((d.create.len(), d.modify.len(), d.remove.len()), (2, 0, 0));
        // A long UID gets folded by EDS; it must still match.
        let long = MirrorItem { uid: "ugc-117623048912784336179-shlomo.serber_greelow.com-abcdefghijklmnopqrstuvwxyz0123456789-1789401600".into(), ..a.clone() };
        let vl = vevent(&long);
        assert!(vl.contains("\r\n "), "generated text is folded");
        assert_eq!(uid_of(&vl).as_deref(), Some(long.uid.as_str()));
        assert_eq!(diff(std::slice::from_ref(&vl), std::slice::from_ref(&long)), Diff::default());
        // Both exist unchanged (EDS reformats DTSTAMP/SEQUENCE): nothing to do.
        let existing = vec![
            va.replace("SEQUENCE:0", "SEQUENCE:3")
                .replace("DTSTAMP:20260910T053320Z", "DTSTAMP:20260914T000000Z"),
            vb.clone(),
        ];
        let d = diff(&existing, &[a.clone(), b.clone()]);
        assert_eq!(d, Diff::default());
        // a changed, b gone, c new.
        let a2 = MirrorItem {
            summary: Some("Renamed".into()),
            updated_ts: 1_789_100_000,
            ..a.clone()
        };
        let c = MirrorItem {
            uid: "ugc-c".into(),
            ..a.clone()
        };
        let d = diff(&existing, &[a2, c]);
        assert_eq!(d.create.len(), 1);
        assert_eq!(d.modify.len(), 1);
        assert_eq!(d.remove, vec!["ugc-b".to_string()]);
        assert!(d.modify[0].contains("SUMMARY:Renamed"));
    }

    #[test]
    fn items_come_from_occurrences_in_window() {
        let mut conn = crate::db::open_memory().unwrap();
        let now = Utc::now();
        let (from, to) = window(now);
        let mk = |id: &str, start: i64| crate::db::queries::events::EventRow {
            account_id: "local".into(),
            calendar_id: "local-personal".into(),
            id: id.into(),
            status: "confirmed".into(),
            summary: Some(id.into()),
            start_ts: Some(start),
            end_ts: Some(start + 600),
            all_day: false,
            attendees: "[]".into(),
            reminders: "{}".into(),
            event_type: "default".into(),
            updated_ts: Some(1),
            ..Default::default()
        };
        let w = crate::recurrence::expand::Window {
            from_ts: 0,
            to_ts: i64::MAX / 2,
        };
        for e in [
            mk("in", now.timestamp() + 3600),
            mk("old", from - 86_400 * 2),
            mk("far", to + 86_400 * 2),
        ] {
            crate::db::queries::events::upsert_event(&conn, &e).unwrap();
            crate::recurrence::materialize_simple(&mut conn, "local", "local-personal", &e.id, w)
                .unwrap();
        }
        let items = items_for(&conn, "local", "local-personal", from, to).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].uid.starts_with("ugc-local-local-personal-in-"));
        assert!(!items[0].uid.contains('|'));
    }
}
