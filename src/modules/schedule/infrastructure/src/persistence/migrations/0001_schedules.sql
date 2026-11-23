-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per schedule (0025 plan D2, D4). The repeat is spread over columns
-- so that no JSON needs parsing to read a due time.
CREATE TABLE schedule_schedules (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL,
    -- The lower-cased name, so a name is unique case-insensitively.
    name_key               TEXT NOT NULL UNIQUE,
    text                   TEXT NOT NULL,
    target_agent_id        TEXT NOT NULL,
    -- One of: once, daily, weekly, every_hours.
    repeat_kind            TEXT NOT NULL
        CHECK (repeat_kind IN ('once', 'daily', 'weekly', 'every_hours')),
    -- YYYY-MM-DD for once, else NULL.
    repeat_date            TEXT,
    -- HH:MM for once, daily, and weekly, else NULL.
    repeat_time            TEXT,
    -- Comma-separated mon..sun for weekly, else NULL.
    repeat_days            TEXT,
    -- N for every_hours, else NULL.
    repeat_hours           INTEGER,
    switched_on            INTEGER NOT NULL CHECK (switched_on IN (0, 1)),
    -- Unix milliseconds, from here on.
    anchor                 INTEGER NOT NULL,
    next_due               INTEGER,
    last_handled_due       INTEGER,
    last_missed            INTEGER,
    -- One of: target_deleted, time_missed; NULL when none.
    off_reason             TEXT CHECK (off_reason IN ('target_deleted', 'time_missed')),
    notified_since_success INTEGER NOT NULL CHECK (notified_since_success IN (0, 1))
);

-- The history of runs, the newest 20 per schedule (D4).
CREATE TABLE schedule_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    schedule_id TEXT NOT NULL REFERENCES schedule_schedules (id) ON DELETE CASCADE,
    due         INTEGER NOT NULL,
    started     INTEGER NOT NULL,
    -- One of: due, now.
    via         TEXT NOT NULL CHECK (via IN ('due', 'now')),
    -- NULL while the run is open.
    result      TEXT,
    reason      TEXT,
    -- The id of the waiting directive in the queue, while there is one.
    queue_entry TEXT
);

CREATE INDEX schedule_runs_by_schedule ON schedule_runs (schedule_id, id);
