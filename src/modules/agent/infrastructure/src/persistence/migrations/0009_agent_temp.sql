-- SPDX-License-Identifier: AGPL-3.0-or-later

-- A temp agent (0020): the objective it was made for and its creator, which is
-- `user` or the id of the agent that made it. Both are set or both are NULL;
-- NULL means a permanent agent.
ALTER TABLE agent_agents ADD COLUMN temp_objective TEXT;
ALTER TABLE agent_agents ADD COLUMN temp_creator TEXT;

-- What an ended temp left behind, kept until the person dismisses it. The
-- agent is gone, so there is no foreign key to it. The report columns are both
-- set or both NULL.
CREATE TABLE agent_finished_temps (
    id              TEXT PRIMARY KEY,
    agent_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    objective       TEXT NOT NULL,
    creator         TEXT NOT NULL,
    parent_name     TEXT,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER NOT NULL,
    reason          TEXT NOT NULL,
    report_kind     TEXT,
    report_text     TEXT,
    kept_folder     TEXT,
    kept_branch     TEXT,
    removal_failure TEXT
);

CREATE INDEX agent_finished_temps_ended_at ON agent_finished_temps (ended_at);
