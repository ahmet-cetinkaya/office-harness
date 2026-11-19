-- SPDX-License-Identifier: AGPL-3.0-or-later

-- A hold: scope is 'office' or an agent's UUID text, so one row per scope
-- makes a second hold of the same scope impossible. set_by is 'user' or
-- 'office' (an automatic hold); since is unix milliseconds.
CREATE TABLE hierarchy_holds (
    scope  TEXT PRIMARY KEY,
    set_by TEXT NOT NULL,
    reason TEXT,
    since  INTEGER NOT NULL
);
