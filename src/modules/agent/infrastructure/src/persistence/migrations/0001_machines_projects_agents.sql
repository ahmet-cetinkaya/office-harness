-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE agent_machines (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

-- name_key is the lower-cased name, so uniqueness ignores case.
CREATE TABLE agent_projects (
    id       TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    name_key TEXT NOT NULL UNIQUE,
    folder   TEXT NOT NULL
);

CREATE TABLE agent_agents (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    name_key       TEXT NOT NULL UNIQUE,
    command        TEXT NOT NULL,
    working_folder TEXT,
    project_id     TEXT REFERENCES agent_projects (id),
    machine_id     TEXT NOT NULL REFERENCES agent_machines (id),
    status         TEXT NOT NULL,
    created_at     INTEGER NOT NULL
);
