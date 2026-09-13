//! Parse an iCalendar document into normalized events.

use std::collections::HashMap;
use std::io::BufReader;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use ical::parser::ical::component::IcalEvent;
use ical::property::Property;

use crate::models::{EventStatus, Rsvp, Transparency};

use super::tz;

/// A VEVENT normalized into the shape we store.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEvent {
    pub uid: String,
    pub recurrence_id: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    /// IANA name of the zone the event was defined in, if any.
    pub timezone: Option<String>,
    pub status: EventStatus,
    pub transparency: Transparency,
    pub rsvp: Rsvp,
    pub categories: Vec<String>,
    pub rrule: Option<String>,
    pub rdates: Vec<DateTime<Utc>>,
    pub exdates: Vec<DateTime<Utc>>,
    pub sequence: i32,
    pub last_modified: Option<DateTime<Utc>>,
}

/// Options that influence how ambiguous data is interpreted.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Email addresses that identify the calendar owner, lower-cased. Used to
    /// find the owner's PARTSTAT among attendees.
    pub owner_emails: Vec<String>,
    /// Zone used for floating (zone-less) date-times.
    pub default_tz: Tz,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self { owner_emails: Vec::new(), default_tz: chrono_tz::UTC }
    }
}

#[derive(Debug, Default)]
pub struct ParseReport {
    pub events: Vec<ParsedEvent>,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

pub fn parse_ics(input: &str, opts: &ParseOptions) -> anyhow::Result<ParseReport> {
    // Normalize line endings; some producers emit bare LF.
    let normalized = input.replace("\r\n", "\n").replace('\n', "\r\n");
    let reader = BufReader::new(normalized.as_bytes());
    let parser = ical::IcalParser::new(reader);

    let mut report = ParseReport::default();
    let mut found_calendar = false;
    for cal in parser {
        let cal = cal.map_err(|e| anyhow::anyhow!("invalid iCalendar data: {e}"))?;
        found_calendar = true;
        let mut opts = opts.clone();
        if let Some(wr_tz) = prop_value(&cal.properties, "X-WR-TIMEZONE").and_then(|v| tz::resolve(&v)) {
            opts.default_tz = wr_tz;
        }
        for ev in &cal.events {
            match parse_event(ev, &opts) {
                Ok(Some(e)) => report.events.push(e),
                Ok(None) => report.skipped += 1,
                Err(e) => {
                    report.skipped += 1;
                    report.warnings.push(e.to_string());
                }
            }
        }
    }
    if !found_calendar {
        anyhow::bail!("no VCALENDAR component found");
    }
    Ok(report)
}

fn parse_event(ev: &IcalEvent, opts: &ParseOptions) -> anyhow::Result<Option<ParsedEvent>> {
    let props = &ev.properties;
    let Some(uid) = prop_value(props, "UID") else {
        return Ok(None);
    };

    let dtstart = find_prop(props, "DTSTART").ok_or_else(|| anyhow::anyhow!("event {uid} has no DTSTART"))?;
    let (start_at, all_day, timezone) = parse_datetime_prop(dtstart, opts)?;

    let end_at = if let Some(dtend) = find_prop(props, "DTEND") {
        parse_datetime_prop(dtend, opts)?.0
    } else if let Some(duration) = prop_value(props, "DURATION") {
        start_at + parse_duration(&duration)?
    } else if all_day {
        start_at + Duration::days(1)
    } else {
        start_at
    };
    // Zero-length all-day events are not valid; guard against negative spans.
    let end_at = if end_at < start_at { start_at } else { end_at };

    let recurrence_id = match find_prop(props, "RECURRENCE-ID") {
        Some(p) => Some(parse_datetime_prop(p, opts)?.0),
        None => None,
    };

    let status = match prop_value(props, "STATUS").as_deref().map(|s| s.to_ascii_uppercase()) {
        Some(ref s) if s == "TENTATIVE" => EventStatus::Tentative,
        Some(ref s) if s == "CANCELLED" => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    };

    let mut transparency = match prop_value(props, "TRANSP").as_deref().map(|s| s.to_ascii_uppercase()) {
        Some(ref s) if s == "TRANSPARENT" => Transparency::Transparent,
        _ => Transparency::Opaque,
    };
    let mut status = status;
    // Outlook expresses free/busy through a vendor property.
    if let Some(busy) = prop_value(props, "X-MICROSOFT-CDO-BUSYSTATUS") {
        match busy.to_ascii_uppercase().as_str() {
            "FREE" => transparency = Transparency::Transparent,
            "TENTATIVE" if status == EventStatus::Confirmed => status = EventStatus::Tentative,
            _ => {}
        }
    }

    let rsvp = owner_rsvp(props, &opts.owner_emails);

    let categories = props
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case("CATEGORIES"))
        .filter_map(|p| p.value.as_ref())
        .flat_map(|v| split_unescaped(v, ','))
        .map(|s| unescape_text(&s))
        .filter(|s| !s.is_empty())
        .collect();

