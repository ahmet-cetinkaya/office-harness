-- SPDX-License-Identifier: AGPL-3.0-or-later

-- name_key holds the name lower-cased by Rust `to_lowercase`, and the write
-- path keeps it unique. SQLite NOCASE folds ASCII letters only, so these
-- indexes refuse a key that differs only by ASCII case; keys that differ by
-- non-ASCII case (for example 'Äna' and 'äna') stay accepted here, and the
-- write path rejects them instead.
CREATE UNIQUE INDEX agent_projects_name_key_nocase ON agent_projects (name_key COLLATE NOCASE);
CREATE UNIQUE INDEX agent_agents_name_key_nocase ON agent_agents (name_key COLLATE NOCASE);
