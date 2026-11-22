# Data model: own worktree per agent (0010)

Date: 2026-11-22. Plan: `PLAN.md` decisions D1 to D7.

## Agent worktree (agent module)

Four nullable columns on `agent_agents`, migration `0007_agent_worktree.sql` of the agent ledger. The four are all set or all NULL; NULL means the agent has no own worktree (PB-1).

| Column | Type | Rule |
|---|---|---|
| `worktree_repository` | TEXT | The main worktree of the repository that holds the chosen folder: the first record of `git worktree list --porcelain`, so a folder inside a linked worktree names the main one. For a bare repository, the top of the chosen folder (`git rev-parse --show-toplevel`). |
| `worktree_base` | TEXT | The base: a local branch name, or the full commit id when the chosen folder was detached (A2). |
| `worktree_branch` | TEXT | The agent's branch, `office-harness/<slug>` or `office-harness/<slug>-<n>` (D3). |
| `worktree_folder` | TEXT | The worktree folder, under the office's worktrees folder (D2). |

- The agent's `working_folder` is the worktree folder plus the chosen folder's subfolder in the repository (FR-002), so the start check and the CLI need no other change.
- A base of 40 or 64 hex digits is a commit; any other base is a branch (`refs/heads/<base>`).
- The columns survive a restart (PB-5, SC-005). A delete removes the row; a kept worktree is no longer stored (A6).

Domain type: `AgentWorktree { repository, base, branch, folder }`, the field `Agent.worktree: Option<AgentWorktree>`.

## Worktrees folder (server configuration)

`<data folder>/worktrees/<repository folder name>-<8 hex of the repository path hash>/<first 8 characters of the agent id>`.

- The worktrees folder must be outside the chosen repository; a data folder inside it refuses the create (A1).
- Every git command sets `core.hooksPath` to the null device (`/dev/null`, `NUL` on Windows), which no one can fill, and switches off the hooks named in git's configuration (`hook.<event>.enabled=false`) for every event the office's commands can fire: `post-checkout`, `reference-transaction`, `post-index-change`, and `pre-auto-gc`. So no repository hook runs (D1).
- Every remove checks that the stored folder is exactly `<repository>-<hash>/<id>` below the worktrees folder before it touches the disk; the undo of a create also checks that after links resolve.

## Branch name (D3)

The slug is the agent name lower-cased, every character outside `[a-z0-9-]` replaced by `-`, runs of `-` collapsed, leading and trailing `-` trimmed, and cut to 40 characters. An empty slug is `agent`. The branch is `office-harness/<slug>`; when that branch exists, the lowest free `office-harness/<slug>-<n>` with `n` from 2. The full name must pass `git check-ref-format --branch`.

## Git commands

All run as `git -c core.hooksPath=<null device> -c hook.<event>.enabled=false ... -c core.fsmonitor=false -C <folder> ...`, with no shell, a null stdin, and the login environment without any `GIT_*`, `GIT_ASKPASS`, or `SSH_ASKPASS` variable, plus `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`, and `GCM_INTERACTIVE=never`. Read commands add `GIT_OPTIONAL_LOCKS=0`.

| Purpose | Command | Timeout |
|---|---|---|
| Version (2.22 or newer) | `git version` | 10 s |
| Repository and subfolder | `rev-parse --show-toplevel --show-prefix` | 10 s |
| Main worktree | `worktree list --porcelain` | 10 s |
| Base | `symbolic-ref --quiet HEAD`, else `rev-parse --verify HEAD^{commit}` | 10 s |
| Local branches | `for-each-ref --format=%(refname) refs/heads/` | 10 s |
| Branch name check | `check-ref-format --branch <name>` | 10 s |
| Create the branch | `branch --no-track -- <branch> <refs/heads/base or commit>` | 10 s |
| Create the worktree | `worktree add -- <folder> <branch>` | 120 s |
| Change count | `status --porcelain=v1 --untracked-files=all --ignore-submodules=none -z` | 10 s |
| Own commits | `rev-list --count <base ref>..refs/heads/<branch> --` | 10 s |
| Base exists | `rev-parse --verify --quiet <base ref>^{commit}` | 10 s |
| Remove a clean worktree | `-c status.showUntrackedFiles=normal worktree remove -- <folder>` | 120 s |
| Remove a changed or missing worktree | `-c status.showUntrackedFiles=normal worktree remove --force -- <folder>` | 120 s |
| Delete the branch | `branch -D -- <branch>` | 10 s |

A `worktree prune` is never run. The office writes no git configuration.

## Status of one worktree (D6)

| Field | Meaning |
|---|---|
| `changed_files` | The line count of the status command above: changed, added, deleted, and every untracked file; ignored files are not counted (A9). |
| `status_hash` | A hash of the status output plus the size and change time of each changed file, hex. Valid within one server run. |
| `own_commits` | Commits on the agent's branch that the base lacks; absent when the base is gone or git cannot count them, which keeps the branch. |
| `base_exists` | The base branch or commit still exists. |
| `folder_exists` | The worktree folder exists. |
| `reachable` | The repository answers and the status could be read. |
