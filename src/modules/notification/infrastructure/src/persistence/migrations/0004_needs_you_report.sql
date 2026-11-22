-- SPDX-License-Identifier: AGPL-3.0-or-later

-- The latest report for the person on an item (0015/FR-008): its kind, its
-- text in visible form, and its time. NULL when the item holds no report, as
-- every row written before this script does.
ALTER TABLE notification_needs_you ADD COLUMN report_kind TEXT;
ALTER TABLE notification_needs_you ADD COLUMN report_text TEXT;
ALTER TABLE notification_needs_you ADD COLUMN report_at INTEGER;
