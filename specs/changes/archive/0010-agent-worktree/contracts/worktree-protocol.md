# Contract: worktree protocol (0010)

Date: 2026-11-22. Extends the 0001 wire protocol and the 0002 contract. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

The agent connection never answers the two new requests; they are outside its allowlist. Its `agent.create` keeps the fields `name`, `command`, `working_folder`, `project_id`, and `role`, so `own_worktree`, `base`, and `branch_name` are refused (A7).

## New requests (user client only)

### `agent.worktree_info` (FR-001)

| Field | Type | Rule |
|---|---|---|
| `folder` | string? | the chosen working folder, absolute |
| `project_id` | string? | used when `folder` is absent or empty: the project's folder |
| `name` | string | the agent name the form holds, for the branch name |

Result, when the folder is not inside a git repository, or no git 2.22 or newer is found:

```
{ in_repository: false }
```

Otherwise:

```
{ in_repository: true,
  base: string,            // the checked-out branch, or the full commit id when detached
  base_is_branch: bool,
  branches: [string],      // the local branches, the choices for base
  branch_name: string,     // the branch a create would make now (D3)
  subfolder: string }      // the chosen folder inside the repository, "" at the top
```

Errors: `invalid` "a folder or a project is needed", or the folder is not absolute; `not_found` for an unknown project. The answer does not depend on an agent id, so the form shows it before the agent exists.

### `agent.worktree_status` (FR-005, FR-006)

| Field | Type | Rule |
|---|---|---|
| `agent_ids` | [string] | the agents the delete question covers |

Result: one entry per listed agent that has an own worktree, in the order given:

```
{ agent_id, name, folder, branch, base,
  folder_exists: bool, reachable: bool,
  changed_files: number, status_hash: string,
  own_commits: number | null, base_exists: bool }   // null: git could not count them; the branch is kept
```

Errors: `invalid` for a missing or malformed id; `not_found` for an unknown agent.

## Changed requests

### `agent.create` (FR-002, FR-004)

New fields, the user client only:

| Field | Type | Rule |
|---|---|---|
| `own_worktree` | bool? | default false |
| `base` | string? | one of `branches` or the detached commit from `agent.worktree_info`; default the folder's base |
| `branch_name` | string? | the name `agent.worktree_info` showed; default the free name now |

`base` and `branch_name` without `own_worktree: true` are `invalid`. The same fields are taken inside a `role` create.

Errors added:

| Code | Message |
|---|---|
| `invalid` | `own worktree needs a git repository` |
| `invalid` | `base is not a branch of the repository: <base>` |
| `invalid` | `branch_name must be office-harness/ and a name of a-z, 0-9 and -` |
| `invalid` | `the office data folder is inside this repository` |
| `invalid` | `the folder <subfolder> does not exist in <base>` |
| `conflict` | `the branch <name> exists; the next free name is <free name>` |
| `internal` | `the worktree could not be created: <git reason>` |

On every failure no agent, branch, folder, or worktree record is left. The git reason has the `user:password@` part of every URL removed and is cut to 500 characters.

### `agent.delete` (FR-005, FR-006, FR-007)

New fields:

| Field | Type | Rule |
|---|---|---|
| `worktrees` | `"keep"` \| `"remove"`? | default `keep` |
| `confirmed_status` | {agent_id: status_hash}? | with `remove`: the `status_hash` the question showed for each worktree |

The result is now an object for every delete:

```
{ deleted_branches: [string], kept_branches: [string], kept_folders: [string] }
```

With `remove`, for each agent with a worktree, deepest first in a subtree delete:

1. Every listed worktree's status is read again and compared with `confirmed_status` before any agent is stopped. A missing or different hash answers `conflict` "the worktree of <name> changed; confirm again" and nothing changes.
2. Every agent whose worktree is removed is stopped, and its status is read and compared again, before the first agent is deleted. A difference answers `conflict` "the worktree of <name> changed while it stopped; it is stopped now, confirm again"; no agent is deleted, and the stopped agents stay stopped.
3. A worktree confirmed with 0 changes is removed without force, so git refuses one that changed since. Otherwise, or when the folder is missing, it is removed with force; only its own record is cleared.
4. The branch is deleted only when it has no own commits and its base exists; otherwise it is kept and named. A branch that git refuses to delete (for example one checked out elsewhere) is kept and named.
5. A repository that cannot be reached keeps the folder and the branch and names them; the agent is still deleted.

With `keep`, the folder and the branch stay and are named in the result (A6).

### Agent record

`agent.list`, `agent.create`, and `agent.changed` carry `worktree: null | {repository, base, branch, folder}` (FR-003).

### Start (FR-008)

`agent.start`, `agent.new_task`, and `session.continue` of an agent with a worktree answer `invalid` "the worktree folder is missing or broken: <folder>" when the folder is gone or its `.git` file does not name an existing git folder. The CLI does not start.
