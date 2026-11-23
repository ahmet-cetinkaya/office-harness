---
status: completed
spec: specs/changes/0020-temporary-agents/SPEC.md
commits: per-task
---

# Plan: Temporary agents (0020)

## Summary

An agent gets an optional temp mark: its objective and its creator. The person creates a temp from the create form; a CEO or a lead creates one with `agent.create` and `temp: true`. The office starts it at once and puts the objective at the head of its queue (0012). A server task, the temp watcher, checks the temps every 2 seconds: a temp that is `stopped`, `exited`, or `failed`, or that has been quiet for the idle limit, is ended. An accepted `done` report ends its sender at once. The end runs in one place, the temp ender: child temps first, then the stop, the keep rule, the delete, a finished temp entry, and an `end` record. The desktop app gets the temp choice in the create form, a Temps dialog, two settings, and a "temp" row in the details.

**What you get:**
- The temp mark on agents (migration), the objective rules, and the temp cap in the create step.
- Finished temp entries (migration, store, `temp.list`, `temp.dismiss`) and `temp.end` for the person.
- The temp watcher and the temp ender in the host, with the `done` hook, the delete hook, end notices, and one notification for an idle end that no parent hears.
- The objective and the end notice as queue entries; a waiting report or end notice of an ended temp stays (MODIFIED 0015/FR-005).
- Desktop: the create form choice, the Temps dialog, the cap and idle limit fields, the stop confirmation for a temp, the details row, mock core, unit and e2e tests.

**Why this approach:**
- The end must work while the desktop app is closed (0001/FR-019), so the server owns the watcher and the settings.
- Ends reuse the ordered `agent.stop` and `agent.delete` requests through `AgentGateway::run`, as the idle sleeper does, so the status order, the events, the Needs you cleanup, and the 0010 worktree handling come from the existing path.
- A poll finds a stopped, exited, or failed temp whatever stopped it (the person, an agent, a subtree stop), with no hook in each stop path.
- The keep rule maps onto the existing delete choice: `Remove` with the status hash for a clean worktree, `Keep` otherwise. The delete step already re-reads the status before it removes anything and keeps the branch when it has own commits (0010/FR-007).

**What it will not do:** the spec's Out of scope list.

**Shape of the work:** 5 tasks in one PR (`feat/0020-temporary-agents`), one commit per task. **Effort:** L.

## Goal

Every FR and SC of `specs/changes/0020-temporary-agents/SPEC.md`, proven by unit, wire, and e2e tests, with CI green on Linux, macOS, and Windows.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 7d1e85d on 2026-11-23.

