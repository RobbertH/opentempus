use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("opentempus/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client");
        Self { db, config: Arc::new(config), http }
    }
}
