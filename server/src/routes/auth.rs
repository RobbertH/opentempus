use axum::{extract::State, Json};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{self, AuthUser},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub timezone: String,
}

impl From<AuthUser> for MeResponse {
    fn from(u: AuthUser) -> Self {
        Self { id: u.id, email: u.email, display_name: u.display_name, timezone: u.timezone }
    }
}

fn validate_timezone(tz: &str) -> AppResult<String> {
    tz.parse::<chrono_tz::Tz>().map(|t| t.name().to_string()).map_err(|_| AppError::bad_request("unknown timezone"))
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterBody>,
) -> AppResult<(CookieJar, Json<MeResponse>)> {
    if !state.config.allow_registration {
        return Err(AppError::forbidden("registration is disabled on this server"));
    }
    let email = body.email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::bad_request("invalid email"));
    }
    if body.password.len() < 8 {
        return Err(AppError::bad_request("password must be at least 8 characters"));
    }
    let display_name = body.display_name.trim().to_string();
    if display_name.is_empty() {
        return Err(AppError::bad_request("display name is required"));
    }
    let timezone = validate_timezone(body.timezone.as_deref().unwrap_or("UTC"))?;
    let hash = auth::hash_password(&body.password)?;
    let id = Uuid::new_v4();
    let res = sqlx::query("INSERT INTO users (id, email, display_name, password_hash, timezone) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(&email)
        .bind(&display_name)
        .bind(&hash)
        .bind(&timezone)
        .execute(&state.db)
        .await;
    if let Err(sqlx::Error::Database(db)) = &res {
        if db.is_unique_violation() {
            return Err(AppError::conflict("an account with this email already exists"));
        }
    }
    res?;
    let session = auth::create_session(&state, id).await?;
    let jar = jar.add(auth::session_cookie(&state, &session.token, session.expires_at));
    Ok((jar, Json(MeResponse { id, email, display_name, timezone })))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    let email = body.email.trim().to_ascii_lowercase();
    let row: Option<(Uuid, String, String, String, String)> =
        sqlx::query_as("SELECT id, email::text, display_name, password_hash, timezone FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    // Always run the verifier to keep timing similar for unknown accounts.
    let dummy = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$Y2Y3ZjQ2ZjA3ZTg3ZjE1ZDg1ZjE0NjY2ZDQ2ZTk5ZTg";
    let (ok, row) = match row {
        Some(r) => (auth::verify_password(&body.password, &r.3), Some(r)),
        None => {
            let _ = auth::verify_password(&body.password, dummy);
            (false, None)
        }
    };
    if !ok {
        return Err(AppError::Unauthorized);
    }
    let (id, email, display_name, _, timezone) = row.unwrap();
    let session = auth::create_session(&state, id).await?;
    let jar = jar.add(auth::session_cookie(&state, &session.token, session.expires_at));
    Ok((
        jar,
        Json(json!({
            "user": MeResponse { id, email, display_name, timezone },
            // Returned for non-browser clients that prefer a bearer token.
            "token": session.token,
            "expires_at": session.expires_at,
        })),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    if let Some(c) = jar.get(auth::SESSION_COOKIE) {
        auth::delete_session(&state, c.value()).await?;
    }
    let jar = jar.add(auth::clear_session_cookie(&state));
    Ok((jar, Json(json!({ "ok": true }))))
}

pub async fn me(user: AuthUser) -> Json<MeResponse> {
    Json(user.into())
}

#[derive(Deserialize)]
pub struct UpdateMeBody {
    pub display_name: Option<String>,
    pub timezone: Option<String>,
}

pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateMeBody>,
) -> AppResult<Json<MeResponse>> {
    let display_name = body.display_name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(user.display_name);
    let timezone = match body.timezone {
        Some(tz) => validate_timezone(&tz)?,
        None => user.timezone,
    };
    sqlx::query("UPDATE users SET display_name = $2, timezone = $3 WHERE id = $1")
        .bind(user.id)
        .bind(&display_name)
        .bind(&timezone)
        .execute(&state.db)
        .await?;
    Ok(Json(MeResponse { id: user.id, email: user.email, display_name, timezone }))
}
