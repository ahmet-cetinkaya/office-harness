-- SPDX-License-Identifier: AGPL-3.0-or-later

-- Why an action happened, when the action alone does not say it (0013): the
-- reason a new session replaced a continue. NULL for every entry written
-- before this script and for every action that needs no reason, so old rows
-- read as they did.
ALTER TABLE audit_actions ADD COLUMN detail TEXT;
