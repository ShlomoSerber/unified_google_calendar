//! Writes against Google accounts. See docs/02-arquitectura.md section 3.3, docs/03 sections
//! 4 and 7, and docs/05 section 2.3. Every call waits for Google's response and upserts it.

use std::time::Duration;

use chrono::{DateTime, TimeZone};
use serde_json::Value;

use crate::commands::events::{draft_to_row, is_local, span_of, Touched};
use crate::commands::types::{CalendarKey, EditScope, EventDetail, EventDraft};
use crate::commands::view;
use crate::config::LOCAL_ICAL_SUFFIX;
use crate::db::queries::{calendars, events as q};
use crate::error::AppError;
use crate::google::body;
use crate::google::types::{Event, SendUpdates};
use crate::recurrence::edit_scope::{self, instance_id, OccurrenceRef};
use crate::recurrence::expand::{self, Window};
use crate::sync::map::event_to_row;
use crate::sync::SyncCtx;

const MEET_POLLS: usize = 3;

fn key(account_id: &str, calendar_id: &str) -> CalendarKey {
    CalendarKey {
        account_id: account_id.into(),
        calendar_id: calendar_id.into(),
    }
}

/// Store a Google response row and re-materialize it. Returns the stored row.
async fn store(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
    ev: &Event,
) -> Result<q::EventRow, AppError> {
    let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
    let tz = ctx
        .db
        .call({
            let (a, c) = (acc.clone(), cal.clone());
            move |conn| Ok(calendars::get_calendar(conn, &a, &c)?.and_then(|r| r.time_zone))
        })
        .await?;
    let row = event_to_row(account_id, calendar_id, ev, tz.as_deref());
    let stored = row.clone();
    ctx.db
        .call(move |c| {
            let w = Window::current(c)?;
            q::upsert_event(c, &row)?;
            expand::materialize_simple(c, &acc, &cal, &row.id, w)?;
            Ok(())
        })
        .await?;
    Ok(stored)
}

async fn window(ctx: &SyncCtx) -> Result<Window, AppError> {
    ctx.db.call(|c| Window::current(c)).await
}

async fn detail_near(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    near: Option<i64>,
) -> Result<EventDetail, AppError> {
    let (acc, cal, id) = (
        account_id.to_string(),
        calendar_id.to_string(),
        event_id.to_string(),
    );
    ctx.db
        .call(move |c| {
            let occ = q::occurrences_of_event(c, &acc, &cal, &id)?;
            let pick = match near {
                Some(t) => occ.iter().min_by_key(|o| (o.start_ts - t).abs()),
                None => occ.first(),
            };
            let occ_id = pick.map(|o| o.id.clone()).ok_or_else(|| {
                AppError::Invalid("The event was saved but is outside the data window".into())
            })?;
            view::get_event(c, &occ_id)
        })
        .await
}

/// Poll `events.get` while the Meet request is still `pending` (docs/05 section 2.3).
async fn settle_meet(ctx: &SyncCtx, token: &str, calendar_id: &str, ev: Event) -> Event {
    let mut ev = ev;
    for _ in 0..MEET_POLLS {
        let pending = ev
            .conference_data
            .as_ref()
            .and_then(|c| c.get("createRequest"))
            .and_then(|r| r.get("status"))
            .and_then(|s| s.get("statusCode"))
            .and_then(Value::as_str)
            == Some("pending");
        if !pending {
            break;
        }
        tokio::time::sleep(meet_poll_delay(ctx)).await;
        match ev.id.as_deref() {
            Some(id) => match ctx.client.event_get(token, calendar_id, id).await {
                Ok(fresh) => ev = fresh,
                Err(e) => {
                    tracing::warn!(error = %e, "events.get while waiting for Meet failed");
                    break;
                }
            },
            None => break,
        }
    }
    ev
}

