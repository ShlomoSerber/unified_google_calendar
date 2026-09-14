//! Push channel lifecycle. See docs/05-sincronizacion.md sections 3.2 and 3.3.
//!
//! One channel per calendar with `full_sync_done=1` and one per account for the calendarList.
//! Channels are renewed when they expire in less than 24 h: the new one is created first, the
//! old one stopped afterwards. A webhook rejection turns `push_enabled` off and leaves polling.

use std::time::Duration;

use crate::commands::types::PushTestResult;
use crate::db::queries::{accounts, calendars, channels, settings};
use crate::error::AppError;
use crate::google::types::WatchRequest;
use crate::sync::SyncCtx;

pub const CHANNEL_TTL_SECS: u64 = 604_800;
pub const RENEW_BEFORE_SECS: i64 = 24 * 3600;
pub const RENEW_INTERVAL: Duration = Duration::from_secs(3600);

/// Push configuration as stored in `settings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushConfig {
    pub enabled: bool,
    pub base_url: Option<String>,
}

pub async fn config(ctx: &SyncCtx) -> Result<PushConfig, AppError> {
    ctx.db
        .call(|c| {
            Ok(PushConfig {
                enabled: settings::get_or(c, "push_enabled", false)?,
                base_url: settings::get::<String>(c, "public_base_url")?.filter(|u| !u.is_empty()),
            })
        })
        .await
}

fn is_webhook_rejection(reason: &str) -> bool {
    let r = reason.to_ascii_lowercase();
    r.contains("webhook")
        || r.contains("push")
        || r.contains("unauthorized")
        || r.contains("domain")
}

async fn disable_push(ctx: &SyncCtx, message: String) {
    tracing::warn!(%message, "push disabled; polling every 60 s");
    let _ = ctx
        .db
        .call(move |c| {
            settings::set(c, "push_enabled", &false)?;
            settings::set(c, "push_error", &message)?;
            settings::log_sync(c, None, None, "error", Some("push disabled"))
        })
        .await;
}

/// Create or renew the channel of one resource. Returns `true` when a watch call was made.
async fn ensure_one(
    ctx: &SyncCtx,
    base_url: &str,
    account_id: &str,
    calendar_id: Option<&str>,
) -> Result<bool, AppError> {
    let (acc, cal) = (account_id.to_string(), calendar_id.map(str::to_string));
    let existing = ctx
        .db
        .call(move |c| channels::find_for(c, &acc, cal.as_deref()))
        .await?;
    let now = crate::db::now_ts();
    if let Some(e) = &existing {
        if e.expiration_ts - now > RENEW_BEFORE_SECS {
            return Ok(false);
        }
    }
    let token = ctx.tokens.token(account_id).await?;
    let req = WatchRequest::web_hook(
        uuid::Uuid::new_v4().to_string(),
        format!("{}/gcal/webhook", base_url.trim_end_matches('/')),
        crate::auth::pkce::random_token(32),
        CHANNEL_TTL_SECS,
    );
    let created = match calendar_id {
        Some(cal) => ctx.client.watch_events(&token, cal, &req).await,
        None => ctx.client.watch_calendar_list(&token, &req).await,
    };
    let channel = match created {
        Ok(c) => c,
        Err(AppError::Google { status, reason })
            if is_webhook_rejection(&reason) || status == 400 =>
        {
            disable_push(ctx, format!("Google rejected the webhook address ({status} {reason}). See docs/09-setup-usuario.md section C.")).await;
            return Err(AppError::Google { status, reason });
        }
        Err(e) => return Err(e),
    };
    let expiration_ts = channel
        .expiration
        .map(|ms| ms / 1000)
        .unwrap_or(now + CHANNEL_TTL_SECS as i64);
    let row = channels::ChannelRow {
        id: channel.id.clone(),
        account_id: account_id.into(),
        calendar_id: calendar_id.map(str::to_string),
        resource_id: channel.resource_id.clone(),
        token: req.token.clone(),
        expiration_ts,
        created_at: now,
    };
    let (acc, cal) = (account_id.to_string(), calendar_id.map(str::to_string));
    let old_id = existing.as_ref().map(|e| e.id.clone());
    ctx.db
        .call(move |c| {
            if let Some(old) = old_id.as_ref() {
                channels::delete(c, old)?;
            }
            channels::insert(c, &row)?;
            settings::log_sync(
                c,
                Some(&acc),
                cal.as_deref(),
                "push",
                Some("channel created"),
            )
        })
        .await?;
    if let Some(old) = existing {
        if let Err(e) = ctx
            .client
            .channel_stop(&token, &old.id, &old.resource_id)
            .await
        {
            tracing::debug!(error = %e, "stopping the previous channel failed (it expires on its own)");
        }
    }
    tracing::info!(account = account_id, calendar = ?calendar_id, "push channel ready");
    Ok(true)
}

