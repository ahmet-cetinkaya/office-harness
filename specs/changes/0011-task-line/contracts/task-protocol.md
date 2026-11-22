# Contract: task protocol (0011)

Date: 2026-11-22. Extends the 0001 wire protocol and the 0002 and 0007 contracts. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## New request (user client only)

The agent connection never answers it; it is outside the agent allowlist and is refused as not allowed (0002 contract).

### `agent.task.set`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to label |
| `text` | string | the task text; empty or only whitespace clears it |

Checks, in order: a blank text clears the task and is not refused; then more than 8,192 characters answers `invalid` with "a task text has at most 8,192 characters". Control characters are accepted and stored as is.

Result: the agent (`agent_json`, with `task`). Errors: `invalid` for a missing or mistyped field or the length; `not_found` for an unknown agent; `internal` when the text cannot be stored. The request writes nothing to the terminal (A2). A success is recorded as `task_text` with the user as actor.

## Changed shapes

- `agent_json` (in `agent.list`, `agent.create`, `agent.changed`, and the result of `agent.task.set`) gains `task`: `{text, set_at, source}` with `set_at` in Unix ms and `source` `"directive"` or `"user"`, or `null` when the agent has no task text.
- Action entries: `action` may be `"task_text"`.

## New event

### `agent.task`

`{type:"agent.task", agent_id, task}` with `task` as above or `null`. It carries no `last_line` and no `status`, so a client patches only the task. It is published to every client after:

- a directive is entered into the target's terminal (also when the directive came over an agent connection); amended by 0012: when it is submitted, text and Enter, not when it waits or is left unsubmitted,
- a successful `agent.task.set`,
- a successful `agent.new_task`, with `task: null`.

`agent.changed` is not used for task changes, because it carries `last_line: null` and the stored status.
