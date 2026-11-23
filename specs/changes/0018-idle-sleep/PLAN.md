---
status: draft
spec: specs/changes/0018-idle-sleep/SPEC.md
commits: per-task
---

# Plan: Put idle agents to sleep (0018)

## Summary

A new office setting `idle_sleep_minutes` (0 = off, the default; else 5 to 1440) turns on idle sleep. A server task checks the agents a few times a minute. An agent that has waited for input for the idle time, with no blocking condition, is put to sleep: one ordered step checks the conditions again, ends the CLI with the normal stop, sets the new status `asleep`, and records one `sleep` entry with the office as actor. A wake (`wake.start`) continues the last session through the 0013 resume step and reuses the 0013 watch: one fallback to a new session, then the office hold with one notification.

**What you get:**
- Status `asleep` and status event `PutToSleep` in the agent domain; the actions `sleep` and `wake` in the action record.
- `SleepAgentHandler` (request `agent.sleep`, made by the office only) with a `SleepGate` port for the last check.
- `ResumeAgentHandler::wake` (request `agent.wake`): the 0013 continue-or-new step for an asleep agent.
- The host sleeper task, its gate, and the request `wake.start` with the event `wake.result`.
- Directives and reports to an agent that is asleep or going to sleep are refused with the reason "asleep".
- The desktop "Idle sleep" dialog, `asleep` everywhere a status shows, and the asleep terminal view with Wake and the Enter key.

**Why this approach:**
- The sleep must work while the desktop app is closed (0001/FR-019), so the server owns the setting and the timer.
- The sleep runs as an ordered request through `AgentGateway::run`, as a stop does. It takes the status order, so a person's stop, start, continue, or wake waits for it and then sees `asleep`. The events, places, and records come from the existing path.
- The last check runs inside that ordered step through a port, so the condition check and the stop are one step with respect to every status change.
- The wake reuses `ResumeAgentHandler` and the 0013 watch. No second fallback or hold logic exists.

**What it will not do:** wake on a directive, report, or agent request; sleep for memory or schedules; per-agent settings; "Wake all" (spec Out of scope).

**Shape of the work:** 5 tasks in one PR (`feat/0018-idle-sleep`), one commit per task. **Effort:** M.

## Goal

Deliver spec 0018: with idle sleep on, idle agents that nobody needs are put to sleep with their session kept and their last output visible; one key or button wakes them with the 0013 fallback and hold; with idle sleep off, nothing changes.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 6e2a8bd on 2026-11-23.

- **Settings:** `Settings` and `SettingKey` (`M/agent/domain/src/value_objects/settings.rs`), key and value rows, read by `SettingsStore::load`. `settings.get` and `settings.set` in `M/agent/presentation/src/handlers/agent_handlers.rs:217`.
- **Statuses:** `AgentStatus` (`M/agent/domain/src/enums/agent_status.rs`, stored as text), `StatusEvent` (`.../status_event.rs`), transitions in `M/agent/domain/src/services/status_machine.rs:11`.
- **Stop:** `StopAgentHandler` (`M/agent/application/src/commands/stop_agent/stop_agent_handler.rs`) and `SessionEnder::stop` (`M/agent/application/src/common/session_ender.rs`): `TerminalHost::stop(agent, grace)`, then `StatusEvent::StoppedByUser`, then `end_running(ExitResult::Signal)`.
- **Ordering:** `AgentGateway::run` (`H/server/agent_requests.rs:142`) takes `status_order` for `ORDERED_REQUESTS` (`:46`) and refuses `STARTING_REQUESTS` (`:56`) while the server stops. `AgentStatusBridge::apply` (`H/bridges/agent_status_bridge.rs:69`) takes the same order, so the exit of a stopped CLI meets an already changed status and is dropped.
- **Resume:** `ResumeAgentHandler` (`M/agent/application/src/commands/resume_agent/resume_agent_handler.rs`): `resume` and `fall_back`; the watch in `H/server/resume_run.rs` (`ResumeContext::watch`, `hold_reason`, `ResumeResult::event`); the gateway `H/server/resume_requests.rs`.
- **Blocking inputs:** `PersonInput` (`H/bridges/person_input.rs`: last input, unsent draft, pending office text); the directive queue `waiting_count` (`M/hierarchy/application/src/commands/send_directive/directive_queue.rs:150`), shared by reports; Needs-you items through `NeedsYouBridge` (`H/bridges/needs_you_bridge.rs`); holds through `HoldService::list` and `hold_refusal` (`M/hierarchy/domain`), as the subtree read uses them (`H/server/agent_call.rs:210`, `H/server/subtree_status.rs`); the tree through `TreeQuery`; `TerminalRuntime::quiet_for` (`M/terminal/application/src/common/interfaces/terminal_runtime.rs:83`).
- **Refusals:** `DirectiveTarget::is_running` (`M/hierarchy/application/src/common/ports/directive_ports.rs:33`, bridge `H/bridges/directive_bridges.rs:83`); `HierarchyError::NotRunning` (`M/hierarchy/domain/src/errors/hierarchy_error.rs:52`), used by `send_directive_handler.rs:165` and `send_report_handler.rs:178`.
- **Desktop:** header in `D/modules/panel/views/panel-view.svelte:406`; the limits dialog pattern (`D/modules/hold/components/limits-dialog.svelte`); the terminal view (`D/modules/terminal/views/terminal-view.svelte`, `isRunning`, `disableStdin`); the resume module (`D/modules/resume/`); e2e mock `src/apps/desktop/e2e/support/mock-core.ts`.

