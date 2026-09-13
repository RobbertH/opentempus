//! Outbound sync targets: mirror a rule's projection into an external
//! calendar.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{SyncTarget, TargetKind},
    sharing::{Filters, SharedEvent, Visibility},
    state::AppState,
    sync::caldav,
};

use super::{
    shares::{evaluate_rule, parse_filters, parse_visibility, resolve_visibility, validate_filters},
    WindowQuery,
};

#[derive(Serialize)]
pub struct TargetView {
    pub id: Uuid,
    pub name: String,
    pub kind: TargetKind,
    pub config: serde_json::Value,
    pub visibility: Visibility,
    pub filters: Filters,
    pub placeholder_title: String,
    pub enabled: bool,
    pub push_interval_secs: i32,
    pub last_pushed_at: Option<DateTime<Utc>>,
    pub last_push_status: crate::models::SyncStatus,
    pub last_push_error: Option<String>,
    pub mirrored_count: i64,
    pub created_at: DateTime<Utc>,
}

fn mask(mut config: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = config.as_object_mut() {
        if obj.contains_key("password") {
            obj.insert("password".into(), json!("••••••"));
        }
    }
    config
}

async fn view(state: &AppState, t: SyncTarget) -> AppResult<TargetView> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM mirrored_events WHERE target_id = $1").bind(t.id).fetch_one(&state.db).await?;
    Ok(TargetView {
        id: t.id,
        name: t.name,
        kind: t.kind,
        config: mask(t.config),
        visibility: parse_visibility(&t.visibility),
        filters: parse_filters(&t.filters),
        placeholder_title: t.placeholder_title,
        enabled: t.enabled,
        push_interval_secs: t.push_interval_secs,
        last_pushed_at: t.last_pushed_at,
        last_push_status: t.last_push_status,
        last_push_error: t.last_push_error,
        mirrored_count: count,
        created_at: t.created_at,
    })
}

