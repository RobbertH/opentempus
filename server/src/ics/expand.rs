//! Expand a recurring master (plus its overrides) into concrete occurrences
//! within a window.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use rrule::{RRuleSet, Tz as RTz};

use super::parse::ParsedEvent;

/// A concrete occurrence of an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
}

/// Hard cap on the number of occurrences generated for one master, to keep
/// a hostile or broken feed from exhausting the database.
pub const MAX_OCCURRENCES: u16 = 2000;

/// Expand `master` into occurrences inside `[from, to)`. Occurrences whose
/// original start matches an entry of `overridden` (the RECURRENCE-IDs of
/// override events) are omitted, since the overrides supply their own
/// occurrence.
pub fn expand(
    master: &ParsedEvent,
    overridden: &HashSet<DateTime<Utc>>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> anyhow::Result<Vec<Occurrence>> {
    let duration = master.end_at - master.start_at;

    let Some(rrule) = master.rrule.as_deref() else {
        // Non-recurring, but still honour RDATEs if present.
        let mut out = Vec::new();
        if !overridden.contains(&master.start_at) {
            out.push(occurrence(master.start_at, duration, master.all_day));
        }
        for rd in &master.rdates {
            if !overridden.contains(rd) && !master.exdates.contains(rd) {
                out.push(occurrence(*rd, duration, master.all_day));
            }
        }
        out.retain(|o| o.end_at > from && o.start_at < to);
        out.sort_by_key(|o| o.start_at);
        out.dedup();
        return Ok(out);
    };

    // Build the DTSTART line in the zone the event was defined in so that
    // DST transitions keep the local wall time.
    let dtstart_line = match (&master.timezone, master.all_day) {
        (Some(zone), false) => {
            let tz: chrono_tz::Tz = zone.parse().map_err(|_| anyhow::anyhow!("unknown zone {zone}"))?;
            let local = master.start_at.with_timezone(&tz);
            format!("DTSTART;TZID={}:{}", tz.name(), local.format("%Y%m%dT%H%M%S"))
        }
        _ => format!("DTSTART:{}", master.start_at.format("%Y%m%dT%H%M%SZ")),
    };
    let text = format!("{dtstart_line}\nRRULE:{rrule}");
    let mut set: RRuleSet = text.parse().map_err(|e| anyhow::anyhow!("invalid RRULE '{rrule}': {e}"))?;

    for rd in &master.rdates {
        set = set.rdate(rd.with_timezone(&RTz::UTC));
    }
    for ex in &master.exdates {
        set = set.exdate(ex.with_timezone(&RTz::UTC));
    }

    // Occurrences that *start* before the window but overlap it still matter,
    // so widen the lower bound by the event duration.
    let lower = (from - duration.max(Duration::zero())).with_timezone(&RTz::UTC);
    let upper = to.with_timezone(&RTz::UTC);
    let result = set.after(lower).before(upper).all(MAX_OCCURRENCES);

    let mut out: Vec<Occurrence> = result
        .dates
        .into_iter()
        .map(|d| d.with_timezone(&Utc))
        .filter(|d| !overridden.contains(d))
        .map(|d| occurrence(d, duration, master.all_day))
        .filter(|o| o.end_at > from && o.start_at < to)
        .collect();
    out.sort_by_key(|o| o.start_at);
    out.dedup();
    Ok(out)
}

fn occurrence(start: DateTime<Utc>, duration: Duration, all_day: bool) -> Occurrence {
    Occurrence { start_at: start, end_at: start + duration, all_day }
}

#[cfg(test)]
pub fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventStatus, Rsvp, Transparency};

    fn master(start: DateTime<Utc>, end: DateTime<Utc>, rrule: Option<&str>, tz: Option<&str>, all_day: bool) -> ParsedEvent {
        ParsedEvent {
            uid: "u".into(),
            recurrence_id: None,
            summary: None,
            description: None,
            location: None,
            start_at: start,
            end_at: end,
            all_day,
            timezone: tz.map(String::from),
            status: EventStatus::Confirmed,
            transparency: Transparency::Opaque,
            rsvp: Rsvp::Organizer,
            categories: vec![],
            rrule: rrule.map(String::from),
            rdates: vec![],
            exdates: vec![],
            sequence: 0,
            last_modified: None,
        }
    }

    #[test]
    fn non_recurring_yields_single_occurrence_inside_window() {
        let m = master(utc(2026, 9, 14, 9, 0), utc(2026, 9, 14, 10, 0), None, None, false);
        let occ = expand(&m, &HashSet::new(), utc(2026, 9, 1, 0, 0), utc(2026, 10, 1, 0, 0)).unwrap();
        assert_eq!(occ.len(), 1);
        let occ = expand(&m, &HashSet::new(), utc(2026, 10, 1, 0, 0), utc(2026, 11, 1, 0, 0)).unwrap();
        assert!(occ.is_empty());
    }

    #[test]
    fn weekly_rule_expands_with_exdate_and_override() {
        // 09:00 Amsterdam (07:00 UTC in CEST)
        let mut m = master(
            utc(2026, 9, 14, 7, 0),
            utc(2026, 9, 14, 7, 30),
            Some("FREQ=WEEKLY;BYDAY=MO,WE;COUNT=6"),
            Some("Europe/Amsterdam"),
            false,
        );
        m.exdates = vec![utc(2026, 9, 16, 7, 0)];
        let overridden: HashSet<_> = [utc(2026, 9, 21, 7, 0)].into_iter().collect();
        let occ = expand(&m, &overridden, utc(2026, 9, 1, 0, 0), utc(2026, 12, 1, 0, 0)).unwrap();
        let starts: Vec<_> = occ.iter().map(|o| o.start_at).collect();
        assert_eq!(starts, vec![utc(2026, 9, 14, 7, 0), utc(2026, 9, 23, 7, 0), utc(2026, 9, 28, 7, 0), utc(2026, 9, 30, 7, 0)]);
        assert!(occ.iter().all(|o| o.end_at - o.start_at == Duration::minutes(30)));
    }

    #[test]
    fn dst_transition_keeps_local_wall_time() {
        // Weekly at 09:00 Amsterdam across the October DST change (2026-10-25).
        let m = master(
            utc(2026, 10, 19, 7, 0),
            utc(2026, 10, 19, 8, 0),
            Some("FREQ=WEEKLY;COUNT=2"),
            Some("Europe/Amsterdam"),
            false,
        );
        let occ = expand(&m, &HashSet::new(), utc(2026, 10, 1, 0, 0), utc(2026, 11, 30, 0, 0)).unwrap();
        assert_eq!(occ[0].start_at, utc(2026, 10, 19, 7, 0));
        // After DST ends, 09:00 local is 08:00 UTC.
        assert_eq!(occ[1].start_at, utc(2026, 10, 26, 8, 0));
    }

    #[test]
    fn all_day_recurring() {
        let m = master(utc(2026, 9, 14, 0, 0), utc(2026, 9, 15, 0, 0), Some("FREQ=DAILY;COUNT=3"), None, true);
        let occ = expand(&m, &HashSet::new(), utc(2026, 9, 1, 0, 0), utc(2026, 10, 1, 0, 0)).unwrap();
        assert_eq!(occ.len(), 3);
        assert!(occ.iter().all(|o| o.all_day));
        assert_eq!(occ[2].start_at, utc(2026, 9, 16, 0, 0));
    }

    #[test]
    fn occurrence_starting_before_window_but_overlapping_is_included() {
        let m = master(utc(2026, 9, 13, 22, 0), utc(2026, 9, 14, 2, 0), Some("FREQ=DAILY;COUNT=1"), None, false);
        let occ = expand(&m, &HashSet::new(), utc(2026, 9, 14, 0, 0), utc(2026, 9, 15, 0, 0)).unwrap();
        assert_eq!(occ.len(), 1);
    }

    #[test]
    fn invalid_rrule_is_an_error() {
        let m = master(utc(2026, 9, 14, 9, 0), utc(2026, 9, 14, 10, 0), Some("FREQ=SOMETIMES"), None, false);
        assert!(expand(&m, &HashSet::new(), utc(2026, 9, 1, 0, 0), utc(2026, 10, 1, 0, 0)).is_err());
    }
}