## Decisions

- **D1 Setting.** `SettingKey::IdleSleepMinutes`, stored as `idle_sleep_minutes`, `u32`, default `0` (off). `settings.set` accepts `0` or a whole number from 5 to 1440 and refuses any other text with `invalid` "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440", keeping the stored value (FR-001). No migration: settings are key and value rows. The desktop shows 30 in the field when the setting is off, so turning it on stores 30 unless the person changes the field.
- **D2 Status.** `AgentStatus::Asleep` (`asleep`), not in `RUNNING`. `StatusEvent::PutToSleep` ("sleep after the idle time"). New transitions: `WaitingForInput + PutToSleep -> Asleep`, `Asleep + Start -> Starting`, `Asleep + StoppedByUser -> Stopped`. The status column is text, so no migration. Server start marks only running agents `interrupted`, so an asleep agent stays asleep (FR-004); `resume.plan` lists only `interrupted` agents (FR-004).
- **D3 Actions.** `AgentAction::Sleep` (`sleep`) and `AgentAction::Wake` (`wake`), stored and sent as text like the other actions. A sleep has the detail "idle for N minutes"; a failed sleep is a `sleep` entry with the failure text (FR-009). A wake that started a new session has the detail "new session: <reason>" (FR-007). A fallback start stays `start` with actor `office` and its 0013 detail.
- **D4 Sleep step.** `SleepAgentHandler::handle(agent_id, idle_minutes)` in `M/agent/application/src/commands/sleep_agent/`, reached by the request `agent.sleep {agent_id, idle_minutes}`, added to `ORDERED_REQUESTS`. Under the status order it: requires `WaitingForInput` (else `conflict`, no record); asks the new port `SleepGate::blocking_reason(agent_id) -> Option<String>` (else `conflict` with the reason, no record, FR-002 last check); calls `SleepGate::begin(agent_id)`; runs `SessionEnder::sleep` (the stop with `PutToSleep` instead of `StoppedByUser`); records `sleep` with the actor from `ActorScope` (the office); and calls `SleepGate::end(agent_id)` on every path. A stop failure records `sleep` with the failure and answers `internal`. The user connection refuses `agent.sleep` with `invalid` "agent.sleep is made by the office only" (`H/server/connection.rs`); the agent-call allowlist does not name it.
- **D5 Stop of an asleep agent.** `StopAgentHandler` accepts `Asleep`: it applies `StoppedByUser`, stores the status, and records `stop`, without a terminal call (FR-004, spec A10 for both actors).
- **D6 Sleeper.** `IdleSleeper` in `H/bridges/idle_sleep.rs`, one tokio task started in `running_server.rs`. Knobs in `ServerOptions`: `idle_sleep_tick` (15 s) and `idle_sleep_minute` (60 s; tests use 100 ms). Each tick it reads the settings. With `0` it forgets its failure marks and returns (PB-2). It remembers the value it saw last and the moment it changed; the first value counts from server start. For each agent with status `waiting_for_input` and no failure mark, when `blocking_reason` is `None`, it runs `agent.sleep` through `AgentGateway::run` on `spawn_blocking` under `ActorScope::enter(Actor::Office)`, one agent after another. An `internal` answer sets a failure mark with its moment. The mark is cleared when the agent's idle-since moment moves past it, that is, after new output or person input (FR-009).
- **D7 Gate.** `IdleSleepGate` (`H/bridges/idle_sleep.rs`) implements `SleepGate` and holds the going-to-sleep set (`SleepingAgents`, a shared `HashSet<AgentId>`). `blocking_reason` returns the first that holds:
  - idle too short: `min(quiet_for - startup quiet period, since last person input, since the setting changed) < minutes x minute` (Terms "Idle since"; `PersonInput::since_last_input` is new);
  - a Needs-you item (FR-002 b);
  - a hold covers the agent (office, own, or ancestor; `hold_refusal` over `HoldService::list` and the tree rows) (c);
  - `waiting_count(agent) > 0` in the directive queue, which also holds reports (d); `SendDirectiveHandler::waiting_count` is new and forwards to the queue;
  - unsent person text or pending office text in the current run (`PersonInput::holds_text`, new: the `holds_directives` rule without the 30-second typing pause) (e);
  - a child with a running status (f).
  It takes no status-order lock, so calling it under the order cannot deadlock. Residual race, accepted: a directive that passed its running check just before `begin` can still enter the queue; the stop then withdraws it with the 0012 reason "the target stopped running", recorded for both agents (0012/FR-005).
