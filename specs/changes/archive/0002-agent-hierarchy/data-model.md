# Data model: 0002 agent hierarchy

Date: 2026-11-16. Style follows the 0001 migrations (`src/modules/agent/infrastructure/src/persistence/migrations/`).

## Principle

Each module owns its prefixed tables and its schema ledger. The hierarchy module owns `hierarchy_*` tables. The role, parent, and team membership of an agent are agent facts, so they live in the agent module's schema as new columns, with values defined by hierarchy value objects that the agent module consumes through the application contract (the existing `Actor`/cross-module port pattern). Every migration is forward-only and keeps all 0001 rows valid.

## Agent module migration 0005 (agent columns)

Shipped as `src/modules/agent/infrastructure/src/persistence/migrations/0005_hierarchy_columns.sql`. The agent module already had scripts 0001 to 0004 when this change landed, so the script is number 5. The columns are plain `TEXT`. They have no `REFERENCES` clause (the first draft of this file had one).

```sql
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- NULL role = an agent outside the hierarchy (0001 behavior, PB-1).
ALTER TABLE agent_agents ADD COLUMN role TEXT;
ALTER TABLE agent_agents ADD COLUMN parent_agent_id TEXT;
ALTER TABLE agent_agents ADD COLUMN team_id TEXT;
```

Note: no column has a constraint or a foreign key, so the allowed role values and the tree invariants are enforced by the application layer and by the property tests of T2(a), not by the database: `role IN ('ceo','lead','member')` or NULL; a lead's parent is the CEO; a member's parent is a lead; a member's team equals its lead's team; no cycles; depth at most 3. `ON DELETE RESTRICT` is emulated in the repository (delete always goes through the counted-subtree path of T3, never a bare SQL delete), because SQLite applies foreign keys only when `PRAGMA foreign_keys = ON` and the 0001 connection layer decides that.

## Hierarchy module migration 0001 (teams, members, ledger)

Shipped as `src/modules/hierarchy/infrastructure/src/persistence/migrations/0001_teams.sql`. The tables have no `REFERENCES` clause. A later change (0007) added script 0002 (`0002_holds.sql`).

```sql
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE hierarchy_teams (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    name_key      TEXT NOT NULL UNIQUE,   -- lower-cased; office-wide unique names
    project_id    TEXT,
    lead_agent_id TEXT NOT NULL
);

-- Membership ledger: one row per (agent, team). The agent row's team_id is
-- the derived view for one read; this table is the source of truth.
CREATE TABLE hierarchy_team_members (
    agent_id  TEXT NOT NULL,
    team_id   TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (agent_id, team_id)
);
```

The ledger table of the module is kept by the migration runner, as in every module.

Cross-module links: `hierarchy_teams.project_id` and `.lead_agent_id` point to agent tables, and `agent_agents.team_id` points to `hierarchy_teams`, by value only (no database foreign key). The 0001 pattern is per-module ledgers over one shared `office.db`; references across module prefixes already exist in 0001 (`agent_agents.project_id`), so the pattern holds. The module migration order at startup becomes agent 0001 to 0004, agent 0005 (the columns), hierarchy 0001 (no foreign key, so the order does not matter for the schema).

## Action record extension (audit module)

`ActionKind` (`src/modules/audit/domain/src/action_kind.rs`) gains variants `TeamCreate`, `TeamDelete`, `Directive`, `Refused`. Later changes added `Hold`, `Release` (0007) and `TaskText` (0011). The audit row keeps the existing shape; the action name is the existing text column, and a directive entry stores the sender, the target, and the text in the existing payload column used for failures. `Actor::Agent(AgentId)` already exists in the kernel and needs no schema change.

## Agent-call tokens (no table)

The token hash is held in memory by the running server, keyed by agent id, and revoked at stop, delete, and process exit. No table: a token is only meaningful inside one server run, and a restart issues new tokens (research R2; the advisor pass reviews lifetime).

## Tree read

`hierarchy.tree` is one SELECT over `agent_agents` (id, name, role, parent_agent_id, team_id, status) plus one over `hierarchy_teams`, assembled in the application layer. No recursive CTE is needed at depth 3.
