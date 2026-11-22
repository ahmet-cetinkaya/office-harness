# Contract: hold protocol (0007)

Date: 2026-11-19. Extends the 0001 wire protocol and the 0002 and 0003 contracts. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## New requests (user client only)

The agent connection never answers these; they are outside its allowlist and are refused as not allowed (0002 contract).

A field a request does not name below is refused with `invalid` and the message `<field> is not a field of <request>`, so a misspelt `agent_id` never acts on the office.

### `hold.set`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string? | the agent to hold; absent means the office |

Result: `{hold}`. Errors: `conflict` "the office is already on hold" or "the agent is already on hold"; `not_found` for an unknown agent; `internal` when the hold cannot be stored (the previous state stays).

### `hold.release`

Same field. Result: `{}`. Errors: `conflict` "the office is not on hold" or "the agent is not on hold"; `not_found`; `internal` as above. Releasing the office leaves agent holds in place (edge case).

### `hold.list`

No fields. Result:

```
{ office: hold?, agents: [hold] }
hold := { agent_id?, set_by: "user"|"office", reason?, since }   // agent_id absent for the office hold; since in Unix ms
```

### `office.actions`

No fields. Result: the office-level action record entries (no target agent), newest first, at most 500. Each entry has the `action.recorded` entry shape with `agent_id: null` (A11).

## New event

### `hold.changed`

`{type:"hold.changed", office: hold?, agents: [hold]}`: the full hold state after any hold, release, automatic hold, or prune. Clients replace their copy.

## Changed shapes

- Action entries: `actor` may be `"office"`; `action` may be `"hold"` or `"release"`; `agent_id` may be `null` for office-level entries.
- `notification` entries and `notification.created`: `event` may be `"auto_hold"`.
- `settings.get` / `settings.set`: the keys `limit_directives_per_agent`, `limit_directives_per_target`, `limit_creates_per_agent`, `limit_lifecycle_per_agent` (whole numbers 1 to 1000, defaults 10, 3, 5, 10).
- `settings.set` with key `focus_shortcut`: `Ctrl+Shift+H` is refused as reserved for the office hold.

## Agent caller answers (`office-harnessd agent-call`)

`directive.send`, `agent.create`, `agent.start`, `agent.stop`, and `agent.new_task` may now also fail with `refused` and one of these messages, checked after the 0002 rules in this order (spec A10):

| Order | Message |
|---|---|
| 1 | `the office is on hold` |
| 2 | `the agent <id> is on hold` (the caller, the target, or one of their ancestors) |
| 3 | `the limit "<name>" allows <n> per 60 seconds` with `<name>` one of `directives sent by one agent`, `directives sent to one target agent`, `agents created by one agent`, `starts, stops, and new tasks by one agent` |

Each refusal is recorded with the calling agent as actor (MODIFIED 0002/FR-009). A refused `directive.send` is recorded in the action record of the sender and in the action record of the target, one entry each, both published as `action.recorded` to every user client; when sender and target are one agent, one entry. An accepted directive stays in the target's record only. An agent placed on hold automatically is recorded as `hold` with actor `office`.
