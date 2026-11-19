-- SPDX-License-Identifier: AGPL-3.0-or-later

-- A NULL target is an office-level entry (an office hold or release). SQLite
-- cannot drop NOT NULL in place, so the table is rebuilt and every row copied.
CREATE TABLE audit_actions_rebuilt (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    actor           TEXT    NOT NULL,
    target_agent_id TEXT,
    action          TEXT    NOT NULL,
    result          TEXT    NOT NULL,
    at              INTEGER NOT NULL
);

INSERT INTO audit_actions_rebuilt (id, actor, target_agent_id, action, result, at)
SELECT id, actor, target_agent_id, action, result, at FROM audit_actions;

DROP TABLE audit_actions;
ALTER TABLE audit_actions_rebuilt RENAME TO audit_actions;

CREATE INDEX audit_actions_by_agent ON audit_actions (target_agent_id, id);