- **Agent:** `Agent` (`M/agent/domain/src/entities/agent.rs:16`, fields up to `worktree`). Migrations `M/agent/infrastructure/src/persistence/migrations/0001` to `0007`, registered in `migrations/mod.rs:21`, run by `H/bootstrap/migrations.rs`.
- **Create:** `CreateAgentHandler` (`M/agent/application/src/commands/create_agent/create_agent_handler.rs:25`, `make_worktree` at `:193`); the user request `CreateAgentRequest::read` (`M/agent/presentation/src/contracts/agent_requests.rs:24`, handler `agent_handlers.rs:259`); the agent connection allowlist `CREATE_FIELDS` (`H/server/agent_call.rs:856`), `refuse_extra_fields` (`:613`), `allowed_fields_of` (`:639`); role creates through `AgentCreator` (`M/hierarchy/application/src/common/ports/agent_creator_ports.rs:79`) and `AgentCreatorBridge` (`H/bridges/role_requests.rs:164`).
- **Worktrees:** `WorktreeManager` (`M/agent/application/src/common/interfaces/worktree_manager.rs:115`: `info`, `create`, `status`, `remove`, `delete_branch`, `discard`), `WorktreeStatus` (`:49`: `reachable`, `changed_files`, `status_hash`, `own_commits`).
- **Delete:** `DeleteAgentHandler` (`M/agent/application/src/commands/delete_agent/delete_agent_handler.rs:162`) with `DeleteWorktrees::{Keep, Remove(hashes), Discard}` (`:22`) and `WorktreeOutcome` (`:36`); subtree delete `M/hierarchy/application/src/commands/subtree/subtree_handler.rs:105`.
- **Ordering:** `AgentGateway::run` and `ORDERED_REQUESTS` (`H/server/agent_requests.rs:49`); `agent.create` is not ordered.
- **Status from the CLI:** `AgentStatusBridge::apply` (`H/bridges/agent_status_bridge.rs:78`); a stop changes the status first, so the exit of a stopped CLI is dropped.
- **Reports and queue:** `answer_report` (`H/server/agent_call.rs:119`); `SendReportHandler` (`M/hierarchy/application/src/commands/send_report/send_report_handler.rs:101`); `WaitingDirective` and `push_report` (`.../send_directive/directive_queue.rs:28`, `:204`); `recheck` withdraws a report whose sender is no longer a child (`send_directive_handler.rs:492`).
- **Idle sleep pattern:** `IdleSleeper` (`H/server/idle_sleep_run.rs:118`), `quiet_for` (`H/bridges/idle_sleep_gate.rs:114`, `TerminalRuntime::quiet_for`), `PersonInput` (`H/bridges/person_input.rs`).
- **Settings:** `Settings` and `SettingKey` (`M/agent/domain/src/value_objects/settings.rs`), key and value rows in `agent_settings`.
- **Desktop:** create form `D/modules/agent/components/agent-form-dialog.svelte` with `WorktreeField`; `NewAgent` and `Agent` (`D/modules/agent/models/agent.ts:37`, `:91`, hand-written); controller `D/modules/agent/state/office-controller.ts` (`createAgent` `:375`, `handleEvent` `:282`); details `agent-details.svelte:121`; `confirm-dialog.svelte`; limits dialog `D/modules/hold/components/limits-dialog.svelte` and `limits.ts:15`; idle sleep dialog and header button `panel-view.svelte:441`; e2e mock `src/apps/desktop/e2e/support/mock-core.ts`; model e2e `e2e/idle-sleep.spec.ts`.
- **Tests:** wire support `H/../tests/support/` (`Office::open_with_options`, `FakeClis`, `print_then_wait`), report helpers in `tests/agent_report.rs` (`call`, `report`, `place`, `running_agent`).
- **Boundaries:** `scripts/check-boundaries.sh`: a module crate never depends on another module crate; cross-module wiring lives in `H/bridges` and `H/bootstrap`.

## Decisions

