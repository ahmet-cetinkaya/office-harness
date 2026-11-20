# Data model: Needs you queue (0008)

Date: 2026-11-20. Plan: `PLAN.md` decisions D1 to D3.

## Needs you item (notification module)

Table `notification_needs_you`, migration `0003_needs_you.sql` of the notification ledger (after `0001_notifications.sql` and `0002_detail.sql`). The script only adds a table. It does not touch the rows of `notification_notifications`.

| Column | Type | Rule |
|---|---|---|
| `agent_id` | TEXT PRIMARY KEY | The agent's id (UUID text). One row per agent (FR-001, Key entities). |
| `first_event_at` | INTEGER NOT NULL | Unix milliseconds. The time of the first attention event that the user has not handled. It sets the wait time (FR-003). A new event does not change it. |
| `latest_kind` | TEXT NOT NULL | One of: `failed`, `waiting_for_input`, `bell`, `auto_hold`, `exited` (FR-002, FR-003). |
| `latest_detail` | TEXT | The detail of the latest event. NULL when the kind has none (see below). |
| `latest_at` | INTEGER NOT NULL | Unix milliseconds. The time of the latest event. |
| `origin` | TEXT | The id of the 0003 notification that made the item. NULL when no notification was made (a bell that 0003 coalesces, FR-001). The wire calls it `origin_notification_id`. An index on it serves the clear on read. |

- No foreign key. The notification ledger has none across modules, and the agent row lives in another module. Items of deleted agents are removed by the delete request and dropped at startup (FR-005).
- A clear deletes the row. The next attention event of that agent inserts a new row with a new `first_event_at` (edge case "dismiss, then a new event").
- No agent name or project column. The desktop reads both from the agent list.
- The name is not stored, so the name tie-break of FR-003 uses a lookup that the caller passes to the order function.

Domain types: `NeedsYouItem { agent_id, first_event_at, latest: AttentionEvent { kind, detail, at }, origin }`.

## Stored detail per kind (D3)

| `latest_kind` | `latest_detail` | Shown as (FR-002) |
|---|---|---|
| `waiting_for_input` | The quiet period in ms, as decimal text, that the terminal runtime started with (for example `5000`). A later `settings.set` does not change it, because the terminal keeps its start value too. | "quiet for 5 s" |
| `failed` | The `detail` of the `agent.status` frame exactly as the server writes it: "the command ended with exit code 2" for a command that ended with a non-zero code, or the launcher's error text for a start failure. | "failed" and the text |
| `exited` | NULL. An exit with code 0 has no frame detail. | "exited" |
| `auto_hold` | The limit name that held the agent. | "put on hold by the office, limit: <name>" |
| `bell` | NULL. | "bell" |

The server stores the facts. The desktop writes the words from `latest_kind` and the stored text. It does not parse the text. The detail is never a guess about the agent's intent (0001 A1, 0003 A3). When a frame has no detail, `latest_detail` is NULL and the row shows the kind alone.

## Order (D1, FR-003)

The order is not stored. A pure function sorts the rows at read time.

| Step | Rule |
|---|---|
| 1 | Group rank by `latest_kind`: `failed` is 0. `waiting_for_input`, `bell`, `auto_hold` are 1. `exited` is 2. |
| 2 | Inside a group: `first_event_at` ascending (longest wait first). |
| 3 | Same `first_event_at`: agent name, ascending. |

A waiting agent that then fails moves to group 0 and keeps its `first_event_at` (A3).

## Record rules (D3, FR-001)

- The bridge creates the 0003 notification first, without publishing it. It then stores the item with `origin` set to the new notification id, then publishes `notification.created`, then `needs_you.changed`. A client that reads the notification on arrival therefore finds the item to clear.
- A second event for an agent that has an item updates `latest_kind`, `latest_detail`, and `latest_at`. It keeps `first_event_at` and `origin`.
- A bell that 0003 coalesces still records the event. It makes no notification. For a new item its `origin` is NULL.
- The record step runs after the notification is created. The coalescing check reads the unread notification state, not this table.
- An agent that no longer exists gets no item. Its notification is still created, with the name "deleted agent".
- One lock covers each queue write, the list read after it, and the publish, so changes publish in the order they happen.
- `stopped` and `interrupted` record nothing (PB-3).

## Clear reasons (D2, FR-004, FR-005)

| Reason | Trigger | Message or source |
|---|---|---|
| Opened | The user opens the agent's terminal by any route. | `notification.read_agent {agent_id}` (FR-004) |
| Read on arrival | The client marks the item's origin notification read. | `notification.read {id}` equal to `origin` (FR-001 edge case "terminal open in the focused window") |
| Dismissed | The user dismisses the item. It marks no notification read and changes no status (A4). | `needs_you.dismiss {agent_id}` (FR-004) |
| Status | The agent changes to `starting`, `working`, or `stopped`. | `agent.status` frame (FR-005) |
| Deleted | The agent is deleted. | `agent.delete` request, and the startup drop of items whose agent is missing (FR-005) |
| Hold released | The hold of the agent is released and `latest_kind` is `auto_hold`. | Hold journal `released` (FR-005) |

- An office hold release does not clear an `auto_hold` item. It releases no agent hold (0007 edge case).
- `interrupted` neither adds nor clears an item (PB-3, edge case "server restarts").
- Every clear that removes a row publishes `needs_you.changed`. A clear that finds no row publishes nothing.

## Survival (FR-006)

The table is in `office.db`. Items survive app closure and a server restart. An event that happens while no desktop app is open is stored, and the next `needs_you.list` returns it with its first event time (SC-004).

## Migration

- `0003_needs_you.sql` is forward-only, like the earlier scripts. A binary that does not know it refuses the upgraded database (`DatabaseTooNew`).
- A database at notification `0002` upgrades with every notification row unchanged (T2 test).