    let rrule = prop_value(props, "RRULE").map(|r| r.trim().to_string()).filter(|r| !r.is_empty());
    let rdates = parse_date_list(props, "RDATE", opts);
    let exdates = parse_date_list(props, "EXDATE", opts);

    let sequence = prop_value(props, "SEQUENCE").and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let last_modified = find_prop(props, "LAST-MODIFIED").and_then(|p| parse_datetime_prop(p, opts).ok()).map(|(dt, _, _)| dt);

    Ok(Some(ParsedEvent {
        uid,
        recurrence_id,
        summary: prop_value(props, "SUMMARY").map(|s| unescape_text(&s)).filter(|s| !s.is_empty()),
        description: prop_value(props, "DESCRIPTION").map(|s| unescape_text(&s)).filter(|s| !s.is_empty()),
        location: prop_value(props, "LOCATION").map(|s| unescape_text(&s)).filter(|s| !s.is_empty()),
        start_at,
        end_at,
        all_day,
        timezone,
        status,
        transparency,
        rsvp,
        categories,
        rrule,
        rdates,
        exdates,
        sequence,
        last_modified,
    }))
}

fn owner_rsvp(props: &[Property], owner_emails: &[String]) -> Rsvp {
    let attendees: Vec<&Property> = props.iter().filter(|p| p.name.eq_ignore_ascii_case("ATTENDEE")).collect();
    let organizer = props.iter().find(|p| p.name.eq_ignore_ascii_case("ORGANIZER"));

    let is_owner = |value: &str| {
        let addr = value.trim().trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_ascii_lowercase();
        owner_emails.iter().any(|e| e == &addr)
    };

    if let Some(org) = organizer {
        if org.value.as_deref().map(is_owner).unwrap_or(false) {
            return Rsvp::Organizer;
        }
    }
    if attendees.is_empty() {
        // An event in my own calendar without an attendee list is mine.
        return Rsvp::Organizer;
    }
    for att in attendees {
        if att.value.as_deref().map(is_owner).unwrap_or(false) {
            let partstat = param(att, "PARTSTAT").map(|s| s.to_ascii_uppercase());
            return match partstat.as_deref() {
                Some("ACCEPTED") => Rsvp::Accepted,
                Some("TENTATIVE") => Rsvp::Tentative,
                Some("DECLINED") => Rsvp::Declined,
                Some("NEEDS-ACTION") => Rsvp::NeedsAction,
                _ => Rsvp::Unknown,
            };
        }
    }
    Rsvp::Unknown
}

fn parse_date_list(props: &[Property], name: &str, opts: &ParseOptions) -> Vec<DateTime<Utc>> {
    let mut out = Vec::new();
    for p in props.iter().filter(|p| p.name.eq_ignore_ascii_case(name)) {
        let Some(value) = p.value.as_ref() else { continue };
        let is_date = param(p, "VALUE").map(|v| v.eq_ignore_ascii_case("DATE")).unwrap_or(false);
        let zone = param(p, "TZID").and_then(|t| tz::resolve(&t));
        for raw in value.split(',') {
            // RDATE may carry PERIOD values ("start/end"); keep the start.
            let raw = raw.split('/').next().unwrap_or(raw).trim();
            if raw.is_empty() {
                continue;
            }
            if let Ok((dt, _)) = parse_datetime_value(raw, is_date, zone, opts.default_tz) {
                out.push(dt);
            }
        }
    }
    out
}

