//! The only IPC surface. See docs/02-arquitectura.md section 5.
//!
//! Every `#[tauri::command]` converts `AppError` into its `user_message()` at this boundary.

pub mod types;
pub mod view;

use crate::db;
use crate::error::AppError;

pub type CmdResult<T> = Result<T, String>;

fn to_ipc(e: AppError) -> String {
    tracing::debug!(error = %e, "command failed");
    e.user_message()
}

#[tauri::command]
pub async fn get_view(from: i64, to: i64, tz: String) -> CmdResult<types::ViewPayload> {
    db::call(move |c| view::get_view(c, from, to, &tz))
        .await
        .map_err(to_ipc)
}

#[tauri::command]
pub async fn get_event(occurrence_id: String) -> CmdResult<types::EventDetail> {
    db::call(move |c| view::get_event(c, &occurrence_id))
        .await
        .map_err(to_ipc)
}

#[tauri::command]
pub async fn list_accounts() -> CmdResult<Vec<types::AccountInfo>> {
    db::call(|c| {
        Ok(db::queries::accounts::list_accounts(c)?
            .into_iter()
            .map(|a| types::AccountInfo {
                id: a.id,
                kind: a.kind,
                email: a.email,
                display_name: a.display_name,
                sort_order: a.sort_order,
                sync_state: a.sync_state,
                sync_error: a.sync_error,
                last_sync_at: a.last_sync_at,
            })
            .collect())
    })
    .await
    .map_err(to_ipc)
}

#[tauri::command]
pub async fn list_calendars() -> CmdResult<Vec<types::CalendarInfo>> {
    db::call(|c| {
        Ok(db::queries::calendars::list_calendars(c)?
            .into_iter()
            .map(|cal| types::CalendarInfo {
                default_reminders: view::parse_default_reminders(&cal.default_reminders),
                is_local: cal.account_id == crate::config::LOCAL_ACCOUNT_ID,
                id: cal.id,
                account_id: cal.account_id,
                summary: cal.summary,
                description: cal.description,
                color_bg: cal.color_bg,
                color_fg: cal.color_fg,
                access_role: cal.access_role,
                is_primary: cal.is_primary,
                visible: cal.visible,
                hidden_remote: cal.hidden_remote,
                time_zone: cal.time_zone,
                sort_order: cal.sort_order,
            })
            .collect())
    })
    .await
    .map_err(to_ipc)
}

#[tauri::command]
pub async fn set_calendar_visible(
    account_id: String,
    calendar_id: String,
    visible: bool,
) -> CmdResult<()> {
    db::call(move |c| {
        let n = db::queries::calendars::set_visible(c, &account_id, &calendar_id, visible)?;
        if n == 0 {
            return Err(AppError::NotFound("The calendar".into()));
        }
        Ok(())
    })
    .await
    .map_err(to_ipc)
}

/// Commands registered with the Tauri builder.
pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_view,
        get_event,
        list_accounts,
        list_calendars,
        set_calendar_visible
    ]
}
