//! Decide when sources are synced and run the sync.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::ics::parse::{parse_ics, ParseOptions};
use crate::models::CalendarSource;
use crate::state::AppState;

use super::connector::{connector_for, FetchResult};
use super::store::store_calendar;

/// A sync that has been "running" longer than this is considered abandoned
/// (for example the process crashed) and is claimed again.
const STALE_RUN_SECS: i64 = 15 * 60;

/// Background loop: claim due sources and sync them with bounded concurrency.
pub async fn run_scheduler(state: AppState) {
    let semaphore = Arc::new(Semaphore::new(state.config.sync_concurrency.max(1)));
    let tick = Duration::from_secs(state.config.scheduler_tick_secs.max(5));
    tracing::info!(tick_secs = tick.as_secs(), concurrency = state.config.sync_concurrency, "scheduler started");
    loop {
        match claim_due_sources(&state, state.config.sync_concurrency.max(1) as i64).await {
            Ok(sources) => {
                for source in sources {
                    let permit = semaphore.clone().acquire_owned().await.expect("semaphore closed");
                    let state = state.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        run_sync(&state, source).await;
                    });
                }
            }
            Err(e) => tracing::error!(error = %e, "failed to claim due sources"),
        }
        tokio::time::sleep(tick).await;
    }
}

async fn claim_due_sources(state: &AppState, limit: i64) -> Result<Vec<CalendarSource>, sqlx::Error> {
    let mut tx = state.db.begin().await?;
    let rows: Vec<CalendarSource> = sqlx::query_as(
        r#"SELECT * FROM calendar_sources
           WHERE enabled
             AND next_sync_at <= now()
             AND (sync_started_at IS NULL OR sync_started_at < now() - make_interval(secs => $2))
           ORDER BY next_sync_at
           LIMIT $1
           FOR UPDATE SKIP LOCKED"#,
    )
    .bind(limit)
    .bind(STALE_RUN_SECS as f64)
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    if !ids.is_empty() {
        sqlx::query("UPDATE calendar_sources SET sync_started_at = now(), last_sync_status = 'running' WHERE id = ANY($1)")
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(rows)
}

/// Request an immediate sync of one source (used by the API). The sync runs
/// on the scheduler's next tick, or right away when `inline` is set.
pub async fn sync_source_now(state: &AppState, source_id: Uuid, inline: bool) -> Result<Option<String>, sqlx::Error> {
    if !inline {
        sqlx::query("UPDATE calendar_sources SET next_sync_at = now() WHERE id = $1").bind(source_id).execute(&state.db).await?;
        return Ok(None);
    }
    let source: Option<CalendarSource> =
        sqlx::query_as("SELECT * FROM calendar_sources WHERE id = $1").bind(source_id).fetch_optional(&state.db).await?;
    let Some(source) = source else { return Ok(None) };
    sqlx::query("UPDATE calendar_sources SET sync_started_at = now(), last_sync_status = 'running' WHERE id = $1")
        .bind(source_id)
        .execute(&state.db)
        .await?;
    Ok(run_sync(state, source).await)
}

/// Sync one source end to end and record the outcome. Returns the error
/// message if the sync failed.
pub async fn run_sync(state: &AppState, source: CalendarSource) -> Option<String> {
    let started = Utc::now();
    let result = sync_inner(state, &source).await;
    let interval = source.sync_interval_secs.max(60) as f64;
    match result {
        Ok(summary) => {
            tracing::info!(source = %source.id, name = %source.name, elapsed_ms = (Utc::now() - started).num_milliseconds(), %summary, "sync ok");
            let _ = sqlx::query(
                r#"UPDATE calendar_sources
                   SET last_synced_at = now(), last_sync_status = 'ok', last_sync_error = NULL,
                       sync_started_at = NULL, next_sync_at = now() + make_interval(secs => $2)
                   WHERE id = $1"#,
            )
            .bind(source.id)
            .bind(interval)
            .execute(&state.db)
            .await;
            None
        }
        Err(e) => {
            let msg = format!("{e:#}");
            tracing::warn!(source = %source.id, name = %source.name, error = %msg, "sync failed");
            // Back off: retry after the normal interval, but at least 5 minutes.
            let _ = sqlx::query(
                r#"UPDATE calendar_sources
                   SET last_sync_status = 'error', last_sync_error = $2,
                       sync_started_at = NULL, next_sync_at = now() + make_interval(secs => $3)
                   WHERE id = $1"#,
            )
            .bind(source.id)
            .bind(&msg)
            .bind(interval.max(300.0))
            .execute(&state.db)
            .await;
            Some(msg)
        }
    }
}

async fn sync_inner(state: &AppState, source: &CalendarSource) -> anyhow::Result<String> {
    let connector = connector_for(source.kind)?;
    let fetched = match connector.fetch(state, source).await? {
        FetchResult::NotModified => return Ok("not modified".into()),
        FetchResult::Changed(f) => f,
    };

    let owner: Option<(String, String)> = sqlx::query_as("SELECT email::text, timezone FROM users WHERE id = $1")
        .bind(source.user_id)
        .fetch_optional(&state.db)
        .await?;
    let (email, tz_name) = owner.ok_or_else(|| anyhow::anyhow!("owner not found"))?;
    let mut owner_emails: Vec<String> = vec![email.to_ascii_lowercase()];
    if let Some(extra) = source.config.get("owner_emails").and_then(|v| v.as_array()) {
        owner_emails.extend(extra.iter().filter_map(|v| v.as_str()).map(|s| s.to_ascii_lowercase()));
    }
    let default_tz = tz_name.parse().unwrap_or(chrono_tz::UTC);

    let report = parse_ics(&fetched.ics, &ParseOptions { owner_emails, default_tz })?;
    let stats = store_calendar(&state.db, source, report.events).await?;

    if let Some(etag) = fetched.etag {
        sqlx::query("UPDATE calendar_sources SET etag = $2 WHERE id = $1").bind(source.id).bind(etag).execute(&state.db).await?;
    }
    for w in report.warnings.iter().chain(stats.warnings.iter()) {
        tracing::debug!(source = %source.id, warning = %w, "sync warning");
    }
    Ok(format!("{} events, {} occurrences, {} removed, {} skipped", stats.events, stats.instances, stats.removed, report.skipped))
}
