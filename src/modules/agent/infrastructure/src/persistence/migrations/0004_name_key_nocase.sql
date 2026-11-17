-- SPDX-License-Identifier: AGPL-3.0-or-later

-- name_key holds the lower-cased name, and the write path keeps it unique.
-- These indexes make the database refuse a key that differs only by case, so
-- a direct edit or a future writer cannot break the rule.
CREATE UNIQUE INDEX agent_projects_name_key_nocase ON agent_projects (name_key COLLATE NOCASE);
CREATE UNIQUE INDEX agent_agents_name_key_nocase ON agent_agents (name_key COLLATE NOCASE);
