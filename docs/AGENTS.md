# Using OpenTempus from an AI agent

An agent never gets an account. It gets a **share token**: a secret string that stands for one rule the owner wrote ("busy blocks only, work and personal calendars, next 30 days"). Whatever the agent asks, the answer is projected through that rule.

All endpoints are unauthenticated apart from the token in the path. Times are RFC 3339 in UTC.

## 1. Discover what a token allows

```
GET /api/v1/public/{token}
```

```json
{
  "name": "Padel agent",
  "owner": "Alice",
  "visibility": { "title": false, "description": false, "location": false, "category": true, "origin_calendar": false, "rsvp": false, "free_busy": false },
  "horizon_past_days": 7,
  "horizon_future_days": 30,
  "feed_url": "…/feeds/{token}.ics",
  "events_url": "…/api/v1/public/{token}/events?from=<rfc3339>&to=<rfc3339>",
  "free_url": "…/api/v1/public/{token}/free?from=<rfc3339>&to=<rfc3339>&min_minutes=60"
}
```

## 2. Events in a window

```
GET /api/v1/public/{token}/events?from=2026-09-14T00:00:00Z&to=2026-09-21T00:00:00Z
```

Fields absent from the visibility mask are omitted, not nulled:

```json
[
  { "id": "…", "start": "2026-09-14T07:00:00Z", "end": "2026-09-14T07:30:00Z", "all_day": false, "busy": true, "category": "work" },
  { "id": "…", "start": "2026-09-17T00:00:00Z", "end": "2026-09-18T00:00:00Z", "all_day": true,  "busy": true, "category": "work" }
]
```

Windows are clamped to the share's horizon. Defaults: `from = now`, `to = from + 14 days`, max 400 days.

## 3. Free slots for one person

```
GET /api/v1/public/{token}/free?from=…&to=…&min_minutes=90
```

```json
{ "from": "…", "to": "…", "min_minutes": 90,
  "slots": [ { "start": "2026-09-16T09:00:00Z", "end": "2026-09-16T17:00:00Z", "duration_minutes": 480 } ] }
```

## 4. Common availability for a group

```
POST /api/v1/public/availability
Content-Type: application/json

{ "tokens": ["<alice>", "<bob>", "<carol>"], "from": "…", "to": "…", "min_minutes": 90 }
```

```json
{ "from": "…", "to": "…", "min_minutes": 90, "participants": ["Alice", "Bob", "Carol"],
  "slots": [ { "start": "2026-09-16T18:00:00Z", "end": "2026-09-16T20:00:00Z", "duration_minutes": 120 } ] }
```

Each token is evaluated with its own filters; the window is the intersection of everybody's horizons. Nobody's events are revealed, only the merged free intervals.

## Practical notes

* Free slots cover the whole window including nights. Apply your own working-hours or "padel court opening hours" constraints on top.
* `busy: false` only appears when the owner enabled *free/busy* visibility; otherwise every returned event counts as busy.
* All-day events block the whole UTC day. If that is too coarse for your use, ask the owner to exclude all-day events in the share's filters.
* Treat the token like a password. The owner can rotate it at any time.
