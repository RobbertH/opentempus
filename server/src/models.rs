//! Database row types and shared enums.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "rsvp_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Rsvp {
    Organizer,
    Accepted,
    Tentative,
    Declined,
    NeedsAction,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transparency", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Transparency {
    Opaque,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "source_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    IcsUrl,
    Caldav,
    Google,
    Microsoft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "sync_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Never,
    Ok,
    Error,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "friendship_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FriendshipStatus {
    Pending,
    Accepted,
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audience_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AudienceKind {
    Friend,
    Link,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CalendarSource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub kind: SourceKind,
    pub config: serde_json::Value,
    pub category: String,
    pub color: String,
    pub horizon_past_days: i32,
    pub horizon_future_days: i32,
    pub sync_interval_secs: i32,
    pub enabled: bool,
    pub next_sync_at: DateTime<Utc>,
    pub sync_started_at: Option<DateTime<Utc>>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_sync_status: SyncStatus,
    pub last_sync_error: Option<String>,
    pub etag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "target_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Caldav,
    Google,
    Microsoft,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SyncTarget {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub kind: TargetKind,
    pub config: serde_json::Value,
    pub visibility: serde_json::Value,
    pub filters: serde_json::Value,
    pub placeholder_title: String,
    pub enabled: bool,
    pub push_interval_secs: i32,
    pub next_push_at: DateTime<Utc>,
    pub push_started_at: Option<DateTime<Utc>>,
    pub last_pushed_at: Option<DateTime<Utc>>,
    pub last_push_status: SyncStatus,
    pub last_push_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MirroredEvent {
    pub instance_id: Uuid,
    pub remote_href: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Share {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub audience_kind: AudienceKind,
    pub audience_user_id: Option<Uuid>,
    pub token: String,
    pub visibility: serde_json::Value,
    pub filters: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One expanded occurrence joined with its event and source. This is the row
/// shape behind every calendar read.
#[derive(Debug, Clone, FromRow)]
pub struct InstanceRow {
    pub instance_id: Uuid,
    pub event_id: Uuid,
    pub source_id: Uuid,
    pub source_name: String,
    pub source_category: String,
    pub source_color: String,
    pub uid: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: EventStatus,
    pub transparency: Transparency,
    pub rsvp: Rsvp,
}

impl From<InstanceRow> for crate::sharing::OwnerEvent {
    fn from(r: InstanceRow) -> Self {
        Self {
            instance_id: r.instance_id,
            event_id: r.event_id,
            source_id: r.source_id,
            source_name: r.source_name,
            source_category: r.source_category,
            uid: r.uid,
            start_at: r.start_at,
            end_at: r.end_at,
            all_day: r.all_day,
            summary: r.summary,
            description: r.description,
            location: r.location,
            status: r.status,
            transparency: r.transparency,
            rsvp: r.rsvp,
        }
    }
}

pub const INSTANCE_SELECT: &str = r#"
    SELECT i.id AS instance_id, e.id AS event_id, s.id AS source_id,
           s.name AS source_name, s.category AS source_category, s.color AS source_color,
           e.uid, i.start_at, i.end_at, i.all_day,
           e.summary, e.description, e.location, e.status, e.transparency, e.rsvp
    FROM event_instances i
    JOIN events e ON e.id = i.event_id
    JOIN calendar_sources s ON s.id = i.source_id
"#;
