---
status: draft
spec: specs/changes/0010-agent-worktree/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #66, main b6d2cb9). Builds after 0009 (god's order 0008, 0011, 0009, 0010); T6 needs 0009 merged. Needs approval through Efe's plan review. -->

# Plan: Opt-in own worktree per agent (0010)

## Summary

When the owner creates an agent in a git repository, he can turn on "own worktree". The office then makes a new branch and a worktree for that agent in a folder that the office owns, and the agent works there. Deleting such an agent asks whether to keep or remove the worktree, with keep as the default. A worktree with uncommitted changes is removed only after a second confirmation, and a branch with its own commits is never deleted. Templates carry the choice.

**What you get:**
- A small git runner in the agent module that calls the system `git` program with a timeout and a clean environment, behind an application port.
- Four nullable worktree columns on the agent, shown in the details and kept across restarts.
- `agent.worktree_info`, which tells the form whether a folder is in a repository and shows the base and the branch name before create.
- `agent.worktree_status`, which feeds the delete question.
- `agent.create` gains `own_worktree` and an optional `base`. `agent.delete` gains the keep or remove choice and the confirmed change counts.
- In the desktop app: the choice in both create forms, the details rows, the delete question with the second confirmation, and the result message.
- Templates (0009) carry `own_worktree`, and creation from a template makes the worktrees all-or-nothing.

**Why this approach:**
- The system `git` program is what the user's repository already uses, and it supports worktrees fully. The office already runs helper programs with a timeout (`M/agent/infrastructure/src/services/login_shell.rs:57`), so the runner copies that pattern.
- The spec says the agent's working folder is the worktree, so the existing start check (`session_launcher.rs:376-392`) already refuses a missing folder (FR-008); only the message changes.
- Agents call `agent.create` through an allowlist of fields (`H/server/agent_call.rs:687`). Leaving `own_worktree` out of that list keeps agent-created agents without worktrees (A7) with no new code.

**What it will not do:** merge, rebase, push, commit, or clean up; separate history; turning the choice on later; worktrees for agent-made agents; managing kept worktrees; copying untracked files; version control systems other than git (spec Out of scope).

**Shape of the work:** 7 tasks in 2 phase PRs into one umbrella branch `feat/0010-agent-worktree`, one local commit per task. **Effort:** M to L.

**Main risks:**
- **Losing work on delete.** D6 checks again on the server, does not trust the client, and refuses a removal whose confirmed counts no longer match. Integration tests use real temporary repositories.
- **Git versions and platforms.** The runner uses only porcelain commands available since git 2.20 (`worktree add -b`, `worktree remove`, `worktree prune`, `status --porcelain`, `rev-list --count`, `check-ref-format --branch`). It sets `LC_ALL=C` and `GIT_TERMINAL_PROMPT=0`. CI runs the tests on three OSes, where git is installed.
- **No git on the machine.** `agent.worktree_info` then answers "not in a repository", and the choice is not offered.

**Decisions made for you:** the git CLI, the runner, the columns, the folder layout, the branch name rule, the requests, the delete protocol, and the template hook.

**Blockers:** none. T6 waits for 0009 to merge. The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0010. In a git repository the owner can give a new agent its own branch and worktree. The agent works there, and the user's folder stays unchanged. The details show branch, base, and folder. Delete never removes uncommitted work without a second confirmation and never deletes a branch with its own commits. A missing worktree refuses the start. Templates carry the choice.

## Context

