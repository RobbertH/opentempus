//! Persist a parsed calendar: upsert events, replace expanded instances and
//! remove events that disappeared from the feed.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ics::{expand, parse::group_by_uid, ParsedEvent};
use crate::models::CalendarSource;

#[derive(Debug, Default)]
pub struct StoreStats {
    pub events: usize,
    pub instances: usize,
    pub removed: usize,
    pub warnings: Vec<String>,
}

pub async fn store_calendar(pool: &PgPool, source: &CalendarSource, events: Vec<ParsedEvent>) -> anyhow::Result<StoreStats> {
    let now = Utc::now();
    let from = now - Duration::days(source.horizon_past_days.max(0) as i64);
    let to = now + Duration::days(source.horizon_future_days.max(1) as i64);

    let mut stats = StoreStats::default();
    let grouped = group_by_uid(events);

    let mut tx = pool.begin().await?;

    // Wipe instances; they are derived data and cheap to rebuild.
    sqlx::query("DELETE FROM event_instances WHERE source_id = $1").bind(source.id).execute(&mut *tx).await?;

    let mut seen: HashSet<(String, Option<DateTime<Utc>>)> = HashSet::new();

    for (uid, group) in grouped {
        let (masters, overrides): (Vec<ParsedEvent>, Vec<ParsedEvent>) =
            group.into_iter().partition(|e| e.recurrence_id.is_none());
        let overridden: HashSet<DateTime<Utc>> = overrides.iter().filter_map(|e| e.recurrence_id).collect();

        // Some feeds contain a duplicate master; keep the highest sequence.
        let master = masters.into_iter().max_by_key(|m| (m.sequence, m.last_modified));

        let mut event_ids: HashMap<Option<DateTime<Utc>>, Uuid> = HashMap::new();
        if let Some(m) = &master {
            let id = upsert_event(&mut tx, source.id, m).await?;
            event_ids.insert(None, id);
            seen.insert((uid.clone(), None));
            stats.events += 1;
        }
        for o in &overrides {
            let id = upsert_event(&mut tx, source.id, o).await?;
            event_ids.insert(o.recurrence_id, id);
            seen.insert((uid.clone(), o.recurrence_id));
            stats.events += 1;
        }

        if let Some(m) = &master {
            match expand(m, &overridden, from, to) {
                Ok(occurrences) => {
                    let event_id = event_ids[&None];
                    for occ in occurrences {
                        insert_instance(&mut tx, event_id, source.id, occ.start_at, occ.end_at, occ.all_day).await?;
                        stats.instances += 1;
                    }
                }
                Err(e) => stats.warnings.push(format!("{uid}: {e}")),
            }
        }
        for o in &overrides {
            if o.end_at > from && o.start_at < to {
                let event_id = event_ids[&o.recurrence_id];
                insert_instance(&mut tx, event_id, source.id, o.start_at, o.end_at, o.all_day).await?;
                stats.instances += 1;
            }
        }
    }

    // Remove events no longer present in the feed.
    let existing: Vec<(Uuid, String, Option<DateTime<Utc>>)> =
        sqlx::query_as("SELECT id, uid, recurrence_id FROM events WHERE source_id = $1")
            .bind(source.id)
            .fetch_all(&mut *tx)
            .await?;
    let stale: Vec<Uuid> =
        existing.into_iter().filter(|(_, uid, rid)| !seen.contains(&(uid.clone(), *rid))).map(|(id, _, _)| id).collect();
    if !stale.is_empty() {
        sqlx::query("DELETE FROM events WHERE id = ANY($1)").bind(&stale).execute(&mut *tx).await?;
        stats.removed = stale.len();
    }

    tx.commit().await?;
    Ok(stats)
}

async fn upsert_event(tx: &mut sqlx::PgConnection, source_id: Uuid, e: &ParsedEvent) -> anyhow::Result<Uuid> {
    let id: (Uuid,) = sqlx::query_as(
        r#"INSERT INTO events (id, source_id, uid, recurrence_id, summary, description, location,
                               start_at, end_at, all_day, timezone, status, transparency, rsvp,
                               categories, rrule, rdates, exdates, sequence, last_modified, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, now())
           ON CONFLICT (source_id, uid, recurrence_id) DO UPDATE SET
               summary = EXCLUDED.summary, description = EXCLUDED.description, location = EXCLUDED.location,
               start_at = EXCLUDED.start_at, end_at = EXCLUDED.end_at, all_day = EXCLUDED.all_day,
               timezone = EXCLUDED.timezone, status = EXCLUDED.status, transparency = EXCLUDED.transparency,
               rsvp = EXCLUDED.rsvp, categories = EXCLUDED.categories, rrule = EXCLUDED.rrule,
               rdates = EXCLUDED.rdates, exdates = EXCLUDED.exdates, sequence = EXCLUDED.sequence,
               last_modified = EXCLUDED.last_modified, updated_at = now()
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(source_id)
    .bind(&e.uid)
    .bind(e.recurrence_id)
    .bind(&e.summary)
    .bind(&e.description)
    .bind(&e.location)
    .bind(e.start_at)
    .bind(e.end_at)
    .bind(e.all_day)
    .bind(&e.timezone)
    .bind(e.status)
    .bind(e.transparency)
    .bind(e.rsvp)
    .bind(&e.categories)
    .bind(&e.rrule)
    .bind(&e.rdates)
    .bind(&e.exdates)
    .bind(e.sequence)
    .bind(e.last_modified)
    .fetch_one(&mut *tx)
    .await?;
    Ok(id.0)
}

/// Occurrence ids are derived from the event id and the start instant, so
/// they survive re-syncs. Feed subscribers and mirrored events rely on that.
pub fn instance_id(event_id: Uuid, start_at: DateTime<Utc>) -> Uuid {
    Uuid::new_v5(&event_id, start_at.to_rfc3339().as_bytes())
}

async fn insert_instance(
    tx: &mut sqlx::PgConnection,
    event_id: Uuid,
    source_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    all_day: bool,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO event_instances (id, event_id, source_id, start_at, end_at, all_day) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
    )
    .bind(instance_id(event_id, start_at))
    .bind(event_id)
    .bind(source_id)
    .bind(start_at)
    .bind(end_at)
    .bind(all_day)
    .execute(&mut *tx)
    .await?;
    Ok(())
}
