# Data model: hold switch and agent action limits (0007)

Date: 2026-11-19. Plan: `PLAN.md` decisions D1 to D7.

## Hold (hierarchy module)

Table `hierarchy_holds`, migration `0002_holds.sql` of the hierarchy ledger.

| Column | Type | Rule |
|---|---|---|
| `scope` | TEXT PRIMARY KEY | `office`, or the held agent's id (UUID text) |
| `set_by` | TEXT NOT NULL | `user` or `office` (the automatic hold of FR-008) |
| `reason` | TEXT | NULL for a user hold; the limit text for an automatic hold |
| `since` | INTEGER NOT NULL | Unix milliseconds |

- At most one office row and at most one row per agent (FR-006, Key entities).
- A release deletes the row. The action record keeps the history (FR-010).
- No foreign key: the hierarchy ledger has none across modules. Holds of deleted agents are removed by the delete request and pruned at startup (D10).

Domain types: `HoldScope { Office, Agent(AgentId) }`, `HoldSetter { User, Office }`, `Hold { scope, set_by, since, reason }`.

## Rate windows (memory only)

| Limit kind | Setting key | Default | Counted per |
|---|---|---|---|
| `directive` | `limit_directives_per_agent` | 10 | sender |
| `directive_target` | `limit_directives_per_target` | 3 | sender and target |
| `create` | `limit_creates_per_agent` | 5 | caller |
| `lifecycle` | `limit_lifecycle_per_agent` | 10 | caller (`agent.start`, `agent.stop`, `agent.new_task`) |

Each window keeps the times of the actions that passed in the last 60 seconds. The automatic-hold tracker keeps the times of each agent's rate-limit refusals in the last 60 seconds; the third one holds the agent (A5). Both start empty after a server restart.

## Settings (agent module)

Four new `SettingKey`s from the table above. A value is a whole number from 1 to 1000; anything else is refused with `invalid` and the message "a limit is a whole number of at least 1" (edge case), the upper bound in the message where the value is above it.

## Action record (audit module)

- `kernel::Actor` gains `Office`; text form `office` (the existing forms are `user` and `agent:<id>`).
- `ActionKind` gains `hold` and `release`.
- `ActionRecord.target_agent_id` becomes optional. The audit migration rebuilds the table with a nullable `target_agent_id` column and copies every row unchanged. A NULL target is an office-level entry (office hold or release, A11).
- Refusals keep the kind `refused`; the result text is the reason (the limit or the hold).

## Notification (notification module)

`NotificationEvent` gains `auto_hold`. One is created when the guard moves an agent from not held to held by the office (FR-008, D7). The focused-terminal exception (b) of 0003/FR-006 does not apply to it.
