-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The place of an agent in the company tree (0002). NULL role = outside the
-- hierarchy, the 0001 behavior. The tree rules are enforced by the
-- hierarchy module; a bare SQL edit can break them, so every write goes
-- through the hierarchy use cases.
ALTER TABLE agent_agents ADD COLUMN role TEXT;
ALTER TABLE agent_agents ADD COLUMN parent_agent_id TEXT;
ALTER TABLE agent_agents ADD COLUMN team_id TEXT;
