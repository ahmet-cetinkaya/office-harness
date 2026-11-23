# Contract: needs-you protocol (0008)

Date: 2026-11-20. Extends the 0001 wire protocol and the 0002, 0003, and 0007 contracts. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## New requests (user client only)

The agent connection never answers these. They are outside its allowlist and are refused as not allowed (0002 contract). The code is `refused`. The message is the not-allowed text that every other user-only request gets.

A field a request does not name below is refused with `invalid` and the message `<field> is not a field of <request>`, so a misspelt `agent_id` never acts on the office.

### `needs_you.list`

No fields. Result:

```
{ items: [item] }
item := {
  agent_id: string,
  kind: "failed"|"waiting_for_input"|"bell"|"auto_hold"|"resume_hold"|"exited",   // resume_hold amended by 0013
  detail: string?,                  // null when the kind has none
  first_event_at: integer,          // Unix ms; the wait time counts from it
  latest_at: integer,               // Unix ms
  origin_notification_id: string?   // null for a coalesced bell
}
```

The items come in the FR-003 order: group (`failed`; then `waiting_for_input`, `bell`, `auto_hold`, `resume_hold`; then `exited`), then `first_event_at` ascending, then agent name. The list holds at most one item per agent. The list is empty when no agent waits (FR-008).

`detail` per `kind` (data model, D3): `waiting_for_input` the quiet period in ms that the terminal runtime started with; `failed` the `agent.status` frame's `detail` exactly as the server writes it, for example "the command ended with exit code 2", or the launcher's error text for a start failure; `exited` null, because an exit with code 0 carries no detail; `auto_hold` the limit name; `bell` null. The desktop shows `kind` and `detail` together and does not parse `detail`. It computes the wait time from `first_event_at` (FR-002).

Errors: `internal` when the items cannot be read.

### `needs_you.dismiss`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent whose item leaves (FR-004) |

Result: `{items}`, the list after the dismiss, in the same order. A dismiss marks no notification read and changes no agent status (A4). It is not recorded in the action record.

Errors:

| Code | Message | When |
|---|---|---|
| `invalid` | `agent_id is not a valid identifier: <text>` | the value is not an id (the code and message that `notification.read_agent` gives) |
| `not_found` | `no needs-you item for the agent` | the agent has no item, or no agent has this id |
| `internal` | as above | the item cannot be removed; the item stays |

## New event

### `needs_you.changed`

`{type:"needs_you.changed", items:[item]}`: the full ordered list after any change. Clients replace their copy, as for `hold.changed`.

The server publishes it after each of these:

- an attention event records or updates an item (FR-001). The item is stored first, then `notification.created` is published for the same event, then `needs_you.changed`;
- a clear: `notification.read_agent`, `notification.read` of the origin, `needs_you.dismiss`, status `starting`, `working`, or `stopped`, an agent delete, or the release of a hold when the item's latest event is `auto_hold` (FR-004, FR-005).

The event arrives within 2 seconds of the attention event. For `waiting_for_input` the clock starts when the quiet period ends (SC-003). A client that connects after the event gets the item from `needs_you.list` (FR-006, SC-004).

The server never learns which agent is on screen. A client that marks the origin notification read on arrival (0003/FR-006 (b)) removes the item through `notification.read`, so the event follows at once.

## Amended by 0015

- An item's `kind` may be `report`, with the report's kind as `detail`; `report` sorts with `waiting_for_input`, `bell`, `auto_hold`, and `resume_hold`.
- Each item carries `report`: `null`, or `{kind, text, at}` with the latest report for the person, its text in visible form. A later event keeps it; a resume or a hold release does not remove an item that holds one.

See `specs/changes/archive/0015-upward-reports/contracts/report-protocol.md`.

## Amended by 0022

- No message changes shape. Only the moments when `needs_you.changed` is sent change.
- An unprompted change to `waiting_for_input` (a screen redraw with no person input, office text, or session start since the last wait) adds no item and changes no item, so no event follows it.
- An unprompted change to `working` leaves the item until the stretch of `working` passes the work bound (the quiet period plus 10 seconds). Then the item leaves as for a resume.

See `specs/changes/archive/0022-waiting-item-sleep/SPEC.md`.

## Unchanged messages

- `notification.read_agent {agent_id}` now also clears the agent's item (FR-004). Its request and result do not change.
- `notification.read {notification_id}` now also clears the item whose `origin_notification_id` equals `notification_id`. Its request and result do not change.
- `notification.list` and `notification.created` do not change (PB-1).
- `agent.status` frames and `agent.delete` do not change. The server hooks them (FR-005).

## Changed shapes

- `settings.get` / `settings.set`, key `focus_shortcut`: the reserved shortcuts are now 12. `Ctrl+Shift+F11` is refused as reserved for the jump shortcut, with the conflict named, as for the other reserved keys (spec edge case "user changes the focus shortcut to the jump shortcut").
- A stored `focus_shortcut` that equals `Ctrl+Shift+F11` after normalizing (`Shift+Ctrl+F11` too) is kept when the settings load. It does not fall back to the default. The jump shortcut then has no key (spec edge case, A1). A stored `F1` to `F10` or `Ctrl+Shift+H` still falls back to the default as before.

## Reserved shortcuts (the 12th)

| Key | Kind | Action |
|---|---|---|
| `Ctrl+Shift+F11` | `jump` | Open the terminal of the agent at the top of Needs you, in the panel; open its desk in the scene (FR-007, FR-010). |

- The first 11 entries are unchanged: `Ctrl+Shift+F1` to `F10` (agents and switcher, 0003 A1) and `Ctrl+Shift+H` (office hold, 0007).
- Every table entry leaves a focused terminal (MODIFIED 0001/FR-017). The key does not reach the CLI.
- The key is fixed in the MVP, as the 0003 agent shortcuts are. The per-OS fallback is not shipped yet.
- While the stored focus shortcut equals `Ctrl+Shift+F11`, the focus shortcut keeps the key and the jump entry has no key.
