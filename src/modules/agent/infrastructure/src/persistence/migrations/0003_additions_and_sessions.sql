-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per agent; the row goes with its agent. additions is a JSON
-- document with the flags, the environment entries, and the startup prompt.
CREATE TABLE agent_office_additions (
    agent_id   TEXT PRIMARY KEY REFERENCES agent_agents (id) ON DELETE CASCADE,
    additions  TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- additions_snapshot is the additions JSON used at start (FR-023). The
-- session's output file is named by its id.
CREATE TABLE agent_sessions (
    id                 TEXT PRIMARY KEY,
    agent_id           TEXT NOT NULL REFERENCES agent_agents (id) ON DELETE CASCADE,
    cli_session_id     TEXT,
    continued_from     TEXT,
    additions_snapshot TEXT NOT NULL,
    started_at         INTEGER NOT NULL,
    ended_at           INTEGER,
    exit_result        TEXT
);

CREATE INDEX agent_sessions_by_agent ON agent_sessions (agent_id, started_at);