- **D1 Temp mark.** `TempMark { objective: Objective, creator: Actor }` in `M/agent/domain`, `Agent::temp: Option<TempMark>`. Migration `0009_agent_temp.sql` (0008 before the 0017 merge) adds `temp_objective TEXT NULL` and `temp_creator TEXT NULL` (`user` or an agent id) to `agent_agents`. `Objective::new` applies the directive text rules: not blank, at most 8,192 characters, no C0, DEL, or C1 character (the same rules as `validate_directive_text`; the agent module may not depend on the hierarchy module, so the rule is restated with a shared test table of cases). Agent JSON gets `temp: {objective, creator: "user" | <agent id>} | null`.
- **D2 Settings.** `SettingKey::TempCap` (`temp_cap`, default 8, whole number 1 to 100) and `SettingKey::TempIdleMinutes` (`temp_idle_minutes`, default 30, whole number 5 to 1440). Refusal texts name the range and keep the stored value. No migration: key and value rows.
- **D3 Create.** `CreateAgentRequest` gets `temp: Option<NewTemp { objective }>`. Under a new `TempCapLock` (a mutex in the handler, held from the count to the insert), the handler counts agents with a temp mark and refuses with `AgentError::TempCapReached { cap }`, text "the temp cap is <n>; end a temp or raise the cap", wire code `refused`. The user request takes `temp: true` and `objective`; `temp` without an objective is `invalid` "a temp needs an objective". A create of a temp from the desktop app defaults `own_worktree` to on when `worktree.info` finds a repository; the form shows it and the person may turn it off.
- **D4 Agent-made create.** `CREATE_FIELDS` gains `temp`, `objective`, and `own_worktree`. A caller that is a temp always creates a temp: `temp` is set to true and a missing objective is `invalid`. For a temp, `own_worktree` defaults to true: the bridge asks `WorktreeManager::info` for the working folder and, when it answers, creates with that base and branch name; `own_worktree: false` or no repository creates none (FR-002). `AgentSeed` gains `temp` and keeps `worktree`. For a non-temp create, `own_worktree` is `invalid` (0010 A7, PB-1). A cap refusal is recorded like other refusals but is not passed to the rate limiter's refusal count (FR-003, as 0015/FR-010 does for "no room for reports").
- **D5 Start and objective.** After a temp create succeeds, the host runs `agent.start` as the creator (user or agent; through `AgentGateway::run` under `ActorScope`), then `SendDirectiveHandler::queue_objective(temp, text, creator)`: a new entry point that skips the sender and descendant checks, the directive rate limits, and the hold check, and pushes a `WaitingDirective` with `sender = temp`, `origin: Origin::Objective(creator)`. It is recorded once as a `directive` entry in the temp's record with the creator as actor. A start failure leaves the temp `failed`, and the watcher ends it (FR-005).
- **D6 Queue origins.** `WaitingDirective::report: Option<String>` becomes `origin: Origin` with `Directive`, `Report { text }`, `Objective { by: Actor }`, and `EndNotice { text }`. `recheck` skips the sender check for `Objective`, and for `Report` and `EndNotice` whose sender is no longer an agent and was a temp (the queue keeps a `from_temp: bool` on the entry, set at push). `EndNotice` is pushed with `push_report` (one place per sender, the 10-report share). Line format: `[temp ended: <name in visible form> (<agent id>): <reason>]`.
- **D7 Temp ender.** `TempEnder` in `H/bridges/temp_ender.rs`, one shared instance. `end(agent_id, reason: EndReason, actor: Actor)`:
  0. For a watcher end of a temp with any status (`stopped`, `exited`, `failed`, or the idle end of a live one): `temp.end_check` (D8); a changed status returns `StatusChanged` and ends nothing. `temp.end_check` joins the `ending` set inside the order, so no start slips in between the check and the delete (Efe r2 N1).
  1. Joins the `ending` set (when step 0 has not) or returns `AlreadyEnding`. While a temp is in the set, `AgentGateway::run` refuses `agent.start`, `agent.new_task`, `session.continue`, `agent.resume`, and `agent.wake` for it with `conflict` "the temp is ending"; the ender's own requests pass.
  2. Ends each child temp first (tree rows), reason `ParentEnded { name }`.
  3. Runs `agent.stop` through `AgentGateway::run` as `actor` when the temp runs.
  4. Reads `WorktreeManager::status`; when `reachable`, `changed_files == 0`, and `own_commits == Some(0)`, uses `DeleteWorktrees::Remove({id: status_hash})`, else `Keep`.
  5. Snapshots the agent (name, objective, creator, parent, created time, worktree), runs `agent.delete` through `AgentGateway::run` as `Actor::Office`, and reads `WorktreeOutcome` to fill the kept folder and branch or the removal failure.
  6. Inserts the finished entry, records `end` (new `AgentAction::End`) in the temp's record with the reason and `actor`, and publishes `temp.changed`.
  7. For `NoOutput` and `Exited`/`Failed`, pushes the end notice to a running parent; for `NoOutput` with no running parent or no room, creates one notification `temp_ended` (FR-007).
  Every path leaves the `ending` set.
