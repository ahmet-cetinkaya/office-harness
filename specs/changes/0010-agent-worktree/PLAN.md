---
status: in_progress
spec: specs/changes/0010-agent-worktree/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #66, main b6d2cb9). Builds after 0009 (god's order 0008, 0011, 0009, 0010); T6 needs 0009 merged. r2 (2026-11-19) applies Efe's plan review r1 (office-harness-plan-0010.md: 4 HIGH, 8 MEDIUM, 3 LOW) and god's decision of one shared create undo mode with 0009. Needs approval through Efe's plan review. -->

# Plan: Opt-in own worktree per agent (0010)

## Summary

When the owner creates an agent in a git repository, he can turn on "own worktree". The office then makes a new branch and a worktree for that agent in a folder that the office owns, and the agent works there. Deleting such an agent asks whether to keep or remove the worktree, with keep as the default. A worktree with uncommitted changes is removed only after a second confirmation, and a branch with its own commits is never deleted. Templates carry the choice.

**What you get:**
- A small git runner in the agent module that calls the system `git` program with a timeout and a clean environment, behind an application port.
- Four nullable worktree columns on the agent, shown in the details and kept across restarts.
- `agent.worktree_info`, which tells the form whether a folder is in a repository and shows the base and the branch name before create.
- `agent.worktree_status`, which feeds the delete question.
- `agent.create` gains `own_worktree`, an optional `base`, and the `branch_name` the form showed. `agent.delete` gains the keep or remove choice and a confirmed status hash per worktree.
- In the desktop app: the choice in both create forms, the details rows, the delete question with the second confirmation, and the result message.
- Templates (0009) carry `own_worktree`, and creation from a template makes the worktrees all-or-nothing.

**Why this approach:**
- The system `git` program is what the user's repository already uses, and it supports worktrees fully. The office already runs helper programs with a timeout (`M/agent/infrastructure/src/services/login_shell.rs:57`), so the runner copies that pattern.
- The spec says the agent's working folder is the worktree, so the existing start check (`session_launcher.rs:376-392`) already refuses a missing folder (FR-008); only the message changes.
- Agents call `agent.create` through an allowlist of fields (`H/server/agent_call.rs:687`). Leaving `own_worktree` out of that list keeps agent-created agents without worktrees (A7) with no new code.

**What it will not do:** merge, rebase, push, commit, or clean up; separate history; turning the choice on later; worktrees for agent-made agents; managing kept worktrees; copying untracked files; version control systems other than git (spec Out of scope).

**Shape of the work:** 7 tasks in 2 phase PRs into one umbrella branch `feat/0010-agent-worktree`, one local commit per task. **Effort:** M to L.

**Main risks:**
- **Losing work on delete.** D6 stops the agent, then counts again on the server, compares with the confirmed status, and only then removes. It counts every untracked file, whatever the user's git settings. A worktree confirmed as clean is removed without `--force`, so git itself refuses one that became dirty. Integration tests use real temporary repositories.
- **Leftovers after a failed create.** D5 creates the branch and the worktree in two steps and undoes both on any later failure or timeout. Every create undo (insert failure, role placement failure, template batch undo) discards the new worktree and branch without asking (the create undo mode shared with 0009).
- **Git versions and platforms.** The runner uses only porcelain commands available in git 2.22 (`branch --no-track`, `worktree add`, `worktree remove`, `status --porcelain=v1`, `rev-list --count`, `rev-parse --show-prefix`, `check-ref-format --branch`). It runs git with a clean environment and no prompt. CI runs the tests on three OSes, where git is installed.
- **No git on the machine.** `agent.worktree_info` then answers "not in a repository", and the choice is not offered.

**Decisions made for you:** the git CLI, the runner and its environment, hooks off, the columns, the folder layout, the branch name rule, the requests, the delete order and protocol, the create undo mode, and the template hook.

**Blockers:** none. T6 waits for 0009 to merge. The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0010. In a git repository the owner can give a new agent its own branch and worktree. The agent works there, and the user's folder stays unchanged. The details show branch, base, and folder. Delete never removes uncommitted work without a second confirmation and never deletes a branch with its own commits. A missing worktree refuses the start. Templates carry the choice.

## Context

