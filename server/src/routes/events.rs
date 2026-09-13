use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppResult,
    models::{EventStatus, InstanceRow, Rsvp, Transparency, INSTANCE_SELECT},
    state::AppState,
};

use super::WindowQuery;

/// The owner's own view of an occurrence: everything.
#[derive(Serialize)]
pub struct MyEvent {
    pub id: Uuid,
    pub event_id: Uuid,
    pub source_id: Uuid,
    pub source_name: String,
    pub category: String,
    pub color: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: EventStatus,
    pub transparency: Transparency,
    pub rsvp: Rsvp,
}

impl From<InstanceRow> for MyEvent {
    fn from(r: InstanceRow) -> Self {
        Self {
            id: r.instance_id,
            event_id: r.event_id,
            source_id: r.source_id,
            source_name: r.source_name,
            category: r.source_category,
            color: r.source_color,
            start: r.start_at,
            end: r.end_at,
            all_day: r.all_day,
            title: r.summary,
            description: r.description,
            location: r.location,
            status: r.status,
            transparency: r.transparency,
            rsvp: r.rsvp,
        }
    }
}

/// Load all occurrences of `user_id` overlapping `[from, to)`.
pub async fn load_instances(
    state: &AppState,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<InstanceRow>, sqlx::Error> {
    let sql = format!(
        "{INSTANCE_SELECT} WHERE s.user_id = $1 AND s.enabled AND i.end_at > $2 AND i.start_at < $3 ORDER BY i.start_at, i.end_at"
    );
    sqlx::query_as::<_, InstanceRow>(&sql).bind(user_id).bind(from).bind(to).fetch_all(&state.db).await
}

pub async fn list_mine(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<WindowQuery>,
) -> AppResult<Json<Vec<MyEvent>>> {
    let (from, to) = q.resolve()?;
    let rows = load_instances(&state, user.id, from, to).await?;
    Ok(Json(rows.into_iter().map(MyEvent::from).collect()))
}
