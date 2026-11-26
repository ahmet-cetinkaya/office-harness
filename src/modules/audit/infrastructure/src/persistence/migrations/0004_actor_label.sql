-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The name a schedule had when it acted (0025 D7), so an entry still shows
-- it after a rename or a delete. NULL for every entry written before this
-- script and for every actor that has no label, so old rows read as they did.
ALTER TABLE audit_actions ADD COLUMN actor_label TEXT;
