# ADR 0001: Postgres as the job queue; no Redpanda, no Temporal (for now)

**Status:** accepted for v0.x, revisit when the workload changes.

## Context

The platform needs background work: poll each inbound calendar on a schedule, react to change notifications, and (soon) push mirrored events to target calendars with retries. Two heavier options were considered:

* **Redpanda / Kafka**: a durable event log. Good when many independent consumers need the same stream, or when event volume is high enough that a database write per event hurts.
* **Temporal**: durable workflow execution. Good for long-running, multi-step processes that must survive crashes mid-way (OAuth token refresh → fetch → diff → write-back → confirm) with retries and timers built in.

## Decision

Use Postgres for everything: `calendar_sources.next_sync_at` is the schedule, and workers claim due rows with `SELECT … FOR UPDATE SKIP LOCKED`. Retries are a `next_sync_at` update; crashed runs are reclaimed after a timeout. Bounded concurrency is a semaphore in-process.

## Reasons

1. **Self-hostability is the product.** Every extra stateful service roughly halves the number of people who will run this. "App + Postgres" is the bar set by Gitea, Miniflux, Plausible and others.
2. **The workload is small.** A personal or friends-scale instance has tens of calendars, polled every ~15 minutes, producing a few thousand event rows. Postgres handles that with a single index and no tuning. Kafka-scale problems do not exist here.
3. **Idempotency removes the need for durable workflows.** A sync is "fetch feed, upsert, rebuild instances" in one transaction. If it dies halfway nothing is lost; the next tick redoes it. Write-back will follow the same pattern: the target calendar is reconciled against the desired projection, so any step can be repeated safely. Temporal's main value (exactly-once progress through non-idempotent steps) is not needed when every step is idempotent.
4. **Multi-replica works anyway.** `SKIP LOCKED` lets several server processes share one database without coordination.

## Consequences

* Scheduling latency is bounded by the tick (default 30 s). Fine for polling; when push notifications arrive they simply set `next_sync_at = now()`.
* There is no event stream for third parties. If someone wants "notify me when a friend's availability changes", that becomes a Postgres `LISTEN/NOTIFY` or an outbox table, not a broker.
* If the project ever runs as a hosted multi-tenant service with thousands of calendars and webhook fan-out, revisit: the natural first step would be an outbox table plus a worker pool, then Redpanda if the fan-out grows. Temporal would only earn its place if write-back needs long, non-idempotent multi-step flows (for example paid bookings with cancellation windows).