- **Repository:** `main` at 037b10f. Specs 0008 to 0011 are merged. The 0011 plan takes agent migration 0006, so this change takes the next free number at build time (0007 if 0011 shipped first). The 0009 plan is on branch `docs/0009-plan` (r2 at 4d5e693), not on main yet; T6 is gated on 0009 being merged.
- **Existing seams** (read at 037b10f on 2026-11-19):
  - **Git:** none in the code. There is no git2 or gix crate. The helper-program pattern is `run_login_shell` (`M/agent/infrastructure/src/services/login_shell.rs:57`), which uses its own process group, a reader thread with a timeout, and a `killpg` on timeout. It is unix-only and discards stderr, so the git runner is a new cross-platform function that keeps stderr for FR-004's reason. The login environment is resolved by `M/agent/infrastructure/src/services/login_environment.rs`. The terminal module ends a Windows process tree in `M/terminal/infrastructure/src/services/process_tree.rs`.
  - **Create validation:** `Agent::new` (`M/agent/domain/src/entities/agent.rs:41-51`) requires an absolute folder or a project.
  - **Start check:** `existing_working_folder` (`M/agent/application/src/common/session_launcher.rs:376-392`), called from `prepare()` (`:124`). It answers `invalid` "the <field> does not exist" and changes nothing.
  - **Delete:**
    - `agent.delete` (`M/agent/presentation/src/handlers/agent_handlers.rs:182,286-293`) goes through the subtree bridge when attached (`:398-407`), reading `include_descendants` and `descendant_count` from the request fields.
    - The subtree application is `M/hierarchy/application/src/commands/subtree/subtree_handler.rs:86-151`, deepest first, with a count check that answers `conflict`. It deletes one agent at a time, and the agents already deleted stay deleted (`:82-98`). The ports are `SubtreeAgentEnder::stop` and `SubtreeAgentRemover::delete(agent_id)` (`M/hierarchy/application/src/common/ports/subtree_ports.rs:13-23`). The handler is reached from `M/hierarchy/presentation/src/handlers/hierarchy_handlers.rs`, wired in `H/bootstrap/hierarchy_services.rs`, and the agent side maps the request in `M/agent/presentation/src/handlers/subtree_requests.rs` and `H/bridges/subtree_requests.rs`.
    - `DeleteAgentHandler` (`M/agent/application/src/commands/delete_agent/delete_agent_handler.rs:43-50`) stops the session, deletes the row, and records the delete.
  - **Data folder:** `Config` (`H/config/server_config.rs:22-88`), with `output_folder_path` (`:74`) as the pattern for a new data subfolder, consumed at bootstrap (`H/bootstrap/terminal_services.rs:66`).
  - **Agent row:** `agent_agents` (`0001_machines_projects_agents.sql:16`), the repository (`agent_repository.rs:13`), and `agent_json` (`M/agent/presentation/src/mappings/entities.rs:9-24`).
  - **Role creates:** `RoleSeed` (`M/agent/presentation/src/handlers/role_requests.rs:41-48`) to `RoleRequestsBridge` (`H/bridges/role_requests.rs:39-129`) to `CreateRoleAgentHandler`, with the hierarchy seed `AgentSeed` (`M/hierarchy/application/src/common/ports/agent_creator_ports.rs`). A role create whose placement fails calls `creator.remove` (`create_role_agent_handler.rs:106`), which is `AgentCreatorBridge::remove` and then `DeleteAgentHandler::handle` (`H/bridges/role_requests.rs:192-196`). The agent allowlist is `CREATE_FIELDS` (`H/server/agent_call.rs:687`), and `refuse_extra_fields` refuses any other field (`:444-452`).
  - **Desktop:**
    - Create forms: `D/modules/agent/components/agent-form-dialog.svelte:66-70` and `D/modules/hierarchy/components/hierarchy-agent-dialog.svelte:43-47`.
    - Details `<dl>`: `D/modules/agent/components/agent-details.svelte:102-122`.
    - Confirm dialogs: `D/modules/agent/components/confirm-dialog.svelte` (Cancel has the first focus) and `D/modules/hierarchy/components/hierarchy-confirm-dialog.svelte` (radio choices).
    - Model: `D/modules/agent/models/agent.ts:20-31,69-74`.
    - Wiring: `D/modules/panel/views/panel-view.svelte`, `D/modules/agent/state/office-controller.ts`, `D/modules/hierarchy/state/hierarchy-controller.ts`.
  - **Templates:** the 0009 plan (branch `docs/0009-plan`, `specs/changes/0009-agent-templates/PLAN.md`, D2 and D5) refuses unknown body fields, creates batches in `TemplateCreateBridge`, and undoes them with `AgentCreator::remove` in the shared create undo mode.
