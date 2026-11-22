# Data model: 0003 panel practicality

Date: 2026-11-17. Style follows the 0001 migrations (`src/modules/agent/infrastructure/src/persistence/migrations/`) and the 0002 data model.

## Principle

Each module owns its prefixed tables and its schema ledger over the one shared `office.db`. The notification module owns `notification_*` tables and the ledger `notification_schema_version`, created by `run_migrations` the same way the agent and audit ledgers are (`src/apps/office-harnessd/src/bootstrap/migrations.rs`). The last output line is terminal state, so the terminal module keeps it in memory only; it is rebuilt from the screen, not stored. The last view is one `agent_settings` row, so the desktop app stores nothing locally.

## Notification module migration 0001 (notifications, ledger)

```sql
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- One row per event that the user must see (D4). The agent name is copied at
-- creation, so the row survives a deleted agent (spec edge cases).
CREATE TABLE notification_notifications (
    id         TEXT PRIMARY KEY,
    agent_id   TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    -- One of: exited, failed, waiting_for_input, bell (D4).
    event      TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    -- NULL while unread (FR-007, FR-009).
    read_at    INTEGER
);

-- Unread-first listing reads this index (D4: unread first, newest first).
CREATE INDEX notification_notifications_unread
    ON notification_notifications (read_at, created_at DESC);
```

The ledger table `notification_schema_version` is created by the shared runner, not by the script. The notification table references no agent table on purpose: a notification outlives its agent, and `ON DELETE` rules would delete what the user still must read (spec edge case: a notification for a deleted agent says so and stays). The bridge copies the agent name at creation from the event it already carries; the notification module never reads the agent module's tables.

## Last output line (no table)

`agent.line` text is runtime state of the terminal module, held next to the session's emulator, cut to 200 characters (D2). A server restart clears it; the client falls back to the status alone until the next output. No table, no migration.

## Last view (agent_settings row)

The `agent_settings` table of 0001 (key `TEXT PRIMARY KEY`, value `TEXT NOT NULL`) gains one row:

| Key | Value | Written by |
|---|---|---|
| `last_view` | JSON `{"project": <id or null>, "agent": <id or null>, "terminal_open": <bool>}`, at most 1 KiB | the desktop app, at most once per second and on close (D6) |

No new table and no agent-module migration; the table is key-value, so the validator (T4) is the only change on the server. The value is validated JSON, never evaluated (plan security pass).

## What is not stored

- No read-state per client: one office, one reader, the read time is on the row (A4).
- No notification body text: the event kind plus the agent name is all the UI shows, and no agent output beyond the 200-character `agent.line` ever enters a notification (plan security pass).
- No shortcut table on the server: the shortcuts live in the desktop app's key-binding table (D8).
