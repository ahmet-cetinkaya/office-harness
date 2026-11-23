---
status: completed
spec: specs/changes/archive/0013-bulk-resume/SPEC.md
commits: per-task
---

# Plan: Resume all interrupted agents (0013)

## Summary

After a server restart, the panel header offers "Resume interrupted (N)". A dialog lists the interrupted agents in start order, members first, with what will happen to each. On confirm, the server continues each checked agent's last session, or starts a new session when the CLI cannot continue. The server then watches each start for 20 seconds. A continued session that ends at once gets one new session, recorded with the office as actor. A new session that also ends at once puts the agent on a 0007 hold with one notification. The dialog shows each result live.

**What you get:**
- `agent.resume` in the agent module: one atomic "still interrupted? then continue or start new" step, and the fallback step "the refused session is still the last one and ended by itself? then start new".
- `resume.plan`, `resume.start`, and the `resume.result` event in a new host gateway, which orders the agents, watches each start, and runs the fallback and the hold.
- `HoldService::hold_by_office` and a `resume_hold` notification event.
- An optional `detail` on action record entries, so each new session that replaces a continue keeps its reason in the agent's record (FR-007).
- A session end result `lost` for sessions that an unclean stop left open, so the dialog can warn before Confirm that CLI processes from before may still run (FR-008).
- The header action, the dialog, and its results in the desktop app.

**Why this approach:**
- The fallback must work when the desktop app is closed (0001/FR-019), so the server runs it, not the app.
- Every start goes through `AgentGateway::run`, as the agent-call gateway's starts do. Each start keeps the status order, the stopping refusal, the events, and the action record without new code.
- The "still interrupted" check runs inside the agent module under the status order, in the same step as the start. A user's start between the check and the start cannot happen.
- The watcher listens to the `agent.status` events that the status bridge publishes after it stored an exit. It needs no polling and no new port, and it never acts before the bridge has ended the session.

**What it will not do:** sweep orphan CLIs, read CLI output, retry more than once, resume `exited`, `failed`, or `stopped` agents, or start anything at server start (spec Out of scope).

**Shape of the work:** 5 tasks in one PR (`feat/0013-bulk-resume`), one commit per task. **Effort:** S to M.

**Main risks:**
- **A user action races the fallback.** The fallback step checks, under the status order, that the refused session is still the agent's last session and that the agent is `exited` or `failed`. A stop (`stopped`), a new task, or a continue changes one of these, so the office does nothing (US2 scenario 4).
- **A start loop.** Each agent gets at most two starts per `resume.start` (D4). The second failure holds the agent.
- **Slow starts.** Each start resolves the login environment (up to 5 s). The request answers after it has queued the starts, not after the starts, so the dialog never waits on 30 login shells (D3).

**Blockers:** none.

## Goal

Deliver spec 0013: one confirmed action resumes the checked interrupted agents in tree order, members first. A refused continue falls back once to a new session. An agent that keeps failing goes on hold with one notification. The results show live, and nothing starts without the user.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 6e3d197 on 2026-11-23.

