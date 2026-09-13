-- Outbound sync targets: mirror a filtered/projected view of the owner's
-- calendars into an external calendar (write-back).

CREATE TYPE target_kind AS ENUM ('caldav', 'google', 'microsoft');

CREATE TABLE sync_targets (
    id                  uuid PRIMARY KEY,
    user_id             uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                text NOT NULL,
    kind                target_kind NOT NULL,
    -- connector specific: { "calendar_url": ..., "username": ..., "password": ... }
    config              jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- same rule model as shares
    visibility          jsonb NOT NULL,
    filters             jsonb NOT NULL,
    -- summary used when the title is hidden
    placeholder_title   text NOT NULL DEFAULT 'Busy',
    enabled             boolean NOT NULL DEFAULT true,
    push_interval_secs  integer NOT NULL DEFAULT 900,
    next_push_at        timestamptz NOT NULL DEFAULT now(),
    push_started_at     timestamptz,
    last_pushed_at      timestamptz,
    last_push_status    sync_status NOT NULL DEFAULT 'never',
    last_push_error     text,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sync_targets_user_idx ON sync_targets(user_id);
CREATE INDEX sync_targets_due_idx ON sync_targets(next_push_at) WHERE enabled;

-- What we have written to each target, so pushes are incremental and
-- deletions propagate.
CREATE TABLE mirrored_events (
    target_id     uuid NOT NULL REFERENCES sync_targets(id) ON DELETE CASCADE,
    -- stable occurrence id (event_instances.id is deterministic)
    instance_id   uuid NOT NULL,
    remote_href   text NOT NULL,
    etag          text,
    content_hash  text NOT NULL,
    updated_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (target_id, instance_id)
);
