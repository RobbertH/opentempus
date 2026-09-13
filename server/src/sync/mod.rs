//! Inbound synchronization: connectors fetch a calendar, the store normalizes
//! it into `events` + `event_instances`, and the scheduler decides when.
//!
//! Scheduling uses Postgres itself as the job queue: a source is "due" when
//! `next_sync_at <= now()`, and workers claim due rows with
//! `FOR UPDATE SKIP LOCKED` so several server replicas can share the work
//! without any extra infrastructure.

pub mod connector;
pub mod ics_url;
pub mod scheduler;
pub mod store;

pub use scheduler::{run_scheduler, sync_source_now};
