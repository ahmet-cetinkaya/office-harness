-- SPDX-License-Identifier: AGPL-3.0-or-later

-- Whether an item has recorded only changes to `waiting for input` (0022/D6).
-- Every row written before this script reads as 0, so it keeps blocking an
-- idle sleep as before.
ALTER TABLE notification_needs_you ADD COLUMN waits_only INTEGER NOT NULL DEFAULT 0;
