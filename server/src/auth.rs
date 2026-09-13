//! Password hashing, opaque session tokens and the `AuthUser` extractor.
//!
//! Sessions are random 256-bit tokens; only their SHA-256 hash is stored so a
//! database leak does not leak live sessions.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

pub const SESSION_COOKIE: &str = "opentempus_session";

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt).map_err(|e| anyhow::anyhow!("hash error: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Generate a URL-safe random token of `bytes` random bytes.
pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Authenticated user, resolved from the session cookie (or a
/// `Authorization: Bearer <session token>` header for API clients).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub timezone: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .or_else(|| {
                parts
                    .headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(|s| s.trim().to_string())
            })
            .ok_or(AppError::Unauthorized)?;

        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            r#"SELECT u.id, u.email::text, u.display_name, u.timezone
               FROM sessions s JOIN users u ON u.id = s.user_id
               WHERE s.token_hash = $1 AND s.expires_at > now()"#,
        )
        .bind(hash_token(&token))
        .fetch_optional(&state.db)
        .await?;

        match row {
            Some((id, email, display_name, timezone)) => Ok(AuthUser { id, email, display_name, timezone }),
            None => Err(AppError::Unauthorized),
        }
    }
}

pub struct NewSession {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn create_session(state: &AppState, user_id: Uuid) -> Result<NewSession, AppError> {
    let token = random_token(32);
    let expires_at = Utc::now() + Duration::days(state.config.session_ttl_days);
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(hash_token(&token))
        .bind(expires_at)
        .execute(&state.db)
        .await?;
    Ok(NewSession { token, expires_at })
}

pub async fn delete_session(state: &AppState, token: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1").bind(hash_token(token)).execute(&state.db).await?;
    Ok(())
}

pub fn session_cookie(state: &AppState, token: &str, expires_at: DateTime<Utc>) -> Cookie<'static> {
    let max_age = (expires_at - Utc::now()).num_seconds().max(0);
    Cookie::build((SESSION_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .secure(state.config.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(::time::Duration::seconds(max_age))
        .build()
}

pub fn clear_session_cookie(state: &AppState) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(state.config.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(::time::Duration::seconds(0))
        .build()
}