fn meet_poll_delay(ctx: &SyncCtx) -> Duration {
    // 2 s in production; the test client uses a millisecond unit.
    ctx.client.backoff_unit * 2
}

async fn ensure_writable(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
) -> Result<calendars::CalendarRow, AppError> {
    let (acc, cal) = (account_id.to_string(), calendar_id.to_string());
    let row = ctx
        .db
        .call(move |c| calendars::get_calendar(c, &acc, &cal))
        .await?
        .ok_or_else(|| AppError::NotFound("The calendar".into()))?;
    if !row.can_write() {
        return Err(AppError::invalid(
            "You do not have permission to write to this calendar",
        ));
    }
    Ok(row)
}

fn has_attendees(ev: &Event) -> bool {
    ev.attendees.as_ref().is_some_and(|a| !a.is_empty())
}

/// Create an event in a Google calendar.
pub async fn create(ctx: &SyncCtx, draft: &EventDraft) -> Result<(EventDetail, Touched), AppError> {
    draft_to_row(draft, None)?; // validation only
    ensure_writable(ctx, &draft.account_id, &draft.calendar_id).await?;
    let token = ctx.tokens.token(&draft.account_id).await?;
    let body = body::from_draft(draft, None, None)?;
    // The creator of a new event is its organizer.
    let send = body::send_updates_for(has_attendees(&body), true);
    let ev = ctx
        .client
        .event_insert(&token, &draft.calendar_id, &body, send)
        .await?;
    let ev = if draft.add_meet {
        settle_meet(ctx, &token, &draft.calendar_id, ev).await
    } else {
        ev
    };
    let row = store(ctx, &draft.account_id, &draft.calendar_id, &ev).await?;
    let detail = detail_near(ctx, &draft.account_id, &draft.calendar_id, &row.id, None).await?;
    let touched = if row.is_recurring_master() {
        let w = window(ctx).await?;
        Touched {
            from: w.from_ts,
            to: w.to_ts,
            calendars: vec![key(&draft.account_id, &draft.calendar_id)],
        }
    } else {
        Touched {
            from: detail.start,
            to: detail.end,
            calendars: vec![key(&draft.account_id, &draft.calendar_id)],
        }
    };
    Ok((detail, touched))
}

struct Target {
    occ: q::OccurrenceRow,
    row: q::EventRow,
    master: Option<q::EventRow>,
}

async fn load_target(ctx: &SyncCtx, occurrence_id: &str) -> Result<Target, AppError> {
    let id = occurrence_id.to_string();
    ctx.db
        .call(move |c| {
            let occ =
                q::get_occurrence(c, &id)?.ok_or_else(|| AppError::NotFound("The event".into()))?;
            let row = q::get_event(c, &occ.account_id, &occ.calendar_id, &occ.event_id)?
                .ok_or_else(|| AppError::NotFound("The event".into()))?;
            let master_id = occ
                .master_id
                .clone()
                .or_else(|| row.recurring_event_id.clone());
            let master = match master_id {
                Some(m) if m != row.id => q::get_event(c, &occ.account_id, &occ.calendar_id, &m)?,
                Some(_) => Some(row.clone()),
                None => None,
            };
            Ok(Target { occ, row, master })
        })
        .await
}

