//! Write-back: reconcile a target calendar with the projection its rule
//! produces. Idempotent: every run computes the desired set, diffs it against
//! what we recorded in `mirrored_events`, and issues only the needed PUTs and
//! DELETEs.

use std::collections::HashMap;

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ics::generate::{mirror_uid, render_mirror_object};
use crate::models::{MirroredEvent, SyncTarget, TargetKind};
use crate::routes::shares::{evaluate_rule, parse_filters, parse_visibility};
use crate::state::AppState;

use super::caldav::{self, Client};

#[derive(Debug, Default)]
pub struct PushStats {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
}

impl std::fmt::Display for PushStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} created, {} updated, {} deleted, {} unchanged", self.created, self.updated, self.deleted, self.unchanged)
    }
}

pub async fn push_target(state: &AppState, target: &SyncTarget) -> anyhow::Result<PushStats> {
    match target.kind {
        TargetKind::Caldav => push_caldav(state, target).await,
        TargetKind::Google | TargetKind::Microsoft => {
            anyhow::bail!("target {:?} is not implemented yet; see ROADMAP.md", target.kind)
        }
    }
}

async fn push_caldav(state: &AppState, target: &SyncTarget) -> anyhow::Result<PushStats> {
    let cfg: caldav::CalDavConfig = serde_json::from_value(target.config.clone())?;
    let client = Client::new(&state.http, &cfg);
    let host = url::Url::parse(&state.config.public_url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_else(|| "opentempus".into());

    // Desired state.
    let visibility = parse_visibility(&target.visibility);
    let filters = parse_filters(&target.filters);
    let now = Utc::now();
    let from = now - Duration::days(filters.horizon_past_days);
    let to = now + Duration::days(filters.horizon_future_days);
    let events = evaluate_rule(state, target.user_id, &visibility, &filters, from, to).await?;
    let mut desired: HashMap<Uuid, (String, String)> = HashMap::new();
    for ev in &events {
        let body = render_mirror_object(ev, &target.placeholder_title, &host);
        let hash = hex::encode(Sha256::digest(body.as_bytes()));
        desired.insert(ev.id, (body, hash));
    }

    // Recorded state.
    let recorded: Vec<MirroredEvent> =
        sqlx::query_as("SELECT instance_id, remote_href, content_hash FROM mirrored_events WHERE target_id = $1")
            .bind(target.id)
            .fetch_all(&state.db)
            .await?;
    let recorded: HashMap<Uuid, MirroredEvent> = recorded.into_iter().map(|m| (m.instance_id, m)).collect();

    let mut stats = PushStats::default();

    // Deletions first so a moved occurrence does not briefly double-book.
    for (id, m) in &recorded {
        if !desired.contains_key(id) {
            client.delete(&m.remote_href).await?;
            sqlx::query("DELETE FROM mirrored_events WHERE target_id = $1 AND instance_id = $2")
                .bind(target.id)
                .bind(id)
                .execute(&state.db)
                .await?;
            stats.deleted += 1;
        }
    }

    for (id, (body, hash)) in &desired {
        match recorded.get(id) {
            Some(m) if &m.content_hash == hash => {
                stats.unchanged += 1;
                continue;
            }
            existing => {
                let href = match existing {
                    Some(m) => m.remote_href.clone(),
                    None => client.event_href(&mirror_uid(*id, &host))?,
                };
                let etag = client.put_event(&href, body).await?;
                sqlx::query(
                    r#"INSERT INTO mirrored_events (target_id, instance_id, remote_href, etag, content_hash, updated_at)
                       VALUES ($1, $2, $3, $4, $5, now())
                       ON CONFLICT (target_id, instance_id) DO UPDATE
                         SET remote_href = EXCLUDED.remote_href, etag = EXCLUDED.etag,
                             content_hash = EXCLUDED.content_hash, updated_at = now()"#,
                )
                .bind(target.id)
                .bind(id)
                .bind(&href)
                .bind(etag)
                .bind(hash)
                .execute(&state.db)
                .await?;
                if existing.is_some() {
                    stats.updated += 1;
                } else {
                    stats.created += 1;
                }
            }
        }
    }
    Ok(stats)
}

/// Remove everything we ever wrote to a target (used when a target is
/// deleted). Best effort: remote failures are logged, not fatal.
pub async fn clear_target(state: &AppState, target: &SyncTarget) -> anyhow::Result<usize> {
    if target.kind != TargetKind::Caldav {
        return Ok(0);
    }
    let cfg: caldav::CalDavConfig = serde_json::from_value(target.config.clone())?;
    let client = Client::new(&state.http, &cfg);
    let recorded: Vec<MirroredEvent> =
        sqlx::query_as("SELECT instance_id, remote_href, content_hash FROM mirrored_events WHERE target_id = $1")
            .bind(target.id)
            .fetch_all(&state.db)
            .await?;
    let mut n = 0;
    for m in &recorded {
        match client.delete(&m.remote_href).await {
            Ok(()) => n += 1,
            Err(e) => tracing::warn!(target = %target.id, href = %m.remote_href, error = %e, "could not delete mirrored event"),
        }
    }
    Ok(n)
}
