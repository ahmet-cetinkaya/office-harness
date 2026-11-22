-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The agent's own branch and worktree (0010): the repository's top folder,
-- the base branch or commit, the agent's branch, and the worktree folder.
-- The four are all set or all NULL; NULL means no own worktree.
ALTER TABLE agent_agents ADD COLUMN worktree_repository TEXT;
ALTER TABLE agent_agents ADD COLUMN worktree_base TEXT;
ALTER TABLE agent_agents ADD COLUMN worktree_branch TEXT;
ALTER TABLE agent_agents ADD COLUMN worktree_folder TEXT;
