use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{random_token, AuthUser},
    error::{AppError, AppResult},
    models::{AudienceKind, Share},
    sharing::{project_all, Filters, SharedEvent, Visibility},
    state::AppState,
};

use super::{events::load_instances, friends::are_friends, WindowQuery};

/// A share as returned to its owner.
#[derive(Serialize)]
pub struct ShareView {
    pub id: Uuid,
    pub name: String,
    pub audience_kind: AudienceKind,
    pub audience_user_id: Option<Uuid>,
    pub audience_display_name: Option<String>,
    pub audience_email: Option<String>,
    pub visibility: Visibility,
    pub filters: Filters,
    pub enabled: bool,
    pub token: String,
    pub feed_url: String,
    pub api_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ShareRow {
    #[sqlx(flatten)]
    share: Share,
    audience_display_name: Option<String>,
    audience_email: Option<String>,
}

const SHARE_SELECT: &str = r#"
    SELECT sh.*, u.display_name AS audience_display_name, u.email::text AS audience_email
    FROM shares sh LEFT JOIN users u ON u.id = sh.audience_user_id
"#;

pub fn parse_visibility(v: &serde_json::Value) -> Visibility {
    serde_json::from_value(v.clone()).unwrap_or_default()
}
pub fn parse_filters(v: &serde_json::Value) -> Filters {
    serde_json::from_value(v.clone()).unwrap_or_default()
}

fn to_view(state: &AppState, r: ShareRow) -> ShareView {
    let s = r.share;
    ShareView {
        id: s.id,
        name: s.name,
        audience_kind: s.audience_kind,
        audience_user_id: s.audience_user_id,
        audience_display_name: r.audience_display_name,
        audience_email: r.audience_email,
        visibility: parse_visibility(&s.visibility),
        filters: parse_filters(&s.filters),
        enabled: s.enabled,
        feed_url: format!("{}/feeds/{}.ics", state.config.public_url, s.token),
        api_url: format!("{}/api/v1/public/{}", state.config.public_url, s.token),
        token: s.token,
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}

async fn load_owned(state: &AppState, user: &AuthUser, id: Uuid) -> AppResult<ShareRow> {
    let sql = format!("{SHARE_SELECT} WHERE sh.id = $1 AND sh.owner_id = $2");
    sqlx::query_as::<_, ShareRow>(&sql)
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("share not found"))
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<ShareView>>> {
    let sql = format!("{SHARE_SELECT} WHERE sh.owner_id = $1 ORDER BY sh.created_at");
    let rows = sqlx::query_as::<_, ShareRow>(&sql).bind(user.id).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(|r| to_view(&state, r)).collect()))
}

#[derive(Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub visibility: Visibility,
}

pub async fn presets() -> Json<Vec<Preset>> {
    Json(vec![
        Preset {
            id: "busy_only",
            label: "Busy only",
            description: "Only that time is blocked. No titles, no categories, nothing else.",
            visibility: Visibility::BUSY_ONLY,
        },
        Preset {
            id: "category",
            label: "Category",
            description: "Busy blocks plus the category (work/personal) and which calendar they come from.",
            visibility: Visibility::CATEGORY,
        },
        Preset {
            id: "details",
            label: "Details",
            description: "Titles, locations, categories and whether you accepted. Descriptions stay hidden.",
            visibility: Visibility::DETAILS,
        },
        Preset { id: "full", label: "Full", description: "Everything, including descriptions.", visibility: Visibility::FULL },
    ])
}

#[derive(Deserialize)]
pub struct CreateShareBody {
    pub name: String,
    pub audience_kind: AudienceKind,
    pub audience_user_id: Option<Uuid>,
    /// Either a preset name or an explicit visibility object.
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub visibility: Option<Visibility>,
    #[serde(default)]
    pub filters: Option<Filters>,
}

pub fn resolve_visibility(preset: Option<String>, explicit: Option<Visibility>) -> AppResult<Visibility> {
    match (preset, explicit) {
        (_, Some(v)) => Ok(v),
        (Some(p), None) => Visibility::preset(&p).ok_or_else(|| AppError::bad_request("unknown preset")),
        (None, None) => Ok(Visibility::BUSY_ONLY),
    }
}

pub async fn validate_filters(state: &AppState, user: &AuthUser, f: &Filters) -> AppResult<()> {
    if f.horizon_past_days < 0 || f.horizon_future_days < 1 || f.horizon_future_days > 3650 || f.horizon_past_days > 3650 {
        return Err(AppError::bad_request("horizon out of range"));
    }
    if let Some(ids) = &f.source_ids {
        let count: (i64,) = sqlx::query_as("SELECT count(*) FROM calendar_sources WHERE user_id = $1 AND id = ANY($2)")
            .bind(user.id)
            .bind(ids)
            .fetch_one(&state.db)
            .await?;
        if count.0 as usize != ids.len() {
            return Err(AppError::bad_request("filters.source_ids contains a calendar you do not own"));
        }
    }
    Ok(())
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateShareBody>,
) -> AppResult<Json<ShareView>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("name is required"));
    }
    let audience_user_id = match body.audience_kind {
        AudienceKind::Friend => {
            let id =
                body.audience_user_id.ok_or_else(|| AppError::bad_request("audience_user_id is required for friend shares"))?;
            if !are_friends(&state, user.id, id).await? {
                return Err(AppError::bad_request("you can only share with accepted friends"));
            }
            Some(id)
        }
        AudienceKind::Link => None,
    };
    let visibility = resolve_visibility(body.preset, body.visibility)?;
    let filters = body.filters.unwrap_or_default();
    validate_filters(&state, &user, &filters).await?;

    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO shares (id, owner_id, name, audience_kind, audience_user_id, token, visibility, filters)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(id)
    .bind(user.id)
    .bind(&name)
    .bind(body.audience_kind)
    .bind(audience_user_id)
    .bind(random_token(24))
    .bind(serde_json::to_value(visibility).unwrap())
    .bind(serde_json::to_value(&filters).unwrap())
    .execute(&state.db)
    .await?;
    Ok(Json(to_view(&state, load_owned(&state, &user, id).await?)))
}

