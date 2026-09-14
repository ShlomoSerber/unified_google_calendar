//! Holiday calendar subscription. See docs/03-modelo-de-datos.md section 1 and
//! docs/09-setup-usuario.md section E: `en.ar#holiday@group.v.calendar.google.com` is inserted
//! in the first Gmail account's calendarList, remembered in `settings.holidays_account`.

use crate::config::HOLIDAY_CALENDAR_ID;
use crate::db::queries::settings;
use crate::error::AppError;
use crate::sync::SyncCtx;

pub fn is_gmail(email: &str) -> bool {
    let e = email.to_ascii_lowercase();
    e.ends_with("@gmail.com") || e.ends_with("@googlemail.com")
}

/// Subscribe `account_id` to the holiday calendar when no account holds it yet (or when
/// `force` moves the subscription). Returns `true` when a subscription was made.
pub async fn ensure_holidays(
    ctx: &SyncCtx,
    account_id: &str,
    email: &str,
    force: bool,
) -> Result<bool, AppError> {
    let current: Option<String> = ctx
        .db
        .call(|c| settings::get(c, "holidays_account"))
        .await?
        .filter(|s: &String| !s.is_empty());
    if !force && (current.is_some() || !is_gmail(email)) {
        return Ok(false);
    }
    if current.as_deref() == Some(account_id) {
        return Ok(false);
    }
    let token = ctx.tokens.token(account_id).await?;
    let entry = ctx
        .client
        .calendar_list_insert(&token, HOLIDAY_CALENDAR_ID)
        .await?;
    let (acc, entry_id) = (account_id.to_string(), entry.id.clone());
    ctx.db
        .call(move |c| {
            settings::set(c, "holidays_account", &acc)?;
            settings::log_sync(
                c,
                Some(&acc),
                Some(&entry_id),
                "incremental",
                Some("holidays subscribed"),
            )
        })
        .await?;
    tracing::info!(account = account_id, "holiday calendar subscribed");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_detection() {
        assert!(is_gmail("Someone@Gmail.com"));
        assert!(is_gmail("x@googlemail.com"));
        assert!(!is_gmail("aaron@greelow.com"));
    }
}