- **Starts:** `ContinueSessionHandler` (`M/agent/application/src/commands/continue_session/continue_session_handler.rs`) checks the continuation with `continuation_of` before it ends a running session, then launches with `LaunchKind::Continue`. `StartAgentHandler::handle` (`M/agent/application/src/commands/start_agent/start_agent_handler.rs:40`) launches `LaunchKind::NewTask`. `SessionStore::latest_for_agent` (`M/agent/application/src/common/interfaces/session_store.rs:27`). `StartOutcome { agent, session, failure }` (`M/agent/application/src/common/session_launcher.rs:39`): a command that cannot start gives status `failed` and a failure text, not an error.
- **Action record:** `ActionJournal::record` takes the actor from `ActorScope::current()` (`M/agent/application/src/common/actor_scope.rs`); `Actor::Office` exists (`src/kernel/src/actor.rs`).
- **Ordering:** `AgentGateway::run` (`H/server/agent_requests.rs:142`) takes `status_order` for `ORDERED_REQUESTS` and refuses `STARTING_REQUESTS` while the server stops. Requests reach gateways by prefix in `H/server/connection.rs:217-251`.
- **Exits:** `AgentStatusBridge::apply` (`H/bridges/agent_status_bridge.rs:69`) holds the status order, ends the session and changes the status on `Exited`, and then publishes `agent.status` on the `EventHub` (`H/server/event_hub.rs:34`, `subscribe`).
- **Tree:** `TreeQuery::row_places` (`M/hierarchy/application/src/queries/tree/tree_query.rs:71`) gives each agent's `role` (CEO, lead, member) or none.
- **Holds:** `HoldService` (`M/hierarchy/application/src/commands/hold/hold_service.rs`) stores office holds in `store_automatic_hold` and notifies through `HoldNotifier::auto_held` (`M/hierarchy/application/src/common/ports/hold_ports.rs`), which `HoldNotifierBridge` (`H/bridges/hold_bridges.rs:123`) turns into an `auto_hold` notification and a `hold.changed` event.
- **Notifications:** `NotificationEvent` and `AttentionKind` (`M/notification/domain/src/enums/notification_event.rs`, `M/notification/domain/src/value_objects/attention_event.rs`) list the events; `NotificationBridge::record_auto_hold` (`H/bridges/notification_bridge.rs:173`); the queue clears hold items on release (`M/notification/application/src/commands/needs_you/clear_reason.rs:22`). Desktop text: `D/modules/notification/models/notification.ts:32`, `D/modules/needs-you/models/needs-you.ts`.
- **Desktop:** header actions in `D/modules/panel/views/panel-view.svelte:390-420`; services follow `D/modules/hold/services/hold-service.ts`; e2e uses `src/apps/desktop/e2e/support/mock-core.ts`.
- **Tests:** in-process server tests use `Office::open` (`H/../tests/support/office_client.rs`) with fake CLIs; `ServerOptions` (`H/server/server_options.rs`) holds test knobs.
- **Research R1:** `H/../tests/server_crash.rs` (in this PR) shows orphan survival and a working continue after a SIGKILL.

## Decisions

- **D1 One-agent step in the agent module.** New command `ResumeAgentHandler` (`M/agent/application/src/commands/resume_agent/`), reached by the request `agent.resume`, added to `ORDERED_REQUESTS` and `STARTING_REQUESTS`.
  - Without `after_session`: the agent must be `interrupted`, else `conflict` "the agent is no longer interrupted". The last session decides: when `continuation_of` accepts it, launch `Continue` and record `continue`; otherwise launch `NewTask`, record `start`, and answer the refusal text as `reason`. Answer: `{session, continued, reason?}`.
  - With `after_session`: the agent's last session must be that id and the agent `exited` or `failed`, else `conflict` "the agent changed after the refused session". Then launch `NewTask` and record `start`. The caller sets the actor; the host sets `Actor::Office` (FR-004).
  - Unknown agent: `not_found`. A command that cannot start answers success with the agent `failed` and a `failure` text, as `agent.start` does.
- **D2 Host gateway.** `ResumeGateway` (`H/server/resume_requests.rs`) answers the `resume.` prefix for user clients only. The agent-call allowlist does not name it (A10).
  - `resume.plan` (no fields): the `interrupted` agents of `agent.list`, ordered by D5. For each, the last session from `session.list` (id, `started_at`, `can_continue`, `reason`) and `held` from the hold list.
  - `resume.start {agent_ids}`: a non-empty list of distinct agent ids, at most 1000, refused with `invalid` otherwise. Unknown fields are refused, as in the hold protocol. The ids are ordered by D5. It answers `{order: [agent_id]}` at once and runs the starts on one background task.
- **D3 Background run.** One tokio task per `resume.start`. It starts the agents one after another: each `agent.resume` runs on `spawn_blocking` through `AgentGateway::run`, and the task does not wait for a check period before the next start (A7). Each started agent gets its own watch task. A `conflict` or `not_found` answer gives `skipped` with the message. A server that stops refuses the start (`refused`), which also gives `skipped`.
- **D4 Watch and fallback.** The watch subscribes to the `EventHub` before the start and waits, within the check period (`ServerOptions::resume_check_period`, default 20 s, A2), for an `agent.status` event of the agent. The bridge publishes that event only after it has stored the exit, so the fallback step always sees the ended session.
  - No `exited` or `failed` status in the period: final `continued` or `new_session`.
  - `stopped` or `interrupted` (a user stop, or a server stop): final, with the last result; no fallback (US2 scenario 4).
  - `exited` or `failed` after a continue, or a continue that answered `failed`: one `agent.resume` with `after_session` under `ActorScope::enter(Actor::Office)`. Success gives `new_session`, with the reason "the CLI ended N s after the continue" or the failure text, and a second watch. A `conflict` (the user started, continued, or began a new task meanwhile) gives the last result, final, and no fallback.
  - `exited` or `failed` of a new session that this run started, or a new session that answered `failed`: the hold (D6), final `held`. Never a third start.
