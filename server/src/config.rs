//! Runtime configuration, read from environment variables (and an optional
//! `.env` file for development). Every setting has a sane default so that a
//! bare `DATABASE_URL` is enough to boot the server.

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    /// Public base URL used to render feed links, e.g. `https://cal.example.com`.
    pub public_url: String,
    /// Whether new accounts may be created through the API.
    pub allow_registration: bool,
    /// Maximum number of calendar sources synced concurrently.
    pub sync_concurrency: usize,
    /// How often the scheduler looks for due sources.
    pub scheduler_tick_secs: u64,
    /// Session lifetime.
    pub session_ttl_days: i64,
    /// Whether the auth cookie is marked `Secure` (requires HTTPS).
    pub secure_cookies: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
        let listen_addr: SocketAddr =
            std::env::var("OPENTEMPUS_LISTEN").unwrap_or_else(|_| "0.0.0.0:8080".to_string()).parse()?;
        let public_url = std::env::var("OPENTEMPUS_PUBLIC_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", listen_addr.port()))
            .trim_end_matches('/')
            .to_string();
        let allow_registration = env_bool("OPENTEMPUS_ALLOW_REGISTRATION", true);
        let sync_concurrency = env_parse("OPENTEMPUS_SYNC_CONCURRENCY", 4usize);
        let scheduler_tick_secs = env_parse("OPENTEMPUS_SCHEDULER_TICK_SECS", 30u64);
        let session_ttl_days = env_parse("OPENTEMPUS_SESSION_TTL_DAYS", 30i64);
        let secure_cookies = env_bool("OPENTEMPUS_SECURE_COOKIES", public_url.starts_with("https://"));
        Ok(Self {
            database_url,
            listen_addr,
            public_url,
            allow_registration,
            sync_concurrency,
            scheduler_tick_secs,
            session_ttl_days,
            secure_cookies,
        })
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
