---
status: draft
spec: specs/changes/0011-task-line/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #67, main 037b10f). Needs approval through Efe's plan review; Efe gates the PR at its head sha. -->

# Plan: Task line per agent (0011)

## Summary

Each agent row gets one more line: the last task the office gave the agent, with its age. The line comes from the last directive the office entered into the agent's terminal, or from a text the user sets in the agent's details. It never comes from terminal output. A new task clears it, continuing keeps it, and it survives restarts.

**What you get:**
- Three nullable columns on `agent_agents` (migration `0006_task_text.sql`) and a `task` value on the agent, sent in `agent_json`.
- The directive path sets the task text right after the terminal write succeeds. A new task clears it. A new request, `agent.task.set`, lets the user set, edit, or clear it, recorded in the action record as `task_text`.
- `agent.changed` is published on every task-text change, so the list updates live.
- In the desktop app: a task line in each row ("Task given 3 min ago: ..."), the full text with Edit and Clear in the details, and control characters shown as visible symbols, never applied.

**Why this approach:**
- The agent module already owns the agent row and its JSON. Hierarchy reaches it through directive ports and bridges (`H/directive_bridges.rs`), so one more port method keeps the module boundary.
- The directive handler already marks the exact point of receipt (`send_directive_handler.rs:120`, after the write). Refused directives return earlier, so they cannot set the text (edge case).

**What it will not do:** read output, prompts, or titles; type anything into the CLI; show the line on the scene's desks or in Needs you (spec Out of scope).

**Shape of the work:** 5 tasks in one PR, one local commit per task. **Effort:** S to M.

**Main risks:**
- **Live updates.** `agent.changed` is published today only on create. Every task-text change publishes it explicitly; a missed path would leave a stale line until reload. Integration tests cover each of the three paths.
- **Safe display.** The desktop has no control-character helper today. D5 adds one, with unit tests over C0, DEL, C1, and escape sequences.

**Decisions made for you:** column storage in the agent module, the hook points, the request name and its rules, the new action kind, and the display helper.

**Blockers:** none. The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0011. Each agent row shows the task given (first line, at most 80 characters, with its age), set by the last received directive or by the user. The details show the full text, and nothing ever applies a control code. A new task clears the text, a continued session keeps it, and a restart changes nothing.

## Context

- **Repository:** `main` at 037b10f.
- **Existing seams** (read at 037b10f on 2026-11-19):
  - Agent entity: `M/agent/domain/src/entities/agent.rs:14-24`, with public fields assigned after `Agent::new`.
  - Agent repository: `M/agent/infrastructure/src/persistence/repositories/agent_repository.rs`. `SELECT_AGENT` is at `:13`, `into_agent` at `:45`, and `update_status` at `:129` (the per-column write precedent).
  - Store port: `M/agent/application/src/common/interfaces/agent_store.rs`. In-memory fake: `application/src/test_support.rs:95`.
  - Agent migrations: they end at `0005_hierarchy_columns.sql` (plain `ADD COLUMN`); the ledger is `migrations/mod.rs:21-29`.
  - JSON: `agent_json` is at `M/agent/presentation/src/mappings/entities.rs:9-24`. `agent_changed_event` is at `mappings/events.rs:28`, emitted only at `agent_handlers.rs:243,280`.
  - Directive: `M/hierarchy/application/src/commands/send_directive/send_directive_handler.rs:59-122`. The write is at `:112-119`, and receipt at `:120`. Ports are in `M/hierarchy/application/src/common/ports/directive_ports.rs`, bridged in `H/directive_bridges.rs` and wired in `H/bootstrap/hierarchy_services.rs:121-136`. `answer_directive` (`H/server/agent_call.rs:159-192`) publishes no event.
  - New task: `M/agent/application/src/commands/start_new_task/start_new_task_handler.rs:45-68`. Continue: `continue_session_handler.rs:53-84`.
  - Action record:
    - `M/audit/domain/src/action_kind.rs:10-23` has 12 kinds, none of them an edit.
    - `ActionJournal::record` is at `M/agent/application/src/common/action_journal.rs:28`.
    - `AgentAction` is at `common/models/agent_action.rs:7-18`, and its match in `H/action_recorder.rs:68-81`.
    - The desktop labels are `D/modules/audit/models/action-entry.ts:16-25`.
    - The `audit_actions.action` column is `TEXT` with no CHECK, so no audit migration is needed.
  - Desktop:
    - The row is `D/modules/panel/components/agent-row.svelte`: last line at `:70-72`, accessible name `entryName` at `:34-43`.
    - Details: `D/modules/agent/components/agent-details.svelte` (`<dl>` at `:102-122`).
    - Edit precedent: `AdditionsEditor`, through `office-controller.ts:404-411` and `agent-service.ts:49-53`.
    - Model: `D/modules/agent/models/agent.ts:20-31`. Events: `office-events.ts:20` (`upsertAgent`).
    - E2e mock core: `e2e/support/mock-core.ts` (`createAgentRow` at `:390-402`).
  - Scene desks (`D/modules/office-scene/components/desk.svelte`) read only name, status, and last line, and stay untouched.