/// Ensure every channel of every Google account. No-op when push is off or unconfigured.
pub async fn ensure_channels(ctx: &SyncCtx) -> Result<usize, AppError> {
    let cfg = config(ctx).await?;
    let (true, Some(base_url)) = (cfg.enabled, cfg.base_url) else {
        return Ok(0);
    };
    let accounts = ctx.db.call(|c| accounts::list_accounts(c)).await?;
    let mut made = 0usize;
    for a in accounts
        .into_iter()
        .filter(|a| !a.is_local() && a.sync_state != "auth_required")
    {
        match ensure_one(ctx, &base_url, &a.id, None).await {
            Ok(true) => made += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(account = %a.id, error = %e, "calendarList channel failed");
                if !config(ctx).await?.enabled {
                    return Err(e);
                }
                continue;
            }
        }
        let id = a.id.clone();
        let cals = ctx
            .db
            .call(move |c| calendars::list_calendars_of_account(c, &id))
            .await?;
        for cal in cals.into_iter().filter(|c| c.full_sync_done && !c.deleted) {
            match ensure_one(ctx, &base_url, &a.id, Some(&cal.id)).await {
                Ok(true) => made += 1,
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(account = %a.id, calendar = %cal.id, error = %e, "events channel failed");
                    if !config(ctx).await?.enabled {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(made)
}

/// Stop and forget every channel of an account (docs/05 section 3.3).
pub async fn stop_account_channels(ctx: &SyncCtx, account_id: &str) -> Result<(), AppError> {
    let id = account_id.to_string();
    let list = ctx
        .db
        .call(move |c| channels::list_of_account(c, &id))
        .await?;
    if list.is_empty() {
        return Ok(());
    }
    let token = ctx.tokens.token(account_id).await.ok();
    for ch in list {
        if let Some(t) = &token {
            if let Err(e) = ctx.client.channel_stop(t, &ch.id, &ch.resource_id).await {
                tracing::debug!(error = %e, channel = %ch.id, "channels.stop failed");
            }
        }
        let cid = ch.id.clone();
        ctx.db.call(move |c| channels::delete(c, &cid)).await?;
    }
    Ok(())
}

/// Stop every channel (used when the user turns push off).
pub async fn stop_all_channels(ctx: &SyncCtx) -> Result<(), AppError> {
    let accounts = ctx.db.call(|c| accounts::list_accounts(c)).await?;
    for a in accounts.into_iter().filter(|a| !a.is_local()) {
        stop_account_channels(ctx, &a.id).await?;
    }
    Ok(())
}

/// `GET <base_url>/healthz` through the public address (docs/05 section 3.2). Enables push on
/// success, disables it on failure, and stores the outcome for the Settings screen.
pub async fn test_public_url(ctx: &SyncCtx, base_url: &str) -> PushTestResult {
    let base = base_url.trim_end_matches('/').to_string();
    let url = format!("{base}/healthz");
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let outcome = match http.get(&url).send().await {
        Ok(r) if r.status().is_success() => match r.text().await {
            Ok(t) if t.trim() == "ok" => Ok(()),
            Ok(t) => Err(format!(
                "{url} answered with unexpected content ({})",
                t.chars().take(40).collect::<String>()
            )),
            Err(e) => Err(format!("{url} body could not be read: {e}")),
        },
        Ok(r) => Err(format!("{url} answered {}", r.status())),
        Err(e) => Err(format!("{url} failed: {e}")),
    };
    let (ok, message) = match outcome {
        Ok(()) => (
            true,
            format!("{url} answered 200 ok. Push notifications are enabled."),
        ),
        Err(m) => (false, format!("{m}. The app keeps polling every 60 s.")),
    };
    let (b, msg, ok2) = (base.clone(), message.clone(), ok);
    let _ = ctx
        .db
        .call(move |c| {
            settings::set(c, "public_base_url", &b)?;
            settings::set(c, "push_enabled", &ok2)?;
            settings::set(c, "push_error", &if ok2 { String::new() } else { msg })?;
            Ok(())
        })
        .await;
    PushTestResult { ok, message }
}

/// Hourly renewal loop (docs/05 section 3.3).
pub async fn renew_loop(ctx: SyncCtx) {
    let mut tick = tokio::time::interval(RENEW_INTERVAL);
    loop {
        tick.tick().await;
        match ensure_channels(&ctx).await {
            Ok(n) if n > 0 => tracing::info!(created = n, "push channels renewed"),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "channel renewal failed"),
        }
    }
}
