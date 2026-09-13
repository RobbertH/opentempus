-- OpenTempus initial schema
-- All timestamps are stored as timestamptz (UTC). All-day events are stored as
-- midnight UTC boundaries with all_day = true and are rendered as DATE values.

CREATE EXTENSION IF NOT EXISTS citext;

-- ---------------------------------------------------------------------------
-- Accounts
-- ---------------------------------------------------------------------------
CREATE TABLE users (
    id            uuid PRIMARY KEY,
    email         citext NOT NULL UNIQUE,
    display_name  text NOT NULL,
    password_hash text NOT NULL,
    timezone      text NOT NULL DEFAULT 'UTC',
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id          uuid PRIMARY KEY,
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  text NOT NULL UNIQUE,
    expires_at  timestamptz NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_idx ON sessions(user_id);

-- ---------------------------------------------------------------------------
-- Friends
-- ---------------------------------------------------------------------------
CREATE TYPE friendship_status AS ENUM ('pending', 'accepted', 'declined');

CREATE TABLE friendships (
    id            uuid PRIMARY KEY,
    requester_id  uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee_id  uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status        friendship_status NOT NULL DEFAULT 'pending',
    created_at    timestamptz NOT NULL DEFAULT now(),
    responded_at  timestamptz,
    CHECK (requester_id <> addressee_id),
    UNIQUE (requester_id, addressee_id)
);
CREATE INDEX friendships_addressee_idx ON friendships(addressee_id);

-- ---------------------------------------------------------------------------
-- Inbound calendar sources
-- ---------------------------------------------------------------------------
CREATE TYPE source_kind AS ENUM ('ics_url', 'caldav', 'google', 'microsoft');
CREATE TYPE sync_status AS ENUM ('never', 'ok', 'error', 'running');

CREATE TABLE calendar_sources (
    id                  uuid PRIMARY KEY,
    user_id             uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                text NOT NULL,
    kind                source_kind NOT NULL,
    -- connector specific configuration (e.g. { "url": "https://..." }).
    config              jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- user assigned category such as "work" or "personal"; events inherit it
    category            text NOT NULL DEFAULT 'personal',
    color               text NOT NULL DEFAULT '#4f6df5',
    -- how far to expand recurring events
    horizon_past_days   integer NOT NULL DEFAULT 30,
    horizon_future_days integer NOT NULL DEFAULT 365,
    sync_interval_secs  integer NOT NULL DEFAULT 900,
    enabled             boolean NOT NULL DEFAULT true,
    next_sync_at        timestamptz NOT NULL DEFAULT now(),
    sync_started_at     timestamptz,
    last_synced_at      timestamptz,
    last_sync_status    sync_status NOT NULL DEFAULT 'never',
    last_sync_error     text,
    etag                text,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX calendar_sources_user_idx ON calendar_sources(user_id);
CREATE INDEX calendar_sources_due_idx ON calendar_sources(next_sync_at) WHERE enabled;

-- ---------------------------------------------------------------------------
-- Normalized events (one row per VEVENT; recurring masters carry the RRULE)
-- ---------------------------------------------------------------------------
CREATE TYPE event_status AS ENUM ('confirmed', 'tentative', 'cancelled');
CREATE TYPE transparency AS ENUM ('opaque', 'transparent');
CREATE TYPE rsvp_status AS ENUM ('organizer', 'accepted', 'tentative', 'declined', 'needs_action', 'unknown');

CREATE TABLE events (
    id             uuid PRIMARY KEY,
    source_id      uuid NOT NULL REFERENCES calendar_sources(id) ON DELETE CASCADE,
    uid            text NOT NULL,
    -- for overridden occurrences of a recurring event (RECURRENCE-ID)
    recurrence_id  timestamptz,
    summary        text,
    description    text,
    location       text,
    start_at       timestamptz NOT NULL,
    end_at         timestamptz NOT NULL,
    all_day        boolean NOT NULL DEFAULT false,
    timezone       text,
    status         event_status NOT NULL DEFAULT 'confirmed',
    transparency   transparency NOT NULL DEFAULT 'opaque',
    rsvp           rsvp_status NOT NULL DEFAULT 'unknown',
    categories     text[] NOT NULL DEFAULT '{}',
    rrule          text,
    rdates         timestamptz[] NOT NULL DEFAULT '{}',
    exdates        timestamptz[] NOT NULL DEFAULT '{}',
    sequence       integer NOT NULL DEFAULT 0,
    last_modified  timestamptz,
    updated_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (source_id, uid, recurrence_id)
);
CREATE INDEX events_source_idx ON events(source_id);

-- Expanded occurrences inside the source horizon. Non-recurring events have
-- exactly one instance. This is what every query for "what is on" reads.
CREATE TABLE event_instances (
    id         uuid PRIMARY KEY,
    event_id   uuid NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    source_id  uuid NOT NULL REFERENCES calendar_sources(id) ON DELETE CASCADE,
    start_at   timestamptz NOT NULL,
    end_at     timestamptz NOT NULL,
    all_day    boolean NOT NULL DEFAULT false
);
CREATE INDEX event_instances_source_time_idx ON event_instances(source_id, start_at, end_at);
CREATE INDEX event_instances_event_idx ON event_instances(event_id);

-- ---------------------------------------------------------------------------
-- Shares: a rule that projects the owner's calendars to an audience.
-- ---------------------------------------------------------------------------
CREATE TYPE audience_kind AS ENUM ('friend', 'link');

CREATE TABLE shares (
    id                uuid PRIMARY KEY,
    owner_id          uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              text NOT NULL,
    audience_kind     audience_kind NOT NULL,
    -- set when audience_kind = 'friend'
    audience_user_id  uuid REFERENCES users(id) ON DELETE CASCADE,
    -- secret used for the ICS feed and the public JSON API
    token             text NOT NULL UNIQUE,
    -- which fields the audience may see (see sharing::Visibility)
    visibility        jsonb NOT NULL,
    -- which events are included at all (see sharing::Filters)
    filters           jsonb NOT NULL,
    enabled           boolean NOT NULL DEFAULT true,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    CHECK ((audience_kind = 'friend') = (audience_user_id IS NOT NULL))
);
CREATE INDEX shares_owner_idx ON shares(owner_id);
CREATE INDEX shares_audience_idx ON shares(audience_user_id);