- **D8 Asleep refusals.** `DirectiveTarget::is_asleep(agent_id) -> bool` (new). The bridge answers true for an agent in `SleepingAgents` or with status `asleep`, and `is_running` answers false for an agent in `SleepingAgents`. `HierarchyError::Asleep { name }` with the text "<name> is not running: it is asleep". The directive and report handlers choose it over `NotRunning` when `is_asleep` is true (FR-005). Records as today.
- **D9 Wake.** `ResumeAgentHandler::wake(agent_id)`: requires `Asleep` (else `conflict` "the agent is no longer asleep"), then continues the last session or starts a new one exactly as `resume`, but records `wake` (with the D3 detail). Request `agent.wake`, in `ORDERED_REQUESTS` and `STARTING_REQUESTS`. The host request `wake.start {agent_id}` (answered by `ResumeGateway`, which now also accepts the `wake.` prefix; user clients only) runs `agent.wake` as the user, answers `{session, continued, reason?}` or the failure, and then runs the 0013 watch on its own task. `ResumeContext` gets a `Flow` (`Resume` or `Wake`) that picks the event name (`resume.result` or `wake.result`) and the hold reason prefix ("the resume failed" or "the wake failed"). The fallback stays `agent.resume` with `after_session`, actor office. The hold reuses `hold_by_office` and the `resume_hold` notification; the desktop text for `resume_hold` becomes "was put on hold: <detail>", since the detail already starts with "the resume failed" or "the wake failed".
- **D10 Desktop.**
  - `D/modules/agent/models/agent.ts`: status `asleep`, label "asleep"; the hierarchy, scene, and action-record label maps get `asleep`, `sleep`, and `wake`.
  - New `D/modules/sleep/`: service (`settings.get`/`settings.set` for `idle_sleep_minutes`, `wake.start`), model (validation 5 to 1440, result texts), controller (the wake result per agent from `wake.result`, "Waking" while `wake.start` waits), `idle-sleep-dialog.svelte` (switch, minutes field with the unit and range in its label, help text, refusal text; the limits-dialog primitives), `wake-panel.svelte` (the asleep text "Asleep. Press Enter or Wake to continue the last session.", the Wake button, and the result in a `role="status"` region).
  - Header button "Idle sleep" in `panel-view.svelte` next to the hold bar opens the dialog.
  - Terminal view: for an asleep agent it shows the last output as for a stopped agent plus `wake-panel`; with keyboard focus in the view, a keydown of Enter calls the wake. `disableStdin` already keeps every key and paste away from a CLI (FR-006).
  - Agent details: a Wake button and the result for an asleep agent.
  - No new visuals beyond the existing primitives (spec Design).

## Scope

**In:** D1 to D10, the contract `contracts/sleep-protocol.md`, README notes, unit, integration, and e2e tests, `qa-report.md`, and the Turkish HTML report.

**Out:** the spec's Out of scope list.

## Tasks

### T1 Agent module: status, setting, actions, sleep and wake steps (D1 to D5, D9 agent part)

