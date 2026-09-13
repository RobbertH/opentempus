//! HTTP routing. Everything under `/api/v1` is JSON; `/feeds/*.ics` serves
//! iCalendar; anything else is the embedded web app.

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod assets;
pub mod auth;
pub mod events;
pub mod friends;
pub mod public;
pub mod shares;
pub mod sources;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        // auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me).patch(auth::update_me))
        // calendar sources
        .route("/sources", get(sources::list).post(sources::create))
        .route("/sources/{id}", get(sources::get_one).patch(sources::update).delete(sources::delete))
        .route("/sources/{id}/sync", post(sources::sync_now))
        // own events
        .route("/events", get(events::list_mine))
        // friends
        .route("/friends", get(friends::list).post(friends::request))
        .route("/friends/{id}/accept", post(friends::accept))
        .route("/friends/{id}/decline", post(friends::decline))
        .route("/friends/{id}", axum::routing::delete(friends::remove))
        // shares I own
        .route("/shares", get(shares::list).post(shares::create))
        .route("/shares/presets", get(shares::presets))
        .route("/shares/{id}", get(shares::get_one).patch(shares::update).delete(shares::delete))
        .route("/shares/{id}/rotate-token", post(shares::rotate_token))
        .route("/shares/{id}/preview", get(shares::preview))
        // shares to me
        .route("/shared-with-me", get(shares::shared_with_me))
        .route("/shared-with-me/{id}/events", get(shares::shared_with_me_events))
        // token access (for friends' calendar apps and AI agents)
        .route("/public/{token}", get(public::info))
        .route("/public/{token}/events", get(public::events))
        .route("/public/{token}/free", get(public::free))
        .route("/public/availability", post(public::availability))
        .route("/health", get(|| async { "ok" }));

    Router::new()
        .nest("/api/v1", api)
        .route("/feeds/{token}", get(public::feed))
        .fallback(assets::serve)
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Shared query parameters for time windows.
#[derive(Debug, serde::Deserialize)]
pub struct WindowQuery {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

impl WindowQuery {
    /// Resolve to a concrete window; defaults to "now .. +14 days".
    pub fn resolve(&self) -> Result<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>), crate::error::AppError> {
        let now = chrono::Utc::now();
        let from = self.from.unwrap_or(now);
        let to = self.to.unwrap_or(from + chrono::Duration::days(14));
        if to <= from {
            return Err(crate::error::AppError::bad_request("'to' must be after 'from'"));
        }
        if to - from > chrono::Duration::days(400) {
            return Err(crate::error::AppError::bad_request("window may not exceed 400 days"));
        }
        Ok((from, to))
    }
}
