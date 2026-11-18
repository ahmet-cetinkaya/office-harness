-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per event that the user must see (D4). The agent name is copied at
-- creation, so the row survives a deleted agent (spec edge cases).
CREATE TABLE notification_notifications (
    id         TEXT PRIMARY KEY,
    agent_id   TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    -- One of: exited, failed, waiting_for_input, bell (D4).
    event      TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    -- NULL while unread (FR-007, FR-009).
    read_at    INTEGER
);

-- Unread-first listing reads this index (D4: unread first, newest first).
CREATE INDEX notification_notifications_unread
    ON notification_notifications (read_at, created_at DESC);
