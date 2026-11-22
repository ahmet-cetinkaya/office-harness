# Contract: hierarchy protocol (0002)

Date: 2026-11-16. Extends the 0001 wire protocol (`src/shared/wire/src/`): a request is `{"id":<u32>,"type":"<name>",...}`, a response `{"id":<u32>,"ok":true,"result":...}` or `{"id":<u32>,"ok":false,"error":{"code":"<code>","message":"<text>"}}`, and an event is a text frame `{"type":"<name>",...}`. Error codes: `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## New requests (user client)

### `team.create`

| Field | Type | Rule |
|---|---|---|
| `name` | string | 1 to 64 characters, not only spaces |
| `project_id` | string? | optional; an existing project |
| `lead` | object? | the lead agent to create: `name`, `command`, and an optional `working_folder`, as in `agent.create`; the team's `project_id` is the lead's; any other field is `invalid` |
| `lead_agent_id` | string? | an existing agent to pick as the lead; exactly one of `lead` and `lead_agent_id` is given, else `invalid` |

Result: `{team}` (see `team` below). Errors: `conflict` when the name (case-insensitive) exists, or when the lead already leads another team (refused, never moved, so no team keeps a stale seat); `invalid` when the lead fields fail validation; `refused` when a second CEO would be needed.

Events on success: with `lead`, first `agent.changed` for the new lead, then `hierarchy.changed` (`team.created`), then the lead's `action.recorded`; with `lead_agent_id`, only `hierarchy.changed`. A failed `team.create` sends no `agent.changed`.

### `team.delete`

| Field | Type | Rule |
|---|---|---|
| `team_id` | string | an existing team |
| `confirm` | boolean | must be `true`; the client asked the counted question first |

Deletes the team record only; its members keep their agent rows and become no-role agents (they leave the hierarchy). The lead keeps its agent row. Result: `{}`. Errors: `not_found`; `invalid` when `confirm` is not `true`.

### `hierarchy.tree`

No fields. Result: `{tree}`:

```
tree := { ceo: agent?, teams: [team], unassigned: [agent] }
team := { id, name, project_id, lead: agent, members: [agent] }
agent := { id, name, role, status, project_id }   // role: "ceo"|"lead"|"member"; in `unassigned` it is null
```

One read; no pagination (FR-005 forbids a cap; the report of a huge office is a 0003 concern).

### Agent rows carry `role` and `team`

Every agent object in an `agent.*` answer (the `agent.list` rows, the `agent.create` result, and the others) carries `role`: `"ceo" | "lead" | "member" | null` and `team`: the team's display name as a string, or `null`. Both are read at answer time, so a placement or a team rename shows in the next answer. `team` is display-only, never a key: joins and identity go through `hierarchy.tree` (team ids live there). An agent outside the hierarchy carries `null` for both. The `agent.changed` event keeps its 0001 shape; clients re-read on `hierarchy.changed`.

## Changed requests (user client)

### `agent.create` — extended

Optional `role` object: `{ kind: "ceo" | "lead" | "member", parent_agent_id: string?, team_id: string? }`. `kind: "ceo"` takes no parent or team. `kind: "lead"` requires `team_id` and no CEO parent (the team is its place; its parent becomes the CEO when one exists, else none); a team that already holds its lead refuses the create with `conflict`, and a lead placement names that team's lead. `kind: "member"` requires `parent_agent_id` of a lead and takes that lead's team. Refusal `refused` names the roles allowed at the given place (FR-003). Omitted `role` = 0001 behavior (PB-1). A member's or lead's project is its team's project (A6): a `project_id` that differs from the team's is `invalid`, and the team's own `project_id` or none is accepted. A member or lead of a team with no project gets none, so a `project_id` sent for it is `invalid` too.

### `agent.stop` / `agent.delete` — extended

Optional `include_descendants: boolean` (default `false`) and, for `agent.delete`, `descendant_count: number`: what the client showed in its confirmation. The server re-counts; when the count differs it refuses with `conflict` and the current count, so a stale confirmation can never delete the wrong set. Result and other fields as in 0001.

## Agent connection (via `office-harnessd agent-call`)

The `auth` frame of an agent adds `agent_id`; its proof is the office proof keyed with the agent's call credential (the secret half of `OFFICE_HARNESS_AGENT_TOKEN`, text `<agent_id>:<secret>`), so a client cannot tell a wrong proof from an agent that does not run. The credential belongs to one session: a new session of the agent gets a new one, and the credential dies when the child exits. An agent connection has no events and no terminals. The server answers only the types below, and refuses every other type with `refused`.

`agent.whoami` (no fields) returns `{ "agent_id": "<id>" }`. It proves the credential and is the only type that T4 implements.

## New requests (agent caller, via `office-harnessd agent-call`)

Same envelope, but the connection proves an agent call token instead of the office token, and the server fixes the actor to that agent. Allowed types: `agent.create` (with `role`, within its own descendants and the role rules), `agent.start`, `agent.new_task`, `agent.stop` (descendants only), `directive.send`. Any other type, including `agent.delete`, `team.*`, and `hierarchy.tree`, is `refused` with the reason recorded (FR-008, FR-009). The `role` object of an agent's `agent.create` follows the user rules above: `kind: "lead"` requires `team_id` of an existing team (the user's `team.create` makes one), `kind: "member"` takes no `team_id` of its own (its lead's team applies, A6), and a field `role` does not take is `invalid`.

Every entry that a request on the agent connection writes, the record of a refusal included, is sent as `action.recorded` to every user client, so an agent's action shows in an open panel without a reload. A request that the agent connection forwards to the office gateway (`agent.create`, `agent.start`, `agent.new_task`, `agent.stop`) publishes its entries once, there.

### `directive.send`

| Field | Type | Rule |
|---|---|---|
| `target_agent_id` | string | must be a running descendant of the caller |
| `text` | string | 1 to 8192 characters; no C0 control characters (the server appends the Enter itself) |

Result: `{}`. Errors: `refused` (not a descendant: names the rule; not running: says so), `invalid` (control characters, empty, oversize). Every refusal is recorded with the actor and the reason, in the action record of the sender and in the action record of the target (one entry when they are the same agent); an accepted directive is recorded in the target's record only.

> Amended by 0007: `directive.send`, `agent.create`, `agent.start`, `agent.stop`, and `agent.new_task` can also fail with `refused` for a hold or a rate limit. The reasons and their order are in `specs/changes/0007-hold-and-limits/contracts/hold-protocol.md`. They are checked after every refusal above.

## New event

### `hierarchy.changed`

Fields: `{ cause: "team.created" | "team.deleted" | "agent.placed" | "agent.unplaced" | "subtree.stopped" | "subtree.deleted" }`. Sent after any change that alters the tree shape or a role, and only after the change succeeded. Clients re-read `hierarchy.tree`. The causes: `team.created` and `team.deleted` for `team.create` and `team.delete`; `agent.placed` for a successful `agent.create` with a `role` (user or agent connection); `agent.unplaced` for a successful `agent.delete` of an agent that held a place; `subtree.stopped` for a successful `agent.stop` with `include_descendants`; `subtree.deleted` for a successful `agent.delete` with `include_descendants` of a placed agent. When a request also sends `agent.changed` for a new agent, that event goes first. No per-node deltas in 0002 (plan D6).

## Environment contract (agent children)

| Variable | Value |
|---|---|
| `OFFICE_HARNESS_AGENT_TOKEN` | the per-agent call token, present only while that agent's session runs |
| `OFFICE_HARNESS_CLI` | absolute path of the running `office-harnessd` binary |

Helper: `office-harnessd agent-call <method> <json>` reads the token from its inherited environment and performs the loopback proof. The office token never appears in any child environment (research R2).
