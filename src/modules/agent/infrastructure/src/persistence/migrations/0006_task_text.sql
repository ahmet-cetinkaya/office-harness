-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The last task the office gave an agent (0011): its text, the time it was
-- set in Unix ms, and its source (`directive` or `user`). The three are all
-- set or all NULL; NULL means the agent shows no task line.
ALTER TABLE agent_agents ADD COLUMN task_text TEXT;
ALTER TABLE agent_agents ADD COLUMN task_set_at INTEGER;
ALTER TABLE agent_agents ADD COLUMN task_source TEXT;
