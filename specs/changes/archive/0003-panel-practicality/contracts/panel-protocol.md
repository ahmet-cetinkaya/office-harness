# Contract: panel protocol (0003)

Date: 2026-11-17. Extends the 0001 wire protocol (`src/shared/wire/src/`): a request is `{"id":<u32>,"type":"<name>",...}`, a response `{"id":<u32>,"ok":true,"result":...}` or `{"id":<u32>,"ok":false,"error":{"code":"<code>","message":"<text>"}}`, and an event is a text frame `{"type":"<name>",...}`. Error codes: `invalid`, `conflict`, `not_found`, `refused`, `internal`.

This contract lists every new type of this change and nothing else. Where 0002 is merged, the agent snapshot below additionally carries `role` and `team`; those fields are 0002's, defined in `specs/changes/archive/0002-agent-hierarchy/contracts/hierarchy-protocol.md`.

## Changed response: `agent.list`

Each agent in the result array gains:

| Field | Type | Meaning |
|---|---|---|
| `last_line` | string or null | The last non-empty screen line of the agent's current terminal, without color and cursor codes, cut to 200 characters (D2). `null` when the agent has no line yet (never started, no output, or blank screen: the spec's edge case) |

## New event: `agent.line`

Sent when an agent's last line changes, at most once per agent per second, only when the text changed (D2).

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent whose line changed |
| `line` | string | the new last line, 0 to 200 characters, no color or cursor codes |

## Changed setting: `settings.get` / `settings.set` with key `last_view`

| Field | Type | Rule |
|---|---|---|
| `key` | string | `last_view` |
| `value` | string | JSON of the shape below, at most 1 KiB (D6) |

Value shape: `{"project": <string or null>, "agent": <string or null>, "terminal_open": <boolean>}`. A value that is not this shape, or longer than 1 KiB, is refused with `invalid` naming the field (T4). `settings.get` returns the stored text unchanged.

## New requests (user client)

### `notification.list`

| Field | Type | Rule |
|---|---|---|
| `unread_only` | boolean? | optional, default `false` |

Result: an array of at most 500 notifications, unread first, then newest first (D4). Each entry:

```
notification := { id, agent_id, agent_name, event, detail?, created_at, read_at }
event := "exited" | "failed" | "waiting_for_input" | "bell" | "auto_hold" | "resume_hold"   // auto_hold and detail added by 0007, resume_hold by 0013
detail := the limit name for auto_hold; the reason for resume_hold; absent or null otherwise
created_at, read_at := unix epoch milliseconds; read_at is null while unread
```

### `notification.read`

| Field | Type | Rule |
|---|---|---|
| `notification_id` | string | an existing notification |

Marks that one notification read: sets `read_at` to now, leaves an already-read row unchanged. Result: `{}`. Errors: `not_found`.

### `notification.read_agent`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | an existing or deleted agent's id |

Marks all of that agent's unread notifications read. Result: `{}`. Errors: none beyond transport errors; an unknown agent id is not an error, because the rows are the target, not the agent.

## New event: `notification.created`

Sent after a notification row is created (D4, D5).

| Field | Type | Rule |
|---|---|---|
| `notification` | object | one `notification` entry, as in `notification.list` |

No other `notification.*` event exists: a read changes no visible state for other clients, so no `notification.read` event is sent (D5 keeps the client that read it as the only actor).

## Additive change by 0007

- `event` may also be `"auto_hold"`: the office put an agent on hold after 3 limit refusals in 60 seconds.
- A `notification` entry and the `notification.created` event carry an optional `detail` string. For `auto_hold` it is the name of the limit. For every other event it is absent or null.

See `specs/changes/archive/0007-hold-and-limits/contracts/hold-protocol.md`.

## Additive change by 0008

- The new requests `needs_you.list` and `needs_you.dismiss` and the event `needs_you.changed` are defined in `specs/changes/0008-needs-you-queue/contracts/needs-you-protocol.md`.
- `notification.read` and `notification.read_agent` now also clear the matching needs-you items, and the server then sends `needs_you.changed`. Their requests and answers keep the shapes above.

## Changed validation: `settings.set` with key `focus_shortcut`

A value that equals one of the fixed shortcuts (`Ctrl+Shift+F1` to `Ctrl+Shift+F9` for the agents, `Ctrl+Shift+F10` for the switcher) is refused with `conflict`, and the message names the conflicting combination (D8, spec edge case). The agent and switcher shortcuts are fixed by this spec, so the server holds the same table the desktop app shows; the desktop client checks before it sends, and the server is the authority that a stale client cannot talk out of.

Later changes added two entries to the same table: `Ctrl+Shift+F11` (jump to the agent that needs you, change 0008) and `Ctrl+Shift+H` (the office hold, change 0007). The table has 12 entries, and the server refuses all of them as a focus shortcut (`src/modules/agent/domain/src/value_objects/settings.rs:33`). A focus shortcut that was stored as `Ctrl+Shift+F11` before change 0008 stays valid, and the jump key is then off.

## What does not change

- The status events that cause notifications keep their 0001 shapes (`agent.status` with `status` in `exited`, `failed`, `waiting_for_input`); the notification bridge consumes them, it does not re-emit them.
- `settings.get` / `settings.set` for the other existing keys keep their 0001 behavior.
