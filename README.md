# OpenTempus

**Self-hostable calendar synchronization and selective sharing.**

All your calendars flow *in* (Google, Outlook, iCloud, Nextcloud, anything with an iCal address). You write **rules** that decide what flows *out* to each friend, calendar app or AI agent: only busy blocks, or also the category (work/personal), the origin calendar, titles, locations, whether you accepted, or everything.

Think of it as an open-source, self-hosted alternative to OneCal, built so you can also hand an AI agent a deliberately narrow view of your life ("here is when I am free, nothing else") and let it find the padel slot that works for you and your friends.

> Status: early but usable. Inbound sync via iCal/ICS URLs and CalDAV, write-back into CalDAV calendars, accounts, friends, sharing rules, ICS feeds out, a JSON API for agents including multi-person availability, a flow map of everything that goes in and out, and a web app. See [ROADMAP.md](docs/ROADMAP.md) for what is next (Google/Microsoft OAuth connectors, push notifications, preferences, mobile).

## How it works

```
   Google / Outlook / iCloud / Nextcloud / ...        ┌──▶ your other calendars (CalDAV write-back: "Busy" blocks)
        │  (iCal URL or CalDAV, polled)               │
        ▼                                             │  ┌──▶ friends' calendar apps (ICS feed) and their OpenTempus view
 ┌──────────────┐    ┌──────────────┐    ┌────────────┴──┴───┐
 │  connectors  │──▶ │  normalized  │──▶ │  rules: filters + │──▶ JSON API for AI agents (events, free slots, group availability)
 │  (sync)      │    │  events      │    │  visibility       │
 └──────────────┘    │  + instances │    └───────────────────┘
                     └──────────────┘
```

* **Sources** are inbound calendars. Each has a *category* you assign (work, personal, sport, …).
* **Events** are normalized into one shape regardless of origin; recurring events are expanded into concrete occurrences.
* **Shares** are rules with two halves:
  * **Filters**: *which* events go out. By calendar, by category, by your RSVP (drop declined by default), free/busy, all-day, and how far into the past/future the audience may look.
  * **Visibility**: *what* the audience sees of each event. Busy blocks are always included; everything else (category, origin calendar, title, location, RSVP, free/busy, description) is opt-in. Presets: *Busy only*, *Category*, *Details*, *Full*.
* A **share** targets either a **friend** (an account on the same instance you have accepted a friend request from) or a **secret link** (for a calendar app subscription or an AI agent). Shares are pure projections evaluated at read time; nothing is copied.
* A **sync target** applies the same kind of rule but *writes* the result into another calendar over CalDAV, the classic "put my personal blockers into my work calendar as Busy". Mirrored events carry a marker so they are never imported back, which makes a calendar that is both a source and a target safe (two-way).
* The **Flows** page draws all of it: calendars in on the left, OpenTempus in the middle, every rule out on the right. Click any node to see what feeds it or what it feeds. Switch to **Who has access** to see the same graph grouped by person, agent or destination calendar, with exactly which fields each one sees.

## Quick start (Docker)

```sh
git clone https://github.com/RobbertH/opentempus && cd opentempus
OPENTEMPUS_PUBLIC_URL=https://cal.example.com docker compose up -d
```

Open the URL, create an account, add a calendar, add a friend, create a share or a sync target.

Getting the address or app password out of your provider is the fiddly part, so the app has click-by-click guides behind the **?** icon in the sidebar, with drawings of each screen: Google, Outlook/Microsoft 365, Proton, mailbox.org, iCloud, Fastmail and Nextcloud. The same guides are in [docs/CONNECTING.md](docs/CONNECTING.md).

Set `OPENTEMPUS_ALLOW_REGISTRATION=false` after your friends have signed up if you do not want an open instance.

## Local development

Requirements: a recent stable Rust (CI and the Docker image use 1.94), Node 22 + pnpm, PostgreSQL 14+.

```sh
cp .env.example .env               # point DATABASE_URL at your Postgres
cd web && pnpm install && pnpm dev # web app on :5173, proxies /api to :8080
cargo run                          # API on :8080, migrations run automatically
```

For a single binary with the web app embedded: `cd web && pnpm build && cargo build --release`.

Tests: `cargo test` (no database needed) and `cd web && pnpm build` (type-checks).

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `DATABASE_URL` | required | Postgres connection string |
| `OPENTEMPUS_LISTEN` | `0.0.0.0:8080` | Bind address |
| `OPENTEMPUS_PUBLIC_URL` | `http://localhost:8080` | URL used in feed links; also decides `Secure` cookies |
| `OPENTEMPUS_ALLOW_REGISTRATION` | `true` | Allow new accounts |
| `OPENTEMPUS_SYNC_CONCURRENCY` | `4` | Sources synced in parallel |
| `OPENTEMPUS_SCHEDULER_TICK_SECS` | `30` | How often the scheduler looks for due sources |
| `OPENTEMPUS_SESSION_TTL_DAYS` | `30` | Login session lifetime |
| `OPENTEMPUS_SECURE_COOKIES` | derived | Force `Secure` on the session cookie |
| `RUST_LOG` | `info,sqlx=warn` | Log filter |

## For AI agents

Create a **secret link** share with the visibility you are comfortable with (for a scheduling agent, *Busy only* is usually enough) and give the agent the API URL. See [docs/AGENTS.md](docs/AGENTS.md) for the three endpoints an agent needs: what the token allows, events in a window, and free slots, plus a cross-person availability call:

```sh
curl -X POST https://cal.example.com/api/v1/public/availability \
  -H 'Content-Type: application/json' \
  -d '{"tokens":["<my token>","<friend 1>","<friend 2>"],"from":"2026-09-16T00:00:00Z","to":"2026-09-20T00:00:00Z","min_minutes":90}'
```

## Repository layout

```
server/            Rust (axum + sqlx + Postgres)
  migrations/      SQL migrations, applied at startup
  src/ics/         iCalendar parsing, recurrence expansion, feed generation
  src/sync/        connectors (ICS URL, CalDAV), CalDAV client, store, write-back, scheduler
  src/sharing.rs   filters, visibility, projection, free-slot maths
  src/routes/      HTTP API
web/               React + TypeScript web app (Vite), embedded in the binary
docs/              architecture, roadmap, decision records, agent and connection guides
```

## License

Proposed: AGPL-3.0. This is not final; it is set in `Cargo.toml` and should be confirmed (and a `LICENSE` file added) before the first public release.