- **Floor rules:** one build or test at a time (ask god; `CARGO_BUILD_JOBS=4`, `--test-threads=4`, the pinned toolchain, per-crate local runs). Efe reviews at the exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Git access:** the system `git` program through `GitRunner` in `M/agent/infrastructure/src/services/git_runner.rs`.
  - It runs with no shell and arguments as a list. Path arguments always follow `--`.
  - Environment: the resolved login environment (`login_environment.rs`), minus every `GIT_*` variable, `GIT_ASKPASS` and `SSH_ASKPASS`, plus `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0` and `GCM_INTERACTIVE=never`. Read commands add `GIT_OPTIONAL_LOCKS=0`. So nothing in the server's environment can override `-C <dir>`.
  - stdin is null. On Windows the child gets `CREATE_NO_WINDOW`.
  - Timeouts: 10 s for read commands, 120 s for `worktree add`. On a timeout the runner ends the whole process group (`killpg`, on its own group as `login_shell.rs` does) or, on Windows, the process tree, with the method of the terminal module's `process_tree.rs`. That code is repeated in the agent module, not imported, to keep the module boundary.
  - stdout is bounded to 1 MiB; more is a failure. stderr is trimmed to 500 characters for messages, after removing the `user:pass@` part of every URL in it.
  - Hooks are off: every command runs with `-c core.hooksPath=<data folder>/worktrees/.no-hooks`, an empty folder the office owns. So `post-checkout` and other repository hooks do not run.
  - Configuration: the office writes no git configuration. `branch --no-track` keeps `branch.autoSetupMerge` from writing `branch.<name>.*`. A T2 test proves `.git/config` is byte-identical after a create and a remove.
  - Network: the office runs no network command itself. Checkout filters set in the repository's configuration (for example the LFS smudge filter) still run during `worktree add`, as they do in the user's own checkout, and can fetch. This is accepted and written in the README.
  - The minimum git version is 2.22. With an older git, or no git, `agent.worktree_info` answers `in_repository: false`.
  - The application port is `WorktreeManager`, with `info`, `create`, `status`, and `remove`. Domain types are `WorktreeInfo` and `AgentWorktree { repository, base, branch, folder }`.
  - Rejected: `git2` (native libgit2 and its build cost on three OSes) and `gix` (worktree add and remove are not complete).
- **D2 Folder layout** (FR-002, A1): `<data folder>/worktrees/<repo folder name>-<8 hex of the repository path hash>/<agent short id>`. `Config::worktrees_folder_path()` follows the `output_folder_path` pattern.
  - The worktrees folder must be outside the chosen repository. `--data-folder` can point inside one, so create refuses a worktrees folder that is inside the repository top, with "the office data folder is inside this repository".
  - The subfolder (the subfolder edge case) comes from `git rev-parse --show-prefix` in the chosen folder, not from comparing paths, so symbolic links such as `/tmp` on macOS do not break it. The agent's working folder is the worktree plus that prefix.
