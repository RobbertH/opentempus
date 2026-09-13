//! The sharing model: what a share *includes* (filters) and what the audience
//! may *see* of each included event (visibility).
//!
//! A share is a pure projection over the owner's normalized events, evaluated
//! on every read. Nothing is copied, so changing a rule takes effect
//! immediately for every consumer (ICS feed, JSON API, friend view).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Rsvp, Transparency};

/// Which event fields the audience of a share is allowed to see.
///
/// Everything defaults to hidden; the minimal share is "busy blocks only".
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Visibility {
    pub title: bool,
    pub description: bool,
    pub location: bool,
    /// The user-assigned category of the origin calendar (work/personal/...).
    pub category: bool,
    /// The name of the calendar the event came from.
    pub origin_calendar: bool,
    /// Whether the owner accepted / tentatively accepted / has not answered.
    pub rsvp: bool,
    /// Whether the event marks the owner busy or free (transparency).
    pub free_busy: bool,
}

impl Visibility {
    pub const BUSY_ONLY: Visibility = Visibility {
        title: false,
        description: false,
        location: false,
        category: false,
        origin_calendar: false,
        rsvp: false,
        free_busy: false,
    };
    pub const CATEGORY: Visibility = Visibility { category: true, origin_calendar: true, ..Self::BUSY_ONLY };
    pub const DETAILS: Visibility = Visibility { title: true, location: true, rsvp: true, free_busy: true, ..Self::CATEGORY };
    pub const FULL: Visibility = Visibility { description: true, ..Self::DETAILS };

    pub fn preset(name: &str) -> Option<Visibility> {
        match name {
            "busy_only" => Some(Self::BUSY_ONLY),
            "category" => Some(Self::CATEGORY),
            "details" => Some(Self::DETAILS),
            "full" => Some(Self::FULL),
            _ => None,
        }
    }
}

/// Which of the owner's events are included in a share at all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Filters {
    /// Restrict to these calendar sources. `None` means all sources.
    pub source_ids: Option<Vec<Uuid>>,
    /// Restrict to these categories (matched against the source category).
    /// `None` means all categories.
    pub categories: Option<Vec<String>>,
    /// Only include events whose RSVP status is in this list.
    pub rsvp: Vec<Rsvp>,
    /// Include events that do not block time (transparent / "free").
    pub include_free: bool,
    /// Include all-day events.
    pub include_all_day: bool,
    /// Include events marked tentative by the organizer.
    pub include_tentative: bool,
    /// How many days into the past the audience may look.
    pub horizon_past_days: i64,
    /// How many days into the future the audience may look.
    pub horizon_future_days: i64,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            source_ids: None,
            categories: None,
            rsvp: vec![Rsvp::Organizer, Rsvp::Accepted, Rsvp::Tentative, Rsvp::NeedsAction, Rsvp::Unknown],
            include_free: false,
            include_all_day: true,
            include_tentative: true,
            horizon_past_days: 7,
            horizon_future_days: 90,
        }
    }
}

impl Filters {
    /// Clamp a requested window to the horizon this share permits.
    pub fn clamp_window(&self, from: DateTime<Utc>, to: DateTime<Utc>, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let min = now - Duration::days(self.horizon_past_days.max(0));
        let max = now + Duration::days(self.horizon_future_days.max(0));
        (from.max(min), to.min(max))
    }
}

/// A fully normalized event occurrence as read from the owner's store, before
/// projection. This is the input to [`project`].
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OwnerEvent {
    pub instance_id: Uuid,
    pub event_id: Uuid,
    pub source_id: Uuid,
    pub source_name: String,
    pub source_category: String,
    pub uid: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: crate::models::EventStatus,
    pub transparency: Transparency,
    pub rsvp: Rsvp,
}

/// What a consumer of a share receives for one event occurrence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedEvent {
    /// Stable identifier for this occurrence; safe to expose (random UUID).
    pub id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    /// Always present: the whole point of sharing is knowing when time is taken.
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_calendar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsvp: Option<Rsvp>,
}