pub async fn get_one(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<ShareView>> {
    Ok(Json(to_view(&state, load_owned(&state, &user, id).await?)))
}

#[derive(Deserialize)]
pub struct UpdateShareBody {
    pub name: Option<String>,
    pub preset: Option<String>,
    pub visibility: Option<Visibility>,
    pub filters: Option<Filters>,
    pub enabled: Option<bool>,
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateShareBody>,
) -> AppResult<Json<ShareView>> {
    let existing = load_owned(&state, &user, id).await?.share;
    let name = body.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or(existing.name);
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
    sqlx::query("UPDATE shares SET name = $2, visibility = $3, filters = $4, enabled = $5, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(&name)
        .bind(serde_json::to_value(visibility).unwrap())
        .bind(serde_json::to_value(&filters).unwrap())
        .bind(body.enabled.unwrap_or(existing.enabled))
        .execute(&state.db)
        .await?;
    Ok(Json(to_view(&state, load_owned(&state, &user, id).await?)))
}

pub async fn delete(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    load_owned(&state, &user, id).await?;
    sqlx::query("DELETE FROM shares WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Replace the secret token; old feed URLs stop working immediately.
pub async fn rotate_token(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<ShareView>> {
    load_owned(&state, &user, id).await?;
    sqlx::query("UPDATE shares SET token = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(random_token(24))
        .execute(&state.db)
        .await?;
    Ok(Json(to_view(&state, load_owned(&state, &user, id).await?)))
}

/// Evaluate a rule (filters + visibility) over an owner's calendars.
pub async fn evaluate_rule(
    state: &AppState,
    owner_id: Uuid,
    visibility: &Visibility,
    filters: &Filters,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<SharedEvent>> {
    let (from, to) = filters.clamp_window(from, to, Utc::now());
    if to <= from {
        return Ok(Vec::new());
    }
    let rows = load_instances(state, owner_id, from, to).await?;
    let owner_events: Vec<crate::sharing::OwnerEvent> = rows.into_iter().map(Into::into).collect();
    Ok(project_all(visibility, filters, &owner_events))
}

/// Evaluate a share and return exactly what its audience would receive.
pub async fn evaluate_share(
    state: &AppState,
    share: &Share,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<SharedEvent>> {
    evaluate_rule(state, share.owner_id, &parse_visibility(&share.visibility), &parse_filters(&share.filters), from, to).await
}

/// Owner-side preview: "what will they see?"
pub async fn preview(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<WindowQuery>,
) -> AppResult<Json<Vec<SharedEvent>>> {
    let share = load_owned(&state, &user, id).await?.share;
    let (from, to) = q.resolve()?;
    Ok(Json(evaluate_share(&state, &share, from, to).await?))
}

/// A share as seen by its audience (a friend).
#[derive(Serialize)]
pub struct SharedWithMeView {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub owner_display_name: String,
    pub owner_email: String,
    pub visibility: Visibility,
    pub feed_url: String,
    pub api_url: String,
    pub enabled: bool,
}

#[derive(sqlx::FromRow)]
struct SharedWithMeRow {
    #[sqlx(flatten)]
    share: Share,
    owner_display_name: String,
    owner_email: String,
}

pub async fn shared_with_me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<SharedWithMeView>>> {
    let rows = sqlx::query_as::<_, SharedWithMeRow>(
        r#"SELECT sh.*, u.display_name AS owner_display_name, u.email::text AS owner_email
           FROM shares sh JOIN users u ON u.id = sh.owner_id
           WHERE sh.audience_user_id = $1
           ORDER BY u.display_name, sh.name"#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SharedWithMeView {
                id: r.share.id,
                name: r.share.name,
                owner_id: r.share.owner_id,
                owner_display_name: r.owner_display_name,
                owner_email: r.owner_email,
                visibility: parse_visibility(&r.share.visibility),
                feed_url: format!("{}/feeds/{}.ics", state.config.public_url, r.share.token),
                api_url: format!("{}/api/v1/public/{}", state.config.public_url, r.share.token),
                enabled: r.share.enabled,
            })
            .collect(),
    ))
}

pub async fn shared_with_me_events(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<WindowQuery>,
) -> AppResult<Json<Vec<SharedEvent>>> {
    let share = sqlx::query_as::<_, Share>("SELECT * FROM shares WHERE id = $1 AND audience_user_id = $2 AND enabled")
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("share not found"))?;
    let (from, to) = q.resolve()?;
    Ok(Json(evaluate_share(&state, &share, from, to).await?))
}