- **Floor rules:** one build or test at a time (ask god; `CARGO_BUILD_JOBS=4`, `--test-threads=4`, the pinned toolchain, per-crate local runs). Efe reviews at the exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Storage:** `Agent.task: Option<TaskText>`, where `TaskText { text: String, set_at: Timestamp, source: TaskSource (Directive | User) }`. Migration `0006_task_text.sql` adds nullable `task_text TEXT`, `task_set_at INTEGER`, `task_source TEXT`. `SELECT_AGENT` reads them. A new store method `update_task(agent, Option<TaskText>)` writes them. The row is deleted with the agent, so the text goes with it (edge case).
- **D2 Directive receipt** (FR-002, A4, edge cases):
  - A new hierarchy port, `DirectiveTask::received(target, text, at)`, is called at `send_directive_handler.rs:120`, after the write succeeds and before the journal call.
  - The bridge sets the task text with source `Directive` and the server clock time, then publishes `agent.changed`. Hierarchy's ports cannot publish, so the bridge holds the `EventHub`.
  - A refused directive returns before this point and changes nothing.
  - Two directives at once: the handler order decides, so the text is the last one written.
  - No record entry is added, because the directive is already recorded (A4).
- **D3 New task and continue** (FR-004, PB-2): `StartNewTaskHandler::begin` clears the task after `launcher.start` succeeds. A failed start keeps it. The `agent.new_task` route adds `agent_changed_event` to its events. Continue and start do not touch the task.
- **D4 User request** (FR-003): `agent.task.set {agent_id, text}`, for user clients only. It is not in the agent connection's `ALLOWED_REQUESTS`.
  - Text that is empty or only whitespace clears the task (edge case).
  - More than 8,192 characters, counted as Unicode scalar values like the directive check, answers `invalid` with "a task text has at most 8,192 characters".
  - Control characters are accepted and stored as is; FR-006 is a display rule.
  - A success sets source `User`, records `ActionKind::TaskText` (text `task_text`, desktop label "task text") with the user as actor, returns the agent, and publishes `agent.changed`.
  - A user edit and a directive in the same moment: the later store write wins, and both are recorded (the edit as `task_text`, the directive as `directive`).
- **D5 Display** (FR-001, FR-005, FR-006, A1, A3): a desktop helper module `D/modules/agent/models/task-text.ts`.
  - `visibleText(s)` maps C0 controls to their Unicode control pictures (U+2400 + code, so ESC shows as ␛). DEL shows as ␡. C1 controls show as `\u00XX`. Applied everywhere the text is shown.
  - `taskLine(s)` takes the first line (split on `\r\n`, `\n`, `\r`) and keeps 80 grapheme clusters (`Intl.Segmenter`), adding "…" only when it cut something. Exactly 80 shows whole.
  - `taskAge(setAt, now)` returns "0 min ago" to "59 min ago", then "N h ago".
  - The row shows `Task given <age>: <line>` under the last line. The row's accessible name includes it.
  - The details show a "Task" row with the full visible text (pre-wrapped), plus Edit (a textarea with Save and Cancel) and Clear buttons, all by keyboard.
  - The age ticks once a minute.
- **D6 Out of the desks:** the scene and Needs you do not read `task`. The scene's like-for-like comparison of 0005 SC-003 is unaffected.
- **Constitution check:** module boundaries hold (the new port is bridged in `H/`), no CLI configuration is touched (PB-3), every new action works from the keyboard (PB-5), and no other project's name is in the repository.

## Scope

**In:** FR-001 to FR-007, MODIFIED 0003/FR-002, PB-1 to PB-5, A1 to A4, SC-001 to SC-005, the spec's edge cases, the contract and README updates, a QA report.

**Out:** everything in the spec's Out of scope.

## Delivery