/// Decide whether an owner event passes the share's filters.
pub fn included(filters: &Filters, ev: &OwnerEvent) -> bool {
    if ev.status == crate::models::EventStatus::Cancelled {
        return false;
    }
    if !filters.include_tentative && ev.status == crate::models::EventStatus::Tentative {
        return false;
    }
    if let Some(ids) = &filters.source_ids {
        if !ids.contains(&ev.source_id) {
            return false;
        }
    }
    if let Some(cats) = &filters.categories {
        if !cats.iter().any(|c| c.eq_ignore_ascii_case(&ev.source_category)) {
            return false;
        }
    }
    if !filters.rsvp.contains(&ev.rsvp) {
        return false;
    }
    if !filters.include_free && ev.transparency == Transparency::Transparent {
        return false;
    }
    if !filters.include_all_day && ev.all_day {
        return false;
    }
    true
}

/// Project an owner event through a visibility mask.
pub fn project(vis: &Visibility, ev: &OwnerEvent) -> SharedEvent {
    let busy = ev.transparency == Transparency::Opaque && ev.rsvp != Rsvp::Declined;
    SharedEvent {
        id: ev.instance_id,
        start: ev.start_at,
        end: ev.end_at,
        all_day: ev.all_day,
        // When free/busy is hidden, everything included is reported as busy.
        busy: if vis.free_busy { busy } else { true },
        title: if vis.title { ev.summary.clone() } else { None },
        description: if vis.description { ev.description.clone() } else { None },
        location: if vis.location { ev.location.clone() } else { None },
        category: if vis.category { Some(ev.source_category.clone()) } else { None },
        origin_calendar: if vis.origin_calendar { Some(ev.source_name.clone()) } else { None },
        rsvp: if vis.rsvp { Some(ev.rsvp) } else { None },
    }
}

/// Apply filters and visibility to a list of owner events.
pub fn project_all(vis: &Visibility, filters: &Filters, events: &[OwnerEvent]) -> Vec<SharedEvent> {
    let mut out: Vec<SharedEvent> = events.iter().filter(|e| included(filters, e)).map(|e| project(vis, e)).collect();
    out.sort_by_key(|e| (e.start, e.end));
    out
}

/// A free slot in a window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeSlot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_minutes: i64,
}

/// Compute free slots in `[from, to)` given busy intervals from any number of
/// calendars. Overlapping busy intervals are merged first; slots shorter than
/// `min_duration` are dropped.
pub fn free_slots(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    busy: impl IntoIterator<Item = (DateTime<Utc>, DateTime<Utc>)>,
    min_duration: Duration,
) -> Vec<FreeSlot> {
    let mut intervals: Vec<(DateTime<Utc>, DateTime<Utc>)> =
        busy.into_iter().filter(|(s, e)| e > s && *e > from && *s < to).map(|(s, e)| (s.max(from), e.min(to))).collect();
    intervals.sort();

    let mut merged: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    for (s, e) in intervals {
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }

    let mut slots = Vec::new();
    let mut cursor = from;
    for (s, e) in merged {
        if s > cursor {
            push_slot(&mut slots, cursor, s, min_duration);
        }
        cursor = cursor.max(e);
    }
    if to > cursor {
        push_slot(&mut slots, cursor, to, min_duration);
    }
    slots
}

