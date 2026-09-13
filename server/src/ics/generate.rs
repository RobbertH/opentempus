//! Render shared events as an iCalendar feed that Google Calendar, Apple
//! Calendar, Outlook and friends can subscribe to.

use chrono::{DateTime, Utc};

use crate::models::Rsvp;
use crate::sharing::SharedEvent;

pub struct FeedOptions<'a> {
    pub calendar_name: &'a str,
    pub host: &'a str,
    pub now: DateTime<Utc>,
}

pub fn render_feed(events: &[SharedEvent], opts: &FeedOptions<'_>) -> String {
    let mut out = String::new();
    line(&mut out, "BEGIN:VCALENDAR");
    line(&mut out, "VERSION:2.0");
    line(&mut out, &format!("PRODID:-//OpenTempus//{}//EN", env!("CARGO_PKG_VERSION")));
    line(&mut out, "CALSCALE:GREGORIAN");
    line(&mut out, "METHOD:PUBLISH");
    line(&mut out, &format!("X-WR-CALNAME:{}", escape_text(opts.calendar_name)));
    line(&mut out, "X-PUBLISHED-TTL:PT15M");
    let dtstamp = opts.now.format("%Y%m%dT%H%M%SZ").to_string();

    for ev in events {
        line(&mut out, "BEGIN:VEVENT");
        line(&mut out, &format!("UID:{}@{}", ev.id, opts.host));
        line(&mut out, &format!("DTSTAMP:{dtstamp}"));
        if ev.all_day {
            line(&mut out, &format!("DTSTART;VALUE=DATE:{}", ev.start.format("%Y%m%d")));
            line(&mut out, &format!("DTEND;VALUE=DATE:{}", ev.end.format("%Y%m%d")));
        } else {
            line(&mut out, &format!("DTSTART:{}", ev.start.format("%Y%m%dT%H%M%SZ")));
            line(&mut out, &format!("DTEND:{}", ev.end.format("%Y%m%dT%H%M%SZ")));
        }
        let summary = ev.title.clone().unwrap_or_else(|| if ev.busy { "Busy".into() } else { "Free".into() });
        line(&mut out, &format!("SUMMARY:{}", escape_text(&summary)));

        let mut description_parts: Vec<String> = Vec::new();
        if let Some(d) = &ev.description {
            description_parts.push(d.clone());
        }
        if let Some(o) = &ev.origin_calendar {
            description_parts.push(format!("Calendar: {o}"));
        }
        if let Some(c) = &ev.category {
            description_parts.push(format!("Category: {c}"));
            line(&mut out, &format!("CATEGORIES:{}", escape_text(c)));
            line(&mut out, &format!("X-OPENTEMPUS-CATEGORY:{}", escape_text(c)));
        }
        if let Some(r) = &ev.rsvp {
            description_parts.push(format!("RSVP: {}", rsvp_label(*r)));
            line(&mut out, &format!("X-OPENTEMPUS-RSVP:{}", rsvp_label(*r)));
        }
        if !description_parts.is_empty() {
            line(&mut out, &format!("DESCRIPTION:{}", escape_text(&description_parts.join("\n"))));
        }
        if let Some(l) = &ev.location {
            line(&mut out, &format!("LOCATION:{}", escape_text(l)));
        }
        line(&mut out, if ev.busy { "TRANSP:OPAQUE" } else { "TRANSP:TRANSPARENT" });
        let status = match ev.rsvp {
            Some(Rsvp::Tentative) | Some(Rsvp::NeedsAction) => "TENTATIVE",
            _ => "CONFIRMED",
        };
        line(&mut out, &format!("STATUS:{status}"));
        line(&mut out, "END:VEVENT");
    }
    line(&mut out, "END:VCALENDAR");
    out
}

