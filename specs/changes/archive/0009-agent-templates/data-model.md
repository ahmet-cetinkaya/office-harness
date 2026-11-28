# Data model: agent and team templates (0009)

Date: 2026-11-22. Plan: `PLAN.md` decisions D1 to D4.

## Template (template module)

Table `template_templates`, migration `0001_templates.sql` of the template ledger `template_schema_version`. The module has its own ledger, so an older program that does not know it opens the database and ignores the `template_*` tables.

| Column | Type | Rule |
|---|---|---|
| `id` | TEXT PRIMARY KEY | UUID text |
| `name` | TEXT NOT NULL | the template name as the user typed it (trimmed) |
| `name_key` | TEXT NOT NULL UNIQUE | the lower-cased name; makes the name unique case-insensitively (Key entities) |
| `kind` | TEXT NOT NULL | `agent` or `team` |
| `body` | TEXT NOT NULL | the JSON body below |
| `updated_at` | INTEGER NOT NULL | Unix milliseconds of the last save |

- No foreign key. A template is not linked to the agents or teams created from it (FR-006), and a team template holds copies of its entries (A5).
- Creating, replacing, and deleting a row writes nothing to the action record (A10).
- The migration is forward-only.

## Body (D2)

The `body` column and the `body` field of the wire hold the same JSON. `body_version` is 1 and is on the top-level body only. A field that this build does not know is refused and named, so a build never drops a field on save or import (0010 adds `own_worktree`; a build without 0010 refuses a body that has it).

### Agent body (FR-001)

```json
{
  "body_version": 1,
  "name_pattern": "reviewer-{n}",
  "command": "claude --model opus",
  "working_folder": "/home/me/code/api",
  "additions": {
    "flags": ["--verbose"],
    "env": [{"name": "API_KEY", "value": null}, {"name": "MODE", "value": "fast"}],
    "startup_prompt": "Review the open change."
  },
  "role": "member",
  "own_worktree": false
}
```

| Field | Type | Rule |
|---|---|---|
| `name_pattern` | string | required; not empty; at most 64 characters when `{team}` is empty and `{n}` is 1 (the creation checks the real name against the 64-character rule of 0001) |
| `command` | string | required; not blank (FR-001, FR-009) |
| `working_folder` | string? | optional; absent or null means the project's folder is used at creation |
| `additions.flags` | list of string | optional, default empty |
| `additions.env` | list of `{name, value}` | optional, default empty; `value` is a string or null; null is "no value" (D3) |
| `additions.startup_prompt` | string? | optional |
| `role` | `"ceo"`, `"lead"`, `"member"`? | optional; applies only to a direct creation (A4) |
| `own_worktree` | boolean? | optional, default `false`; a file without it imports as `false` (0010 FR-009). When `true`, a creation makes the agent's own git branch and worktree from the base of the chosen folder. The base and the branch name are never stored: the creation reads the base from the folder (0010 A2) and takes the next free branch name |

### Team body (FR-003)

```json
{
  "body_version": 1,
  "lead": { "name_pattern": "{team}-lead", "command": "claude", "additions": {} },
  "members": [
    { "entry": { "name_pattern": "{team}-member-{n}", "command": "claude", "additions": {} }, "count": 6 }
  ]
}
```

`lead` and each `entry` are agent bodies without `body_version`, copied from agent templates (A5). The `role` of an entry is kept as saved and ignored at team creation (A4). The `own_worktree` of an entry is used at team creation: each agent made from an entry with `true` gets its own branch and worktree (0010 FR-009).

### Save rules (FR-001, FR-003, FR-010)

| Rule | Refusal text |
|---|---|
| The template name is not blank, at most 100 characters, with no control character. | `the template name is empty`, `the template name is longer than 100 characters`, `the template name has a control character` |
| The name is unique case-insensitively. | `conflict`: `template name "<name>" is already used` |
| Each CLI command is not blank. | `<where>: the CLI command is empty` |
| Each name pattern is not empty and at most 64 characters when expanded. | `<where>: the name pattern is empty`, `<where>: the name pattern "<pattern>" makes a name longer than 64 characters` |
| Each member count is at least 1. | `members[<i>]: the count is below 1` |
| A team has at least one member entry. | `a team template needs at least one member entry` |
| 1 plus the sum of the counts is at most 50. | `the team would create <n> agents; the limit is 50` |

`<where>` is `body` for an agent body, `lead` or `members[<i>].entry` in a team body.

## Name patterns (D4, A2)

| Part | Meaning |
|---|---|
| `{team}` | the team name of the creation; for a direct creation, the team of the place picked; with no team the creation is refused and names the pattern |
| `{n}` | the lowest positive integer that makes the name unique, case-insensitively, among the office's agents and the agents planned in the same creation |
| no `{n}` | the name is used as it is; when it is taken, `-{n}` is added |

Defaults: an agent copied from agent `x` gets `x-{n}`; a lead entry gets `{team}-lead`; a member entry gets `{team}-member-{n}`.

## Template file (D2, D7)

One JSON document, at most 256 KiB:

```json
{
  "format": "office-harness-templates",
  "version": 1,
  "templates": [
    { "name": "Reviewer", "kind": "agent", "body": { "body_version": 1, "name_pattern": "reviewer-{n}", "command": "claude", "additions": {"env": [{"name": "API_KEY", "value": null}]} } }
  ]
}
```

- Export writes every environment `value` as `null` (FR-007, A6).
- Import reads the file as untrusted text. The preview and the confirm set every environment `value` to `null`, whatever the file holds (D3), so a crafted value is never saved.
- A template of the file is saved under its own name, or under the name with ` (2)`, ` (3)`, and so on, the lowest free one, counted against the office's templates and the earlier templates of the same file (A3). Import never replaces a template.
- A refusal of the whole file names the template (its name, or `template <i>` when it has none) and the reason, and saves nothing (FR-009).

| FR-009 case | Reason text |
|---|---|
| unknown format version | `the file version <v> is not supported; this build reads version 1` |
| not a template file | `the file is not a template file: <detail>` |
| larger than 256 KiB | `the file is larger than 256 KiB` |
| team over the limit | `template "<name>": the team would create <n> agents; the limit is 50` |
| a save rule broken | `template "<name>": <save rule text>` |
| unknown field | `template "<name>": <path> is not a field of <what>` |

## "No value" (D3, FR-011)

`EnvEntry.value` is optional in every path that carries an environment value as text.

| Path | `None` ("no value") | `Some("")` |
|---|---|---|
| domain `EnvEntry` | `None` | `Some("")` |
| stored additions JSON (`agent_office_additions`, session snapshot) | `["NAME", null]` | `["NAME", ""]` |
| wire (`additions.env`) | `{"name":"NAME","value":null}` | `{"name":"NAME","value":""}` |
| hierarchy port seed | `("NAME", None)` | `("NAME", Some(""))` |
| desktop editor text | `NAME` | `NAME=` |

- The launcher skips `None`, so the login environment's value stays (0001/FR-020). `Some("")` is set to empty.
- Older rows hold only strings and load unchanged; no migration is needed.
- A program from before this change cannot read a stored `null`; rollback uses the `office.db` copy taken before the upgrade (PLAN, Rollback).

## FR map

| Requirement | Where it is defined |
|---|---|
| FR-001, FR-003 | Body, Save rules |
| FR-002, FR-004, FR-005 | Name patterns here; the requests `template.create_agent` and `template.create_team` in `contracts/template-protocol.md` |
| FR-006 | no link column; Template table notes |
| FR-007, FR-008, FR-009 | Template file |
| FR-010 | Save rules (the 50-agent row) |
| FR-011 | "No value" |