One PR, `feat/0011-task-line` → `main`. The work touches one column set, two hooks, one request, and two desktop components; it is reviewable in one pass.

## Tasks

Paths: `M/` = `src/modules/`, `D/` = `src/apps/desktop/src/`, `H/` = `src/apps/office-harnessd/src/`.

### T1: Design artifacts (no product code)

- **Files:** `specs/changes/0011-task-line/{data-model.md,contracts/task-protocol.md}`.
- **Work:** the three columns and their rules (D1); `agent.task.set` with codes and the 8,192 message; the `task` object in `agent_json` (`{text, set_at, source}` or `null`); `agent.changed` on each change; the `task_text` action kind.
- **Depends on:** none.
- **Acceptance:** every server-side FR names its message or column; style as `0007-hold-and-limits/contracts/hold-protocol.md`.

### T2: Agent task value, storage, action kind (FR-003 rules, FR-004, SC-005)

- **Files:** `M/agent/domain/src/` (`entities/agent.rs`, new `value_objects/task_text.rs` with tests), `M/agent/application/src/` (store port, `test_support.rs`, `common/models/agent_action.rs`, new `commands/set_task_text/`, `commands/start_new_task/`), `M/agent/infrastructure/src/persistence/` (migration, repository, port adapter), `M/audit/domain/src/action_kind.rs`, `H/action_recorder.rs`.
- **Work:** the D1 value and storage; `SetTaskTextHandler` with the D4 rules and journal; the D3 clear in `begin`; `ActionKind::TaskText`.
- **Depends on:** T1.
- **Acceptance:**
  - Unit tests: 8,192 characters accepted and 8,193 refused with the message; blank text clears; control characters stored unchanged; the record entry has kind `task_text` and the user as actor.
  - The new task clears after a successful start and keeps the text after a failed start. Continue keeps it.
  - Repository: the task survives closing and reopening the database (SC-005).
  - Migration test: an office at agent `0005` upgrades with every agent intact and `task` empty.
  - fmt, clippy, per-crate tests.

### T3: Directive hook, request, events (FR-002, FR-003, FR-007, PB-1, SC-001, SC-002, SC-004)

- **Files:** `M/hierarchy/application/src/common/ports/directive_ports.rs`, `send_directive_handler.rs` (+ tests), `H/directive_bridges.rs` (+ tests), `H/bootstrap/hierarchy_services.rs`, `M/agent/presentation/src/{handlers/agent_handlers.rs,mappings/entities.rs,contracts/}`, `src/apps/office-harnessd/tests/task_text.rs`.
- **Work:** the D2 port, bridge, and publish; the `agent.task.set` route; `task` in `agent_json`; `agent_changed_event` on new task.
- **Depends on:** T2.
- **Acceptance:** integration tests through real clients:
  - A lead's directive sets the member's task text, and `agent.changed` with the text arrives within 2 s (SC-001).
  - A refused directive (a non-descendant, blank text, a held office) changes nothing.
  - The terminal receives exactly the directive bytes as before (PB-1).
  - A member that prints output for a while keeps its task text (FR-007, SC-002, machine part).
  - `agent.new_task` clears the text and publishes it (SC-004). `session.continue` keeps it (PB-2).
  - A server restart keeps text and time (SC-005).
  - An agent connection asking for `agent.task.set` is refused as not allowed.
  - fmt, clippy, per-crate tests, `scripts/check-boundaries.sh`.

### T4: Desktop line, details, safe text (FR-001, FR-005, FR-006, MODIFIED 0003/FR-002, PB-4, PB-5, SC-003)

- **Files:** `D/modules/agent/models/{agent.ts,task-text.ts,task-text.spec.ts}`, `D/modules/panel/components/agent-row.svelte`, `D/modules/agent/components/agent-details.svelte` (+ a small task editor component), `D/modules/agent/{state/office-controller.ts,services/agent-service.ts}`, `D/modules/audit/models/action-entry.ts`, `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/task-line.spec.ts`.
- **Work:** the D5 helper, row line, details row, editor, and the label.
- **Depends on:** T3.
- **Acceptance:**
  - `bun test`: `visibleText` maps ESC, BEL, DEL, and a C1 code to visible symbols. `taskLine` covers a multi-line text, exactly 80 graphemes (no ellipsis), 81 (ellipsis), and an emoji counting as one. `taskAge` covers 0, 59, 60, and 125 minutes.
  - Playwright with axe on the mock core:
    - A pushed `agent.changed` shows "Task given 0 min ago: ..." in the row and in its accessible name.
    - A text with `\u001b[31m` and a newline shows as one line with ␛ visible and no color applied (SC-003).
    - Set, edit, and clear by keyboard in the details, and the record shows "task text".
    - 8,193 characters show the refusal message.
    - Status and last line render as before (PB-4).
    - Scene desks show no task text (D6).
  - `svelte-check`, `eslint`, `prettier --check`.

