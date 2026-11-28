# Data model: task line per agent (0011)

Date: 2026-11-22. Extends the agent module of 0001 and 0002.

## Columns

The agent module's migration `0006_task_text.sql` adds three nullable columns to `agent_agents` (D1). The three are all set or all null.

| Column | Type | Meaning |
|---|---|---|
| `task_text` | `TEXT` | The task text, stored as entered. Control characters are kept (FR-006 is a display rule). |
| `task_set_at` | `INTEGER` | The time the text was set, in Unix ms, from the server clock. |
| `task_source` | `TEXT` | `directive` or `user`. |

An office at agent version 5 upgrades with every agent intact and the three columns null. The migration is forward-only. Deleting the agent deletes its row, so its text goes with it (edge case).

## Rules

- A directive that the office entered into the terminal sets the text, the time, and the source `directive` (FR-002, D2). A refused directive or a failed write changes nothing.
- `agent.task.set` sets the source `user` (FR-003, D4). Text that is empty or only whitespace clears all three columns. More than 8,192 characters, counted as Unicode scalar values, is refused (FR-003).
- A successful `agent.new_task` clears all three columns (FR-004, D3). `agent.start` and `session.continue` keep them (PB-2).
- Terminal output and window titles never write these columns (FR-007).

## Action record

A new action kind `task_text` is recorded for each `agent.task.set`, with the user as actor and the agent as target. A directive adds no entry of its own, because it is already recorded as `directive` (A4). The `audit_actions.action` column is `TEXT` with no CHECK, so no audit migration is needed.
