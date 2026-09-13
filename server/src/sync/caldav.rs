//! A small CalDAV client: enough of RFC 4791 / RFC 6764 to discover
//! calendars, detect changes, fetch events and write mirrored events.

use reqwest::{header, Method, StatusCode};
use serde::{Deserialize, Serialize};

const DAV: &str = "DAV:";
const CALDAV: &str = "urn:ietf:params:xml:ns:caldav";
const CS: &str = "http://calendarserver.org/ns/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDavConfig {
    /// Server base URL or a calendar collection URL.
    pub url: String,
    pub username: String,
    pub password: String,
    /// The calendar collection to use (absolute URL). Filled by discovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_url: Option<String>,
}

impl CalDavConfig {
    pub fn calendar(&self) -> anyhow::Result<url::Url> {
        let raw = self.calendar_url.as_deref().unwrap_or(&self.url);
        let mut u = url::Url::parse(raw).map_err(|e| anyhow::anyhow!("invalid calendar url: {e}"))?;
        if !u.path().ends_with('/') {
            u.set_path(&format!("{}/", u.path()));
        }
        Ok(u)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiscoveredCalendar {
    pub url: String,
    pub name: String,
}

pub struct Client<'a> {
    http: &'a reqwest::Client,
    cfg: &'a CalDavConfig,
}

impl<'a> Client<'a> {
    pub fn new(http: &'a reqwest::Client, cfg: &'a CalDavConfig) -> Self {
        Self { http, cfg }
    }

    fn req(&self, method: &str, url: &str) -> reqwest::RequestBuilder {
        self.http
            .request(Method::from_bytes(method.as_bytes()).expect("method"), url)
            .basic_auth(&self.cfg.username, Some(&self.cfg.password))
    }

    async fn xml(&self, method: &str, url: &str, depth: &str, body: &str) -> anyhow::Result<(StatusCode, String)> {
        let resp = self
            .req(method, url)
            .header("Depth", depth)
            .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{method} {url} failed: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            anyhow::bail!("{method} {url}: authentication failed (HTTP {status})");
        }
        Ok((status, text))
    }

    /// Find calendar collections for the configured account. Works when
    /// `url` is the server root, a principal, a calendar home or a calendar.
    pub async fn discover(&self) -> anyhow::Result<Vec<DiscoveredCalendar>> {
        let base = url::Url::parse(&self.cfg.url).map_err(|e| anyhow::anyhow!("invalid url: {e}"))?;

        // 1. Is the URL itself a calendar (or a folder of calendars)?
        let direct = self.list_calendars(base.as_str()).await.unwrap_or_default();
        if !direct.is_empty() {
            return Ok(direct);
        }

        // 2. Well-known → principal → calendar home.
        let mut principal = self.propfind_href(base.as_str(), "current-user-principal").await;
        if principal.is_none() {
            let wk = base.join("/.well-known/caldav").unwrap();
            principal = self.propfind_href(wk.as_str(), "current-user-principal").await;
        }
        let principal = principal.ok_or_else(|| anyhow::anyhow!("could not find the CalDAV principal for this account"))?;
        let principal_url = base.join(&principal)?;
        let home = self
            .propfind_href(principal_url.as_str(), "calendar-home-set")
            .await
            .ok_or_else(|| anyhow::anyhow!("could not find the calendar home"))?;
        let home_url = base.join(&home)?;
        self.list_calendars(home_url.as_str()).await
    }

