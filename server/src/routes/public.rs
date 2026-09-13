//! Token-authenticated access for calendar apps and AI agents. Every endpoint
//! here is reachable with nothing but the share token, so responses only ever
//! contain what the share's visibility allows.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    ics::generate::{render_feed, FeedOptions},
    models::Share,
    sharing::{free_slots, FreeSlot, SharedEvent, Visibility},
    state::AppState,
};

use super::{
    shares::{evaluate_share, parse_filters, parse_visibility},
    WindowQuery,
};

async fn load_by_token(state: &AppState, token: &str) -> AppResult<(Share, String)> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT display_name FROM users u JOIN shares s ON s.owner_id = u.id WHERE s.token = $1")
            .bind(token)
            .fetch_optional(&state.db)
            .await?;
    let share = sqlx::query_as::<_, Share>("SELECT * FROM shares WHERE token = $1 AND enabled")
        .bind(token)
        .fetch_optional(&state.db)
        .await?;
    match (share, row) {
        (Some(s), Some((name,))) => Ok((s, name)),
        _ => Err(AppError::not_found("unknown or disabled share")),
    }
}

#[derive(Serialize)]
pub struct PublicInfo {
    pub name: String,
    pub owner: String,
    pub visibility: Visibility,
    pub horizon_past_days: i64,
    pub horizon_future_days: i64,
    pub feed_url: String,
    pub events_url: String,
    pub free_url: String,
}

/// Describe what this token gives access to. Useful for agents.
pub async fn info(State(state): State<AppState>, Path(token): Path<String>) -> AppResult<Json<PublicInfo>> {
    let (share, owner) = load_by_token(&state, &token).await?;
    let filters = parse_filters(&share.filters);
    let base = &state.config.public_url;
    Ok(Json(PublicInfo {
        name: share.name,
        owner,
        visibility: parse_visibility(&share.visibility),
        horizon_past_days: filters.horizon_past_days,
        horizon_future_days: filters.horizon_future_days,
        feed_url: format!("{base}/feeds/{token}.ics"),
        events_url: format!("{base}/api/v1/public/{token}/events?from=<rfc3339>&to=<rfc3339>"),
        free_url: format!("{base}/api/v1/public/{token}/free?from=<rfc3339>&to=<rfc3339>&min_minutes=60"),
    }))
}

pub async fn events(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(q): Query<WindowQuery>,
) -> AppResult<Json<Vec<SharedEvent>>> {
    let (share, _) = load_by_token(&state, &token).await?;
    let (from, to) = q.resolve()?;
    Ok(Json(evaluate_share(&state, &share, from, to).await?))
}

/// The ICS feed. Subscribe to this URL from Google/Apple/Outlook.
pub async fn feed(State(state): State<AppState>, Path(token): Path<String>) -> AppResult<impl IntoResponse> {
    let token = token.strip_suffix(".ics").unwrap_or(&token).to_string();
    let (share, owner) = load_by_token(&state, &token).await?;
    let filters = parse_filters(&share.filters);
    let now = Utc::now();
    let from = now - Duration::days(filters.horizon_past_days);
    let to = now + Duration::days(filters.horizon_future_days);
    let events = evaluate_share(&state, &share, from, to).await?;
    let host = url::Url::parse(&state.config.public_url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_else(|| "opentempus".into());
    let name = format!("{owner} · {}", share.name);
    let body = render_feed(&events, &FeedOptions { calendar_name: &name, host: &host, now });
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/calendar; charset=utf-8".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "private, max-age=300".parse().unwrap());
    headers.insert(header::CONTENT_DISPOSITION, "inline; filename=\"calendar.ics\"".parse().unwrap());
    Ok((StatusCode::OK, headers, body))
}

#[derive(Deserialize)]
pub struct FreeQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// Minimum slot length to report.
    #[serde(default = "default_min_minutes")]
    pub min_minutes: i64,
}
fn default_min_minutes() -> i64 {
    30
}

#[derive(Serialize)]
pub struct FreeResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub min_minutes: i64,
    pub slots: Vec<FreeSlot>,
}

/// Free slots for one share.
pub async fn free(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(q): Query<FreeQuery>,
) -> AppResult<Json<FreeResponse>> {
    let (share, _) = load_by_token(&state, &token).await?;
    let (from, to) = WindowQuery { from: q.from, to: q.to }.resolve()?;
    let filters = parse_filters(&share.filters);
    let (from, to) = filters.clamp_window(from, to, Utc::now());
    let events = evaluate_share(&state, &share, from, to).await?;
    let busy = events.iter().filter(|e| e.busy).map(|e| (e.start, e.end));
    let slots = free_slots(from, to, busy, Duration::minutes(q.min_minutes.max(1)));
    Ok(Json(FreeResponse { from, to, min_minutes: q.min_minutes, slots }))
}

#[derive(Deserialize)]
pub struct AvailabilityBody {
    /// Tokens of every participant's share.
    pub tokens: Vec<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_min_minutes")]
    pub min_minutes: i64,
}

#[derive(Serialize)]
pub struct AvailabilityResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub min_minutes: i64,
    pub participants: Vec<String>,
    /// Slots where every participant is free.
    pub slots: Vec<FreeSlot>,
}

/// Common free slots across several shares, e.g. "when can the four of us
/// play padel this week?". Each token is evaluated with its own filters and
/// horizon, so nobody exposes more than they chose to.
pub async fn availability(
    State(state): State<AppState>,
    Json(body): Json<AvailabilityBody>,
) -> AppResult<Json<AvailabilityResponse>> {
    if body.tokens.is_empty() || body.tokens.len() > 50 {
        return Err(AppError::bad_request("between 1 and 50 tokens are required"));
    }
    let (from, to) = WindowQuery { from: body.from, to: body.to }.resolve()?;
    let mut busy: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    let mut participants = Vec::new();
    let mut from_eff = from;
    let mut to_eff = to;
    for token in &body.tokens {
        let (share, owner) = load_by_token(&state, token).await?;
        let filters = parse_filters(&share.filters);
        // The common window is the intersection of everybody's horizon.
        let (f, t) = filters.clamp_window(from, to, Utc::now());
        from_eff = from_eff.max(f);
        to_eff = to_eff.min(t);
        let events = evaluate_share(&state, &share, from, to).await?;
        busy.extend(events.iter().filter(|e| e.busy).map(|e| (e.start, e.end)));
        participants.push(owner);
    }
    if to_eff <= from_eff {
        return Ok(Json(AvailabilityResponse {
            from: from_eff,
            to: from_eff,
            min_minutes: body.min_minutes,
            participants,
            slots: vec![],
        }));
    }
    let slots = free_slots(from_eff, to_eff, busy, Duration::minutes(body.min_minutes.max(1)));
    Ok(Json(AvailabilityResponse { from: from_eff, to: to_eff, min_minutes: body.min_minutes, participants, slots }))
}
