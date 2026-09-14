//! calendarList sync per account. See docs/05-sincronizacion.md section 2.1.

use crate::db::queries::{accounts, calendars, settings};
use crate::error::AppError;
use crate::sync::ctx::SyncCtx;
use crate::sync::map::calendar_to_row;

/// Outcome of a calendarList sync: calendars that need a full events sync (new or reset).
#[derive(Debug, Default)]
pub struct CalendarListOutcome {
    pub new_calendars: Vec<String>,
    pub deleted_calendars: Vec<String>,
}

pub async fn sync_calendar_list(
    ctx: &SyncCtx,
    account_id: &str,
) -> Result<CalendarListOutcome, AppError> {
    let acc = account_id.to_string();
    let account = ctx
        .db
        .call(move |c| accounts::get_account(c, &acc))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Account {account_id}")))?;
    let token = ctx.tokens.token(account_id).await?;
    let mut sync_token = account.calendar_list_sync_token.clone();
    let mut page_token: Option<String> = None;
    let mut outcome = CalendarListOutcome::default();
    let mut retried_after_410 = false;
    loop {
        let page = match ctx
            .client
            .calendar_list_page(&token, page_token.as_deref(), sync_token.as_deref())
            .await
        {
            Ok(p) => p,
            Err(AppError::Google { status: 410, .. })
                if sync_token.is_some() && !retried_after_410 =>
            {
                tracing::info!(
                    account = account_id,
                    "calendarList sync token gone; full list"
                );
                sync_token = None;
                page_token = None;
                retried_after_410 = true;
                continue;
            }
            Err(e) => return Err(e),
        };
        let acc = account_id.to_string();
        let items = page.items;
        let (new_ids, deleted_ids) = ctx
            .db
            .call(move |c| {
                let mut new_ids = Vec::new();
                let mut deleted_ids = Vec::new();
                let existing_all = calendars::list_calendars_of_account(c, &acc)?;
                let next_order = existing_all.iter().map(|x| x.sort_order).max().unwrap_or(0) + 1;
                for (i, entry) in items.iter().enumerate() {
                    let existing = calendars::get_calendar(c, &acc, &entry.id)?;
                    if entry.deleted.unwrap_or(false) {
                        if existing.is_some() {
                            calendars::mark_deleted(c, &acc, &entry.id)?;
                            deleted_ids.push(entry.id.clone());
                        }
                        continue;
                    }
                    let order = existing
                        .as_ref()
                        .map(|e| e.sort_order)
                        .unwrap_or(next_order + i as i64);
                    let row = calendar_to_row(&acc, entry, existing.as_ref(), order);
                    calendars::upsert_calendar(c, &row)?;
                    if existing.as_ref().is_none_or(|e| e.deleted) {
                        // A resurrected calendar starts from scratch.
                        calendars::set_sync_token(c, &acc, &entry.id, None, false)?;
                        c.execute(
                            "UPDATE calendars SET deleted=0 WHERE account_id=?1 AND id=?2",
                            rusqlite::params![acc, entry.id],
                        )?;
                        new_ids.push(entry.id.clone());
                    }
                }
                Ok((new_ids, deleted_ids))
            })
            .await?;
        outcome.new_calendars.extend(new_ids);
        outcome.deleted_calendars.extend(deleted_ids);
        if let Some(st) = page.next_sync_token {
            let (acc, st2) = (account_id.to_string(), st.clone());
            ctx.db
                .call(move |c| {
                    accounts::set_calendar_list_sync_token(c, &acc, Some(&st2))?;
                    settings::log_sync(c, Some(&acc), None, "incremental", Some("calendarList"))
                })
                .await?;
            break;
        }
        match page.next_page_token {
            Some(p) => page_token = Some(p),
            None => {
                return Err(AppError::Network(
                    "calendarList.list ended without nextSyncToken".into(),
                ))
            }
        }
    }
    Ok(outcome)
}
