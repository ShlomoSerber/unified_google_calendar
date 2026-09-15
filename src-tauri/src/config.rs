//! Paths, ports and constants. See docs/07-empaquetado.md section 4 and docs/03 section 3.

use std::path::PathBuf;

pub const APP_NAME: &str = "Unified Google Calendar";
pub const DIR_NAME: &str = "unified-google-calendar";
pub const DEFAULT_WEBHOOK_PORT: u16 = 8080;
pub const DEFAULT_DATA_WINDOW_PAST_DAYS: i64 = 365;
pub const DEFAULT_DATA_WINDOW_FUTURE_DAYS: i64 = 730;
pub const LOCAL_ACCOUNT_ID: &str = "local";
pub const LOCAL_CALENDAR_ID: &str = "local-personal";
pub const LOCAL_ICAL_SUFFIX: &str = "@unified-google-calendar";
pub const HOLIDAY_CALENDAR_ID: &str = "en.ar#holiday@group.v.calendar.google.com";
pub const DEFAULT_PRIMARY_TZ: &str = "America/Argentina/Buenos_Aires";
pub const DEFAULT_SECONDARY_TZ: &str = "America/Mexico_City";
/// Maximum instances materialized per recurring master (docs/03 section 3).
pub const MAX_INSTANCES_PER_MASTER: usize = 5000;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// `$XDG_CONFIG_HOME/unified-google-calendar`, default `~/.config/unified-google-calendar`.
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join(DIR_NAME)
}

/// `$XDG_DATA_HOME/unified-google-calendar`, default `~/.local/share/unified-google-calendar`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join(DIR_NAME)
}

/// `$XDG_DATA_HOME/icons/hicolor`, default `~/.local/share/icons/hicolor`: the user's icon theme
/// directory, searched before `/usr/share/icons` by GTK and GNOME Shell. The app drops the
/// day-of-month icon there so the dock shows it (docs/06 section 2, tray.rs).
pub fn user_hicolor_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join("icons")
        .join("hicolor")
}

pub fn oauth_json_path() -> PathBuf {
    config_dir().join("oauth.json")
}

pub fn verify_dir() -> PathBuf {
    config_dir().join("verify")
}

pub fn db_path() -> PathBuf {
    data_dir().join("data.db")
}

pub fn tokens_path() -> PathBuf {
    data_dir().join("tokens.bin")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_end_with_expected_names() {
        assert!(db_path().ends_with("unified-google-calendar/data.db"));
        assert!(tokens_path().ends_with("unified-google-calendar/tokens.bin"));
        assert!(logs_dir().ends_with("unified-google-calendar/logs"));
        assert!(oauth_json_path().ends_with("unified-google-calendar/oauth.json"));
        assert!(verify_dir().ends_with("unified-google-calendar/verify"));
    }
}