- **D5 Start order** (FR-003, A6). Sort key from `row_places`: member or no role = 0, lead = 1, CEO = 2. The sort is stable, so within a key the order is the `agent.list` order. A unit test covers it.
- **D6 Office hold for a failed resume.** `HoldService::hold_by_office(agent, reason) -> Result<Option<Hold>, _>`: under the lock, `None` when the agent is already held (A9); otherwise it stores `Hold { set_by: Office, reason: Some(reason) }`, journals `held`, and calls the new `HoldNotifier::resume_held(agent, reason)`. `HoldNotifierBridge::resume_held` records a `resume_hold` notification with the reason as detail and publishes `hold.changed`. A store failure gives final `held` with "the hold could not be stored: ..." and the storage alert (edge case).
- **D7 Notification event `resume_hold`.** Added to `NotificationEvent`, `AttentionKind`, the Needs-you kinds that wait for the user, and the hold-release clear reason, all next to `auto_hold`. Desktop text: "was put on hold after its resume failed: <detail>". The notification table stores the event as text, so no migration is needed (checked in T2).
- **D8 Event `resume.result`.** `{type, agent_id, result, reason?, is_final}` with `result` one of `checking`, `continued`, `new_session`, `held`, `skipped`. `checking` carries what was started (`continued` true or false). The event goes to every user client. The dialog shows the results of its own request and ignores ids that it did not send.
- **D10 Reason in the action record** (FR-004, FR-007). Migration `0003_detail.sql` in `M/audit/infrastructure/src/persistence/migrations/` adds a nullable `detail TEXT` column to `audit_actions` (forward only, as the earlier audit migrations). `AgentActionEntry` gains `detail: Option<String>`. `ActionJournal::record_detailed(agent, action, failure, detail)` writes it, and `record` passes `None`. The audit presentation writes `detail` (null when absent) into entries and `action.recorded`.
  - `ResumeAgentHandler` builds the text itself from the sessions, so the host passes no free text. A new session that the CLI's lack of support caused reads "new session: <the continue refusal text>". A fallback reads "new session: the CLI ended <n> s after continuing the session of <start time>, exit code <c>", or "... did not start: <failure>".
  - Desktop: `action-record.svelte` shows the detail after the result text.
- **D11 Unclean stop mark** (FR-008, research R1 and R3). `ExitResult::Lost`, stored as `lost`, means the session was still open when the server stopped uncleanly. `SessionRepository::mark_running_as_interrupted`, which runs only at server start (FR-004), writes `lost` instead of `interrupted`. A clean stop keeps writing `interrupted` through `InterruptAgentsHandler`. `continuation_of` treats `lost` like `interrupted`, since it special-cases only a spawn error. `resume.plan` sets `after_unclean_stop: true` on an agent whose last session is `lost`. The session list in the agent details shows `lost` as "lost: the server stopped uncleanly".
- **D9 Desktop.**
  - `D/modules/resume/`: service (`resume.plan`, `resume.start`), model (result text), and a controller that keeps the results from `resume.result`.
  - Header button "Resume interrupted (N)" from the agent list, shown only for N > 0 (FR-001).
  - `resume-dialog.svelte`: loading, the check list in plan order (name, role, plan text, "on hold", "was running at an unclean stop"), a warning above Confirm when any listed agent has `after_unclean_stop` (linked to Confirm with `aria-describedby`) with an acknowledgement check box that Confirm waits for while a marked agent is checked, Confirm disabled with "Check at least one agent" when nothing is checked, then the results list in a `role="status"` region. Escape and Close keep the server running. It reuses the existing dialog primitive and button styles; no new visuals.

## Scope

**In:** D1 to D9, the contract `contracts/resume-protocol.md`, README notes, unit, integration, and e2e tests, `qa-report.md`, and the Turkish HTML report.

