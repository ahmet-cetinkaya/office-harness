-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One-time upkeep that the server runs at a start and has finished (0023): a
-- row means the task is done, so the next start skips it.
CREATE TABLE agent_maintenance (
    task    TEXT PRIMARY KEY,
    done_at INTEGER NOT NULL
);
