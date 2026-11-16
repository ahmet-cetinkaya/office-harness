# Data model: 0002 agent hierarchy

Date: 2026-11-16. Style follows the 0001 migrations (`src/modules/agent/infrastructure/src/persistence/migrations/`).

## Principle

Each module owns its prefixed tables and its schema ledger. The hierarchy module owns `hierarchy_*` tables. The role, parent, and team membership of an agent are agent facts, so they live in the agent module's schema as new columns, with values defined by hierarchy value objects that the agent module consumes through the application contract (the existing `Actor`/cross-module port pattern). Every migration is forward-only and keeps all 0001 rows valid.

## Agent module migration 0004 (agent columns)

```sql
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- NULL role = an agent outside the hierarchy (0001 behavior, PB-1).
-- One agent with role 'ceo' exists at most (application layer + test).
ALTER TABLE agent_agents ADD COLUMN role TEXT
    REFERENCES hierarchy_enum_check(role) -- see note below
    ;
ALTER TABLE agent_agents ADD COLUMN parent_agent_id TEXT
    REFERENCES agent_agents (id) ON DELETE RESTRICT
    ;
ALTER TABLE agent_agents ADD COLUMN team_id TEXT
    REFERENCES hierarchy_teams (id) ON DELETE RESTRICT
    ;
```

Note: SQLite has no ALTER ADD CONSTRAINT, so the allowed role values and the tree invariants are enforced by the application layer and by the property tests of T2(a): `role IN ('ceo','lead','member')` or NULL; a lead's parent is the CEO; a member's parent is a lead; a member's team equals its lead's team; no cycles; depth at most 3. `ON DELETE RESTRICT` is emulated in the repository (delete always goes through the counted-subtree path of T3, never a bare SQL delete), because SQLite applies foreign keys only when `PRAGMA foreign_keys = ON` and the 0001 connection layer decides that.

## Hierarchy module migration 0001 (teams, members, ledger)

```sql
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE hierarchy_teams (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    name_key     TEXT NOT NULL UNIQUE,   -- lower-cased; office-wide unique names
    project_id   TEXT REFERENCES agent_projects (id),
    lead_agent_id TEXT NOT NULL REFERENCES agent_agents (id)
);

-- Membership ledger: one row per (agent, team). The agent row's team_id is
-- the derived view for one read; this table is the source of truth for team
-- membership history and the 'one lead per team' check.
CREATE TABLE hierarchy_team_members (
    agent_id TEXT NOT NULL REFERENCES agent_agents (id),
    team_id  TEXT NOT NULL REFERENCES hierarchy_teams (id),
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (agent_id, team_id)
);

-- Ledger, as in every module.
CREATE TABLE hierarchy_schema_version (
    script INTEGER PRIMARY KEY,
    ran_at INTEGER NOT NULL
);
```

Cross-module foreign keys: `hierarchy_teams.project_id` and `.lead_agent_id` reference agent tables, and `agent_agents.team_id` references `hierarchy_teams`. The 0001 pattern is per-module ledgers over one shared `office.db`; references across module prefixes already exist in 0001 (`agent_agents.project_id`), so the pattern holds. The module migration order at startup becomes agent 0001 to 0003, hierarchy 0001, agent 0004 (the columns need `hierarchy_teams` to exist).

## Action record extension (audit module)

`AgentAction` gains variants `TeamCreate`, `TeamDelete`, `Directive`, `Refused`. The audit row keeps the existing shape; the action name is the existing text column, and a directive entry stores the sender, the target, and the text in the existing payload column used for failures. `Actor::Agent(AgentId)` already exists in the kernel and needs no schema change.

## Agent-call tokens (no table)

The token hash is held in memory by the running server, keyed by agent id, and revoked at stop, delete, and process exit. No table: a token is only meaningful inside one server run, and a restart issues new tokens (research R2; the advisor pass reviews lifetime).

## Tree read

`hierarchy.tree` is one SELECT over `agent_agents` (id, name, role, parent_agent_id, team_id, status) plus one over `hierarchy_teams`, assembled in the application layer. No recursive CTE is needed at depth 3.