/// Render one shared event as a standalone iCalendar object suitable for a
/// CalDAV PUT. The result is deterministic for the same input (no DTSTAMP
/// variation) so callers can hash it to detect changes.
pub fn render_mirror_object(ev: &SharedEvent, placeholder_title: &str, host: &str) -> String {
    let mut out = String::new();
    line(&mut out, "BEGIN:VCALENDAR");
    line(&mut out, "VERSION:2.0");
    line(&mut out, &format!("PRODID:-//OpenTempus//{}//EN", env!("CARGO_PKG_VERSION")));
    line(&mut out, "BEGIN:VEVENT");
    line(&mut out, &format!("UID:{}", mirror_uid(ev.id, host)));
    line(&mut out, &format!("DTSTAMP:{}", ev.start.format("%Y%m%dT%H%M%SZ")));
    line(&mut out, &format!("{}:1", crate::ics::parse::MIRROR_PROPERTY));
    if ev.all_day {
        line(&mut out, &format!("DTSTART;VALUE=DATE:{}", ev.start.format("%Y%m%d")));
        line(&mut out, &format!("DTEND;VALUE=DATE:{}", ev.end.format("%Y%m%d")));
    } else {
        line(&mut out, &format!("DTSTART:{}", ev.start.format("%Y%m%dT%H%M%SZ")));
        line(&mut out, &format!("DTEND:{}", ev.end.format("%Y%m%dT%H%M%SZ")));
    }
    let summary = ev.title.clone().unwrap_or_else(|| placeholder_title.to_string());
    line(&mut out, &format!("SUMMARY:{}", escape_text(&summary)));
    let mut description_parts: Vec<String> = Vec::new();
    if let Some(d) = &ev.description {
        description_parts.push(d.clone());
    }
    if let Some(o) = &ev.origin_calendar {
        description_parts.push(format!("Calendar: {o}"));
    }
    if let Some(c) = &ev.category {
        description_parts.push(format!("Category: {c}"));
        line(&mut out, &format!("CATEGORIES:{}", escape_text(c)));
    }
    if let Some(r) = &ev.rsvp {
        description_parts.push(format!("RSVP: {}", rsvp_label(*r)));
    }
    if !description_parts.is_empty() {
        line(&mut out, &format!("DESCRIPTION:{}", escape_text(&description_parts.join("\n"))));
    }
    if let Some(l) = &ev.location {
        line(&mut out, &format!("LOCATION:{}", escape_text(l)));
    }
    line(&mut out, if ev.busy { "TRANSP:OPAQUE" } else { "TRANSP:TRANSPARENT" });
    let status = match ev.rsvp {
        Some(Rsvp::Tentative) | Some(Rsvp::NeedsAction) => "TENTATIVE",
        _ => "CONFIRMED",
    };
    line(&mut out, &format!("STATUS:{status}"));
    line(&mut out, "END:VEVENT");
    line(&mut out, "END:VCALENDAR");
    out
}

pub fn mirror_uid(instance_id: uuid::Uuid, host: &str) -> String {
    format!("ot-{instance_id}@{host}")
}

fn rsvp_label(r: Rsvp) -> &'static str {
    match r {
        Rsvp::Organizer => "organizer",
        Rsvp::Accepted => "accepted",
        Rsvp::Tentative => "tentative",
        Rsvp::Declined => "declined",
        Rsvp::NeedsAction => "needs-action",
        Rsvp::Unknown => "unknown",
    }
}

/// Escape per RFC 5545 section 3.3.11.
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Write a content line, folding at 75 octets as required by RFC 5545.
fn line(out: &mut String, content: &str) {
    const LIMIT: usize = 75;
    let mut current = 0usize;
    let mut first = true;
    for ch in content.chars() {
        let len = ch.len_utf8();
        if current + len > LIMIT - if first { 0 } else { 1 } {
            out.push_str("\r\n ");
            current = 0;
            first = false;
        }
        out.push(ch);
        current += len;
    }
    out.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    #[test]
    fn renders_busy_block_without_details() {
        let ev = SharedEvent {
            id: Uuid::nil(),
            start: Utc.with_ymd_and_hms(2026, 9, 14, 7, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 9, 14, 8, 0, 0).unwrap(),
            all_day: false,
            busy: true,
            title: None,
            description: None,
            location: None,
            category: None,
            origin_calendar: None,
            rsvp: None,
        };
        let ics = render_feed(&[ev], &FeedOptions { calendar_name: "Robbert (busy)", host: "cal.test", now: Utc::now() });
        assert!(ics.contains("SUMMARY:Busy\r\n"));
        assert!(ics.contains("DTSTART:20260914T070000Z\r\n"));
        assert!(!ics.contains("LOCATION"));
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn folds_long_lines_and_escapes() {
        let ev = SharedEvent {
            id: Uuid::nil(),
            start: Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap(),
            all_day: true,
            busy: true,
            title: Some("A; very, long title ".repeat(6)),
            description: Some("l1\nl2".into()),
            location: None,
            category: Some("work".into()),
            origin_calendar: Some("Work".into()),
            rsvp: Some(Rsvp::Accepted),
        };
        let ics = render_feed(&[ev], &FeedOptions { calendar_name: "x", host: "h", now: Utc::now() });
        assert!(ics.contains("DTSTART;VALUE=DATE:20260914\r\n"));
        assert!(ics.contains("\\;"));
        assert!(ics.contains("l1\\nl2"));
        for l in ics.split("\r\n") {
            assert!(l.len() <= 75, "line too long: {l:?}");
        }
    }
}