- **D8 Temp watcher.** `TempWatcher` in `H/server/temp_run.rs`, a tokio task started in `running_server.rs`, ticks every `temp_tick` (2 s; tests 50 ms) on `spawn_blocking`. Knob `temp_minute` (60 s; tests 100 ms). Each round lists the agents with a temp mark and, for each one not in `ending`:
  - `stopped`: ends with `Stopped { by }`, where `by` is the actor of the temp's last `stop` record (user, agent name, or office);
  - `exited` or `failed`: ends with `Exited { code }` or `Failed { reason }` from the agent's exit result;
  - `interrupted`: nothing (PB-6);
  - running or `asleep`: computes the quiet-since moment as the latest of `now - quiet_for` (when a terminal runs), `PersonInput::since_last_input`, and the watcher's own `blocked_at` mark, which it sets to `now` while a hold covers the temp (`hold_refusal` over `HoldService::list` and the tree), a child of it runs, or it has a Needs you item (`NeedsYouBridge`, as the 0018 gate reads it); the mark of an asleep temp keeps the last known quiet-since moment. When `now - quiet_since >= temp_idle_minutes x temp_minute`, ends with `NoOutput { minutes }` as `Actor::Office`.
  The round reads statuses without the order only to pick candidates. For every end the watcher picks (the idle end passes the status it read), the ender's first step (D7 step 0) takes `status_order` through a new ordered request `temp.end_check {agent_id, seen}` on `AgentGateway::run`, re-reads the stored status, and goes on only when it is still the status the round saw; a new task or a continued session holds the same order, so the watcher sees `starting` or a running status after it and ends nothing (spec FR-005, Efe r1 M1). `temp.end_check` is office-only: the user connection and the agent connection refuse it.
  End reasons as text: `done`, `no output for N minutes`, `exited with code N`, `failed: <reason>`, `stopped by the person` / `stopped by <agent name>` / `stopped by the office`, `ended by the person`, `deleted by the person`, `its parent <name> was deleted`, `its parent <name> ended`.
