//! The connector abstraction. Every inbound calendar kind implements this.

use async_trait::async_trait;

use crate::models::CalendarSource;
use crate::state::AppState;

/// A fetched calendar, ready for parsing.
pub struct FetchedCalendar {
    /// Raw iCalendar text.
    pub ics: String,
    /// Opaque validator for conditional requests next time (ETag or similar).
    pub etag: Option<String>,
}

pub enum FetchResult {
    /// The calendar changed (or we could not tell); here is the new content.
    Changed(FetchedCalendar),
    /// The remote confirmed nothing changed since `source.etag`.
    NotModified,
}

#[async_trait]
pub trait Connector: Send + Sync {
    /// Validate connector-specific configuration when a source is created or
    /// updated. Returns the normalized config to store.
    fn validate_config(&self, config: &serde_json::Value) -> anyhow::Result<serde_json::Value>;

    /// Fetch the calendar for a source.
    async fn fetch(&self, state: &AppState, source: &CalendarSource) -> anyhow::Result<FetchResult>;
}

/// Resolve the connector for a source kind.
pub fn connector_for(kind: crate::models::SourceKind) -> anyhow::Result<Box<dyn Connector>> {
    use crate::models::SourceKind::*;
    match kind {
        IcsUrl => Ok(Box::new(super::ics_url::IcsUrlConnector)),
        Caldav => Ok(Box::new(super::caldav_source::CalDavConnector)),
        Google | Microsoft => anyhow::bail!("connector {kind:?} is not implemented yet; see ROADMAP.md"),
    }
}
