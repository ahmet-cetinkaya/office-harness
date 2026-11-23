# Contract: temp protocol (0020)

Date: 2026-11-23. Extends the 0001 client protocol (`specs/changes/archive/0001-office-server-and-terminal/contracts/client-protocol.md`), the 0002 hierarchy contract (`specs/changes/archive/0002-agent-hierarchy/contracts/hierarchy-protocol.md`), the 0010 worktree contract (`specs/changes/0010-agent-worktree/contracts/worktree-protocol.md`), and the 0015 report contract (`specs/changes/archive/0015-upward-reports/contracts/report-protocol.md`). Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`. Times are Unix milliseconds, as `created_at` in 0001.

## Changed shapes

- **Agent object.** Every agent object (`agent.list` rows, the `agent.create` result, `agent.changed`) carries `temp`: `null` for an agent that is not a temp, else `{objective: string, creator: "user" | "<agent id>"}`.
- **Actions.** Action entries may carry the action `"end"`: the actor is `user`, an agent id, or `office`; `detail` is the end reason (see End reasons). A dismiss adds the action `"dismiss"` to the ended temp's record: the actor is `user`; `detail` is the entry id.
- **Settings.** `settings.get` and `settings.set` know two keys:
  - `temp_cap`: the text of a whole number from 1 to 100, default `8`. Any other value is refused with `invalid` "temp_cap is a whole number from 1 to 100"; the stored value stays.
  - `temp_idle_minutes`: the text of a whole number from 5 to 1440, default `30`. Any other value is refused with `invalid` "temp_idle_minutes is a whole number of minutes from 5 to 1440"; the stored value stays.
- **Notification.** The kind `temp_ended` with `detail` = the end reason. Sent only for an idle-limit end whose end notice reaches no queue (spec FR-007).

## `agent.create`, extended

User client fields, beside the 0001 and 0010 fields (`own_worktree`, `base`, `branch_name`):

| Field | Type | Rule |
|---|---|---|
| `temp` | boolean? | `true` creates a temp; absent or `false` creates an agent that is not a temp |
| `objective` | string? | required with `temp: true`, refused without it; not blank, at most 8,192 characters, no C0, DEL, or C1 character |

A created temp is started at once; its objective waits at the head of its queue (0012) and is typed when the CLI waits for input. The answer is the agent object, as before.

Agent connection fields: `name`, `command`, `working_folder?`, `project_id?`, `role?` (0002), and:

| Field | Type | Rule |
|---|---|---|
| `temp` | boolean? | as above; a caller that is a temp always creates a temp |
| `objective` | string? | as above; required when the created agent is a temp |
| `own_worktree` | boolean? | only for a temp; default `true`. With `true`, the office gives the temp its own branch and worktree from the working folder's base when the folder is inside a git repository, and none otherwise. For an agent that is not a temp the field is refused with `invalid` "own_worktree is only for a temp" |

Errors, beside the 0001, 0002, 0007, and 0010 ones:

- `invalid` "a temp needs an objective"; `invalid` "objective is only taken with temp"; `invalid` with the directive text rule for a bad objective.
- `refused` "the temp cap is <n>; end a temp or raise the cap". This refusal is recorded and counts toward no rate limit and no automatic hold.

## New requests (user client only)

The agent connection refuses every `temp.*` request as not allowed (0002 contract). A field that a request does not name is refused with `invalid` "<field> is not a field of <request>".

### `temp.list`

No fields. Result:

```json
{
  "temps": [
    {
      "agent": { "...": "the agent object" },
      "creator_name": "the person | <agent name> | null when that agent is gone",
      "parent_name": "<name> | null",
      "run_seconds": 0,
      "quiet_seconds": 0
    }
  ],
  "finished": [
    {
      "id": "<entry id>",
      "agent_id": "<agent id>",
      "name": "<name>",
      "objective": "<objective>",
      "creator": "user | <agent id>",
      "creator_name": "the person | <agent name>",
      "parent_name": "<name> | null",
      "started_at": 0,
      "ended_at": 0,
      "reason": "<end reason>",
      "report": { "kind": "done", "text": "<visible text>" },
      "kept_folder": "<folder> | null",
      "kept_branch": "<branch> | null",
      "removal_failure": "<text> | null"
    }
  ]
}
```

`temps` holds every temp in the office; `quiet_seconds` is `null` for a temp that is not running or asleep. `finished` is newest first. `report` is `null` when the temp sent no `done` report. `kept_folder` and `kept_branch` are both `null` when nothing stayed; the desktop app then says that files git ignores were removed with the worktree.

### `temp.end`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | a temp |

Ends the temp with the reason "ended by the person". Result `{}` once the temp is removed and its finished entry stored.

Errors: `not_found` for an unknown agent; `invalid` "the agent is not a temp"; `conflict` "the temp already ended" when it is ending or gone.

### `temp.dismiss`

| Field | Type | Rule |
|---|---|---|
| `entry_id` | string | a finished entry |

Removes the entry only; no folder, branch, or file changes. Records `dismiss` on the ended temp's record. Result `{}`. Error: `not_found` for an unknown entry.

## Request made by the office only

### `temp.end_check`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | a temp |
| `seen` | string | the status the watcher read |

Runs under the status order; answers `{ending: true}` and claims the temp for its end when the stored status is still `seen`, else `{ending: false}`. A user client gets `invalid` "temp.end_check is made by the office only"; the agent-call allowlist does not name it.

## Refusal while a temp ends

`agent.start`, `agent.new_task`, `session.continue`, `agent.resume`, `agent.wake`, and `wake.start` for a temp that is ending are refused with `conflict` "the temp is ending".

## Event

`temp.changed` with no fields, after each create of a temp, each end, and each dismiss. Clients re-read `temp.list`.

## End reasons

The `reason` text of a finished entry and the `detail` of an `end` action:

| Reason | When |
|---|---|
| `done` | the office accepted a `done` report from the temp |
| `no output for <n> minutes` | the idle limit |
| `exited with code <n>` | the CLI exited by itself |
| `failed: <reason>` | the CLI failed or could not start |
| `stopped by the person`, `stopped by <agent name>`, `stopped by the office` | a stop, alone or with an ancestor |
| `ended by the person` | `temp.end` |
| `deleted by the person` | `agent.delete` of the temp |
| `its parent <name> was deleted` | `agent.delete` of an ancestor |
| `its parent <name> ended` | its parent temp ended |

## End notice

One line typed into a running parent's queue as a report of the ended temp (0015/FR-004, FR-010), for the reasons `no output for <n> minutes`, `exited with code <n>`, and `failed: <reason>`:

```
[temp ended: <name in visible form> (<agent id>): <reason>]
```