- **Repository:** `main` at 037b10f. Specs 0008 to 0011 are merged. The 0011 plan takes agent migration 0006, so this change takes the next free number at build time (0007 if 0011 shipped first).
- **Existing seams** (read at 037b10f on 2026-11-19):
  - **Git:** none in the code. There is no git2 or gix crate. The helper-program pattern is `run_login_shell` (`M/agent/infrastructure/src/services/login_shell.rs:57`), which uses its own process group, a reader thread with a timeout, and a kill on timeout. It is unix-only and discards stderr, so the git runner is a new cross-platform function that keeps stderr for FR-004's reason.
  - **Create validation:** `Agent::new` (`M/agent/domain/src/entities/agent.rs:41-51`) requires an absolute folder or a project.
  - **Start check:** `existing_working_folder` (`M/agent/application/src/common/session_launcher.rs:376-392`), called from `prepare()` (`:124`). It answers `invalid` "the <field> does not exist" and changes nothing.
  - **Delete:**
    - `agent.delete` (`M/agent/presentation/src/handlers/agent_handlers.rs:182,286-293`) goes through the subtree bridge when attached (`:398-407`), reading `include_descendants` and `descendant_count` from the request fields.
    - The subtree application is `M/hierarchy/application/src/commands/subtree/subtree_handler.rs:86-151`, deepest first, with a count check that answers `conflict`.
    - `DeleteAgentHandler` (`M/agent/application/src/commands/delete_agent/delete_agent_handler.rs:43-50`) stops the session, deletes the row, and records the delete.
  - **Data folder:** `Config` (`H/config/server_config.rs:22-88`), with `output_folder_path` (`:74`) as the pattern for a new data subfolder, consumed at bootstrap (`H/bootstrap/terminal_services.rs:66`).
  - **Agent row:** `agent_agents` (`0001_machines_projects_agents.sql:16`), the repository (`agent_repository.rs:13`), and `agent_json` (`M/agent/presentation/src/mappings/entities.rs:9-24`).
  - **Role creates:** `RoleSeed` (`M/agent/presentation/src/handlers/role_requests.rs:41-48`) to `RoleRequestsBridge` (`H/bridges/role_requests.rs:39-129`) to `CreateRoleAgentHandler`. The agent allowlist is `CREATE_FIELDS` (`H/server/agent_call.rs:687`).
  - **Desktop:**
    - Create forms: `D/modules/agent/components/agent-form-dialog.svelte:66-70` and `D/modules/hierarchy/components/hierarchy-agent-dialog.svelte:43-47`.
    - Details `<dl>`: `D/modules/agent/components/agent-details.svelte:102-122`.
    - Confirm dialogs: `D/modules/agent/components/confirm-dialog.svelte` (Cancel has the first focus) and `D/modules/hierarchy/components/hierarchy-confirm-dialog.svelte` (radio choices).
    - Model: `D/modules/agent/models/agent.ts:20-31,69-74`.
  - **Templates:** the 0009 plan (`specs/changes/0009-agent-templates/PLAN.md`, D2 and D5) keeps the body additive and creates batches in `TemplateCreateBridge` with an undo.
- **Floor rules:** one build or test at a time (ask god; `CARGO_BUILD_JOBS=4`, `--test-threads=4`, the pinned toolchain, per-crate local runs). Efe reviews at the exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Git access:** the system `git` program through `GitRunner` in `M/agent/infrastructure/src/services/git_runner.rs`.
  - It runs with no shell and arguments as a list.
  - It sets `-C <dir>` and `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`, and `GIT_OPTIONAL_LOCKS=0` for read commands.
  - It has a 10 s timeout with a kill, and captures stdout and stderr (stderr trimmed to 500 characters for messages).
  - It never writes any git configuration.
  - The application port is `WorktreeManager`, with `info`, `create`, `status`, `remove`, and `prune`. Domain types are `WorktreeInfo` and `AgentWorktree { repository, base, branch, folder }`.
  - Rejected: `git2` (native libgit2 and its build cost on three OSes) and `gix` (worktree add and remove are not complete).
- **D2 Folder layout** (FR-002, A1): `<data folder>/worktrees/<repo folder name>-<8 hex of the repository path hash>/<agent short id>`. It is outside every repository. The agent's working folder is the worktree plus the chosen folder's path relative to the repository top (the subfolder edge case). `Config::worktrees_folder_path()` follows the `output_folder_path` pattern.
- **D3 Base and branch name** (A2, A3, FR-001, edge cases):
  - The base is the branch checked out in the chosen folder, or the short commit id when it is detached. The form can pick another local branch.
  - The branch name is `office-harness/<agent name>-<short id>`. Each character that `git check-ref-format --branch` refuses is replaced by `-`, runs of `-` collapse to one, and leading and trailing `.` and `-` are trimmed.
  - When the name exists, the next free short id is used. The name shown in the form is the one that will be created, checked again at create.
- **D4 Info request:** `agent.worktree_info {folder?, project_id?, name}`, where the folder or the project's folder is resolved on the server.
  - It returns `{in_repository: false}`, or `{in_repository: true, base, base_is_branch, branches: [local branch names], branch_name}`.
  - With no git program it answers `in_repository: false`.
