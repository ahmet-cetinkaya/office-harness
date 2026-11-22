# Contract: resume protocol (0013)

Date: 2026-11-23. Extends the 0001 wire protocol and the 0007 hold contract. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## New requests (user client only)

The agent connection never answers these; they are outside its allowlist and are refused as not allowed (0002 contract). A field that a request does not name below is refused with `invalid` and the message `<field> is not a field of <request>`.

### `agent.resume`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to resume |
| `after_session` | string? | the fallback step: the session the CLI refused |

Without `after_session`: the agent must be `interrupted`. The server continues its last session when the CLI can continue it, otherwise it starts a new session.

With `after_session`: that session must still be the agent's last session, and the agent must be `exited` or `failed`. The server then starts a new session.

Result: `{session, continued: bool, reason?}`. `reason` says why a new session started instead of a continue. A command that cannot start answers success; the session's agent is `failed`, and the failure text is in the `agent.status` event, as for `agent.start`.

Errors:
- `conflict` "the agent is no longer interrupted".
- `conflict` "the agent changed after the refused session".
- `not_found` for an unknown agent.
- `refused` "the server is stopping and starts no agent".

### `resume.plan`

No fields. Result, in start order (members and agents outside the tree, then leads, then the CEO):

```
{ agents: [ { agent_id, last_session: session?, can_continue: bool, reason?, held: bool, after_unclean_stop: bool } ] }
session := the session.list entry shape (id, started_at in Unix ms, ...)
```

`reason` says why the last session cannot be continued. `after_unclean_stop` is true when the last session's `exit_result` is `lost`. Only `interrupted` agents are listed.

### `resume.start`

| Field | Type | Rule |
|---|---|---|
| `agent_ids` | [string] | 1 to 1000 distinct agent ids |

Result: `{order: [agent_id]}`, the ids in start order. The server answers before it starts anyone, and reports each agent through `resume.result`.

Errors: `invalid` for an empty list, a duplicate id, more than 1000 ids, or an id that is not an agent id. An unknown or no longer interrupted agent is not an error: its result is `skipped`.

## New event

### `resume.result`

```
{ type: "resume.result", agent_id, result, continued?: bool, reason?, is_final: bool }
result := "checking" | "continued" | "new_session" | "held" | "skipped"
```

- `checking`: a start is in its check period; `continued` tells whether it was a continue. `is_final` is false.
- `continued`, `new_session`: the session ran through its check period, or the user acted on the agent. `new_session` has a `reason`.
- `held`: the new session also ended within its check period, or did not start; `reason` names why. When the hold could not be stored, `reason` says so.
- `skipped`: the agent was not resumed; `reason` names why.

Every user client receives it.

## Changed shapes

- Action entries (`agent.actions`, `office.actions`, `action.recorded`): a new field `detail`, a string or null. It is set only on a `start` that began a new session instead of a continue (spec FR-007), with the reason. A fallback start is `start` with actor `office`; a new session started at once because the CLI cannot continue is `start` with actor `user`.
- Sessions (`session.list`, `resume.plan`): `exit_result` may be `"lost"`, for a session that was still open when the server stopped uncleanly (set at the next server start). A clean stop still gives `"interrupted"`. A `lost` session can be continued like an `interrupted` one.
- Holds: `set_by: "office"` with a `reason` can now also come from a failed resume.
- `notification` entries, `notification.created`, and Needs-you items: `event` and `kind` may be `"resume_hold"`, with the reason as `detail`. Releasing the hold clears the Needs-you item, as for `auto_hold`. This extends the event list of the archived panel protocol (0003) and the notification note of the archived hold protocol (0007); those archived files stay as they were, and the living spec takes this in when 0013 is archived.