**Out:** the spec's Out of scope list.

## Tasks

### T1 Agent module and record detail: `agent.resume` (D1, D10)

- `ResumeAgentHandler` with `resume(agent_id)` and `fall_back(agent_id, after_session)`; presentation parsing for `agent_id` and the optional `after_session`; wiring in `H/bootstrap/agent_services.rs`; `ORDERED_REQUESTS` and `STARTING_REQUESTS`.
- The audit `detail` column, entry field, journal method, and JSON field (D10).
- `ExitResult::Lost` and the start-up recovery change (D11).
- **Acceptance:**
  - Unit tests prove each D1 branch: continue, new session when the CLI cannot continue, `conflict` when not interrupted, fallback only after an end by itself, `conflict` after a stop or a newer session, and the actor of the record.
  - Each new session of D1 has its D10 detail; a continue and every other action have none.
  - The migration test counts are updated.
  - A session open at server start ends as `lost`; a clean stop ends it as `interrupted`; a `lost` session can be continued.
  - The tests that expected `interrupted` after a restart (`H/../tests/server_restart.rs`, `M/agent/infrastructure/tests/persistence.rs`) are split into clean stop = `interrupted` and unclean stop = `lost`.
  - `cargo test -p agent-application -p agent-presentation -p audit-infrastructure -p audit-presentation` passes.

### T2 Office hold and `resume_hold` notification (D6, D7)

- `hold_by_office`, `HoldNotifier::resume_held`, the bridge, the notification event, attention kind, Needs-you kinds and clear reason, and the desktop text for both lists.
- **Acceptance:** unit tests: a hold is stored with `set_by: Office` and the reason; an agent already held gets no second hold and no notification; releasing clears the Needs-you item. Rust and `bun run test` pass for the touched modules.

### T3 Host gateway and watcher (D2 to D5, D8) — depends on T1, T2

- `ResumeGateway`, routing in `connection.rs`, `ServerOptions::resume_check_period`, the background run and the watches.
- **Acceptance:** `tests/bulk_resume.rs` with fake CLIs and a 2-second check period proves:
  - continue, and a new session for a CLI that cannot continue (US1, US2 scenario 2);
  - a CLI that ends at once on `--resume` gets one new session recorded with actor `office` (SC-002);
  - a CLI that ends at once every time is started twice, held, and notified once (SC-003);
  - start order in a CEO, lead, and members tree (SC-005);
  - `resume.plan` marks `after_unclean_stop` after a SIGKILL of the server and not after a clean `server.restart`, using the binary as `server_crash.rs` does (SC-006);
  - `skipped` for an agent no longer interrupted;
  - no fallback after a user stop inside the period;
  - agent callers get `resume.*` refused.
  The existing `server_process.rs` test still shows that nothing starts at server start (SC-004). The whole office-harnessd test suite passes.

### T4 Desktop (D9) — depends on T3 for the contract only

- Module, header button, dialog, wiring in `panel-view.svelte`, the action record detail (D10), and the mock-core requests and event.
- **Acceptance:**
  - Vitest covers the model and the controller.
  - Playwright `e2e/resume.spec.ts` covers: the button count and its absence; uncheck and confirm; live results, including `new_session` and `held`; the reason in the agent's action record; the unclean-stop marks, the warning, and Confirm unavailable until the acknowledgement, and no warning without marks; Confirm disabled with nothing checked; keyboard only, with Escape; an axe scan of the dialog with no violation.
  - `bun run check`, `lint`, `format:check`, `test`, and the Playwright suite pass.

### T5 Docs and evidence — depends on T1 to T4

- `contracts/resume-protocol.md` final; the 0001 data model and `contracts/client-protocol.md` amended ("Amended by 0013") where session end results are listed; README section, `qa-report.md` (each SC and FR with its evidence), and the spec Deviations if any. The Turkish HTML report goes to the shared spec-reports folder and is added to its index.
- **Acceptance:** each FR and SC has a qa row with evidence. The no-other-project grep of the diff is clean.

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright.
- The repository's boundary check script.
- CI green on Linux, macOS, and Windows.
- Hands-on: a real `claude` agent resumed after a SIGKILL of the server, when the owner or the gate runs it. Otherwise the qa row stays BLOCKED, never PASS.