    async fn propfind_href(&self, url: &str, prop: &str) -> Option<String> {
        let ns = if prop == "calendar-home-set" { "C" } else { "D" };
        let body = format!(
            r#"<?xml version="1.0" encoding="utf-8"?><D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><D:prop><{ns}:{prop}/></D:prop></D:propfind>"#
        );
        let (status, text) = self.xml("PROPFIND", url, "0", &body).await.ok()?;
        if !status.is_success() {
            return None;
        }
        let doc = roxmltree::Document::parse(&text).ok()?;
        doc.descendants()
            .find(|n| n.is_element() && n.tag_name().name() == prop)
            .and_then(|n| n.descendants().find(|h| h.is_element() && h.tag_name().name() == "href"))
            .and_then(|h| h.text().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
    }

    async fn list_calendars(&self, url: &str) -> anyhow::Result<Vec<DiscoveredCalendar>> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?><D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><D:prop><D:resourcetype/><D:displayname/><C:supported-calendar-component-set/></D:prop></D:propfind>"#;
        let (status, text) = self.xml("PROPFIND", url, "1", body).await?;
        if !status.is_success() && status != StatusCode::MULTI_STATUS {
            anyhow::bail!("PROPFIND {url} returned HTTP {status}");
        }
        let base = url::Url::parse(url)?;
        let doc = roxmltree::Document::parse(&text).map_err(|e| anyhow::anyhow!("bad XML from server: {e}"))?;
        let mut out = Vec::new();
        for resp in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "response") {
            let href = resp
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "href")
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string());
            let Some(href) = href else { continue };
            let is_calendar = resp
                .descendants()
                .any(|n| n.is_element() && n.tag_name().name() == "calendar" && n.tag_name().namespace() == Some(CALDAV));
            if !is_calendar {
                continue;
            }
            let supports_events = resp
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "comp")
                .map(|n| n.attribute("name").unwrap_or("").to_string())
                .collect::<Vec<_>>();
            if !supports_events.is_empty() && !supports_events.iter().any(|c| c == "VEVENT") {
                continue;
            }
            let name = resp
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "displayname" && n.tag_name().namespace() == Some(DAV))
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| href.trim_end_matches('/').rsplit('/').next().unwrap_or("calendar").to_string());
            let abs = base.join(&href)?;
            out.push(DiscoveredCalendar { url: abs.to_string(), name });
        }
        Ok(out)
    }

    /// The collection's change tag, if the server provides one.
    pub async fn ctag(&self) -> anyhow::Result<Option<String>> {
        let url = self.cfg.calendar()?;
        let body = r#"<?xml version="1.0" encoding="utf-8"?><D:propfind xmlns:D="DAV:" xmlns:CS="http://calendarserver.org/ns/"><D:prop><CS:getctag/><D:sync-token/></D:prop></D:propfind>"#;
        let (status, text) = self.xml("PROPFIND", url.as_str(), "0", body).await?;
        if !status.is_success() && status != StatusCode::MULTI_STATUS {
            anyhow::bail!("PROPFIND {url} returned HTTP {status}");
        }
        let doc = roxmltree::Document::parse(&text)?;
        let tag = doc
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "getctag" && n.tag_name().namespace() == Some(CS))
            .or_else(|| doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "sync-token"))
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Ok(tag)
    }

    /// Fetch every VEVENT in the calendar as iCalendar text blocks.
    pub async fn fetch_events(&self) -> anyhow::Result<Vec<(String, String)>> {
        let url = self.cfg.calendar()?;
        let body = r#"<?xml version="1.0" encoding="utf-8"?><C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><D:prop><D:getetag/><C:calendar-data/></D:prop><C:filter><C:comp-filter name="VCALENDAR"><C:comp-filter name="VEVENT"/></C:comp-filter></C:filter></C:calendar-query>"#;
        let (status, text) = self.xml("REPORT", url.as_str(), "1", body).await?;
        if status != StatusCode::MULTI_STATUS && !status.is_success() {
            anyhow::bail!("REPORT {url} returned HTTP {status}");
        }
        let doc = roxmltree::Document::parse(&text).map_err(|e| anyhow::anyhow!("bad XML from server: {e}"))?;
        let mut out = Vec::new();
        for resp in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "response") {
            let href =
                resp.children().find(|n| n.is_element() && n.tag_name().name() == "href").and_then(|n| n.text()).unwrap_or("");
            if let Some(data) =
                resp.descendants().find(|n| n.is_element() && n.tag_name().name() == "calendar-data").and_then(|n| n.text())
            {
                out.push((href.to_string(), data.to_string()));
            }
        }
        Ok(out)
    }

    /// Create or replace one event resource. Returns the new ETag if given.
    pub async fn put_event(&self, href: &str, ics: &str) -> anyhow::Result<Option<String>> {
        let resp = self
            .req("PUT", href)
            .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
            .body(ics.to_string())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("PUT {href} failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("PUT {href} returned HTTP {status}");
        }
        Ok(resp.headers().get(header::ETAG).and_then(|v| v.to_str().ok()).map(String::from))
    }

    pub async fn delete(&self, href: &str) -> anyhow::Result<()> {
        let resp = self.req("DELETE", href).send().await.map_err(|e| anyhow::anyhow!("DELETE {href} failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            anyhow::bail!("DELETE {href} returned HTTP {status}");
        }
        Ok(())
    }

    /// Href for a mirrored event resource inside the calendar collection.
    pub fn event_href(&self, uid: &str) -> anyhow::Result<String> {
        Ok(self.cfg.calendar()?.join(&format!("{uid}.ics"))?.to_string())
    }
}

/// Validate and normalize a CalDAV config from user input.
pub fn validate_config(config: &serde_json::Value) -> anyhow::Result<CalDavConfig> {
    let mut cfg: CalDavConfig = serde_json::from_value(config.clone()).map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    cfg.url = cfg.url.trim().to_string();
    let parsed = url::Url::parse(&cfg.url).map_err(|e| anyhow::anyhow!("invalid url: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!("url must use http or https");
    }
    if cfg.username.trim().is_empty() {
        anyhow::bail!("username is required");
    }
    if let Some(c) = &cfg.calendar_url {
        url::Url::parse(c).map_err(|e| anyhow::anyhow!("invalid calendar url: {e}"))?;
    }
    Ok(cfg)
}