- **D5 Create** (FR-002, FR-004, A5, A7):
  - `agent.create` gains `own_worktree: bool` (default false) and `base?: string`, for the user path and the user's role creates (`RoleSeed`, `seed_of`).
  - `CREATE_FIELDS` in the agent allowlist stays unchanged, so an agent's `own_worktree` is refused (A7).
  - The order:
    1. Validate the agent.
    2. Run `git worktree add -b <branch> <folder> <base>`.
    3. Insert the agent with the four columns.
  - If step 2 fails, nothing is inserted and the git stderr reason is answered. If step 3 fails, `git worktree remove --force` and `git branch -D` undo step 2, so no agent, branch, or folder is left (FR-004).
- **D6 Delete** (FR-005, FR-006, FR-007, A6, A9, US2):
  - `agent.worktree_status {agent_ids}` returns, for each agent with a worktree, `{agent_id, folder, branch, base, folder_exists, changed_files, own_commits, base_exists}`.
    - `changed_files` is the line count of `git status --porcelain`, which excludes ignored files (A9).
    - `own_commits` is `git rev-list --count <base>..<branch>`, when the base exists.
  - `agent.delete` gains `worktrees: "keep" | "remove"` (default `keep`) and, for `remove`, `confirmed_changes: {agent_id: count}` for every worktree with changes.
  - The server computes the status again under the delete. A `remove` whose worktree with changes has no matching confirmed count answers `conflict` "the worktree of <name> has <n> changed files; confirm again". That count check works the same way as `descendant_count`.
  - Removal per agent, after the 0001 stop:
    1. `git worktree remove --force <folder>` (or `git worktree prune` when the folder is missing, which clears the record).
    2. `git branch -D <branch>` only when `own_commits` is 0 and the base exists. Otherwise the branch is kept and named. A branch checked out somewhere else fails to delete, and is then kept and named.
  - `keep` leaves folder and branch and names them once (A6).
  - The answer lists `{deleted_branches, kept_branches, kept_folders}`.
  - The 0002 subtree delete passes the choice and the counts through, so one question covers the whole subtree.
- **D7 Start** (FR-008): `existing_working_folder` names the worktree when the agent has one: "the worktree folder is missing: <path>". Nothing else changes.
- **D8 Templates** (FR-009, MODIFIED 0009/FR-001 and FR-003):
  - The 0009 agent body gains `own_worktree: bool` (default false), so the editor and the file round-trip carry it.
  - `TemplateCreateBridge` plans a worktree for each such agent from the base of the chosen folder. If the folder is not in a repository, the creation is refused with "own worktree needs a git repository". It creates the worktrees inside the batch and undoes them with the agents.
  - The base is never stored in a template (A2).
- **D9 Desktop:**
  - Both create forms call `agent.worktree_info` when the folder or project changes.
  - When the folder is in a repository, they show an "Own worktree" checkbox (off), the base select, the branch name, and the note "Untracked files such as local environment files are not copied".
  - The details show Branch, Base, and Worktree folder rows when present.
  - Delete of agents with worktrees:
    - A dialog asks "Keep worktrees" (initial focus) or "Remove worktrees". It lists each worktree with "N changed files" or "no changes" as text, and says that ignored files are removed.
    - Remove with changes opens a second confirmation that names the counts and is announced.
    - The result message lists the deleted and kept branches and the kept folders.
  - No visuals beyond these controls.
- **Constitution check:**
  - Module boundaries hold: git lives in the agent module's infrastructure; the subtree pass-through is in `H/bridges`.
  - No CLI configuration and nothing in the user's own folder is written (PB-2). Only a branch and a worktree record are added to the repository's git data.
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
- **Work:** D1 to D3, plus the D6 status and removal mechanics.
- **Depends on:** T1.
- **Acceptance:** tests on temporary repositories created by the test:
  - Info in a repository, in a subfolder, when detached (short id), outside a repository, and with git missing (a `PATH` without git) answers not in a repository.
  - A name with spaces, `..`, `~`, `:`, and `*` gives a valid branch name. An existing name gets the next short id.
  - Create and remove leave no folder, record, or branch. A failed create leaves nothing.
  - The status counts changed, added, deleted, and untracked files, and not ignored ones.
  - `own_commits` counts commits the base lacks. A missing base keeps the branch. A branch checked out elsewhere is kept.
  - A timeout kills git.
  - fmt, clippy, per-crate tests.

### T3: Agent columns, create, start (FR-002, FR-003, FR-004, FR-008, PB-1, PB-5, SC-004, SC-005)

