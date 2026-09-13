# Architecture

## Goals

1. **One binary + Postgres.** Self-hosting must be `docker compose up`. No brokers, no workflow engines, no Redis.
2. **Sharing is a projection, not a copy.** A share is evaluated on every read from the normalized store. Rules take effect immediately and there is no second copy of anybody's data to keep consistent or to leak.
3. **Least privilege by default.** The minimal share is "busy blocks", every other field is opt-in, declined events and free time are excluded unless asked for, and each token only ever yields what its own rule allows, even when combined with other tokens in an availability query.
4. **Connectors are pluggable.** Anything that can produce iCalendar (or be mapped to it) can be a source.

## Data flow

```
connector.fetch() ──▶ ics::parse ──▶ sync::store (events + event_instances) ──▶ read paths
                                                                                  ├─ /api/v1/events            (owner, everything)
                                                                                  ├─ sharing::project_all       (filters + visibility)
                                                                                  │    ├─ /api/v1/public/{token}/events
                                                                                  │    ├─ /feeds/{token}.ics
                                                                                  │    └─ /api/v1/shared-with-me/{id}/events
                                                                                  └─ sharing::free_slots        (/free, /availability)
```

## Server (`server/`)

* **axum** for HTTP, **sqlx** (no compile-time macros, so contributors do not need a database to build), **tokio**.
* `config.rs` reads environment variables. `state.rs` holds the pool, config and an HTTP client.
* `auth.rs`: argon2id password hashes; sessions are random 256-bit tokens stored as SHA-256 hashes, delivered as an `HttpOnly` cookie or usable as a bearer token.
* `models.rs`: row types and the Postgres enums (`rsvp_status`, `event_status`, `transparency`, …).
* `ics/`:
  * `parse.rs` turns a VCALENDAR into `ParsedEvent`s: DATE vs DATE-TIME, TZID (IANA, Windows names, mozilla-style prefixes), floating times via `X-WR-TIMEZONE` or the user's zone, DURATION, RRULE/RDATE/EXDATE, RECURRENCE-ID overrides, STATUS, TRANSP, `X-MICROSOFT-CDO-BUSYSTATUS`, and the owner's PARTSTAT (matched by e-mail, ORGANIZER wins).
  * `expand.rs` expands a master + overrides into occurrences in a window using the `rrule` crate, in the event's own zone so DST keeps wall-clock time. Capped at 2000 occurrences per master.
  * `generate.rs` renders outbound feeds (folded lines, escaping, `DATE` for all-day, `Busy` placeholder titles, `X-OPENTEMPUS-*` properties).
  * `tz.rs` maps zone names.
* `sync/`:
  * `connector.rs`: the `Connector` trait (`validate_config`, `fetch`). `ics_url.rs` implements it with ETag support and a size cap.
  * `store.rs`: one transaction per sync: upsert events keyed on `(source, uid, recurrence_id)`, rebuild `event_instances` for the source's horizon, delete events that vanished from the feed.
  * `scheduler.rs`: every tick, claim due sources with `FOR UPDATE SKIP LOCKED`, sync with bounded concurrency, record status and back off on errors. Stale "running" rows (a crashed process) are reclaimed after 15 minutes. Multiple replicas can share one database safely.
* `sharing.rs`: `Filters`, `Visibility`, `project`, `free_slots` (interval merge). Pure functions, unit tested.
* `routes/`: thin handlers. `public.rs` is the only surface reachable with just a token and never returns more than `project` allows.

## Storage

* `events` holds one row per VEVENT (masters carry RRULE; overrides carry `recurrence_id`).
* `event_instances` holds expanded occurrences inside `[now - horizon_past, now + horizon_future]` per source. All reads go through this table with a simple range index. It is derived data and rebuilt on every sync, so it can be wiped at any time.
* All timestamps are `timestamptz`. All-day events are stored as UTC-midnight boundaries with `all_day = true` and rendered as `DATE` values.
* Share `visibility` and `filters` are `jsonb`, deserialized into typed structs with defaults, so adding a field is a code change, not a migration.

## Web app (`web/`)

Vite + React + TypeScript, no UI framework. Built into `web/dist` and embedded into the binary with `rust-embed`; any unknown path serves `index.html` so client-side routing works. In development Vite proxies `/api` and `/feeds` to the server.

## Security notes

* Share tokens are 192-bit random URL-safe strings; rotating one invalidates old feed URLs instantly.
* Friend requests to unknown addresses succeed silently so the API does not reveal who has an account.
* Source configs may contain secrets (Google secret iCal URLs, basic-auth passwords). They are stored as-is today; encrypting them at rest with a server key is on the roadmap. The API masks passwords on read.
* Feeds are fetched server-side. Deployments that must not reach internal networks should restrict egress (SSRF protection at the connector level is on the roadmap).
