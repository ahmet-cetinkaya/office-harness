-- SPDX-License-Identifier: AGPL-3.0-or-later

-- A team: a name, an optional project, and exactly one lead. name_key is the
-- lower-cased name, so uniqueness ignores case.
CREATE TABLE hierarchy_teams (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    name_key      TEXT NOT NULL UNIQUE,
    project_id    TEXT,
    lead_agent_id TEXT NOT NULL
);

-- Membership ledger: one row per agent and team. The agent row's team_id is
-- the derived view; this table is the source of truth.
CREATE TABLE hierarchy_team_members (
    agent_id  TEXT NOT NULL,
    team_id   TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (agent_id, team_id)
);
