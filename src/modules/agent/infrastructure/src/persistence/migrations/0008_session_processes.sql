-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The processes of a session that the office knows (0017): its CLI and the
-- helpers that descend from it. started_at is the start time in the unit of
-- the OS (clock ticks, seconds, or 100 ns FILETIME units; all fit 63 bits).
-- command_hash is the hash of the whole command line, for the match;
-- command_display is the capped text for people. A session that ends marks
-- its rows 'ended'; a helper that ends is deleted. The unique key stays the
-- same when a process replaces itself with another program (exec), so the
-- row is updated in place.
CREATE TABLE agent_session_processes (
    id              INTEGER PRIMARY KEY,
    session_id      TEXT    NOT NULL REFERENCES agent_sessions (id) ON DELETE CASCADE,
    pid             INTEGER NOT NULL,
    started_at      INTEGER NOT NULL,
    command_hash    BLOB    NOT NULL,
    command_display TEXT    NOT NULL CHECK (length(command_display) <= 1000),
    boot            TEXT    NOT NULL,
    kind            TEXT    NOT NULL CHECK (kind IN ('cli', 'helper')),
    state           TEXT    NOT NULL CHECK (state IN ('open', 'ended')),
    -- What the sweep of an unclean stop found, for resume.plan; NULL for a
    -- row that a session end or a helper end closed.
    sweep_outcome   TEXT    CHECK (sweep_outcome IN
        ('ended', 'could_not_end', 'not_readable', 'boot_time_changed', 'left')),
    UNIQUE (session_id, pid, started_at)
);

CREATE INDEX agent_session_processes_open ON agent_session_processes (session_id, state);

-- Set when the helper limit stopped the office from storing more helpers of
-- the session, so its stored identities are not complete.
ALTER TABLE agent_sessions ADD COLUMN helpers_capped INTEGER NOT NULL DEFAULT 0;
