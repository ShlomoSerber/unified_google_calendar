//! Human readable recurrence text for the event popup ("Weekly on Monday"). See docs/03 section 9.
//! Wording follows Google Calendar's English UI. Anything not covered says "Custom".

use chrono::{Datelike, NaiveDate, Weekday};

use crate::recurrence::expand::{rrule_part, split_line};

fn weekday_name(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

fn month_name(m: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    NAMES
        .get((m as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("")
}

fn short_month(m: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES
        .get((m as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("")
}

fn byday_weekday(code: &str) -> Option<Weekday> {
    match code {
        "MO" => Some(Weekday::Mon),
        "TU" => Some(Weekday::Tue),
        "WE" => Some(Weekday::Wed),
        "TH" => Some(Weekday::Thu),
        "FR" => Some(Weekday::Fri),
        "SA" => Some(Weekday::Sat),
        "SU" => Some(Weekday::Sun),
        _ => None,
    }
}

fn ordinal(n: i32) -> String {
    match n {
        1 => "first".into(),
        2 => "second".into(),
        3 => "third".into(),
        4 => "fourth".into(),
        5 => "fifth".into(),
        -1 => "last".into(),
        _ => n.to_string(),
    }
}

/// Order weekdays Monday first, as Google lists them.
fn sorted_days(days: &[Weekday]) -> Vec<Weekday> {
    let mut v: Vec<Weekday> = days.to_vec();
    v.sort_by_key(|d| d.num_days_from_monday());
    v.dedup();
    v
}

/// Describe the RRULE of `lines` for a series starting on `start` (local date of the master).
pub fn describe(lines: &[String], start: NaiveDate) -> Option<String> {
    let rule = lines
        .iter()
        .find_map(|l| split_line(l).filter(|c| c.name == "RRULE").map(|c| c.value))?;
    let freq = rrule_part(&rule, "FREQ")?.to_ascii_uppercase();
    let interval: u32 = rrule_part(&rule, "INTERVAL")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let count: Option<u32> = rrule_part(&rule, "COUNT").and_then(|v| v.parse().ok());
    let until = rrule_part(&rule, "UNTIL")
        .and_then(|v| NaiveDate::parse_from_str(&v[..8.min(v.len())], "%Y%m%d").ok());
    let byday: Vec<&str> = rrule_part(&rule, "BYDAY")
        .map(|v| v.split(',').collect())
        .unwrap_or_default();
    let bymonthday: Vec<i32> = rrule_part(&rule, "BYMONTHDAY")
        .map(|v| v.split(',').filter_map(|d| d.parse().ok()).collect())
        .unwrap_or_default();
    let bymonth: Vec<u32> = rrule_part(&rule, "BYMONTH")
        .map(|v| v.split(',').filter_map(|d| d.parse().ok()).collect())
        .unwrap_or_default();
    let bysetpos: Vec<i32> = rrule_part(&rule, "BYSETPOS")
        .map(|v| v.split(',').filter_map(|d| d.parse().ok()).collect())
        .unwrap_or_default();

    let head = match freq.as_str() {
        "DAILY" => {
            if !byday.is_empty() || !bymonthday.is_empty() {
                "Custom".to_string()
            } else if interval == 1 {
                "Daily".to_string()
            } else {
                format!("Every {interval} days")
            }
        }
        "WEEKLY" => {
            let days: Vec<Weekday> = if byday.is_empty() {
                vec![start.weekday()]
            } else {
                let parsed: Option<Vec<Weekday>> = byday.iter().map(|d| byday_weekday(d)).collect();
                match parsed {
                    Some(v) => sorted_days(&v),
                    None => return Some("Custom".into()),
                }
            };
            let weekdays = [
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ];
            if interval == 1 && days.len() == 5 && weekdays.iter().all(|d| days.contains(d)) {
                "Every weekday (Monday to Friday)".to_string()
            } else {
                let names: Vec<&str> = days.iter().map(|d| weekday_name(*d)).collect();
                if interval == 1 {
                    format!("Weekly on {}", names.join(", "))
                } else {
                    format!("Every {interval} weeks on {}", names.join(", "))
                }
            }
        }
        "MONTHLY" => {
            let prefix = if interval == 1 {
                "Monthly".to_string()
            } else {
                format!("Every {interval} months")
            };
            if !byday.is_empty() && byday.len() == 1 {
                let code = byday[0];
                let (n, day) = match code.find(|c: char| c.is_ascii_alphabetic()) {
                    Some(0) => (bysetpos.first().copied(), code),
                    Some(i) => (code[..i].parse::<i32>().ok(), &code[i..]),
                    None => return Some("Custom".into()),
                };
                match (n, byday_weekday(day)) {
                    (Some(n), Some(d)) => {
                        format!("{prefix} on the {} {}", ordinal(n), weekday_name(d))
                    }
                    _ => "Custom".to_string(),
                }
            } else if byday.is_empty() {
                let day = bymonthday.first().copied().unwrap_or(start.day() as i32);
                if bymonthday.len() > 1 {
                    "Custom".to_string()
                } else if day == -1 {
                    format!("{prefix} on the last day")
                } else {
                    format!("{prefix} on day {day}")
                }
            } else {
                "Custom".to_string()
            }
        }
        "YEARLY" => {
            let month = bymonth.first().copied().unwrap_or(start.month());
            let day = bymonthday.first().copied().unwrap_or(start.day() as i32);
            if !byday.is_empty() || bymonth.len() > 1 || bymonthday.len() > 1 {
                "Custom".to_string()
            } else if interval == 1 {
                format!("Annually on {} {day}", month_name(month))
            } else {
                format!("Every {interval} years on {} {day}", month_name(month))
            }
        }
        _ => "Custom".to_string(),
    };
    let tail = if let Some(n) = count {
        format!(", {n} times")
    } else if let Some(u) = until {
        format!(
            ", until {} {}, {}",
            short_month(u.month()),
            u.day(),
            u.year()
        )
    } else {
        String::new()
    };
    Some(format!("{head}{tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn t(rule: &str, start: &str) -> String {
        describe(&[format!("RRULE:{rule}")], d(start)).unwrap()
    }

    #[test]
    fn google_wording() {
        assert_eq!(t("FREQ=DAILY", "2026-03-02"), "Daily");
        assert_eq!(
            t("FREQ=DAILY;INTERVAL=2;COUNT=5", "2026-03-02"),
            "Every 2 days, 5 times"
        );
        assert_eq!(t("FREQ=WEEKLY", "2026-03-02"), "Weekly on Monday");
        assert_eq!(
            t(
                "FREQ=WEEKLY;BYDAY=WE,MO;UNTIL=20260318T115959Z",
                "2026-03-02"
            ),
            "Weekly on Monday, Wednesday, until Mar 18, 2026"
        );
        assert_eq!(
            t("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO", "2026-03-02"),
            "Every 2 weeks on Monday"
        );
        assert_eq!(
            t("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", "2026-03-02"),
            "Every weekday (Monday to Friday)"
        );
        assert_eq!(t("FREQ=MONTHLY", "2026-03-15"), "Monthly on day 15");
        assert_eq!(
            t("FREQ=MONTHLY;BYDAY=3MO", "2026-03-16"),
            "Monthly on the third Monday"
        );
        assert_eq!(
            t("FREQ=MONTHLY;BYDAY=MO;BYSETPOS=-1", "2026-03-30"),
            "Monthly on the last Monday"
        );
        assert_eq!(
            t("FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=-1", "2026-03-31"),
            "Every 3 months on the last day"
        );
        assert_eq!(t("FREQ=YEARLY", "2026-03-02"), "Annually on March 2");
        assert_eq!(
            t("FREQ=YEARLY;INTERVAL=2", "2026-03-02"),
            "Every 2 years on March 2"
        );
        assert_eq!(t("FREQ=HOURLY", "2026-03-02"), "Custom");
        assert_eq!(t("FREQ=WEEKLY;BYDAY=XX", "2026-03-02"), "Custom");
        assert!(describe(&["EXDATE:20260302".into()], d("2026-03-02")).is_none());
    }
}
