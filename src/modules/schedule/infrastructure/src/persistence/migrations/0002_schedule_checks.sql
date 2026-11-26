-- SPDX-License-Identifier: AGPL-3.0-or-later

-- CHECK constraints on the schedule tables (0025 follow-up): the lengths, the
-- hours range, the repeat columns that each repeat kind needs, and the run
-- results. SQLite adds no CHECK to an existing table, so both tables are
-- rebuilt. The runs go to a holding table first, because dropping the
-- schedules while the runs refer to them would delete the runs.
CREATE TABLE schedule_runs_held AS SELECT * FROM schedule_runs;
DROP TABLE schedule_runs;

CREATE TABLE schedule_schedules_checked (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    -- The lower-cased name, so a name is unique case-insensitively.
    name_key               TEXT NOT NULL UNIQUE CHECK (length(name_key) BETWEEN 1 AND 100),
    text                   TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 8192),
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
    -- N for every_hours (1 to 168), else NULL.
    repeat_hours           INTEGER CHECK (repeat_hours BETWEEN 1 AND 168),
    switched_on            INTEGER NOT NULL CHECK (switched_on IN (0, 1)),
    -- Unix milliseconds, from here on.
    anchor                 INTEGER NOT NULL,
    next_due               INTEGER,
    last_handled_due       INTEGER,
    last_missed            INTEGER,
    -- One of: target_deleted, time_missed; NULL when none.
    off_reason             TEXT CHECK (off_reason IN ('target_deleted', 'time_missed')),
    notified_since_success INTEGER NOT NULL CHECK (notified_since_success IN (0, 1)),
    CHECK (
        (repeat_kind = 'once' AND repeat_date IS NOT NULL AND repeat_time IS NOT NULL
            AND repeat_days IS NULL AND repeat_hours IS NULL)
        OR (repeat_kind = 'daily' AND repeat_date IS NULL AND repeat_time IS NOT NULL
            AND repeat_days IS NULL AND repeat_hours IS NULL)
        OR (repeat_kind = 'weekly' AND repeat_date IS NULL AND repeat_time IS NOT NULL
            AND repeat_days IS NOT NULL AND length(repeat_days) > 0 AND repeat_hours IS NULL)
        OR (repeat_kind = 'every_hours' AND repeat_date IS NULL AND repeat_time IS NULL
            AND repeat_days IS NULL AND repeat_hours IS NOT NULL)
    )
);
INSERT INTO schedule_schedules_checked SELECT * FROM schedule_schedules;
DROP TABLE schedule_schedules;
ALTER TABLE schedule_schedules_checked RENAME TO schedule_schedules;

-- The history of runs, the newest 20 per schedule (D4).
CREATE TABLE schedule_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    schedule_id TEXT NOT NULL REFERENCES schedule_schedules (id) ON DELETE CASCADE,
    due         INTEGER NOT NULL,
    started     INTEGER NOT NULL,
    -- One of: due, now.
    via         TEXT NOT NULL CHECK (via IN ('due', 'now')),
    -- NULL while the run is open.
    result      TEXT CHECK (result IN (
        'submitted', 'waiting', 'unsubmitted', 'withdrawn',
        'refused', 'skipped', 'failed', 'missed'
    )),
    reason      TEXT,
    -- The id of the waiting directive in the queue, while there is one.
    queue_entry TEXT
);
INSERT INTO schedule_runs (id, schedule_id, due, started, via, result, reason, queue_entry)
    SELECT id, schedule_id, due, started, via, result, reason, queue_entry
    FROM schedule_runs_held;
DROP TABLE schedule_runs_held;
CREATE INDEX schedule_runs_by_schedule ON schedule_runs (schedule_id, id);