- **Files:** `M/agent/domain/src/entities/agent.rs`, `M/agent/infrastructure/src/persistence/` (migration with the next free number, repository), `M/agent/application/src/commands/create_agent/`, `common/session_launcher.rs`, `M/agent/presentation/src/{handlers,mappings}/`, `M/agent/presentation/src/handlers/role_requests.rs`, `H/bridges/role_requests.rs`, `H/config/server_config.rs`, `H/bootstrap/`.
- **Work:** D2, D5, D7, and the worktree in `agent_json`.
- **Depends on:** T2.
- **Acceptance:**
  - Handler tests: without the choice nothing changes (PB-1, SC-004). With it, the working folder is the worktree subfolder. An insert failure undoes the worktree and the branch.
  - The start of an agent whose worktree folder was deleted is refused with the message.
  - Repository: the columns survive close and reopen (SC-005).
  - Migration test: an office from before upgrades with every agent intact.
  - The agent connection's `agent.create` with `own_worktree` is refused (A7).
  - fmt, clippy, per-crate tests, boundaries.

### T4: Delete, status, protocol, integration (FR-005, FR-006, FR-007, PB-3, PB-4, SC-001 to SC-003)

- **Files:** `M/agent/application/src/commands/delete_agent/`, `M/agent/presentation/src/handlers/agent_handlers.rs`, `M/hierarchy/application/src/commands/subtree/` (pass-through), `H/bridges/subtree_requests.rs`, `src/apps/office-harnessd/tests/agent_worktree.rs`.
- **Work:** D4 and D6 end to end.
- **Depends on:** T3.
- **Acceptance:** integration tests through a real client on temporary repositories:
  - Two agents with worktrees change the same file: each change shows only in its own worktree, and the repository folder shows 0 changed files (SC-001).
  - Delete 10 agents with no changes, choosing remove: 0 folders, 0 records, and 0 branches without own commits remain (SC-002).
  - Remove with changes and no confirmed count answers `conflict` and keeps everything. With the matching count it removes (SC-003). A stale count (a file added after the question) is refused.
  - A branch with own commits stays and is named.
  - Keep leaves folder and branch and names them.
  - A subtree delete of a lead and 3 members gives one question covering all 4.
  - Each create and delete is recorded with its actor (PB-4).
  - A restart keeps branch, base, and folder (SC-005).
  - fmt, clippy, per-crate tests, boundaries.

### T5: Desktop forms, details, delete dialog (FR-001, FR-003, FR-005, FR-006, PB-3, accessibility)

- **Files:** `D/modules/agent/{models/agent.ts,components/agent-form-dialog.svelte,components/agent-details.svelte,components/worktree-delete-dialog.svelte,services/agent-service.ts}`, `D/modules/hierarchy/{components/hierarchy-agent-dialog.svelte,views/hierarchy-view.svelte,services/hierarchy-service.ts}`, `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/agent-worktree.spec.ts`.
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
  - A failure at the last member undoes every worktree.
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
  - Branch names checked by `check-ref-format`.
  - Paths stay under the data folder: the remove path is the stored folder, and it must be inside `worktrees/`, else it is refused.
  - The server checks the change counts again before any removal.
  - The agent connection cannot set `own_worktree`.
  - Git is never prompted for credentials (`GIT_TERMINAL_PROMPT=0`), and no network command is run.
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
| FR-004 | T2, T3 | failed create leaves nothing; insert-failure undo |
| FR-005 | T4, T5 | one question, keep default; subtree run |
| FR-006 | T4, T5 | conflict without count; second confirmation |
| FR-007 | T2, T4 | branch kept with own commits, base gone, or checked out elsewhere |
| FR-008 | T3 | start refused with the worktree message |
| FR-009 | T6 | template team run; refusal outside a repository |
| 0009/FR-001, FR-003 (modified) | T6 | body field round trip |
| PB-1 | T3 | create without the choice unchanged |
| PB-2 | T2, T4 | repository folder 0 changes; no configuration written |
| PB-3 | T4, T5 | 0002 subtree count and confirmation kept |
| PB-4 | T4 | create and delete records |
| PB-5 | T3, T4 | restart run |
| A1 to A9 | T2 to T6 | per decision above |
| SC-001 | T4 | two-agent same-file run |
| SC-002 | T4 | 10-agent remove run |
| SC-003 | T4 | conflict without count |
| SC-004 | T2, T3 | outside a repository: 0 offers, 0 worktrees |
| SC-005 | T4 | restart run |
| US1, US2 | T2 to T5 | the T4 runs and the T5 Playwright runs |
