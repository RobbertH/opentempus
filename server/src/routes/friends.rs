use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::FriendshipStatus,
    state::AppState,
};

#[derive(Serialize, sqlx::FromRow)]
pub struct FriendView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: FriendshipStatus,
    /// True when the other person sent the request and I still need to answer.
    pub incoming: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<FriendView>>> {
    let rows = sqlx::query_as::<_, FriendView>(
        r#"SELECT f.id,
                  u.id AS user_id, u.email::text AS email, u.display_name,
                  f.status,
                  (f.addressee_id = $1) AS incoming,
                  f.created_at
           FROM friendships f
           JOIN users u ON u.id = CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END
           WHERE (f.requester_id = $1 OR f.addressee_id = $1) AND f.status <> 'declined'
           ORDER BY f.status, u.display_name"#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct RequestBody {
    pub email: String,
}

pub async fn request(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<RequestBody>,
) -> AppResult<Json<serde_json::Value>> {
    let email = body.email.trim().to_ascii_lowercase();
    let other: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1").bind(&email).fetch_optional(&state.db).await?;
    // Do not reveal whether an account exists; a friend request to an unknown
    // address simply has no effect.
    let Some((other_id,)) = other else {
        return Ok(Json(json!({ "ok": true, "status": "pending" })));
    };
    if other_id == user.id {
        return Err(AppError::bad_request("you cannot befriend yourself"));
    }

    // If they already asked me, accept instead of creating a duplicate.
    let reverse: Option<(Uuid, FriendshipStatus)> =
        sqlx::query_as("SELECT id, status FROM friendships WHERE requester_id = $1 AND addressee_id = $2")
            .bind(other_id)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
    if let Some((id, status)) = reverse {
        if status != FriendshipStatus::Accepted {
            sqlx::query("UPDATE friendships SET status = 'accepted', responded_at = now() WHERE id = $1")
                .bind(id)
                .execute(&state.db)
                .await?;
        }
        return Ok(Json(json!({ "ok": true, "status": "accepted" })));
    }

    sqlx::query(
        r#"INSERT INTO friendships (id, requester_id, addressee_id, status)
           VALUES ($1, $2, $3, 'pending')
           ON CONFLICT (requester_id, addressee_id) DO UPDATE
             SET status = CASE WHEN friendships.status = 'accepted' THEN 'accepted'::friendship_status ELSE 'pending'::friendship_status END,
                 created_at = now(), responded_at = NULL"#,
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(other_id)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "status": "pending" })))
}

async fn respond(state: &AppState, user: &AuthUser, id: Uuid, status: FriendshipStatus) -> AppResult<()> {
    let res = sqlx::query(
        "UPDATE friendships SET status = $3, responded_at = now() WHERE id = $1 AND addressee_id = $2 AND status = 'pending'",
    )
    .bind(id)
    .bind(user.id)
    .bind(status)
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("no pending request with that id"));
    }
    Ok(())
}

pub async fn accept(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    respond(&state, &user, id, FriendshipStatus::Accepted).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn decline(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    respond(&state, &user, id, FriendshipStatus::Declined).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Remove a friendship (or withdraw a request). Shares between the two users
/// are deleted as well, in both directions.
pub async fn remove(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT requester_id, addressee_id FROM friendships WHERE id = $1 AND (requester_id = $2 OR addressee_id = $2)",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    let Some((a, b)) = row else { return Err(AppError::not_found("friendship not found")) };
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM friendships WHERE id = $1").bind(id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM shares WHERE audience_kind = 'friend' AND ((owner_id = $1 AND audience_user_id = $2) OR (owner_id = $2 AND audience_user_id = $1))")
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}

/// True when `a` and `b` are accepted friends.
pub async fn are_friends(state: &AppState, a: Uuid, b: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1::bigint FROM friendships WHERE status = 'accepted' AND ((requester_id = $1 AND addressee_id = $2) OR (requester_id = $2 AND addressee_id = $1))",
    )
    .bind(a)
    .bind(b)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.is_some())
}