/// Returns (instant, is_all_day, iana zone name).
fn parse_datetime_prop(p: &Property, opts: &ParseOptions) -> anyhow::Result<(DateTime<Utc>, bool, Option<String>)> {
    let value = p.value.as_deref().ok_or_else(|| anyhow::anyhow!("{} has no value", p.name))?;
    let is_date = param(p, "VALUE").map(|v| v.eq_ignore_ascii_case("DATE")).unwrap_or(false) || value.len() == 8;
    let tzid = param(p, "TZID");
    let zone = tzid.as_deref().and_then(tz::resolve);
    let (dt, zone_used) = parse_datetime_value(value.trim(), is_date, zone, opts.default_tz)?;
    let name = if is_date { None } else { zone_used.map(|z| z.name().to_string()) };
    Ok((dt, is_date, name))
}

/// Parse an iCalendar DATE or DATE-TIME value. Returns the instant and the
/// zone that was used to interpret a local time (None for UTC/date values).
fn parse_datetime_value(
    value: &str,
    is_date: bool,
    zone: Option<Tz>,
    default_tz: Tz,
) -> anyhow::Result<(DateTime<Utc>, Option<Tz>)> {
    if is_date || value.len() == 8 {
        let d = NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|e| anyhow::anyhow!("bad date {value}: {e}"))?;
        let dt = Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap());
        return Ok((dt, None));
    }
    if let Some(stripped) = value.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S")
            .map_err(|e| anyhow::anyhow!("bad date-time {value}: {e}"))?;
        return Ok((Utc.from_utc_datetime(&naive), None));
    }
    let naive =
        NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").map_err(|e| anyhow::anyhow!("bad date-time {value}: {e}"))?;
    let z = zone.unwrap_or(default_tz);
    let local = z
        .from_local_datetime(&naive)
        .single()
        .or_else(|| z.from_local_datetime(&naive).earliest())
        .ok_or_else(|| anyhow::anyhow!("nonexistent local time {value} in {}", z.name()))?;
    Ok((local.with_timezone(&Utc), Some(z)))
}

/// Parse an RFC 5545 DURATION such as `P1DT2H30M`, `-PT15M` or `P2W`.
pub fn parse_duration(s: &str) -> anyhow::Result<Duration> {
    let s = s.trim();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let rest = rest.strip_prefix('P').ok_or_else(|| anyhow::anyhow!("bad duration {s}"))?;
    let mut total = Duration::zero();
    let mut num = String::new();
    let mut in_time = false;
    for c in rest.chars() {
        match c {
            '0'..='9' => num.push(c),
            'T' => in_time = true,
            'W' | 'D' | 'H' | 'M' | 'S' => {
                let n: i64 = num.parse().map_err(|_| anyhow::anyhow!("bad duration {s}"))?;
                num.clear();
                total += match c {
                    'W' => Duration::weeks(n),
                    'D' => Duration::days(n),
                    'H' => Duration::hours(n),
                    'M' if in_time => Duration::minutes(n),
                    'S' => Duration::seconds(n),
                    _ => anyhow::bail!("bad duration {s}"),
                };
            }
            _ => anyhow::bail!("bad duration {s}"),
        }
    }
    Ok(if neg { -total } else { total })
}

fn find_prop<'a>(props: &'a [Property], name: &str) -> Option<&'a Property> {
    props.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

fn prop_value(props: &[Property], name: &str) -> Option<String> {
    find_prop(props, name).and_then(|p| p.value.clone())
}

fn param(p: &Property, name: &str) -> Option<String> {
    p.params.as_ref()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).and_then(|(_, v)| v.first().cloned())
}

