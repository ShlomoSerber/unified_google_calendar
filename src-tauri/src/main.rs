// Prevents an extra console window on Windows; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    unified_google_calendar_lib::run()
}
