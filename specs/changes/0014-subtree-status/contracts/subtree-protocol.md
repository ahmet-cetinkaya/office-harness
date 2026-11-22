# Subtree status protocol (0014)

One new request on the agent connection (`office-harnessd agent-call`, 0002 contract). The envelope, the proof, and the actor rules are the 0002 ones. User clients are not offered this request: on the office connection it is an unknown `agent.*` type, answered `invalid` like any other unknown type.

## `agent.subtree` (agent caller)

Takes no fields. A frame that carries any field is `invalid`, and the refusal is recorded with the caller as the actor (FR-005). A frame that names `actor` or `caller` is `refused` first, as for every agent request (0002 contract).

Result: an array with one entry per descendant of the caller, in tree order: each entry follows its parent, and siblings keep the order of the tree rows. The caller itself, its ancestors, and every other agent are absent (FR-002). A caller without descendants gets `[]`.

| Field | Type | Meaning |
|---|---|---|
| `agent_id` | string | The descendant. |
| `name` | string | Its name, in visible form (below). |
| `parent_agent_id` | string | Its parent; the caller for a child of the caller. |
| `role` | `"lead"` or `"member"` | Its role. A descendant is always placed, so the role is never null. |
| `status` | string | One of the 0001 statuses: `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`. |
| `last_output_at` | number or null | Unix ms of the last write of its CLI to the terminal in the current session; the session's start when the CLI wrote nothing yet; `null` when the CLI does not run (A1). |
| `needs_you` | boolean | Whether it has an item in Needs you (0008). |
| `held` | boolean | Whether the office hold or an agent hold on it or on one of its ancestors covers it (0007/FR-004, A4). |
| `task` | object or null | `{line, set_at}`, or `null` when the agent has no task text. `line` is the task line: the first line of the task text (cut at the first CR or LF), at most 80 characters (Unicode scalar values) followed by `…` when anything of the text is left out (A9), in visible form (below). `set_at` is the Unix ms of the task text (0011). The full text and its `source` are not sent (FR-003). |

**Visible form.** `name` and `task.line` never carry a control or bidirectional format character as itself; the server replaces each one, after the cut, the same way the desktop's task line does (0011/FR-006): a C0 character U+0000 to U+001F becomes its control picture U+2400 to U+241F, DEL U+007F becomes `␡`, and a C1 character U+0080 to U+009F or a bidirectional format character U+202A to U+202E or U+2066 to U+2069 becomes the six-character text `\uXXXX` (upper-case hex). So U+001B arrives as `␛`, U+009B as `\u009B`, and U+202E as `\u202E` (SC-007). No other field is sent. In particular the entry never carries `last_line`, `command`, `working_folder`, `machine_id`, `worktree`, `additions`, or any terminal bytes (FR-003). The server builds each entry field by field, so a field that a later change adds to the agent object does not reach this answer by accident.

Errors:

| Code | When | Recorded |
|---|---|---|
| `invalid` | The frame carries a field. | Yes, the caller as actor. |
| `refused` | The frame names `actor` or `caller`. | Yes. |
| `internal` | The tree, the agents, or Needs you cannot be read. | Yes. |

An answered read writes no action record entry and publishes no event (FR-005, A3). Holds and rate limits do not apply to it (FR-004): it is not one of the agent-made actions of 0007.

## Amends

- 0002 `hierarchy-protocol.md`, "New requests (agent caller)": `agent.subtree` joins the allowed types.
- The not-allowed refusal text of the agent connection names the read among what an agent may do.
