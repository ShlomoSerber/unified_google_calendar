// Prevents an extra console window on Windows; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's DMABUF renderer draws blurry, heavy text on the user's Wayland session (AMD,
    // GNOME); the legacy renderer is crisp. Verified by the user on 2026-09-15 (docs/99). Set
    // before the webview exists; an explicit value in the environment wins.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    unified_google_calendar_lib::run()
}