/// Google instance id of the occurrence; falls back to `events.instances?originalStart` when
/// the constructed id is unknown (docs/03 section 4).
async fn resolve_instance_id(
    ctx: &SyncCtx,
    token: &str,
    calendar_id: &str,
    master: &q::EventRow,
    occ_start: i64,
) -> Result<String, AppError> {
    let constructed = instance_id(&master.id, occ_start, master.all_day);
    match ctx.client.event_get(token, calendar_id, &constructed).await {
        Ok(_) => Ok(constructed),
        Err(AppError::Google { status: 404, .. }) => {
            let original = if master.all_day {
                DateTime::from_timestamp(occ_start, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d")
                    .to_string()
            } else {
                let tz: chrono_tz::Tz = master
                    .time_zone
                    .as_deref()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(chrono_tz::UTC);
                tz.timestamp_opt(occ_start, 0)
                    .single()
                    .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
                    .unwrap_or_default()
            };
            let page = ctx
                .client
                .event_instances(token, calendar_id, &master.id, Some(&original))
                .await?;
            let id = page
                .items
                .into_iter()
                .find_map(|e| e.id)
                .ok_or_else(|| AppError::NotFound("The occurrence".into()))?;
            tracing::info!(constructed, resolved = %id, "instance id resolved through events.instances");
            Ok(id)
        }
        Err(e) => Err(e),
    }
}

async fn delete_exceptions_from(
    ctx: &SyncCtx,
    account_id: &str,
    calendar_id: &str,
    master_id: &str,
    from_ts: i64,
) -> Result<(), AppError> {
    let (acc, cal, m) = (
        account_id.to_string(),
        calendar_id.to_string(),
        master_id.to_string(),
    );
    ctx.db
        .call(move |c| {
            for exc in q::exceptions_of(c, &acc, &cal, &m)? {
                let orig = exc.original_start_ts.or_else(|| {
                    exc.original_start_date
                        .as_deref()
                        .and_then(|d| expand::parse_date(d).ok())
                        .map(expand::date_to_ts)
                });
                if orig.is_some_and(|o| o >= from_ts) {
                    q::delete_event(c, &acc, &cal, &exc.id)?;
                }
            }
            Ok(())
        })
        .await
}

/// Update with scope (docs/03 section 4, column "Google").
pub async fn update(
    ctx: &SyncCtx,
    occurrence_id: &str,
    draft: &EventDraft,
    scope: EditScope,
) -> Result<(EventDetail, Touched), AppError> {
    draft_to_row(draft, None)?;
    let t = load_target(ctx, occurrence_id).await?;
    let (account_id, calendar_id) = (t.occ.account_id.clone(), t.occ.calendar_id.clone());
    ensure_writable(ctx, &account_id, &calendar_id).await?;
    let token = ctx.tokens.token(&account_id).await?;
    let w = window(ctx).await?;
    let whole = Touched {
        from: w.from_ts,
        to: w.to_ts,
        calendars: vec![key(&account_id, &calendar_id)],
    };
    let Some(master) = t.master.clone() else {
        // Single event.
        let raw = t.row.raw.clone().unwrap_or_default();
        let body = body::from_draft(draft, Some(&raw), None)?;
        let send = body::send_updates_for(has_attendees(&body), t.row.organizer_self);
        let ev = ctx
            .client
            .event_update(&token, &calendar_id, &t.row.id, &body, send)
            .await?;
        let ev = if draft.add_meet {
            settle_meet(ctx, &token, &calendar_id, ev).await
        } else {
            ev
        };
        let row = store(ctx, &account_id, &calendar_id, &ev).await?;
        let detail = detail_near(ctx, &account_id, &calendar_id, &row.id, row.start_ts).await?;
        let touched = if row.is_recurring_master() {
            whole
        } else {
            Touched {
                from: t.occ.start_ts.min(detail.start),
                to: t.occ.end_ts.max(detail.end),
                calendars: vec![key(&account_id, &calendar_id)],
            }
        };
        return Ok((detail, touched));
    };
    match scope {
        EditScope::This => {
            let id = if t.row.is_exception() {
                t.row.id.clone()
            } else {
                resolve_instance_id(ctx, &token, &calendar_id, &master, t.occ.start_ts).await?
            };
            let base_raw = if t.row.is_exception() {
                t.row.raw.clone()
            } else {
                master.raw.clone()
            }
            .unwrap_or_default();
            let mut body = body::from_draft(draft, Some(&base_raw), Some(&[]))?;
            body.id = Some(id.clone());
            body.recurrence = None;
            let send = body::send_updates_for(has_attendees(&body), master.organizer_self);
            let ev = ctx
                .client
                .event_update(&token, &calendar_id, &id, &body, send)
                .await?;
            let ev = if draft.add_meet {
                settle_meet(ctx, &token, &calendar_id, ev).await
            } else {
                ev
            };
            let row = store(ctx, &account_id, &calendar_id, &ev).await?;
            let detail = detail_near(ctx, &account_id, &calendar_id, &row.id, row.start_ts).await?;
            Ok((detail, whole))
        }
        EditScope::Following => {
            let first_start = span_of(&master).map(|(s, _)| s).unwrap_or(0);
            if t.occ.start_ts <= first_start {
                return Box::pin(update(ctx, occurrence_id, draft, EditScope::All)).await;
            }
            let lines = master.recurrence.clone().unwrap_or_default();
            let consumed = edit_scope::instances_before(&master, t.occ.start_ts)?;
            let truncated = edit_scope::truncate_recurrence(&lines, t.occ.start_ts, master.all_day);
            let master_body =
                body::with_recurrence(&master.raw.clone().unwrap_or_default(), &truncated)?;
            let send = body::send_updates_for(has_attendees(&master_body), master.organizer_self);
            let updated_master = ctx
                .client
                .event_update(&token, &calendar_id, &master.id, &master_body, send)
                .await?;
            store(ctx, &account_id, &calendar_id, &updated_master).await?;
            delete_exceptions_from(ctx, &account_id, &calendar_id, &master.id, t.occ.start_ts)
                .await?;
            let remaining = if draft.recurrence.is_empty() {
                edit_scope::remaining_recurrence(&lines, consumed)
            } else {
                draft.recurrence.clone()
            };
            let new_body = body::from_draft(
                draft,
                Some(&master.raw.clone().unwrap_or_default()),
                Some(&remaining),
            )?;
            let mut new_body = new_body;
            new_body.id = None;
            new_body.recurring_event_id = None;
            new_body.original_start_time = None;
            let send_new = body::send_updates_for(has_attendees(&new_body), true);
            let created = ctx
                .client
                .event_insert(&token, &calendar_id, &new_body, send_new)
                .await?;
            let created = if draft.add_meet {
                settle_meet(ctx, &token, &calendar_id, created).await
            } else {
                created
            };
            let row = store(ctx, &account_id, &calendar_id, &created).await?;
            let (acc, cal, m) = (account_id.clone(), calendar_id.clone(), master.id.clone());
            ctx.db
                .call(move |c| expand::expand_master(c, &acc, &cal, &m, w).map(|_| ()))
                .await?;
            let detail = detail_near(ctx, &account_id, &calendar_id, &row.id, row.start_ts).await?;
            Ok((detail, whole))
        }
        EditScope::All => {
            let raw = master.raw.clone().unwrap_or_default();
            let (ds, de) = if draft.all_day {
                let s = expand::parse_date(draft.start_date.as_deref().unwrap_or_default())?;
                let e = expand::parse_date(draft.end_date.as_deref().unwrap_or_default())?;
                (expand::date_to_ts(s), expand::date_to_ts(e))
            } else {
                (draft.start.unwrap_or(0), draft.end.unwrap_or(0))
            };
            let delta = ds - t.occ.start_ts;
            let shifted = if draft.all_day == master.all_day {
                body::shifted(&raw, delta, de - ds, master.all_day)?
            } else {
                serde_json::from_str::<Value>(&raw)?
                    .as_object()
                    .cloned()
                    .unwrap_or_default()
            };
            let shifted_raw = serde_json::to_string(&Value::Object(shifted))?;
            let mut body = body::from_draft(draft, Some(&shifted_raw), Some(&draft.recurrence))?;
            // Keep the series start computed above, not the instance's.
            if draft.all_day == master.all_day {
                let v: Value = serde_json::from_str(&shifted_raw)?;
                body.start = serde_json::from_value(v["start"].clone()).ok();
                body.end = serde_json::from_value(v["end"].clone()).ok();
            }
            if draft.recurrence.is_empty() {
                body.recurrence = master.recurrence.clone();
            }
            body.id = Some(master.id.clone());
            let send = body::send_updates_for(has_attendees(&body), master.organizer_self);
            let ev = ctx
                .client
                .event_update(&token, &calendar_id, &master.id, &body, send)
                .await?;
            let ev = if draft.add_meet {
                settle_meet(ctx, &token, &calendar_id, ev).await
            } else {
                ev
            };
            if delta != 0 || draft.all_day != master.all_day {
                delete_exceptions_from(ctx, &account_id, &calendar_id, &master.id, 0).await?;
            }
            let row = store(ctx, &account_id, &calendar_id, &ev).await?;
            let detail = detail_near(ctx, &account_id, &calendar_id, &row.id, Some(ds)).await?;
            Ok((detail, whole))
        }
    }
}

/// Delete with scope.
pub async fn delete(
    ctx: &SyncCtx,
    occurrence_id: &str,
    scope: EditScope,
) -> Result<Touched, AppError> {
    let t = load_target(ctx, occurrence_id).await?;
    let (account_id, calendar_id) = (t.occ.account_id.clone(), t.occ.calendar_id.clone());
    ensure_writable(ctx, &account_id, &calendar_id).await?;
    let token = ctx.tokens.token(&account_id).await?;
    let w = window(ctx).await?;
    let whole = Touched {
        from: w.from_ts,
        to: w.to_ts,
        calendars: vec![key(&account_id, &calendar_id)],
    };
    let occ_ref = OccurrenceRef {
        account_id: account_id.clone(),
        calendar_id: calendar_id.clone(),
        event_id: t.occ.event_id.clone(),
        master_id: t.occ.master_id.clone(),
        start_ts: t.occ.start_ts,
    };
    let send_for_row =
        |row: &q::EventRow| body::send_updates_for(row.attendees != "[]", row.organizer_self);
    let Some(master) = t.master.clone() else {
        ctx.client
            .event_delete(&token, &calendar_id, &t.row.id, send_for_row(&t.row))
            .await?;
        let (acc, cal, id) = (account_id.clone(), calendar_id.clone(), t.row.id.clone());
        ctx.db
            .call(move |c| q::delete_event(c, &acc, &cal, &id).map(|_| ()))
            .await?;
        return Ok(Touched {
            from: t.occ.start_ts,
            to: t.occ.end_ts,
            calendars: vec![key(&account_id, &calendar_id)],
        });
    };
    match scope {
        EditScope::This => {
            let id = if t.row.is_exception() {
                t.row.id.clone()
            } else {
                resolve_instance_id(ctx, &token, &calendar_id, &master, t.occ.start_ts).await?
            };
            ctx.client
                .event_delete(&token, &calendar_id, &id, send_for_row(&master))
                .await?;
            ctx.db
                .call(move |c| edit_scope::apply_delete(c, EditScope::This, &occ_ref, w))
                .await?;
            Ok(Touched {
                from: t.occ.start_ts,
                to: t.occ.end_ts,
                calendars: vec![key(&account_id, &calendar_id)],
            })
        }
        EditScope::Following => {
            let first_start = span_of(&master).map(|(s, _)| s).unwrap_or(0);
            if t.occ.start_ts <= first_start {
                return Box::pin(delete(ctx, occurrence_id, EditScope::All)).await;
            }
            let lines = master.recurrence.clone().unwrap_or_default();
            let truncated = edit_scope::truncate_recurrence(&lines, t.occ.start_ts, master.all_day);
            let master_body =
                body::with_recurrence(&master.raw.clone().unwrap_or_default(), &truncated)?;
            let updated = ctx
                .client
                .event_update(
                    &token,
                    &calendar_id,
                    &master.id,
                    &master_body,
                    send_for_row(&master),
                )
                .await?;
            delete_exceptions_from(ctx, &account_id, &calendar_id, &master.id, t.occ.start_ts)
                .await?;
            store(ctx, &account_id, &calendar_id, &updated).await?;
            Ok(whole)
        }
        EditScope::All => {
            ctx.client
                .event_delete(&token, &calendar_id, &master.id, send_for_row(&master))
                .await?;
            let (acc, cal, id) = (account_id.clone(), calendar_id.clone(), master.id.clone());
            ctx.db
                .call(move |c| q::delete_event(c, &acc, &cal, &id).map(|_| ()))
                .await?;
            Ok(whole)
        }
    }
}

/// RSVP: `patch` with the full `attendees` array (docs/05 section 2.3).
pub async fn rsvp(
    ctx: &SyncCtx,
    occurrence_id: &str,
    status: &str,
    send_updates: bool,
) -> Result<(EventDetail, Touched), AppError> {
    if !matches!(
        status,
        "accepted" | "declined" | "tentative" | "needsAction"
    ) {
        return Err(AppError::invalid(
            "The response must be accepted, declined or tentative",
        ));
    }
    let t = load_target(ctx, occurrence_id).await?;
    let (account_id, calendar_id) = (t.occ.account_id.clone(), t.occ.calendar_id.clone());
    if is_local(&account_id) {
        return Err(AppError::invalid(
            "Local events have no guests to respond to",
        ));
    }
    let token = ctx.tokens.token(&account_id).await?;
    // Responding on the instance shown, or on the master when the occurrence has no exception.
    let (target_id, raw) = if t.row.is_exception() || t.master.is_none() {
        (t.row.id.clone(), t.row.raw.clone().unwrap_or_default())
    } else {
        let m = t.master.clone().unwrap_or_else(|| t.row.clone());
        (m.id.clone(), m.raw.clone().unwrap_or_default())
    };
    let attendees = body::rsvp_attendees(&raw, status)?;
    let send = if send_updates {
        SendUpdates::All
    } else {
        SendUpdates::None
    };
    let ev = ctx
        .client
        .event_patch(
            &token,
            &calendar_id,
            &target_id,
            &serde_json::json!({ "attendees": attendees }),
            send,
        )
        .await?;
    let row = store(ctx, &account_id, &calendar_id, &ev).await?;
    let detail = detail_near(
        ctx,
        &account_id,
        &calendar_id,
        &row.id,
        Some(t.occ.start_ts),
    )
    .await?;
    Ok((
        detail,
        Touched {
            from: t.occ.start_ts,
            to: t.occ.end_ts,
            calendars: vec![key(&account_id, &calendar_id)],
        },
    ))
}

/// Move between calendars and accounts: the four paths of docs/03 section 7.
pub async fn move_event(
    ctx: &SyncCtx,
    occurrence_id: &str,
    target_account_id: &str,
    target_calendar_id: &str,
) -> Result<(EventDetail, Touched), AppError> {
    let t = load_target(ctx, occurrence_id).await?;
    let (src_account, src_calendar) = (t.occ.account_id.clone(), t.occ.calendar_id.clone());
    let master = t.master.clone().unwrap_or_else(|| t.row.clone());
    if master.event_type != "default" {
        return Err(AppError::invalid("Only regular events can be moved"));
    }
    let w = window(ctx).await?;
    let touched = Touched {
        from: w.from_ts,
        to: w.to_ts,
        calendars: vec![
            key(&src_account, &src_calendar),
            key(target_account_id, target_calendar_id),
        ],
    };
    ensure_writable(ctx, target_account_id, target_calendar_id).await?;

    let (a1, a2) = (src_account.clone(), target_account_id.to_string());
    let kinds = ctx
        .db
        .call(move |c| {
            let s = crate::db::queries::accounts::get_account(c, &a1)?.map(|a| a.kind);
            let d = crate::db::queries::accounts::get_account(c, &a2)?.map(|a| a.kind);
            Ok((s, d))
        })
        .await?;
    if kinds.0.as_deref() == Some("ical") || kinds.1.as_deref() == Some("ical") {
        return Err(AppError::invalid(
            "Events of an iCal subscription cannot be moved; it is read-only",
        ));
    }
    let src_local = is_local(&src_account);
    let dst_local = is_local(target_account_id);
    if src_local && dst_local {
        let (occ_id, ta, tc) = (
            occurrence_id.to_string(),
            target_account_id.to_string(),
            target_calendar_id.to_string(),
        );
        return ctx
            .db
            .call(move |c| crate::commands::events::move_local_to_local(c, &occ_id, &ta, &tc, w))
            .await;
    }
    if src_local {
        // Local → Google: insert with the local iCalUID, then drop the local row.
        let token = ctx.tokens.token(target_account_id).await?;
        let raw = local_row_as_google_json(&master)?;
        let mut body = body::for_import(
            &raw,
            master
                .ical_uid
                .as_deref()
                .unwrap_or(&format!("{}{}", master.id, LOCAL_ICAL_SUFFIX)),
        )?;
        body.attendees = None;
        let created = ctx
            .client
            .event_insert(&token, target_calendar_id, &body, SendUpdates::None)
            .await?;
        let row = store(ctx, target_account_id, target_calendar_id, &created).await?;
        let (acc, cal, id) = (src_account.clone(), src_calendar.clone(), master.id.clone());
        ctx.db
            .call(move |c| q::delete_event(c, &acc, &cal, &id).map(|_| ()))
            .await?;
        let detail = detail_near(
            ctx,
            target_account_id,
            target_calendar_id,
            &row.id,
            Some(t.occ.start_ts),
        )
        .await?;
        return Ok((detail, touched));
    }
    let src_token = ctx.tokens.token(&src_account).await?;
    if dst_local {
        // Google → local: copy fields, new iCalUID, delete on Google without notifications.
        let mut local = master.clone();
        local.account_id = target_account_id.into();
        local.calendar_id = target_calendar_id.into();
        local.id = uuid::Uuid::new_v4().to_string();
        local.ical_uid = Some(format!("{}{}", local.id, LOCAL_ICAL_SUFFIX));
        local.etag = None;
        local.attendees = "[]".into();
        local.organizer_email = None;
        local.organizer_self = true;
        local.hangout_link = None;
        local.conference = None;
        local.html_link = None;
        local.raw = None;
        local.recurring_event_id = None;
        local.original_start_ts = None;
        local.original_start_date = None;
        ctx.client
            .event_delete(&src_token, &src_calendar, &master.id, SendUpdates::None)
            .await?;
        let (acc, cal, id, new_row) = (
            src_account.clone(),
            src_calendar.clone(),
            master.id.clone(),
            local.clone(),
        );
        ctx.db
            .call(move |c| {
                q::delete_event(c, &acc, &cal, &id)?;
                q::upsert_event(c, &new_row)?;
                expand::materialize_simple(
                    c,
                    &new_row.account_id,
                    &new_row.calendar_id,
                    &new_row.id,
                    w,
                )
            })
            .await?;
        let detail = detail_near(
            ctx,
            target_account_id,
            target_calendar_id,
            &local.id,
            Some(t.occ.start_ts),
        )
        .await?;
        return Ok((detail, touched));
    }
    if src_account == target_account_id {
        // Same account: events.move.
        let moved = ctx
            .client
            .event_move(&src_token, &src_calendar, &master.id, target_calendar_id)
            .await?;
        let (acc, cal, id) = (src_account.clone(), src_calendar.clone(), master.id.clone());
        ctx.db
            .call(move |c| q::delete_event(c, &acc, &cal, &id).map(|_| ()))
            .await?;
        let row = store(ctx, target_account_id, target_calendar_id, &moved).await?;
        let detail = detail_near(
            ctx,
            target_account_id,
            target_calendar_id,
            &row.id,
            Some(t.occ.start_ts),
        )
        .await?;
        return Ok((detail, touched));
    }
    // Different accounts: import in the target, then delete in the source.
    let dst_token = ctx.tokens.token(target_account_id).await?;
    let raw = master.raw.clone().unwrap_or_default();
    let ical = master
        .ical_uid
        .clone()
        .unwrap_or_else(|| format!("{}@google.com", master.id));
    let body = body::for_import(&raw, &ical)?;
    let imported = ctx
        .client
        .event_import(&dst_token, target_calendar_id, &body)
        .await?;
    let row = store(ctx, target_account_id, target_calendar_id, &imported).await?;
    match ctx
        .client
        .event_delete(&src_token, &src_calendar, &master.id, SendUpdates::None)
        .await
    {
        Ok(()) => {
            let (acc, cal, id) = (src_account.clone(), src_calendar.clone(), master.id.clone());
            ctx.db
                .call(move |c| q::delete_event(c, &acc, &cal, &id).map(|_| ()))
                .await?;
        }
        Err(e) => {
            tracing::warn!(error = %e, "import succeeded but delete in the source failed; keeping both copies");
            return Err(AppError::Invalid(format!(
                "The event was copied to the other account, but it could not be deleted from the original calendar ({}). Delete it there by hand",
                e.user_message()
            )));
        }
    }
    let detail = detail_near(
        ctx,
        target_account_id,
        target_calendar_id,
        &row.id,
        Some(t.occ.start_ts),
    )
    .await?;
    Ok((detail, touched))
}

/// Google-shaped JSON for a local row (used when moving local → Google).
fn local_row_as_google_json(row: &q::EventRow) -> Result<String, AppError> {
    let mut obj = serde_json::Map::new();
    if let Some(s) = &row.summary {
        obj.insert("summary".into(), Value::String(s.clone()));
    }
    if let Some(d) = &row.description {
        obj.insert("description".into(), Value::String(d.clone()));
    }
    if let Some(l) = &row.location {
        obj.insert("location".into(), Value::String(l.clone()));
    }
    if let Some(c) = &row.color_id {
        obj.insert("colorId".into(), Value::String(c.clone()));
    }
    if let Some(t) = &row.transparency {
        obj.insert("transparency".into(), Value::String(t.clone()));
    }
    if row.all_day {
        obj.insert(
            "start".into(),
            serde_json::json!({ "date": row.start_date }),
        );
        obj.insert("end".into(), serde_json::json!({ "date": row.end_date }));
    } else {
        let tz = row
            .time_zone
            .clone()
            .unwrap_or_else(|| crate::config::DEFAULT_PRIMARY_TZ.into());
        obj.insert("start".into(), serde_json::json!({ "dateTime": rfc3339(row.start_ts.unwrap_or(0), &tz)?, "timeZone": tz }));
        obj.insert("end".into(), serde_json::json!({ "dateTime": rfc3339(row.end_ts.unwrap_or(0), &tz)?, "timeZone": tz }));
    }
    if let Some(r) = &row.recurrence {
        obj.insert("recurrence".into(), serde_json::json!(r));
    }
    obj.insert(
        "reminders".into(),
        serde_json::from_str(&row.reminders).unwrap_or(serde_json::json!({ "useDefault": true })),
    );
    Ok(Value::Object(obj).to_string())
}

fn rfc3339(ts: i64, tz: &str) -> Result<String, AppError> {
    let zone: chrono_tz::Tz = tz
        .parse()
        .map_err(|_| AppError::invalid(format!("{tz} is not a valid time zone name")))?;
    Ok(zone
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
        .unwrap_or_default())
}