### T5: Docs and QA (all criteria)

- **Files:** `README.md` (the task line and how to set it), `specs/changes/0001-office-server-and-terminal/contracts/client-protocol.md` (a cross-reference to the `task` field), `specs/changes/0011-task-line/qa-report.md`.
- **Work:** document the flow; QA rows for SC-001 to SC-005 and the edge cases. Hands-on rows (screen reader names of the line, 200 percent zoom of the row and details, SC-002's 5-minute real CLI title changes) are recorded as BLOCKED until run.
- **Depends on:** T4.
- **Acceptance:** every Traceability row has evidence or a recorded gap; `rg` finds no other project's name.

## Dependencies

T1 → T2 → T3 → T4 → T5. `Cargo.lock`, the migration ledger, and the e2e mock core belong to one running task at a time. Gate: this plan approved (Efe plan review PASS).

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged).

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0011-task-line/{data-model.md,contracts/}` | `docs(spec-0011): add the task text data model and protocol` |
| 2 | T2 | `M/agent/`, `M/audit/domain/`, `H/action_recorder.rs` | `feat(agent): keep a task text per agent` |
| 3 | T3 | `M/hierarchy/application/`, `H/`, `M/agent/presentation/`, `src/apps/office-harnessd/tests/` | `feat(server): set the task text from directives and the user` |
| 4 | T4 | `D/`, `src/apps/desktop/e2e/` | `feat(desktop): show the task line and edit the task text` |
| 5 | T5 | `README.md`, `specs/changes/0001-office-server-and-terminal/contracts/client-protocol.md`, `specs/changes/0011-task-line/qa-report.md` | `docs(spec-0011): document the task line and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, and Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **PR:** CI green on the matrix; Efe's review at the head sha.
- **Security pass** (`secure-engineering`, light: the user text crosses the trust boundary):
  - The length cap is enforced on the server.
  - The text is never written to a terminal by `agent.task.set` (A2).
  - The display never applies control codes, and Svelte escaping covers HTML.
  - The agent connection cannot reach `agent.task.set`.
- **Rollback:** revert the PR. The agent migration is forward-only (`DatabaseTooNew`); a rollback on a user's machine is the reverted binary plus the `office.db` copy taken before the upgrade, as the README says.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-005 and the edge cases.
2. **Architecture:** `scripts/check-boundaries.sh` passes.
3. **Code quality:** lints are clean; `/ac:review` of the diff has no open blocking finding.
4. **Migration safety:** an office from before the change opens with every agent intact (T2 test, CI on three OSes).
5. **Scope fidelity:** desks and Needs you are unchanged; no output-derived text.
6. **Readiness:** `/ac:verify` passes on the PR head.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T4 | row line, 80-grapheme and age tests |
| FR-002 | T3 | directive integration run |
| FR-003 | T2, T3, T4 | handler rules; request run; keyboard edit |
| FR-004 | T2, T3 | clear after start; `agent.new_task` run |
| FR-005 | T4 | details full text |
| FR-006 | T4 | `visibleText` tests; escape-code Playwright run |
| FR-007 | T3 | output does not change the text |
| 0003/FR-002 (modified) | T4 | row shows the task line |
| PB-1 | T3 | terminal bytes unchanged |
| PB-2 | T2, T3 | continue keeps the text |
| PB-3 | T3 | no prompt, title, or configuration read or written |
| PB-4 | T4 | status and last line unchanged |
| PB-5 | T4 | keyboard Playwright with axe |
| A1, A3 | T4 | age and grapheme tests |
| A2 | T3 | `agent.task.set` writes nothing to the terminal |
| A4 | T3 | no extra record entry for a directive |
| SC-001 | T3, T4 | 2-second directive run; row shows it |
| SC-002 | T3, T5 | output run; 5-minute real CLI row BLOCKED until run |
| SC-003 | T4 | escape-code Playwright run |
| SC-004 | T3 | new-task run |
| SC-005 | T2, T3 | reopen and restart runs |
| US1, US2 | T2 to T4 | the T3 runs and the T4 Playwright runs |