- **D9 Done hook.** `answer_report` (`H/server/agent_call.rs`) calls `TempEnder::end(caller, Done, Actor::Office)` on its own task after `SendReportHandler::handle` returns `Ok` for kind `done` and the caller has a temp mark, for a report to the parent and for a report for the person. The finished entry keeps the report's kind and visible text.
- **D10 Delete hook.** Before `agent.delete` runs for the person (user connection), the gateway snapshots the temps in the affected set; after it succeeds, it inserts one finished entry per deleted temp with `DeletedByPerson` for the named agent and `ParentDeleted { name }` for the others, filling kept folders and branches from the `WorktreeOutcome`. The ender's own deletes skip this hook.
- **D11 Finished entries.** `FinishedTemp { id, agent_id, name, objective, creator, parent_name, started_at, ended_at, reason, report: Option<(kind, text)>, kept_folder, kept_branch, removal_failure }` in `M/agent/domain`; table `agent_finished_temps` in the same migration `0008`; store port `FinishedTempStore { insert, list (newest first), dismiss }`. Requests (user connection only): `temp.list` answers `{temps: [{agent, creator_name, parent_name, run_seconds, quiet_seconds}], finished: [...]}` (quiet seconds from the watcher's last round), `temp.end {agent_id}` (runs the ender with `EndedByPerson`, answers `conflict` "the temp already ended" when it is gone or ending), `temp.dismiss {entry_id}`. Event `temp.changed {}` after each create of a temp, end, and dismiss. The agent connection refuses `temp.*` (not in its allowlist).
- **D12 Desktop.**
  - `D/modules/agent/models/agent.ts`: `Agent.temp` and `NewAgent.temp`/`objective`; the action label map gets `end`.
  - Create form: a "Temporary agent (one job)" checkbox and an "Objective" textarea (required when checked, with the rule in its hint); when checked and the folder is in a repository, `WorktreeField` starts on.
  - New `D/modules/temps/`: service (`temp.list`, `temp.end`, `temp.dismiss`), model (durations as text, reason text), controller (list state, refresh on `temp.changed`, `agent.changed`, and every 10 s while open), `temps-dialog.svelte` (two lists: "Temps" with name, creator, parent, objective, status, run time, quiet time, and End; "Finished" with name, objective, reason, report, kept folder and branch (or "nothing kept; files that git ignores were removed with the worktree"), ended time, and Dismiss; empty texts), header button "Temps" with the count.
  - End and a single stop of a temp ask `confirm-dialog` with "<name> will be removed. Work that is not merged stays in its worktree and branch. Files that git ignores are removed with a removed worktree."; Cancel has the first focus; Escape cancels.
  - Limits dialog: "Temp cap" (1 to 100) and "Temp idle limit, minutes" (5 to 1440) fields with their own validation, saved through `settings.set`.
  - Details: a "Temp" row with the objective and the creator.
  - Mock core: `temp.*` requests, the temp fields, settings validation, and `temp.changed`.
  - No new visual style: existing dialog, list, and field primitives (spec Design).

## Scope

**In:** D1 to D12, the contract `contracts/temp-protocol.md`, README section, unit, wire, and e2e tests, `qa-report.md`.

**Out:** the spec's Out of scope list. Template entries (0009) gain no temp choice.

## Tasks

### T1 Agent module: temp mark, objective, settings, cap, finished entries (D1 to D3, D11 module part)

- `TempMark`, `Objective`, migration `0008`, settings keys, the create handler's temp input and cap with `TempCapLock`, `AgentAction::End`, `FinishedTemp`, `FinishedTempStore` and its SQLite store, agent JSON, presentation parsing for the user `agent.create` temp fields and for `temp.list`/`temp.end`/`temp.dismiss` bodies.
- **Acceptance:** unit tests prove: `Objective` accepts 1 and 8,192 characters and refuses blank, 8,193, ESC, DEL, and U+009B; settings accept 1 and 100 (cap) and 5 and 1440 (idle) and refuse 0, 101, 4, 1441, 2.5, and text with the old value kept; a temp create stores the mark and survives a reload; the cap refuses the 9th temp at the default and counts temps of every status; two parallel creates at cap minus one give one temp; finished entries list newest first, survive a reopen, and dismiss removes one row. `cargo test -p agent-domain -p agent-application -p agent-infrastructure -p agent-presentation` passes.

### T2 Hierarchy module: queue origins, objective entry, end notice, report stays (D4 module part, D5, D6)

- `Origin`, `queue_objective`, `push_end_notice`, the `recheck` exception, `AgentSeed.temp`, the cap refusal kept out of the automatic-hold count.
- **Acceptance:** unit tests prove: an objective is queued for a target that is not the sender's descendant, counts toward no directive limit, and is recorded once with the creator as actor; a waiting report and an end notice of a removed temp stay and are typed; a waiting report of a removed non-temp sender is withdrawn as before (0015 regression); an end notice takes the sender's one report place; a cap refusal adds no refusal count. `cargo test -p hierarchy-domain -p hierarchy-application` passes.

### T3 Host: agent-call fields, create flow, watcher, ender, hooks, requests (D3 to D11 host part) — depends on T1, T2

- `CREATE_FIELDS`, the temp-caller rule, `own_worktree` default, start and objective after create, `TempEnder`, `TempWatcher`, `ServerOptions` knobs, the done and delete hooks, `temp.*` requests, `temp.changed`, the `temp_ended` notification.
- **Acceptance:** `tests/temps.rs` with fake CLIs in a temporary git repository, `temp_minute` 100 ms, tick 50 ms, proves:
  - the person creates a temp: own worktree, started, objective typed with Enter once the fake CLI waits (SC-001);
  - a lead creates 3 member temps, each reports `done`: 3 report lines in the lead's fake CLI record, 3 temps gone within 10 s, 3 finished entries with the reports (SC-002);
  - the keep rule on clean, uncommitted, committed, and both (SC-003);
  - cap 3: the 4th create by an agent and by the person is refused with the cap text, and 5 agent refusals give no hold (SC-004);
  - idle limit 5 units: a silent temp ends with "no output for 5 minutes" and the parent's record shows the end notice; a temp printing every unit and a temp under a hold stay (SC-005);
  - the person deletes a lead with 3 temps and keeps worktrees: 3 entries "its parent L1 was deleted", 3 worktrees stay (SC-006);
  - a lead temp's `done` ends its 2 member temps first (SC-007, order from the record times);
  - 5 entries survive `serve_again`; dismiss removes the entry and no folder (SC-008);
  - `temp.list` lists 10 temps at several places (SC-009 server part);
  - a `done` report waiting in a working parent's queue is typed after the temp ended (SC-010);
  - a temp whose CLI exits with code 1 ends within 5 s with "exited with code 1" (SC-011);
  - 10 `agent.new_task` and 10 `session.continue` on running temps, with the watcher tick at 10 ms, end 0 temps (SC-012, Efe r1 M1);
  - a silent temp with a Needs you item is not ended by the idle limit; after `needs_you.dismiss` it ends one idle limit later (FR-004, Efe r1 M2);
  - a member temp's create is refused; a temp's create of a non-temp makes a temp; an agent-made non-temp create with `own_worktree` is `invalid`; agent callers get `temp.*` refused; `temp.end` on an ending or gone temp answers `conflict`; a stop of a temp by its parent ends it with "stopped by <name>"; an `interrupted` temp after `serve_again` is not ended.
  The whole office-harnessd test suite passes.

### T4 Desktop (D12) — depends on T3 for the contract only

- Models, create form, `temps` module, header button, limits fields, details row, stop confirmation, mock core.
- **Acceptance:**
  - `bun test` covers the temps model (durations, reasons) and controller (refresh on events, end and dismiss results), and the create form's temp validation.
  - Playwright `e2e/temps.spec.ts` covers: the create form with a temp and objective sends `temp` and `objective`, refuses a blank objective, and starts own worktree on in a repository; the Temps dialog lists temps and finished entries with every field and the empty texts; End asks the confirmation, Cancel and Escape end nothing, confirm sends `temp.end`; Dismiss sends `temp.dismiss`; a stop of a temp from the details asks the confirmation; the details show the "Temp" row; the limits dialog refuses cap 0 and 101 and idle 4 and 1441; keyboard only with visible focus; 360 px width without horizontal scroll; an axe scan of the dialog with no serious or critical violation.
  - `bun run check`, `lint`, `format:check`, `test`, and the Playwright suite pass.

### T5 Docs and evidence — depends on T1 to T4

- `contracts/temp-protocol.md` (fields, requests, events, end reasons, end notice format); README section; `qa-report.md` (each FR and SC with its evidence); the spec Deviations if any; the plan Result.
- **Acceptance:** each FR and SC has a qa row with evidence. The no-other-project grep of the diff is clean.

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`, local `RUSTFLAGS='-Clinker-features=-lld'`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright on a private port.
- `scripts/check-boundaries.sh`.
- CI green on Linux, macOS, and Windows.
- Hands-on: a real `claude` temp that reports `done` when the gate runs it; otherwise the qa row stays BLOCKED, never PASS.

## Risks

- **The poll and an ordered request race.** The ender runs stop and delete through `AgentGateway::run`, which takes the status order, and the `ending` set prevents a second end; a person's request that arrives first wins and the ender sees the changed state.
- **Delete answers vary per OS.** Worktree removal on Windows can fail on a locked file; the keep path then records the failure in the entry instead of losing the worktree (FR-006).
- **Queue entry refactor.** `report: Option<String>` to `Origin` touches the 0012 and 0015 paths; T2 keeps the existing queue tests green before adding new ones.

## Result

- **Delivered:** T1 15e5bd2, T2 a49099e, T4 992c49c, T3 31b6449, T5 5960859 and the qa commit; `main` merged at fa83dff in c05eb62.
- **Evidence:** `qa-report.md` gives every FR, PB, and SC a PASS with the test that proves it, except FR-004 and SC-005, which are PARTIAL because their tests run the idle limit on a shortened minute; the hands-on run with a real `claude` temp stays BLOCKED.
- **Follow-up fixed:** the idle end runs through `temp.end_check` with the status it read, so a new task that holds the status order first wins (final review r3, N1; `qa-report.md`, Known follow-up).
- **Deviations:** the dismiss record, the `Refused` kind of the hierarchy creator error, the creating guard of the watcher, and one settings store in the create handler after the 0021 merge (`qa-report.md`, Deviations from the plan).
