-- SPDX-License-Identifier: AGPL-3.0-or-later

-- What an event adds to its name (0007 D7): the limit that held the agent for
-- an auto_hold notification. NULL for the four 0003 events and every row
-- written before this script, so old rows read as they did.
ALTER TABLE notification_notifications ADD COLUMN detail TEXT;
