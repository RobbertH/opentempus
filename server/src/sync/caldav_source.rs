//! Inbound connector for CalDAV calendars (iCloud, Fastmail, Nextcloud,
//! Radicale, Google via CalDAV, …).

use async_trait::async_trait;

use crate::models::CalendarSource;
use crate::state::AppState;

use super::caldav::{self, Client};
use super::connector::{Connector, FetchResult, FetchedCalendar};

pub struct CalDavConnector;

#[async_trait]
impl Connector for CalDavConnector {
    fn validate_config(&self, config: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let cfg = caldav::validate_config(config)?;
        if cfg.calendar_url.is_none() {
            anyhow::bail!("pick a calendar first (calendar_url is missing)");
        }
        Ok(serde_json::to_value(cfg)?)
    }

    async fn fetch(&self, state: &AppState, source: &CalendarSource) -> anyhow::Result<FetchResult> {
        let cfg: caldav::CalDavConfig = serde_json::from_value(source.config.clone())?;
        let client = Client::new(&state.http, &cfg);
        let ctag = client.ctag().await?;
        if ctag.is_some() && ctag == source.etag {
            return Ok(FetchResult::NotModified);
        }
        let objects = client.fetch_events().await?;
        // Each resource is its own VCALENDAR; the parser accepts a sequence.
        let mut ics = String::new();
        for (_, data) in objects {
            ics.push_str(data.trim());
            ics.push_str("\r\n");
        }
        if ics.is_empty() {
            ics.push_str("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n");
        }
        Ok(FetchResult::Changed(FetchedCalendar { ics, etag: ctag }))
    }
}
