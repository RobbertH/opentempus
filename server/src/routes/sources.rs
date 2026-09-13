use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{CalendarSource, SourceKind},
    state::AppState,
    sync::connector::connector_for,
};

/// Public view of a source. Secrets inside `config` are masked.
#[derive(Serialize)]
pub struct SourceView {
    #[serde(flatten)]
    pub source: CalendarSource,
}

fn view(mut s: CalendarSource) -> SourceView {
    if let Some(obj) = s.config.as_object_mut() {
        if obj.contains_key("password") {
            obj.insert("password".into(), json!("••••••"));
        }
    }
    SourceView { source: s }
}

async fn load_owned(state: &AppState, user: &AuthUser, id: Uuid) -> AppResult<CalendarSource> {
    sqlx::query_as::<_, CalendarSource>("SELECT * FROM calendar_sources WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("source not found"))
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<SourceView>>> {
    let rows = sqlx::query_as::<_, CalendarSource>("SELECT * FROM calendar_sources WHERE user_id = $1 ORDER BY created_at")
        .bind(user.id)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(view).collect()))
}

#[derive(Deserialize)]
pub struct CreateSourceBody {
    pub name: String,
    pub kind: SourceKind,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sync_interval_secs: Option<i32>,
    #[serde(default)]
    pub horizon_past_days: Option<i32>,
    #[serde(default)]
    pub horizon_future_days: Option<i32>,
}

fn clean_category(c: Option<String>) -> String {
    let c = c.unwrap_or_default().trim().to_ascii_lowercase();
    if c.is_empty() {
        "personal".into()
    } else {
        c
    }
}

fn clean_color(c: Option<String>) -> AppResult<Option<String>> {
    match c {
        None => Ok(None),
        Some(c) => {
            let c = c.trim().to_string();
            let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
            if ok {
                Ok(Some(c))
            } else {
                Err(AppError::bad_request("color must look like #rrggbb"))
            }
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateSourceBody>,
) -> AppResult<Json<SourceView>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("name is required"));
    }
    let connector = connector_for(body.kind).map_err(|e| AppError::bad_request(e.to_string()))?;
    let config = connector.validate_config(&body.config).map_err(|e| AppError::bad_request(e.to_string()))?;
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, CalendarSource>(
        r#"INSERT INTO calendar_sources (id, user_id, name, kind, config, category, color, sync_interval_secs, horizon_past_days, horizon_future_days)
           VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, '#4f6df5'), $8, $9, $10)
           RETURNING *"#,
    )
    .bind(id)
    .bind(user.id)
    .bind(&name)
    .bind(body.kind)
    .bind(&config)
    .bind(clean_category(body.category))
    .bind(clean_color(body.color)?)
    .bind(body.sync_interval_secs.unwrap_or(900).clamp(60, 86_400))
    .bind(body.horizon_past_days.unwrap_or(30).clamp(0, 3650))
    .bind(body.horizon_future_days.unwrap_or(365).clamp(1, 3650))
    .fetch_one(&state.db)
    .await?;

    // First sync happens right away so the user sees data immediately.
    let error = crate::sync::sync_source_now(&state, id, true).await?;
    let row = if error.is_some() { load_owned(&state, &user, id).await? } else { load_owned(&state, &user, row.id).await? };
    Ok(Json(view(row)))
}

pub async fn get_one(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<SourceView>> {
    Ok(Json(view(load_owned(&state, &user, id).await?)))
}

#[derive(Deserialize)]
pub struct UpdateSourceBody {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub category: Option<String>,
    pub color: Option<String>,
    pub enabled: Option<bool>,
    pub sync_interval_secs: Option<i32>,
    pub horizon_past_days: Option<i32>,
    pub horizon_future_days: Option<i32>,
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateSourceBody>,
) -> AppResult<Json<SourceView>> {
    let existing = load_owned(&state, &user, id).await?;
    let name = body.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or(existing.name);
    let config = match body.config {
        Some(mut cfg) => {
            // Keep the stored password when the client sends the mask back.
            if let (Some(obj), Some(old)) = (cfg.as_object_mut(), existing.config.as_object()) {
                if obj.get("password").and_then(|v| v.as_str()) == Some("••••••") {
                    if let Some(p) = old.get("password") {
                        obj.insert("password".into(), p.clone());
                    }
                }
            }
            connector_for(existing.kind)
                .map_err(|e| AppError::bad_request(e.to_string()))?
                .validate_config(&cfg)
                .map_err(|e| AppError::bad_request(e.to_string()))?
        }
        None => existing.config,
    };
    let category = body.category.map(|c| clean_category(Some(c))).unwrap_or(existing.category);
    let color = clean_color(body.color)?.unwrap_or(existing.color);
    let row = sqlx::query_as::<_, CalendarSource>(
        r#"UPDATE calendar_sources
           SET name = $2, config = $3, category = $4, color = $5, enabled = $6,
               sync_interval_secs = $7, horizon_past_days = $8, horizon_future_days = $9,
               etag = NULL, next_sync_at = now(), updated_at = now()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(&name)
    .bind(&config)
    .bind(&category)
    .bind(&color)
    .bind(body.enabled.unwrap_or(existing.enabled))
    .bind(body.sync_interval_secs.unwrap_or(existing.sync_interval_secs).clamp(60, 86_400))
    .bind(body.horizon_past_days.unwrap_or(existing.horizon_past_days).clamp(0, 3650))
    .bind(body.horizon_future_days.unwrap_or(existing.horizon_future_days).clamp(1, 3650))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(view(row)))
}

pub async fn delete(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    load_owned(&state, &user, id).await?;
    sqlx::query("DELETE FROM calendar_sources WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct SyncQuery {
    /// When true (default) the request waits for the sync to finish.
    #[serde(default = "default_true")]
    pub wait: bool,
}
fn default_true() -> bool {
    true
}

pub async fn sync_now(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<SyncQuery>,
) -> AppResult<Json<SourceView>> {
    load_owned(&state, &user, id).await?;
    crate::sync::sync_source_now(&state, id, q.wait).await?;
    Ok(Json(view(load_owned(&state, &user, id).await?)))
}
