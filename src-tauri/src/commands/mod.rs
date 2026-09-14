//! The only IPC surface. See docs/02-arquitectura.md section 5.
//!
//! Every `#[tauri::command]` converts `AppError` into its `user_message()` at this boundary.

pub mod events;
pub mod types;
pub mod view;

use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::recurrence::Window;

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
            .map(account_info)
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

/// Emit `calendar:updated` for a touched range (docs/02 section 5).
pub fn emit_updated(app: &tauri::AppHandle, t: &events::Touched) {
    let payload = types::CalendarUpdated {
        from: t.from,
        to: t.to,
        calendar_ids: t.calendars.clone(),
    };
    if let Err(e) = app.emit(types::EVENT_CALENDAR_UPDATED, &payload) {
        tracing::warn!(error = %e, "failed to emit calendar:updated");
    }
}

fn not_google_yet(account_id: &str) -> Result<(), AppError> {
    if events::is_local(account_id) {
        Ok(())
    } else {
        Err(AppError::invalid(
            "Writing to Google accounts is not available in this build yet",
        ))
    }
}

#[tauri::command]
pub async fn create_event(
    app: tauri::AppHandle,
    draft: types::EventDraft,
) -> CmdResult<types::EventDetail> {
    not_google_yet(&draft.account_id).map_err(to_ipc)?;
    let (detail, touched) = db::call(move |c| {
        let w = Window::current(c)?;
        events::create_local(c, &draft, w)
    })
    .await
    .map_err(to_ipc)?;
    emit_updated(&app, &touched);
    Ok(detail)
}

#[tauri::command]
pub async fn update_event(
    app: tauri::AppHandle,
    occurrence_id: String,
    draft: types::EventDraft,
    scope: types::EditScope,
) -> CmdResult<types::EventDetail> {
    not_google_yet(&draft.account_id).map_err(to_ipc)?;
    let (detail, touched) = db::call(move |c| {
        let w = Window::current(c)?;
        events::update_local(c, &occurrence_id, &draft, scope, w)
    })
    .await
    .map_err(to_ipc)?;
    emit_updated(&app, &touched);
    Ok(detail)
}

#[tauri::command]
pub async fn delete_event(
    app: tauri::AppHandle,
    occurrence_id: String,
    scope: types::EditScope,
) -> CmdResult<()> {
    let touched = db::call(move |c| {
        let (account_id, _, _) = db::queries::events::parse_occurrence_id(&occurrence_id)
            .ok_or_else(|| AppError::NotFound("The event".into()))?;
        not_google_yet(&account_id)?;
        let w = Window::current(c)?;
        events::delete_local(c, &occurrence_id, scope, w)
    })
    .await
    .map_err(to_ipc)?;
    emit_updated(&app, &touched);
    Ok(())
}

#[tauri::command]
pub async fn move_event_account(
    app: tauri::AppHandle,
    occurrence_id: String,
    target_account_id: String,
    target_calendar_id: String,
) -> CmdResult<types::EventDetail> {
    let (detail, touched) = db::call(move |c| {
        let w = Window::current(c)?;
        events::move_local_to_local(
            c,
            &occurrence_id,
            &target_account_id,
            &target_calendar_id,
            w,
        )
    })
    .await
    .map_err(to_ipc)?;
    emit_updated(&app, &touched);
    Ok(detail)
}

fn account_info(a: db::queries::accounts::AccountRow) -> types::AccountInfo {
    types::AccountInfo {
        id: a.id,
        kind: a.kind,
        email: a.email,
        display_name: a.display_name,
        sort_order: a.sort_order,
        sync_state: a.sync_state,
        sync_error: a.sync_error,
        last_sync_at: a.last_sync_at,
    }
}

/// Emit `account:changed` with the current account list.
pub async fn emit_accounts(app: &tauri::AppHandle) {
    if let Ok(list) = db::call(|c| {
        Ok(db::queries::accounts::list_accounts(c)?
            .into_iter()
            .map(account_info)
            .collect::<Vec<_>>())
    })
    .await
    {
        let _ = app.emit(types::EVENT_ACCOUNT_CHANGED, &list);
    }
}

#[tauri::command]
pub async fn add_account(app: tauri::AppHandle) -> CmdResult<types::AccountInfo> {
    let info = crate::auth::add_account(&app).await.map_err(to_ipc)?;
    emit_accounts(&app).await;
    // calendarList + full sync of every calendar of the new account (docs/05 section 1.2 step 8).
    if let Ok(engine) = crate::sync::engine::engine() {
        let id = info.id.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = engine.sync_account(&id, "add_account").await {
                tracing::warn!(account = %id, error = %e, "initial sync failed");
            }
        });
    }
    Ok(info)
}

#[tauri::command]
pub async fn sync_now(app: tauri::AppHandle) -> CmdResult<()> {
    let engine = crate::sync::engine::engine().map_err(to_ipc)?;
    tauri::async_runtime::spawn(async move {
        let _ = engine.sync_all("manual").await;
    });
    emit_accounts(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn remove_account(app: tauri::AppHandle, account_id: String) -> CmdResult<()> {
    if events::is_local(&account_id) {
        return Err(AppError::invalid("The local account cannot be removed").user_message());
    }
    let id = account_id.clone();
    db::call(move |c| {
        // Tokens first, so a crash leaves no orphan credentials behind.
        let _ = crate::auth::oauth::remove_tokens(&id);
        let n = db::queries::accounts::delete_account(c, &id)?;
        if n == 0 {
            return Err(AppError::NotFound("The account".into()));
        }
        Ok(())
    })
    .await
    .map_err(to_ipc)?;
    emit_accounts(&app).await;
    emit_updated(
        &app,
        &events::Touched {
            from: 0,
            to: i64::MAX,
            calendars: vec![],
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    if !url.starts_with("https://") {
        return Err(AppError::invalid("Only https links can be opened").user_message());
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("The link could not be opened ({e})."))
}

#[tauri::command]
pub fn get_colors() -> CmdResult<types::ColorPalette> {
    Ok(types::ColorPalette {
        events: crate::google::colors::EVENT_PALETTE
            .iter()
            .map(|c| types::ColorEntry {
                id: c.id.into(),
                name: c.name.into(),
                bg: c.bg.into(),
            })
            .collect(),
    })
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
