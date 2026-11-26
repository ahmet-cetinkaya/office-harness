-- SPDX-License-Identifier: AGPL-3.0-or-later

-- A notification may have no agent: a schedule's "missed runs" notice (0025
-- D14). SQLite cannot drop NOT NULL in place, so the table is rebuilt and
-- every row copied with its id, read state, and rowid (the listing order
-- breaks ties by rowid).
CREATE TABLE notification_notifications_rebuilt (
    id         TEXT PRIMARY KEY,
    agent_id   TEXT,
    agent_name TEXT NOT NULL,
    event      TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    read_at    INTEGER,
    detail     TEXT
);

INSERT INTO notification_notifications_rebuilt
    (rowid, id, agent_id, agent_name, event, created_at, read_at, detail)
SELECT rowid, id, agent_id, agent_name, event, created_at, read_at, detail
FROM notification_notifications;

DROP TABLE notification_notifications;
ALTER TABLE notification_notifications_rebuilt RENAME TO notification_notifications;

CREATE INDEX notification_notifications_unread
    ON notification_notifications (read_at, created_at DESC);
