# Contract: template protocol (0009)

Date: 2026-11-22. Extends the 0001 wire protocol and the 0002 and 0007 contracts. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## Who may call

Every request here is for user clients only. The agent connection never answers them; they are outside its allowlist and are refused as not allowed (0002 contract, A9).

A field a request does not name below is refused with `invalid` and the message `<field> is not a field of <request>`, so a misspelt `template_id` never acts on the office.

## Shapes

```
template := { id, name, kind: "agent"|"team", body, updated_at }   // updated_at in Unix ms; body as in data-model.md
reason   := text                                                   // one refusal reason
```

`invalid` answers carry every reason in the message, one per line (FR-005).

## Requests

### `template.list`

No fields. Result: `{templates: [template]}`, ordered by name case-insensitively.

### `template.get`

| Field | Type | Rule |
|---|---|---|
| `id` | string | the template |

Result: `{template}`. Errors: `not_found` "template <id> does not exist".

### `template.save` (FR-001, FR-003, FR-006, FR-010)

| Field | Type | Rule |
|---|---|---|
| `id` | string? | absent creates; present replaces that template |
| `name` | string | the save rules of data-model.md |
| `kind` | string | `agent` or `team`; must match the body |
| `body` | object | an agent body or a team body |

Result: `{template}`. Errors: `invalid` with the save rule text, or "`kind` is \"<kind>\" but the body is a <other> body"; `conflict` "template name \"<name>\" is already used"; `not_found` for an unknown `id`. Sends `template.changed`. Saving an agent leaves every agent and team created from it unchanged (FR-006), and writes nothing to the action record (A10).

### `template.delete` (FR-006)

| Field | Type | Rule |
|---|---|---|
| `id` | string | the template |

Result: `{}`. Errors: `not_found`. Sends `template.changed`. A team template keeps its own copy of each entry (A5).

### `template.from_agent` (FR-001)

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to copy |

Result: `{body}`, an unsaved agent body with the agent's command, working folder, office additions, and role, and the pattern `<agent name>-{n}` (A2). Nothing is saved and the agent does not change. Errors: `not_found` for an unknown agent.

### `template.create_agent` (FR-002, FR-005, PB-1, PB-2)

| Field | Type | Rule |
|---|---|---|
| `template_id` | string | an agent template |
| `place` | `{parent_agent_id, team_id}`? | where the template's role goes (0002); absent for a template with no role, refused with `invalid` when given for one |
| `project_id` | string? | the project; its folder is the working folder when the template has none |

Result: `{agent}`. The agent is created, not started (A1). Errors: `invalid` with every reason (a taken or too long name, an empty command, a bad environment name, a missing folder named, "a working folder is needed", an unknown project, a `{team}` pattern with no team, naming the pattern); `refused` with the 0002 text that names the allowed roles; `not_found`. Sends `agent.changed` and `hierarchy.changed`.

### `template.create_team` (FR-004, FR-005, SC-005)

| Field | Type | Rule |
|---|---|---|
| `template_id` | string | a team template |
| `team_name` | string | the 0002 team name rules; free |
| `parent_agent_id` | string | the parent of the lead |
| `project_id` | string? | as above |

Result: `{team, agents: [agent]}`, the lead first. The team, its lead under the parent, and each member under the lead are created in one action, or none is (all-or-nothing); the position sets each role (A4). Errors: `invalid` with every reason, `conflict` "team name \"<name>\" is already used", `refused`, `not_found`. A failed undo names the first reason and each agent left behind. Sends one `agent.changed` for each created agent and one `hierarchy.changed`, and nothing for a failed creation.

### `template.export` (FR-007)

| Field | Type | Rule |
|---|---|---|
| `ids` | list of string? | absent exports every template |

Result: `{text}`, the file text of data-model.md with every environment value `null`. The desktop app says before writing that flags and startup prompts are written as they are.

### `template.import.preview` (FR-008, FR-009)

| Field | Type | Rule |
|---|---|---|
| `text` | string | the file text; at most 256 KiB, else `invalid` "the file is larger than 256 KiB" |

Result: `{templates: [{name, save_name, kind, body}]}`: every template with the name it will be saved under (A3) and every environment value `null` (D3). Saves nothing. Errors: `invalid` with the FR-009 reason that names the template.

### `template.import.confirm` (FR-008)

Same field and refusals. Validates the text again, sets every environment value to `null`, and saves all templates in one transaction. Result: `{templates: [template]}`. Creates no agent. Sends `template.changed`. Nothing is saved when any template is refused.

## New event

### `template.changed`

`{type:"template.changed", templates: [template]}`: the full list after any save, delete, or import confirm. Clients replace their copy.

## Codes

| Code | Used for |
|---|---|
| `invalid` | rule failures, with each reason; an oversized or unreadable file; an unknown field |
| `conflict` | a template name or a team name that is taken |
| `not_found` | an unknown template, agent, or project |
| `refused` | the 0002 tree rules, with their text |
| `internal` | a storage failure |

## FR map

| Requirement | Request |
|---|---|
| FR-001 | `template.save`, `template.from_agent` |
| FR-002 | `template.create_agent` |
| FR-003, FR-010 | `template.save` |
| FR-004, FR-005 | `template.create_team`, `template.create_agent` |
| FR-006 | `template.save`, `template.delete` |
| FR-007 | `template.export` |
| FR-008 | `template.import.preview`, `template.import.confirm` |
| FR-009 | `template.import.preview`, `template.import.confirm` |
| FR-011 | `template.import.*` (values dropped), additions shapes in data-model.md |
