//! `get_settings` / `set_settings`. See docs/03-modelo-de-datos.md section 1 (`settings` keys)
//! and docs/02-arquitectura.md section 5.

use rusqlite::Connection;

use crate::commands::types::{CalendarKey, Settings};
use crate::db::queries::settings as s;
use crate::error::AppError;

pub fn load(conn: &Connection) -> Result<Settings, AppError> {
    let d = Settings::default();
    Ok(Settings {
        primary_tz: s::get_or(conn, "primary_tz", d.primary_tz)?,
        secondary_tz: s::get_or(conn, "secondary_tz", d.secondary_tz)?,
        week_start: s::get_or(conn, "week_start", d.week_start)?,
        hour_format: s::get_or(conn, "hour_format", d.hour_format)?,
        theme: s::get_or(conn, "theme", d.theme)?,
        default_calendar: s::get::<CalendarKey>(conn, "default_calendar")?,
        data_window_past_days: s::get_or(conn, "data_window_past_days", d.data_window_past_days)?,
        data_window_future_days: s::get_or(
            conn,
            "data_window_future_days",
            d.data_window_future_days,
        )?,
        webhook_port: s::get_or(conn, "webhook_port", d.webhook_port)?,
        public_base_url: s::get::<String>(conn, "public_base_url")?.filter(|u| !u.is_empty()),
        push_enabled: s::get_or(conn, "push_enabled", d.push_enabled)?,
        holidays_account: s::get::<String>(conn, "holidays_account")?.filter(|u| !u.is_empty()),
        push_error: s::get::<String>(conn, "push_error")?.filter(|u| !u.is_empty()),
        oauth_configured: crate::auth::oauth::OAuthConfig::load().is_ok(),
    })
}

fn valid_tz(name: &str) -> Result<(), AppError> {
    name.parse::<chrono_tz::Tz>()
        .map(|_| ())
        .map_err(|_| AppError::invalid(format!("{name} is not a valid time zone name")))
}

pub fn save(conn: &Connection, v: &Settings) -> Result<(), AppError> {
    valid_tz(&v.primary_tz)?;
    if let Some(tz) = &v.secondary_tz {
        valid_tz(tz)?;
    }
    if v.week_start != 1 {
        return Err(AppError::invalid(
            "The week starts on Monday in this version",
        ));
    }
    if v.hour_format != "24" {
        return Err(AppError::invalid(
            "Only the 24-hour format is available in this version",
        ));
    }
    if let Some(u) = &v.public_base_url {
        if !u.starts_with("https://") || u.ends_with('/') {
            return Err(AppError::invalid(
                "The push URL must start with https:// and have no trailing slash",
            ));
        }
    }
    if v.data_window_past_days < 30 || v.data_window_future_days < 30 {
        return Err(AppError::invalid(
            "The data window must cover at least 30 days on each side",
        ));
    }
    s::set(conn, "primary_tz", &v.primary_tz)?;
    s::set(conn, "secondary_tz", &v.secondary_tz)?;
    s::set(conn, "week_start", &v.week_start)?;
    s::set(conn, "hour_format", &v.hour_format)?;
    s::set(conn, "theme", &v.theme)?;
    match &v.default_calendar {
        Some(k) => s::set(conn, "default_calendar", k)?,
        None => s::delete(conn, "default_calendar")?,
    }
    s::set(conn, "data_window_past_days", &v.data_window_past_days)?;
    s::set(conn, "data_window_future_days", &v.data_window_future_days)?;
    s::set(conn, "webhook_port", &v.webhook_port)?;
    s::set(
        conn,
        "public_base_url",
        &v.public_base_url.clone().unwrap_or_default(),
    )?;
    s::set(conn, "push_enabled", &v.push_enabled)?;
    s::set(
        conn,
        "holidays_account",
        &v.holidays_account.clone().unwrap_or_default(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_and_validation() {
        let conn = crate::db::open_memory().unwrap();
        let mut d = load(&conn).unwrap();
        d.oauth_configured = false; // depends on the developer machine
        assert_eq!(d, Settings::default());
        let mut v = d.clone();
        v.secondary_tz = None;
        v.public_base_url = Some("https://pc.tail.ts.net".into());
        v.holidays_account = Some("acc".into());
        save(&conn, &v).unwrap();
        let mut loaded = load(&conn).unwrap();
        loaded.oauth_configured = false;
        assert_eq!(loaded, v);
        v.public_base_url = Some("http://x".into());
        assert!(save(&conn, &v).is_err());
        v.public_base_url = None;
        v.primary_tz = "Nope/Zone".into();
        assert!(save(&conn, &v).is_err());
    }
}
