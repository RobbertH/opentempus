# Roadmap

Rough order. Nothing here is committed to a date.

## Now: calendar sync and sharing (v0.1)

- [x] Accounts, sessions, friends
- [x] ICS/iCal URL sources (Google secret address, Outlook published, iCloud public, Nextcloud, Fastmail, …)
- [x] Recurrence expansion with overrides, EXDATE/RDATE, DST-correct
- [x] Shares with filters (calendar, category, RSVP, free/busy, all-day, horizon) and field-level visibility
- [x] Outbound ICS feeds per share; JSON API per share; free slots; multi-person availability
- [x] Web app: week view, calendars, sharing, friends, shared with me
- [x] Docker image + compose

## Next: real two-way sync

- [ ] **Write-back connectors** ("put my personal blockers into my work calendar", the core OneCal use case). A *sync target* is a share plus a destination calendar; the server creates/updates/deletes mirrored events there and tags them so they are never mirrored back.
  - [ ] CalDAV target (iCloud with app password, Fastmail, Nextcloud, Radicale)
  - [ ] Google Calendar target (OAuth, `events.insert`/`patch`/`delete` with a private extended property as marker)
  - [ ] Microsoft Graph target
- [ ] **Push-based inbound**: Google push notifications and Graph subscriptions so changes land in seconds instead of on the poll interval; CalDAV `sync-collection` for cheap incremental fetches.
- [ ] Inbound CalDAV / Google / Microsoft connectors (the trait exists, only `ics_url` is implemented). OAuth apps need client credentials configured by the instance admin.
- [ ] Encrypt source secrets at rest with a server key (`OPENTEMPUS_SECRET_KEY`).
- [ ] SSRF guard on fetched URLs (deny link-local/private ranges unless allowed by config).
- [ ] Per-event overrides: hide or reveal a single event regardless of rule; a keyword filter ("never share events containing 'doctor'").
- [ ] Event-level categories from `CATEGORIES` and colour, not only the source category.

## Agents

- [ ] **MCP server** exposing `list_shared_events`, `find_free_slots`, `find_common_availability` over the same tokens, so any MCP-capable assistant can use an OpenTempus share directly.
- [ ] Booking: an agent proposes a slot, the owner (or the friends) confirm in the web app, and the event is written to the chosen calendar.
- [ ] Rate limiting and audit log per token ("agent X read your free slots 14 times today").

## Preferences and planning (later)

- [ ] Preferences as soft constraints: "sport at least once every 2 days", "gym at least once every 3 days", "no more than N evening events in a row", each with a weight so a nice run of social events can outweigh a missed gym day.
- [ ] A planner that, given preferences and everybody's availability, ranks candidate slots instead of only listing free ones.
- [ ] Weekly review: how well the past week matched preferences.

## Product

- [ ] Mobile app (the API is designed so a native or React Native client needs nothing new; the ICS feeds already work in every phone calendar today).
- [ ] Email notifications for friend requests and share changes.
- [ ] OIDC login for instances that already run an identity provider.
- [ ] Groups ("the padel crew") to share with several people with one rule.
- [ ] i18n.
