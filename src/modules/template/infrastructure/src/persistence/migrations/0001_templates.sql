-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per saved template (0009 D1). A template is a copy: no column links
-- it to the agents or teams created from it (FR-006).
CREATE TABLE template_templates (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    -- The lower-cased name, so a name is unique case-insensitively.
    name_key   TEXT NOT NULL UNIQUE,
    -- One of: agent, team.
    kind       TEXT NOT NULL,
    -- The JSON body of the template (0009 D2).
    body       TEXT NOT NULL,
    -- Unix milliseconds of the last save.
    updated_at INTEGER NOT NULL
);