- **D3 Base and branch name** (A2, A3, FR-001, edge cases):
  - The base is the branch checked out in the chosen folder, or the commit when it is detached. The form can pick another local branch.
  - `base` from a client must be one of the branches that `worktree_info` returned, or the detached commit it returned; anything else is `invalid`. The runner passes `refs/heads/<name>` or the full commit id, never the raw text, so a value that starts with `-` cannot be read as an option. A detached base is stored as the full commit id.
  - The branch name is `office-harness/<slug>`, where the slug is the agent name lower-cased, with every character outside `[a-z0-9-]` replaced by `-`, runs of `-` collapsed, leading and trailing `-` trimmed, and the length capped at 40. An empty slug becomes `agent`. This rules out `..`, `@{`, `/.`, `.lock`, `//`, a `/` in the name, non-ASCII names, and names that differ only by case.
  - When `office-harness/<slug>` exists, the lowest free `office-harness/<slug>-<n>` (n from 2) is used.
  - The full name is then checked with `git check-ref-format --branch`; a refused name is `invalid`.
  - **Deviation from spec A3** (recorded 2026-11-19, Efe's plan review r2): A3 says the branch is `<agent name>-<short id>`. This plan uses `office-harness/<slug>` or `office-harness/<slug>-<n>` with no short id, because the form must show the branch name before the agent exists, and the agent id (and so its short id) is made only at create (Efe r1 MEDIUM-2). The name still says which agent owns the branch and stays unique. The QA report records this as a planned deviation, not a fail.
- **D4 Info request:** `agent.worktree_info {folder?, project_id?, name}`, where the folder or the project's folder is resolved on the server.
  - It returns `{in_repository: false}`, or `{in_repository: true, base, base_is_branch, branches: [local branch names], branch_name}`. `branch_name` is the D3 name that will be created; it does not depend on the agent id, so the form can show it before the agent exists.
  - With no git program, or git older than 2.22, it answers `in_repository: false`.
- **D5 Create** (FR-002, FR-004, A5, A7):
  - `agent.create` gains `own_worktree: bool` (default false), `base?: string` and `branch_name?: string` (the name `worktree_info` returned), for the user path and the user's role creates (`RoleSeed`, `seed_of`, and the hierarchy `AgentSeed`).
  - The server checks `branch_name`: the `office-harness/` prefix, the D3 slug rule, `check-ref-format --branch`, and that the branch does not exist. On a collision it answers `conflict` with a fresh free name, which the form shows and sends again.
  - `CREATE_FIELDS` in the agent allowlist stays unchanged, so an agent's `own_worktree`, `base` and `branch_name` are refused (A7).
  - The order:
    1. Validate the agent, the base (D3) and the branch name.
    2. `git branch --no-track -- <branch> <base ref>`.
    3. `git worktree add -- <folder> <branch>` (120 s timeout).
    4. Check that the agent's working folder (worktree plus prefix) exists in the worktree. A subfolder that is untracked or missing in the base does not exist there, so the create is refused with "the folder <prefix> does not exist in <base>".
    5. Insert the agent with the four columns.
  - Undo on any failure after step 2, a timeout included: `git worktree remove --force -- <folder>` (when step 3 made a record), remove the folder if it still exists, then `git branch -D -- <branch>`. Then answer with the reason (the git stderr reason, cleaned as in D1). No agent, branch, folder, or worktree record is left (FR-004).
  - **Create undo mode (shared with 0009):** every create undo discards without asking. That covers the insert failure above, a role create whose placement fails (`creator.remove`), and the 0009 template batch undo. `AgentCreator::remove` is used only for create undo; `AgentCreatorBridge::remove` calls `DeleteAgentHandler` with the internal choice `Discard`, which stops nothing that is running (the agent never started), removes the worktree with `--force`, and deletes the branch the create made. `Discard` is not on the wire; the user delete offers only `keep` and `remove`.
- **D6 Delete** (FR-005, FR-006, FR-007, A6, A9, US2):
  - `agent.worktree_status {agent_ids}` returns, for each agent with a worktree, `{agent_id, folder, branch, base, folder_exists, reachable, changed_files, status_hash, own_commits, base_exists}`.
    - `changed_files` is the line count of `git status --porcelain=v1 --untracked-files=all --ignore-submodules=none`. Every untracked file counts, whatever `status.showUntrackedFiles` says, and ignored files do not count (A9).
    - `status_hash` is a hash of that status output (`std::hash::DefaultHasher`, hex). It is valid within one server run; after a restart the confirm fails and the dialog asks again.
    - `own_commits` is `git rev-list --count <base ref>..refs/heads/<branch>`, when the base exists.
    - `reachable` is false when the repository cannot be reached or `status` fails.
  - `agent.delete` gains `worktrees: "keep" | "remove"` (default `keep`) and, for `remove`, `confirmed_status: {agent_id: status_hash}` for every worktree.
  - Removal order per agent, for `remove`:
    1. Stop the agent (the 0001 stop).
    2. Count again and hash again.
    3. Compare with the confirmed hash. A difference answers `conflict` "the worktree of <name> changed; confirm again" and keeps everything.
    4. Remove: when the confirmed count is 0, plain `git worktree remove -- <folder>` (no `--force`), so git itself refuses a worktree that became dirty; with changes, `git worktree remove --force -- <folder>`. A missing folder uses `git worktree remove --force -- <folder>` too, which clears only that record (proven by a T2 test on git 2.22). A repository-wide `git worktree prune` is never run.
    5. `git branch -D -- <branch>` only when `own_commits` is 0 and the base exists. Otherwise the branch is kept and named. A branch checked out somewhere else fails to delete, and is then kept and named.
  - Fail closed: when the repository cannot be reached or `status` fails, the folder and the branch are kept and named, and the agent is still deleted.
  - `keep` leaves folder and branch and names them once (A6).
  - The answer lists `{deleted_branches, kept_branches, kept_folders}`.
  - Subtree delete (one question for the whole subtree):
    - `affected_for_delete` checks every agent's confirmation (stop is not needed for this first check) before the first delete. A stale hash on any member answers `conflict` and deletes nothing.
    - Each member then runs the per-agent order above. A member whose hash changed between the first check and its own stop is kept with its worktree, and the answer names it; the members already deleted stay deleted, as the 0002 subtree delete does today.
    - `SubtreeAgentRemover::delete` takes the worktree choice and returns each member's branch and folder outcome, which the answer collects.
- **D7 Start** (FR-008): when the agent has a worktree, the start also checks that `<worktree>/.git` names a `gitdir` that exists. A missing folder or a broken link (for example after the repository moved) is refused with "the worktree folder is missing or broken: <path>". Nothing else changes.
- **D8 Templates** (FR-009, MODIFIED 0009/FR-001 and FR-003):
  - The 0009 agent body gains `own_worktree: bool` (default false), so the editor and the file round-trip carry it.
  - `TemplateCreateBridge` plans a worktree for each such agent from the base of the chosen folder, and picks each branch name under the tree exclusion (D3). If the folder is not in a repository, the creation is refused with "own worktree needs a git repository". It creates the worktrees inside the batch and undoes them with the agents through `AgentCreator::remove`, in the shared create undo mode (D5).
  - The base is never stored in a template (A2).
- **D9 Desktop:**
  - Both create forms call `agent.worktree_info` when the folder or project changes.
  - When the folder is in a repository, they show an "Own worktree" checkbox (off), the base select, the branch name, and the note "Untracked files such as local environment files are not copied".
  - The details show Branch, Base, and Worktree folder rows when present.
  - Delete of agents with worktrees:
    - A dialog asks "Keep worktrees" (initial focus) or "Remove worktrees". It lists each worktree with "N changed files", "no changes", or "cannot be read, will be kept" as text, and says that ignored files are removed. It sends the `status_hash` of each worktree with remove.
    - Remove with changes opens a second confirmation that names the counts and is announced.
    - The result message lists the deleted and kept branches and the kept folders.
  - No visuals beyond these controls.
- **Constitution check:**
  - Module boundaries hold: git lives in the agent module's infrastructure; the subtree pass-through is in `H/bridges`.
  - No CLI configuration, no git configuration, and nothing in the user's own folder is written (PB-2). Only a branch and a worktree record are added to the repository's git data.
  - Keyboard access is kept, and keep has the initial focus.
  - No other project's name is in the repository.

## Scope

**In:** FR-001 to FR-009, MODIFIED 0009/FR-001 and FR-003, PB-1 to PB-5, A1 to A9, SC-001 to SC-005, the spec's edge cases, the contract and README updates, a QA report.

**Out:** everything in the spec's Out of scope.

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Server | Git runner, create, start, delete, protocol | T1 to T4 | `feat/0010-p1-server` (umbrella) | p1 → umbrella |
| 2 Desktop, templates, ship | Forms, details, delete dialog, template hook, docs, QA | T5 to T7 | `feat/0010-p2-desktop` (umbrella after p1) | p2 → umbrella, then umbrella → `main` |

## Tasks

Paths: `M/` = `src/modules/`, `D/` = `src/apps/desktop/src/`, `H/` = `src/apps/office-harnessd/src/`.

### T1: Design artifacts (no product code)

- **Files:** `specs/changes/0010-agent-worktree/{data-model.md,contracts/worktree-protocol.md}`.
- **Work:** the four columns, the folder layout, the branch name rule, the git commands used, and the D4 to D6 requests, fields, codes, and answers.
- **Depends on:** none.
- **Acceptance:** every FR names its message or column; style as `0007-hold-and-limits/contracts/hold-protocol.md`.

### T2: Git runner and worktree manager (FR-001, FR-002, FR-004, FR-007 mechanics)

- **Files:** `M/agent/domain/src/value_objects/agent_worktree.rs` (+ tests), `M/agent/application/src/common/ports/` (`WorktreeManager`), `M/agent/infrastructure/src/services/{git_runner.rs,git_worktrees.rs}` (+ tests), `M/agent/infrastructure/Cargo.toml` (dev: `tempfile` if not present).
- **Work:** D1 to D3, plus the D5 create steps and the D6 status and removal mechanics.
- **Depends on:** T1.
- **Acceptance:** tests on temporary repositories created by the test:
  - Info in a repository, in a subfolder (prefix from `--show-prefix`, also through a symbolic link), when detached (full commit id), outside a repository, and with git missing (a `PATH` without git) answers not in a repository.
  - Branch names: `..`, `@{`, `a/.b`, `x.lock`, an empty name, `日本語`, and `Bob` beside `bob` each give a valid, distinct name; an existing name gets the next `-<n>`.
  - A `base` that starts with `-` or is not in the info list is refused before git runs.
  - Create and remove leave no folder, record, or branch, and `.git/config` is byte-identical before and after.
  - A failed create leaves no branch, folder, or worktree record: an unwritable target folder, and a `worktree add` killed by a timeout.
  - A subfolder that is untracked in the base refuses the create and leaves nothing.
  - A repository with a `post-checkout` hook that writes a file: the file is not written.
  - The status, with `status.showUntrackedFiles=no` set and an untracked folder of 3 files, counts 3. Changed, added, and deleted files count; ignored files do not. The hash changes when one file is reverted and another added.
  - `own_commits` counts commits the base lacks. A missing base keeps the branch. A branch checked out elsewhere is kept.
  - A worktree confirmed as clean that became dirty is refused by plain `worktree remove`.
  - A missing folder: `worktree remove --force` clears only its record, and another worktree whose folder is missing keeps its record.
  - The runner drops `GIT_DIR` and `GIT_WORK_TREE` set in the server's environment, bounds stdout to 1 MiB, and removes `user:pass@` from stderr.
  - A timeout ends git and its children (a hook-like child that sleeps).
  - fmt, clippy, per-crate tests.

### T3: Agent columns, create, start (FR-002, FR-003, FR-004, FR-008, PB-1, PB-5, SC-004, SC-005)

- **Files:** `M/agent/domain/src/entities/agent.rs`, `M/agent/infrastructure/src/persistence/` (migration with the next free number, repository), `M/agent/application/src/commands/{create_agent,delete_agent}/` (the internal `Discard` choice), `common/session_launcher.rs`, `M/agent/presentation/src/{handlers,mappings}/`, `M/agent/presentation/src/handlers/role_requests.rs`, `M/hierarchy/application/src/common/ports/agent_creator_ports.rs` (`AgentSeed`), `H/bridges/role_requests.rs` (`AgentCreatorBridge`), `H/config/server_config.rs`, `H/bootstrap/`.
- **Work:** D2, D5, D7, and the worktree in `agent_json`.
- **Depends on:** T2.
- **Acceptance:**
  - Handler tests: without the choice nothing changes (PB-1, SC-004). With it, the working folder is the worktree subfolder. An insert failure undoes the worktree and the branch.
  - A role create with own worktree whose placement fails leaves no agent, branch, or folder (create undo mode).
  - A `branch_name` that is taken answers `conflict` with a fresh name; one without the prefix is `invalid`.
  - A data folder inside the chosen repository is refused.
  - The start of an agent whose worktree folder was deleted, or whose `.git` link points nowhere, is refused with the message.
  - A new task or a continued session of an agent keeps the same worktree folder (A4).
  - Repository: the columns survive close and reopen (SC-005).
  - Migration test: an office from before upgrades with every agent intact.
  - The agent connection's `agent.create` with `own_worktree` is refused (A7).
  - fmt, clippy, per-crate tests, boundaries.

### T4: Delete, status, protocol, integration (FR-005, FR-006, FR-007, PB-3, PB-4, SC-001 to SC-003)

- **Files:** `M/agent/application/src/commands/delete_agent/`, `M/agent/presentation/src/handlers/{agent_handlers.rs,subtree_requests.rs}`, `M/hierarchy/application/src/commands/subtree/`, `M/hierarchy/application/src/common/ports/subtree_ports.rs`, `M/hierarchy/presentation/src/handlers/hierarchy_handlers.rs`, `H/bridges/subtree_requests.rs`, `H/bootstrap/hierarchy_services.rs`, `src/apps/office-harnessd/tests/agent_worktree.rs`.
- **Work:** D4 and D6 end to end.
- **Depends on:** T3.
- **Acceptance:** integration tests through a real client on temporary repositories:
  - Two agents with worktrees change the same file: each change shows only in its own worktree, and the repository folder shows 0 changed files (SC-001).
  - Delete 10 agents with no changes, choosing remove: 0 folders, 0 records, and 0 branches without own commits remain (SC-002).
  - Remove with changes and no confirmed hash answers `conflict` and keeps everything. With the matching hash it removes (SC-003). A stale hash (a file added after the question, or one file reverted and another added) is refused.
  - A running agent that writes a file between the question and the delete: the delete stops it first, the recount sees the file, and the removal is refused with `conflict`.
  - A repository that cannot be reached keeps the folder and names it.
  - A branch with own commits stays and is named.
  - Keep leaves folder and branch and names them.
  - A subtree delete of a lead and 3 members gives one question covering all 4. A stale hash on the third member answers `conflict` before any member is deleted.
  - Each create and delete is recorded with its actor (PB-4).
  - A restart keeps branch, base, and folder (SC-005).
  - fmt, clippy, per-crate tests, boundaries.

### T5: Desktop forms, details, delete dialog (FR-001, FR-003, FR-005, FR-006, PB-3, accessibility)

- **Files:** `D/modules/agent/{models/agent.ts,components/agent-form-dialog.svelte,components/agent-details.svelte,components/worktree-delete-dialog.svelte,services/agent-service.ts}`, `D/modules/hierarchy/{components/hierarchy-agent-dialog.svelte,views/hierarchy-view.svelte,services/hierarchy-service.ts,state/hierarchy-controller.ts}`, `D/modules/agent/state/office-controller.ts`, `D/modules/panel/views/panel-view.svelte`, `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/agent-worktree.spec.ts`.
- **Work:** D9.
- **Depends on:** T4.
- **Acceptance:**
  - `bun test` for the service and the dialog state.
  - Playwright with axe on the mock core, all by keyboard:
    - the choice is not offered outside a repository, and is offered off inside one with base, branch name, and the note;
    - the details show the three rows;
    - the delete dialog has keep focused first, the change state as text, and the ignored-files note;
    - remove with changes asks again, names the counts, and is announced, while cancel keeps the worktree;
    - the result lists the kept and deleted branches;
    - a subtree delete shows one question with each worktree.
  - `svelte-check`, `eslint`, `prettier --check`.

### T6: Templates carry the choice (FR-009, MODIFIED 0009/FR-001 and FR-003)

- **Files:** `M/template/domain/` (body field), `H/bridges/template_create.rs` (+ tests), `D/modules/template/` (editor checkbox), `src/apps/office-harnessd/tests/templates.rs`.
- **Work:** D8.
- **Depends on:** T4 and 0009 merged.
- **Acceptance:**
  - A team template with own worktree on for the members creates 1 lead and 6 members, each member with its own worktree.
  - A folder outside a repository refuses the whole creation with the message and leaves no agent, team, branch, or folder.
  - A failure at the last member undoes every worktree and branch, without asking (create undo mode).
  - Export and import keep the choice (0009 SC-002).
  - fmt, clippy, tests, `bun test`, Playwright for the checkbox.

### T7: Docs and QA (all criteria)

- **Files:** `README.md` (own worktree, the delete question, kept worktrees, the git requirement), `specs/changes/0010-agent-worktree/qa-report.md`.
- **Work:**
  - Document the flows.
  - QA rows for SC-001 to SC-005 and the edge cases.
  - Hands-on rows are recorded as BLOCKED until run: real repositories on macOS and Windows, a real CLI working in a worktree, the screen reader pass, and 200 percent zoom.
- **Depends on:** T6.
- **Acceptance:** every Traceability row has evidence or a recorded gap; `rg` finds no other project's name.

## Dependencies

T1 → T2 → T3 → T4 → T5 → T7, and T4 + 0009 merged → T6 → T7. `Cargo.lock`, the migration ledger, and the e2e mock core belong to one running task at a time. Gate: this plan approved (Efe plan review PASS).

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged).

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0010-agent-worktree/{data-model.md,contracts/}` | `docs(spec-0010): add the worktree data model and protocol` |
| 2 | T2 | `M/agent/domain/`, `M/agent/application/src/common/ports/`, `M/agent/infrastructure/` | `feat(agent): manage git worktrees through the git program` |
| 3 | T3 | `M/agent/`, `H/bridges/role_requests.rs`, `H/config/`, `H/bootstrap/` | `feat(agent): create agents with their own worktree` |
| 4 | T4 | `M/agent/`, `M/hierarchy/application/`, `H/bridges/subtree_requests.rs`, `src/apps/office-harnessd/tests/` | `feat(server): keep or remove worktrees on delete` |
| 5 | T5 | `D/modules/agent/`, `D/modules/hierarchy/`, `src/apps/desktop/e2e/` | `feat(desktop): offer own worktree and ask on delete` |
| 6 | T6 | `M/template/`, `H/bridges/template_create.rs`, `D/modules/template/`, `src/apps/office-harnessd/tests/templates.rs` | `feat(template): carry the own worktree choice` |
| 7 | T7 | `README.md`, `specs/changes/0010-agent-worktree/qa-report.md` | `docs(spec-0010): document own worktrees and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, and Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **Per phase PR:** CI green on the matrix (git tests on three OSes); Efe's review at the head sha.
- **Security pass** (`secure-engineering`: process execution and file deletion):
  - No shell; arguments as a list.
  - Branch names follow the D3 slug rule and pass `check-ref-format`. `base` is one of the listed refs, passed as `refs/heads/<name>` or a full commit id. Path arguments follow `--`.
  - The child environment drops every `GIT_*` and askpass variable; stdin is null; stdout is bounded; a timeout ends the whole process tree.
  - Paths stay under the data folder: the remove path is the stored folder, and it must be inside `worktrees/`, else it is refused. The worktrees folder must be outside the chosen repository.
  - The server stops the agent, counts and hashes again, and compares before any removal. A clean confirmation removes without `--force`. No repository-wide prune.
  - The agent connection cannot set `own_worktree`, `base`, or `branch_name`.
  - Git is never prompted for credentials (`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, no askpass). Repository hooks do not run. The office runs no network command itself; checkout filters such as LFS may fetch, as in the user's own checkout (documented).
  - stderr shown or logged has URL credentials removed.
- **Rollback:** each phase PR is reverted in the umbrella. The agent migration is forward-only (`DatabaseTooNew`). Worktrees made before a rollback stay on disk and in the repository's git data. The README says how to remove them with `git worktree remove`.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-005 and the edge cases.
2. **Architecture:** `scripts/check-boundaries.sh` passes.
3. **Code quality:** lints are clean; `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office from before the change opens with every agent intact (T3 test, CI on three OSes).
5. **Scope fidelity:** no merge, push, or commit by the office; agent-made agents have no worktree.
6. **Readiness:** `/ac:verify` passes on the umbrella head before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T2, T5 | info cases; form choice |
| FR-002 | T2, T3 | create and subfolder tests |
| FR-003 | T3, T5 | `agent_json` fields; details rows |
| FR-004 | T2, T3, T6 | failed and timed-out create leaves nothing; insert-failure, role-placement and batch undo |
| FR-005 | T4, T5 | one question, keep default; subtree run |
| FR-006 | T2, T4, T5 | all untracked files counted; stop then recount; conflict on a stale hash; second confirmation |
| FR-007 | T2, T4 | branch kept with own commits, base gone, or checked out elsewhere |
| FR-008 | T3 | start refused for a missing folder or a broken `.git` link |
| FR-009 | T6 | template team run; refusal outside a repository |
| 0009/FR-001, FR-003 (modified) | T6 | body field round trip |
| PB-1 | T3 | create without the choice unchanged |
| PB-2 | T2, T4 | repository folder 0 changes; `.git/config` byte-identical; hooks off |
| PB-3 | T4, T5 | 0002 subtree count and confirmation kept |
| PB-4 | T4 | create and delete records |
| PB-5 | T3, T4 | restart run |
| A1 to A9 | T2 to T6 | per decision above; A1 data folder inside the repository refused (T3); A4 same worktree after a new task (T3) |
| SC-001 | T4 | two-agent same-file run |
| SC-002 | T4 | 10-agent remove run |
| SC-003 | T4 | conflict without count |
| SC-004 | T2, T3 | outside a repository: 0 offers, 0 worktrees |
| SC-005 | T4 | restart run |
| US1, US2 | T2 to T5 | the T4 runs and the T5 Playwright runs |
