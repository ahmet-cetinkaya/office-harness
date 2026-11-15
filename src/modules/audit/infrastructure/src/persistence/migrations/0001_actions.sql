-- SPDX-License-Identifier: AGPL-3.0-or-later

-- No foreign key on target_agent_id: entries outlive the agent they name.
CREATE TABLE audit_actions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    actor           TEXT    NOT NULL,
    target_agent_id TEXT    NOT NULL,
    action          TEXT    NOT NULL,
    result          TEXT    NOT NULL,
    at              INTEGER NOT NULL
);

CREATE INDEX audit_actions_by_agent ON audit_actions (target_agent_id, id);