fn push_slot(slots: &mut Vec<FreeSlot>, start: DateTime<Utc>, end: DateTime<Utc>, min: Duration) {
    let dur = end - start;
    if dur >= min && dur > Duration::zero() {
        slots.push(FreeSlot { start, end, duration_minutes: dur.num_minutes() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EventStatus;
    use chrono::TimeZone;

    fn ev(rsvp: Rsvp, transparency: Transparency) -> OwnerEvent {
        OwnerEvent {
            instance_id: Uuid::new_v4(),
            event_id: Uuid::new_v4(),
            source_id: Uuid::nil(),
            source_name: "Work".into(),
            source_category: "work".into(),
            uid: "abc".into(),
            start_at: Utc.with_ymd_and_hms(2026, 9, 14, 9, 0, 0).unwrap(),
            end_at: Utc.with_ymd_and_hms(2026, 9, 14, 10, 0, 0).unwrap(),
            all_day: false,
            summary: Some("Standup".into()),
            description: Some("Daily".into()),
            location: Some("Room 1".into()),
            status: EventStatus::Confirmed,
            transparency,
            rsvp,
        }
    }

    #[test]
    fn busy_only_hides_everything() {
        let e = ev(Rsvp::Accepted, Transparency::Opaque);
        let p = project(&Visibility::BUSY_ONLY, &e);
        assert!(p.busy);
        assert_eq!(p.title, None);
        assert_eq!(p.location, None);
        assert_eq!(p.category, None);
        assert_eq!(p.origin_calendar, None);
        assert_eq!(p.rsvp, None);
    }

    #[test]
    fn category_preset_shows_category_and_origin_only() {
        let e = ev(Rsvp::Accepted, Transparency::Opaque);
        let p = project(&Visibility::CATEGORY, &e);
        assert_eq!(p.category.as_deref(), Some("work"));
        assert_eq!(p.origin_calendar.as_deref(), Some("Work"));
        assert_eq!(p.title, None);
    }

    #[test]
    fn full_preset_shows_everything() {
        let e = ev(Rsvp::Tentative, Transparency::Opaque);
        let p = project(&Visibility::FULL, &e);
        assert_eq!(p.title.as_deref(), Some("Standup"));
        assert_eq!(p.description.as_deref(), Some("Daily"));
        assert_eq!(p.rsvp, Some(Rsvp::Tentative));
    }

    #[test]
    fn declined_events_are_excluded_by_default() {
        let f = Filters::default();
        assert!(!included(&f, &ev(Rsvp::Declined, Transparency::Opaque)));
        assert!(included(&f, &ev(Rsvp::Accepted, Transparency::Opaque)));
    }

    #[test]
    fn transparent_events_excluded_unless_requested() {
        let mut f = Filters::default();
        assert!(!included(&f, &ev(Rsvp::Accepted, Transparency::Transparent)));
        f.include_free = true;
        assert!(included(&f, &ev(Rsvp::Accepted, Transparency::Transparent)));
    }

    #[test]
    fn category_filter_is_case_insensitive() {
        let f = Filters { categories: Some(vec!["WORK".into()]), ..Default::default() };
        assert!(included(&f, &ev(Rsvp::Accepted, Transparency::Opaque)));
        let f = Filters { categories: Some(vec!["personal".into()]), ..Default::default() };
        assert!(!included(&f, &ev(Rsvp::Accepted, Transparency::Opaque)));
    }

    #[test]
    fn free_slots_merge_overlaps_and_respect_min_duration() {
        let t = |h: u32, m: u32| Utc.with_ymd_and_hms(2026, 9, 14, h, m, 0).unwrap();
        let busy = vec![(t(9, 0), t(10, 0)), (t(9, 30), t(11, 0)), (t(13, 0), t(13, 20))];
        let slots = free_slots(t(8, 0), t(18, 0), busy, Duration::minutes(30));
        assert_eq!(
            slots,
            vec![
                FreeSlot { start: t(8, 0), end: t(9, 0), duration_minutes: 60 },
                FreeSlot { start: t(11, 0), end: t(13, 0), duration_minutes: 120 },
                FreeSlot { start: t(13, 20), end: t(18, 0), duration_minutes: 280 },
            ]
        );
    }

    #[test]
    fn free_slots_with_nothing_busy_is_whole_window() {
        let t = |h: u32| Utc.with_ymd_and_hms(2026, 9, 14, h, 0, 0).unwrap();
        let slots = free_slots(t(8), t(9), Vec::new(), Duration::minutes(15));
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].duration_minutes, 60);
    }

    #[test]
    fn clamp_window_respects_horizon() {
        let f = Filters { horizon_past_days: 1, horizon_future_days: 2, ..Default::default() };
        let now = Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap();
        let (a, b) = f.clamp_window(now - Duration::days(10), now + Duration::days(10), now);
        assert_eq!(a, now - Duration::days(1));
        assert_eq!(b, now + Duration::days(2));
    }
}
