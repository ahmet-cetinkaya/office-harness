-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per agent that waits for the user (0008 D1). The primary key is the
-- agent, so a second event replaces the row's latest event and never adds a
-- row. Notification rows are not touched.
CREATE TABLE notification_needs_you (
    agent_id        TEXT PRIMARY KEY,
    first_event_at  INTEGER NOT NULL,
    -- One of: failed, waiting_for_input, bell, auto_hold, exited.
    latest_kind     TEXT NOT NULL,
    latest_detail   TEXT,
    latest_at       INTEGER NOT NULL,
    -- The notification that created the item; NULL for a coalesced bell. A
    -- read of it clears the item.
    origin          TEXT
);

-- clear_on_read looks the item up by its origin notification.
CREATE INDEX notification_needs_you_origin ON notification_needs_you (origin);
