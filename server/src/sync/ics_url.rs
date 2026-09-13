//! Connector for any calendar that exposes an iCalendar URL: Google's
//! "secret address in iCal format", Outlook's published calendars, iCloud
//! public calendars, Nextcloud, Fastmail and so on.

use async_trait::async_trait;
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::models::CalendarSource;
use crate::state::AppState;

use super::connector::{Connector, FetchResult, FetchedCalendar};

/// Refuse feeds larger than this to protect the parser.
const MAX_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct IcsUrlConfig {
    pub url: String,
    /// Optional basic-auth credentials for protected feeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

pub struct IcsUrlConnector;

#[async_trait]
impl Connector for IcsUrlConnector {
    fn validate_config(&self, config: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let mut cfg: IcsUrlConfig = serde_json::from_value(config.clone()).map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
        cfg.url = cfg.url.trim().to_string();
        // Apple and others hand out webcal:// links; they are plain HTTPS.
        if let Some(rest) = cfg.url.strip_prefix("webcal://") {
            cfg.url = format!("https://{rest}");
        } else if let Some(rest) = cfg.url.strip_prefix("webcals://") {
            cfg.url = format!("https://{rest}");
        }
        let parsed = url::Url::parse(&cfg.url).map_err(|e| anyhow::anyhow!("invalid url: {e}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            anyhow::bail!("url must use http or https");
        }
        if parsed.host_str().is_none() {
            anyhow::bail!("url must have a host");
        }
        if cfg.username.as_deref().map(str::is_empty).unwrap_or(false) {
            cfg.username = None;
        }
        if cfg.password.as_deref().map(str::is_empty).unwrap_or(false) {
            cfg.password = None;
        }
        Ok(serde_json::to_value(cfg)?)
    }

    async fn fetch(&self, state: &AppState, source: &CalendarSource) -> anyhow::Result<FetchResult> {
        let cfg: IcsUrlConfig = serde_json::from_value(source.config.clone())?;
        let mut req = state.http.get(&cfg.url).header(header::ACCEPT, "text/calendar, */*;q=0.5");
        if let Some(user) = &cfg.username {
            req = req.basic_auth(user, cfg.password.as_deref());
        }
        if let Some(etag) = &source.etag {
            req = req.header(header::IF_NONE_MATCH, etag);
        }
        let resp = req.send().await.map_err(|e| anyhow::anyhow!("request failed: {e}"))?;
        if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(FetchResult::NotModified);
        }
        if !resp.status().is_success() {
            anyhow::bail!("remote returned HTTP {}", resp.status());
        }
        let etag = resp.headers().get(header::ETAG).and_then(|v| v.to_str().ok()).map(String::from);
        if let Some(len) = resp.content_length() {
            if len as usize > MAX_BYTES {
                anyhow::bail!("feed too large ({len} bytes)");
            }
        }
        let bytes = resp.bytes().await.map_err(|e| anyhow::anyhow!("read failed: {e}"))?;
        if bytes.len() > MAX_BYTES {
            anyhow::bail!("feed too large ({} bytes)", bytes.len());
        }
        let ics = String::from_utf8_lossy(&bytes).into_owned();
        Ok(FetchResult::Changed(FetchedCalendar { ics, etag }))
    }
}