async fn load_owned(state: &AppState, user: &AuthUser, id: Uuid) -> AppResult<SyncTarget> {
    sqlx::query_as::<_, SyncTarget>("SELECT * FROM sync_targets WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("target not found"))
}

fn validate_target_config(kind: TargetKind, config: &serde_json::Value) -> AppResult<serde_json::Value> {
    match kind {
        TargetKind::Caldav => {
            let cfg = caldav::validate_config(config).map_err(|e| AppError::bad_request(e.to_string()))?;
            if cfg.calendar_url.is_none() {
                return Err(AppError::bad_request("pick a calendar first (calendar_url is missing)"));
            }
            Ok(serde_json::to_value(cfg).unwrap())
        }
        TargetKind::Google | TargetKind::Microsoft => {
            Err(AppError::bad_request(format!("{kind:?} targets are not implemented yet; see ROADMAP.md")))
        }
    }
}

/// Keep the stored password when the client echoes the mask back.
fn merge_password(mut incoming: serde_json::Value, existing: &serde_json::Value) -> serde_json::Value {
    if let (Some(obj), Some(old)) = (incoming.as_object_mut(), existing.as_object()) {
        if obj.get("password").and_then(|v| v.as_str()).map(|p| p == "••••••" || p.is_empty()).unwrap_or(true) {
            if let Some(p) = old.get("password") {
                obj.insert("password".into(), p.clone());
            }
        }
    }
    incoming
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<TargetView>>> {
    let rows = sqlx::query_as::<_, SyncTarget>("SELECT * FROM sync_targets WHERE user_id = $1 ORDER BY created_at")
        .bind(user.id)
        .fetch_all(&state.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(view(&state, r).await?);
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct CreateTargetBody {
    pub name: String,
    pub kind: TargetKind,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub visibility: Option<Visibility>,
    #[serde(default)]
    pub filters: Option<Filters>,
    #[serde(default)]
    pub placeholder_title: Option<String>,
    #[serde(default)]
    pub push_interval_secs: Option<i32>,
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateTargetBody>,
) -> AppResult<Json<TargetView>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("name is required"));
    }
    let config = validate_target_config(body.kind, &body.config)?;
    let visibility = resolve_visibility(body.preset, body.visibility)?;
    let filters = body.filters.unwrap_or_default();
    validate_filters(&state, &user, &filters).await?;
    let placeholder =
        body.placeholder_title.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Busy".into());
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO sync_targets (id, user_id, name, kind, config, visibility, filters, placeholder_title, push_interval_secs)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(id)
    .bind(user.id)
    .bind(&name)
    .bind(body.kind)
    .bind(&config)
    .bind(serde_json::to_value(visibility).unwrap())
    .bind(serde_json::to_value(&filters).unwrap())
    .bind(&placeholder)
    .bind(body.push_interval_secs.unwrap_or(900).clamp(60, 86_400))
    .execute(&state.db)
    .await?;
    // First push right away so the user sees the result (and any error).
    crate::sync::push_target_now(&state, id, true).await?;
    Ok(Json(view(&state, load_owned(&state, &user, id).await?).await?))
}

pub async fn get_one(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<TargetView>> {
    Ok(Json(view(&state, load_owned(&state, &user, id).await?).await?))
}

#[derive(Deserialize)]
pub struct UpdateTargetBody {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub preset: Option<String>,
    pub visibility: Option<Visibility>,
    pub filters: Option<Filters>,
    pub placeholder_title: Option<String>,
    pub enabled: Option<bool>,
    pub push_interval_secs: Option<i32>,
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTargetBody>,
) -> AppResult<Json<TargetView>> {
    let existing = load_owned(&state, &user, id).await?;
    let name = body.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or(existing.name);
    let config = match body.config {
        Some(cfg) => validate_target_config(existing.kind, &merge_password(cfg, &existing.config))?,
        None => existing.config,
    };
    let visibility = match (body.preset, body.visibility) {
        (None, None) => parse_visibility(&existing.visibility),
        (p, v) => resolve_visibility(p, v)?,
    };
    let filters = match body.filters {
        Some(f) => {
            validate_filters(&state, &user, &f).await?;
            f
        }
        None => parse_filters(&existing.filters),
    };
    let placeholder =
        body.placeholder_title.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(existing.placeholder_title);
    sqlx::query(
        r#"UPDATE sync_targets
           SET name = $2, config = $3, visibility = $4, filters = $5, placeholder_title = $6, enabled = $7,
               push_interval_secs = $8, next_push_at = now(), updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(&name)
    .bind(&config)
    .bind(serde_json::to_value(visibility).unwrap())
    .bind(serde_json::to_value(&filters).unwrap())
    .bind(&placeholder)
    .bind(body.enabled.unwrap_or(existing.enabled))
    .bind(body.push_interval_secs.unwrap_or(existing.push_interval_secs).clamp(60, 86_400))
    .execute(&state.db)
    .await?;
    Ok(Json(view(&state, load_owned(&state, &user, id).await?).await?))
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    /// Also delete the mirrored events from the remote calendar (default true).
    #[serde(default = "default_true")]
    pub clear_remote: bool,
}
fn default_true() -> bool {
    true
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<DeleteQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let target = load_owned(&state, &user, id).await?;
    let mut removed = 0;
    if q.clear_remote {
        removed = crate::sync::push::clear_target(&state, &target).await.unwrap_or(0);
    }
    sqlx::query("DELETE FROM sync_targets WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true, "removed_remote": removed })))
}

#[derive(Deserialize)]
pub struct PushQuery {
    #[serde(default = "default_true")]
    pub wait: bool,
}

pub async fn push_now(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<PushQuery>,
) -> AppResult<Json<TargetView>> {
    load_owned(&state, &user, id).await?;
    crate::sync::push_target_now(&state, id, q.wait).await?;
    Ok(Json(view(&state, load_owned(&state, &user, id).await?).await?))
}

/// What would be written to the target.
pub async fn preview(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<WindowQuery>,
) -> AppResult<Json<Vec<SharedEvent>>> {
    let t = load_owned(&state, &user, id).await?;
    let (from, to) = q.resolve()?;
    Ok(Json(evaluate_rule(&state, user.id, &parse_visibility(&t.visibility), &parse_filters(&t.filters), from, to).await?))
}

// ---- CalDAV discovery -------------------------------------------------------

#[derive(Deserialize)]
pub struct DiscoverBody {
    pub url: String,
    pub username: String,
    pub password: String,
}

/// List the calendars behind a CalDAV account so the user can pick one.
pub async fn caldav_discover(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(body): Json<DiscoverBody>,
) -> AppResult<Json<Vec<caldav::DiscoveredCalendar>>> {
    let cfg = caldav::validate_config(&json!({ "url": body.url, "username": body.username, "password": body.password }))
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let client = caldav::Client::new(&state.http, &cfg);
    let calendars = client.discover().await.map_err(|e| AppError::bad_request(format!("{e:#}")))?;
    if calendars.is_empty() {
        return Err(AppError::bad_request("no calendars found for this account"));
    }
    Ok(Json(calendars))
}