- `Asleep`, `PutToSleep`, transitions; `IdleSleepMinutes`; `Sleep` and `Wake` actions with their text forms; `SleepGate` port; `SleepAgentHandler`; `SessionEnder::sleep`; `StopAgentHandler` for `Asleep`; `ResumeAgentHandler::wake`; presentation parsing for `agent.sleep` and `agent.wake`; `ORDERED_REQUESTS` and `STARTING_REQUESTS`.
- **Acceptance:** unit tests prove: each new transition and the refused ones (`Asleep + QuietPeriodElapsed`, `Working + PutToSleep`); the setting accepts 0, 5, 1440 and refuses 4, 1441, 2.5, text, and keeps the old value; a sleep needs `waiting_for_input`; a gate reason refuses without a record and without a stop; a sleep records one `sleep` entry with the detail and the office actor and ends the session; a stop failure records the failure; `begin` and `end` pair on every path; a stop of an asleep agent gives `stopped` without a terminal call; `wake` continues, starts new with the reason, refuses a non-asleep agent, and records one `wake` entry. `cargo test -p agent-domain -p agent-application -p agent-presentation` passes.

### T2 Hierarchy: asleep refusals (D8 module part)

- `DirectiveTarget::is_asleep`, `HierarchyError::Asleep`, the directive and report handlers, `SendDirectiveHandler::waiting_count`.
- **Acceptance:** unit tests: a directive and a report to an asleep receiver are refused with the asleep text and journaled in both records; a not-running receiver keeps the 0002 and 0015 texts. `cargo test -p hierarchy-application -p hierarchy-domain` passes.

### T3 Host: sleeper, gate, wake run (D6, D7, D8 bridge, D9 host part) — depends on T1, T2

- `IdleSleeper`, `IdleSleepGate`, `SleepingAgents`, `PersonInput` additions, the `ServerOptions` knobs, the connection refusal of `agent.sleep`, `wake.start`, the `Flow` of the watch, `wake.result`.
- **Acceptance:** `tests/idle_sleep.rs` with fake CLIs, `idle_sleep_minute` 100 ms, tick 50 ms, and a 2-second check period proves:
  - off by default: idle agents for 20 minute-units cause 0 sleeps (SC-001);
  - on: idle agents become `asleep` after the idle time, one `sleep` entry each with actor `office`, no notification and no Needs-you item, the session ended (SC-002, SC-006);
  - turning the setting on or changing it restarts the count (SC-004);
  - each blocking condition b to f alone keeps the agent awake (SC-003), and output or person input between checks prevents the stop;
  - a directive and a report to an asleep agent are refused with the asleep text (SC-009);
  - `wake.start` continues the same session, records one `wake` entry with actor `user`, and publishes `checking` then `continued` (SC-005);
  - a CLI that ends at once on every start is started at most twice, then held with one notification (SC-007);
  - `wake.start` on a non-asleep agent answers `conflict` "the agent is no longer asleep"; two parallel `wake.start` give one start;
  - an asleep agent stays `asleep` after a `server.restart`, nothing starts, `resume.plan` does not list it (SC-008, clean part); the SIGKILL part uses the binary as `server_crash.rs` does;
  - a user client gets `agent.sleep` refused; agent callers get `wake.*` refused.
  The whole office-harnessd test suite passes.

### T4 Desktop (D10) — depends on T3 for the contract only

- Module, header button, dialog, terminal view and details changes, label maps, mock-core.
- **Acceptance:**
  - Vitest covers the model (validation, texts) and the controller (results, pending state).
  - Playwright `e2e/idle-sleep.spec.ts` covers: the setting off by default, on with 30, refusal of 4 and 1441 with the old value kept; `asleep` in the list, the scene, and the terminal view with the asleep text and the last output; Enter in the focused terminal view wakes; other keys and a paste do not; Wake in the details; results `checking`, `continued`, `new_session`, `held`, and the no-longer-asleep text; keyboard only with visible focus and Escape; an axe scan of the dialog and the asleep terminal view with no violation.
  - `bun run check`, `lint`, `format:check`, `test`, and the Playwright suite pass.

### T5 Docs and evidence — depends on T1 to T4

- `contracts/sleep-protocol.md`; the 0001 client protocol and data model notes "Amended by 0018" where statuses, actions, and settings are listed; README section; `qa-report.md` (each FR and SC with its evidence); the spec Deviations if any. The Turkish HTML report goes to the shared spec-reports folder, with its index row.
- **Acceptance:** each FR and SC has a qa row with evidence. The no-other-project grep of the diff is clean.

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright.
- The repository's boundary check script.
- CI green on Linux, macOS, and Windows.
- Hands-on: a real `claude` agent put to sleep and woken with Enter, when the owner or the gate runs it. Otherwise the qa row stays BLOCKED, never PASS.