/// Split on a separator unless it is escaped with a backslash.
fn split_unescaped(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            cur.push(c);
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if c == sep {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

/// Undo RFC 5545 TEXT escaping.
pub fn unescape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Return a map of `uid -> (master, overrides)` grouping.
pub fn group_by_uid(events: Vec<ParsedEvent>) -> HashMap<String, Vec<ParsedEvent>> {
    let mut map: HashMap<String, Vec<ParsedEvent>> = HashMap::new();
    for e in events {
        map.entry(e.uid.clone()).or_default().push(e);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test//EN\r\nX-WR-TIMEZONE:Europe/Amsterdam\r\n\
BEGIN:VEVENT\r\nUID:one@example.com\r\nDTSTART;TZID=Europe/Amsterdam:20260914T090000\r\nDTEND;TZID=Europe/Amsterdam:20260914T093000\r\n\
SUMMARY:Daily standup\\, team A\r\nDESCRIPTION:Line one\\nLine two\r\nLOCATION:Room 1\r\nRRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR\r\n\
EXDATE;TZID=Europe/Amsterdam:20260916T090000\r\nORGANIZER:mailto:boss@example.com\r\n\
ATTENDEE;PARTSTAT=TENTATIVE;CN=Me:mailto:me@example.com\r\nCATEGORIES:Work,Meetings\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:two@example.com\r\nDTSTART;VALUE=DATE:20260920\r\nSUMMARY:Holiday\r\nTRANSP:TRANSPARENT\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:three@example.com\r\nDTSTART:20260921T120000Z\r\nDURATION:PT45M\r\nSUMMARY:Call\r\nSTATUS:CANCELLED\r\n\
X-MICROSOFT-CDO-BUSYSTATUS:FREE\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    fn opts() -> ParseOptions {
        ParseOptions { owner_emails: vec!["me@example.com".into()], default_tz: chrono_tz::UTC }
    }

    #[test]
    fn parses_timed_allday_and_duration_events() {
        let report = parse_ics(SAMPLE, &opts()).unwrap();
        assert_eq!(report.events.len(), 3, "{:?}", report.warnings);
        let one = &report.events[0];
        assert_eq!(one.summary.as_deref(), Some("Daily standup, team A"));
        assert_eq!(one.description.as_deref(), Some("Line one\nLine two"));
        // 09:00 Amsterdam in September is 07:00 UTC
        assert_eq!(one.start_at, Utc.with_ymd_and_hms(2026, 9, 14, 7, 0, 0).unwrap());
        assert_eq!(one.end_at, Utc.with_ymd_and_hms(2026, 9, 14, 7, 30, 0).unwrap());
        assert_eq!(one.timezone.as_deref(), Some("Europe/Amsterdam"));
        assert_eq!(one.rsvp, Rsvp::Tentative);
        assert_eq!(one.rrule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"));
        assert_eq!(one.exdates, vec![Utc.with_ymd_and_hms(2026, 9, 16, 7, 0, 0).unwrap()]);
        assert_eq!(one.categories, vec!["Work", "Meetings"]);

        let two = &report.events[1];
        assert!(two.all_day);
        assert_eq!(two.start_at, Utc.with_ymd_and_hms(2026, 9, 20, 0, 0, 0).unwrap());
        assert_eq!(two.end_at, Utc.with_ymd_and_hms(2026, 9, 21, 0, 0, 0).unwrap());
        assert_eq!(two.transparency, Transparency::Transparent);
        assert_eq!(two.rsvp, Rsvp::Organizer);

        let three = &report.events[2];
        assert_eq!(three.end_at - three.start_at, Duration::minutes(45));
        assert_eq!(three.status, EventStatus::Cancelled);
        assert_eq!(three.transparency, Transparency::Transparent);
    }

    #[test]
    fn floating_times_use_calendar_default_zone() {
        let ics = "BEGIN:VCALENDAR\r\nX-WR-TIMEZONE:America/New_York\r\nBEGIN:VEVENT\r\nUID:x\r\nDTSTART:20260701T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let report = parse_ics(ics, &opts()).unwrap();
        assert_eq!(report.events[0].start_at, Utc.with_ymd_and_hms(2026, 7, 1, 13, 0, 0).unwrap());
    }

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("PT1H30M").unwrap(), Duration::minutes(90));
        assert_eq!(parse_duration("P2W").unwrap(), Duration::weeks(2));
        assert_eq!(parse_duration("P1DT12H").unwrap(), Duration::hours(36));
        assert_eq!(parse_duration("-PT15M").unwrap(), -Duration::minutes(15));
        assert!(parse_duration("1H").is_err());
    }

    #[test]
    fn organizer_wins_over_attendee() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\nDTSTART:20260701T090000Z\r\nORGANIZER:mailto:ME@example.com\r\nATTENDEE;PARTSTAT=DECLINED:mailto:me@example.com\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let report = parse_ics(ics, &opts()).unwrap();
        assert_eq!(report.events[0].rsvp, Rsvp::Organizer);
    }

    #[test]
    fn rejects_non_calendar_input() {
        assert!(parse_ics("<html>not a calendar</html>", &opts()).is_err());
    }
}
