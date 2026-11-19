---
status: draft
spec: specs/changes/0007-hold-and-limits/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (ACF-144). Status: draft until god approves the plan; Efe gates each PR at its head sha. -->

# Plan: hold switch and agent action limits (0007)

## Summary

This change gives the owner a brake on agent-made actions. One action holds the whole office, another holds one agent and its subtree, and four rate limits refuse an agent that acts too fast; an agent refused by a limit three times in 60 seconds is held automatically and the owner gets one notification. Terminals keep running, the owner's own actions are never held, holds survive restarts, and every hold, release, and refusal reaches the action record.

**What you get:**
- One gate in the agent-call gateway (`H/server/agent_call.rs`) that runs after the 0002 rules and before any use case: hold check first, then the rate limits, then the automatic hold (the A10 order).
- Holds stored by the `hierarchy` module (new migration `0002_holds.sql`), so they survive server and machine restarts; rate counts live in memory and start at 0 after a restart, as the spec's edge case says.
- Four new office settings for the limits, with defaults 10, 3, 5, 10 and the rule "a whole number of at least 1".
- `hold`, `release`, and the office as an actor in the action record; an `auto_hold` notification through the 0003 channel.
- In the desktop app: "Office on hold" as text with hold and release actions and a keyboard shortcut, "on hold" in agent rows and details, a limits dialog, and the office-level record entries.

**Why this approach:**
- The gateway already funnels every agent-made action through two functions (`answer_directive`, `answer_office_request`) and journals every refusal with the caller as actor (0002/FR-009, FR-011), so one gate there covers all five action kinds and the MODIFIED 0002/FR-009 without touching the use cases.
- The hold rule needs the tree (subtree of the caller and of the target, A2), and `hierarchy` already owns the tree snapshot and `check_descendant`, so the rule lives beside it as a pure domain function.
- Settings already have a typed key list with validation (`M/agent/domain/src/value_objects/settings.rs`) and the `settings.get`/`.set` requests, so the limits reuse that path through one bridge, without a cross-module dependency.

**What it will not do:** token, cost, or turn limits; stopping a CLI on hold; loop detection from terminal output; limits on the user's own actions; queueing refused actions; size caps on the tree (owner answer, 2026-11-19).

**Shape of the work:** 8 tasks in 2 phase PRs into one umbrella branch `feat/0007-hold-and-limits`, one local commit per task. **Effort:** M.

**Main risks:**
- A gate bug could refuse the owner's own actions. Mitigated: the gate lives only on the agent connection; the user client's dispatcher never calls it (PB-1), and an integration test sends the same five actions as the user while the office is on hold.
- The office has no actor or target today for an office-level record entry (`ActionRecord.target_agent_id` is required, `Actor` has only `User` and `Agent`). Mitigated: T2 adds `Actor::Office` and an optional target through an additive audit migration with an upgrade test.
- SC-005 asks for one key "from any panel view". While a terminal has focus, keys go to the CLI except the documented app shortcuts (0003's MODIFIED 0001/FR-017). See D9: the default keeps the terminal untouched and needs god's or the owner's yes or no.

**Decisions made for you:** the gate order, the hold table shape, in-memory rolling windows, the four setting keys, `Actor::Office`, the optional record target, the `hold.*` protocol, `Ctrl+Shift+H` as the hold shortcut outside the terminal, and the limits dialog.

**Blockers:** god approves this plan; D9 needs a yes or no before T7's shortcut step (every other task proceeds). The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0007: the owner holds and releases the office or one agent with one action, agents that act beyond a rate limit are refused with a reason that names the limit, three limit refusals in 60 seconds hold the agent and notify the owner once, holds survive restarts, the owner's own actions and typing are never held, and every hold, release, and refusal is in the action record.

## Context

- **Repository:** `main` at 2df9f59 (0001, 0002, 0003 merged; #62 agent rows with role and team). The spec is on PR #63 (`docs/0007-hold-limits-spec`), approved by the owner on 2026-11-19.
- **Existing seams** (read at 2df9f59 on 2026-11-19):
  - Agent gateway: `src/apps/office-harnessd/src/server/agent_call.rs`. `ALLOWED_REQUESTS` holds the six agent types; `answer_directive` parses then runs `send_directive` under `ActorScope::enter(Actor::Agent)`; `answer_office_request` runs the field allowlist, the self refusal, and the subtree check on `hierarchy_tree.store_snapshot()`, then `state.agents.run`. Every refusal calls `state.directive_journal.refused(caller, reason)` and answers `ErrorCode::Refused`.
  - Tree: `hierarchy_domain::{TreeRow, check_descendant, is_descendant_of}`; store interface `M/hierarchy/application/src/common/interfaces/hierarchy_store.rs`; migrations `M/hierarchy/infrastructure/src/persistence/migrations/` (one file, `0001_teams.sql`, no cross-module foreign keys).
  - Settings: `M/agent/domain/src/value_objects/settings.rs` (`Settings`, `SettingKey::ALL`, `RESERVED_SHORTCUTS`), handler `M/agent/application/src/commands/set_setting/`.
  - Audit: `M/audit/domain/src/action_kind.rs` (ten kinds, `Refused` included), `action_record.rs` (`target_agent_id: AgentId` required), `M/audit/infrastructure/src/persistence/action_repository.rs` (`"user"` and `"agent:<id>"` actor text).
  - Actor: `src/kernel/src/actor.rs` (`User`, `Agent(AgentId)`).
  - Notifications: `M/notification/domain/src/enums/notification_event.rs` (`Exited`, `Failed`, `WaitingForInput`, `Bell`); host bridge `H/bridges/notification_bridge.rs`.
  - Desktop: shortcuts `D/core/shortcuts/app-shortcuts.ts`; panel `D/modules/panel/components/agent-row.svelte`; details `D/modules/agent/components/agent-details.svelte`; record `D/modules/audit/components/action-record.svelte`; notifications `D/modules/notification/`; e2e mock core `src/apps/desktop/e2e/support/mock-core.ts`. The app has no settings screen today (README: the focus shortcut is set by `settings.set`).
  - Contracts: `specs/changes/0001-office-server-and-terminal/contracts/client-protocol.md`, `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md`, `specs/changes/0003-panel-practicality/contracts/panel-protocol.md`.
- **Not related:** the branch `feat/0007-c-ui` named in the dispatch belongs to another project's own spec 0007 (Ethem's booking work); it does not exist in this repository and nothing here aligns with it.
- **Floor rules:** one build or test at a time on the floor; ask god for the build slot (`CARGO_BUILD_JOBS=4`, `--test-threads=4`). Local lint needs the pinned toolchain (`$HOME/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin` first in `PATH`); workspace-wide runs fail locally on `libdbus-sys` (desktop crate), so local Rust checks run per crate (`-p office-harnessd -p hierarchy-* -p agent-* -p audit-* -p notification-* -p kernel`) and CI covers the workspace. Efe reviews each PR at its exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Placement:** the hold rule, the rate windows, and the automatic-hold counter live in the `hierarchy` module. Domain: `Hold { scope: HoldScope (Office | Agent(AgentId)), set_by: HoldSetter (User | Office), since: Timestamp, reason: Option<String> }`; `hold_refusal(holds, rows, caller, target) -> Option<HoldRefusal>` (pure); `RateLimits` (four values); `RateWindows` (rolling 60-second counts per agent and kind, plus per sender and target for directives, timestamps passed in); `AutoHoldTracker` (limit refusals per agent in 60 seconds, A5). Application: `SetHold`, `ReleaseHold`, `ListHolds`, `GuardAgentAction`. Infrastructure: the hold repository and migration.
  - Rejected: a new `guard` module. It would need the tree from `hierarchy` and add a crate quartet for about four types.
- **D2 Gate order and counting:** in `agent_call.rs`, after the 0002 refusals (allowlist, smuggled field, field allowlist, self, subtree, role) and before the use case: (1) office hold, (2) agent hold on the caller's ancestors-or-self or on the target's ancestors-or-self (A2, owner answer Q2), (3) the rate limits. An action that passes counts toward its windows at once, also when the use case later fails (a failed create still consumed a slot: the agent asked). Refused actions never count toward a rate window (A4). Hold refusals never count toward the automatic hold (A5). This is the A10 order: after the automatic hold, every later request is refused with the hold reason, as SC-001 expects. The check and the count run under one mutex in `GuardAgentAction`, so two concurrent requests cannot both take the last slot (edge case "exactly as many as the limit").
- **D3 Limit kinds:** `directive` (directives sent by one agent, default 10), `directive_target` (directives one agent sends to one target, default 3), `create` (agents created by one agent, 5), `lifecycle` (`agent.start`, `agent.stop`, `agent.new_task` by one agent, 10). A directive counts in both directive windows; the refusal names the first limit it exceeds. Reason text: `the limit "<name>" allows <n> per 60 seconds`.
- **D4 Settings:** four new `SettingKey`s, `limit_directives_per_agent`, `limit_directives_per_target`, `limit_creates_per_agent`, `limit_lifecycle_per_agent`, whole numbers from 1 to 1000; other values refused with "a limit is a whole number of at least 1" (edge case; the upper bound keeps a typo from turning the limit off). `GuardAgentAction` reads them through a hierarchy port `RateLimitSource`, bridged in `H/bridges/` to the agent settings query, on each action, so a change applies to the next action (FR-009).
- **D5 Storage:** migration `0002_holds.sql`: `hierarchy_holds(scope TEXT PRIMARY KEY, set_by TEXT NOT NULL, reason TEXT, since INTEGER NOT NULL)`, `scope` is `office` or the agent id. Writes go through the existing tree-write mutex. A failed write returns an error and the in-memory state does not change (edge case "cannot be stored"). Rate windows and the tracker are memory only (edge case "restart within a window").
- **D6 Record:** `Actor::Office` joins `kernel::Actor` (text `office`); `ActionKind` gains `Hold` and `Release`; `ActionRecord.target_agent_id` becomes `Option<AgentId>` (`None` for the office hold), through an audit migration that rebuilds the table with a nullable column and copies every row. Refusals keep `ActionKind::Refused` with the D3 or hold reason. `action.list` without an agent id returns the office-level entries.
- **D7 Notification:** `NotificationEvent::AutoHold`, created once when `GuardAgentAction` moves an agent from not held to held by the office (FR-008, SC-004); the desktop text is `<agent> was put on hold after 3 limit refusals`. It follows 0003's rules (unread until read) through MODIFIED 0003/FR-006, except that the focused-terminal exception (b) does not apply: it always sends the OS notification.
- **D8 Protocol:** user client requests `hold.set {agent_id?}`, `hold.release {agent_id?}` (no `agent_id` means the office), `hold.list` → `{office: hold|null, agents: [hold]}`, and event `hold.changed`. Refused codes: `conflict` for "already held" or "not held", `internal` for a failed store write. The agent connection never answers `hold.*` (not in `ALLOWED_REQUESTS`). T1 writes the contract.
- **D9 Hold shortcut (open, needs a yes or no):** `Ctrl+Shift+H` holds or releases the office while focus is in the panel outside a terminal; with a terminal focused, the user presses the focus shortcut first (2 keys). This is what the spec states (A9, SC-005) and keeps 0003's terminal passthrough unchanged. The alternative adds `Ctrl+Shift+H` to the terminal's app shortcuts so SC-005 holds from a focused terminal too; that changes 0003's MODIFIED 0001/FR-017 and needs a small 0003 amendment and a matching edit of A9 and SC-005. Recommended: the alternative, because a looping agent is exactly when the owner sits in a terminal. T7 builds the default and switches with one table row if the answer is yes.
- **D10 Delete:** when the user deletes agents (0001, 0002/FR-006), the host removes their holds through a bridge in the same request; at startup the server also drops holds of agents that no longer exist, so no hold remains for a missing agent.
- **D11 Desktop:** the panel header shows "Office on hold" as text with the release action, or a "Hold office" action; agent rows and details show "on hold" as text and hold or release actions; a "Limits" dialog edits the four values with their defaults and validation; the hold state changes are announced in the existing live region. No new visuals beyond these controls (spec Design rule).
- **Constitution check:** module boundaries hold (no new cross-module dependency; bridges in `H/bridges/`, `scripts/check-boundaries.sh` passes); no file in a CLI's configuration is touched (PB-3); every new action is keyboard-reachable with visible focus and text states (PB-4); no other project's name in the repository.

## Scope

**In:** FR-001 to FR-011, MODIFIED 0002/FR-009 and 0003/FR-006 (in part), PB-1 to PB-5, A9 to A12, SC-001 to SC-005, the spec's edge cases, the contract and README updates, a QA report.

**Out:** everything in the spec's Out of scope; templates, worktrees, and the needs-you queue (0008 to 0011); changing the 0002 tree rules.

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Server | Holds, limits, automatic hold, record, notification, protocol | T1 to T6 | `feat/0007-p1-server` (umbrella) | p1 → umbrella |
| 2 Desktop and ship | Panel controls, limits dialog, docs, QA | T7, T8 | `feat/0007-p2-desktop` (umbrella after p1) | p2 → umbrella, then umbrella → `main` |

The umbrella `feat/0007-hold-and-limits` is created from `main` after PR #63 merges. A single PR is the fallback if god prefers it; the tasks do not change.

## Tasks

Paths: `M/` = `src/modules/`, `D/` = `src/apps/desktop/src/`, `H/` = `src/apps/office-harnessd/src/`.

### T1: Design artifacts (no product code)

- **Files:** `specs/changes/0007-hold-and-limits/{data-model.md,contracts/hold-protocol.md}`.
- **Work:** (a) data model: the hold table, the audit migration (nullable target, `office` actor), the four setting keys, the notification event. (b) Contract: `hold.set`, `hold.release`, `hold.list`, `hold.changed`, the refusal reasons of D3 and D2 as agent-call answers, `action.list` without an agent id, the new setting keys.
- **Depends on:** none.
- **Acceptance:** every FR has its message or table named; the contract lists codes and shapes in the style of `hierarchy-protocol.md`; Efe can review it with the p1 PR.

### T2: Office actor, hold kinds, optional target (FR-010)

- **Files:** `src/kernel/src/actor.rs`, `M/audit/{domain,infrastructure}/`, every exhaustive `match` on `Actor` (compiler-led), `D/modules/audit/`.
- **Work:** (a) `Actor::Office`, text `office`, parse and print. (b) `ActionKind::{Hold, Release}`. (c) `target_agent_id: Option<AgentId>` and the audit migration that rebuilds the table and copies all rows. (d) `action.list` without an agent id returns office-level entries. (e) Desktop record labels `office`, `hold`, `release`.
- **Depends on:** T1.
- **Acceptance:** round-trip tests for the new actor and kinds; migration test: a database with 0001-0003 rows upgrades with every row intact and a `None` target insert works; `bun test` for the labels; fmt, clippy, per-crate tests, `scripts/check-boundaries.sh`.

### T3: Hold rule, rate windows, automatic-hold tracker (FR-002, FR-004, FR-007, FR-008 rules)

- **Files:** `M/hierarchy/domain/src/` (new `hold`, `rate_limits`, `rate_windows`, `auto_hold_tracker` files with sibling `.tests.rs`).
- **Work:** the D1 types and the pure functions of D2 and D3, with the clock passed in.
- **Depends on:** T1.
- **Acceptance:** unit tests: office hold refuses every kind; an agent hold refuses actions by the agent, by its descendants, on the agent, and on its descendants, including from its parent and the CEO (owner answer Q2); an agent joining the subtree later is covered (A2); exactly the limit passes and the next is refused (boundary); a window older than 60 seconds frees its slot; a directive counts in both directive windows; three limit refusals in 60 seconds trigger exactly one automatic hold and hold refusals do not count (A5); fmt, clippy, `cargo test -p hierarchy-domain`.

### T4: Hold store, commands, and the guard use case (FR-001, FR-003, FR-005, FR-006, FR-008, FR-009)

- **Files:** `M/hierarchy/application/src/commands/{set_hold,release_hold,guard_agent_action}/`, `M/hierarchy/application/src/queries/list_holds/`, `M/hierarchy/application/src/common/ports/` (`RateLimitSource`, `HoldNotifier`, the existing recorder port), `M/hierarchy/infrastructure/src/persistence/` (migration `0002_holds.sql`, repository).
- **Work:** (a) `SetHold` and `ReleaseHold` for the user: already held or not held answers `conflict` and changes nothing; a failed write keeps the prior state (D5); each success records `hold` or `release` with the user as actor. (b) `GuardAgentAction`: the D2 order under one mutex, reading limits from `RateLimitSource` per call, recording each refusal with the caller as actor, and on the third limit refusal in 60 seconds storing an automatic hold with `HoldSetter::Office`, recording it with `Actor::Office`, and calling `HoldNotifier` once; an agent that is already on hold gets no second hold and no second notification. (c) `ListHolds`, and pruning holds of agents not in the tree (D10).
- **Depends on:** T2, T3.
- **Acceptance:** application tests with in-memory fakes for every path above; repository test: holds survive closing and reopening the database (FR-006); a forced write failure leaves the previous state (edge case); the office release leaves an automatic agent hold in place (edge case); fmt, clippy, per-crate tests, boundaries.

### T5: Limit settings (FR-009, A3)

- **Files:** `M/agent/domain/src/value_objects/settings.rs` (+ tests), `M/agent/application/src/commands/set_setting/`, `H/bridges/` (new `rate_limit_source.rs`).
- **Work:** the four D4 keys with defaults 10, 3, 5, 10 and the 1 to 1000 rule; the bridge implementing `RateLimitSource`; `Ctrl+Shift+H` added to `RESERVED_SHORTCUTS` so the focus shortcut cannot take it.
- **Depends on:** T4.
- **Acceptance:** settings tests: defaults, 0, negative, text, 1001 refused with the edge-case message, 1 and 1000 accepted; a changed value applies to the next guard call (bridge test); the focus shortcut refuses `Ctrl+Shift+H`; fmt, clippy, tests.

### T6: Gateway gate, protocol, notification, delete hook (FR-002, FR-004, FR-005, FR-007, FR-008, FR-010, MODIFIED 0002/FR-009, SC-001 to SC-004)

- **Files:** `H/server/agent_call.rs` (+ tests), `H/server/hierarchy_requests.rs` or a new `hold_requests.rs`, `H/server/router.rs`, `H/server/event_hub.rs`, `H/bootstrap/{hierarchy_services.rs,migrations.rs}`, `H/bridges/{notification_bridge.rs,hold_notifier.rs}`, the user `agent.delete` path in `H/server/agent_requests.rs`, `M/notification/domain/src/enums/notification_event.rs`, `H/tests/`.
- **Work:** (a) call `GuardAgentAction` in `answer_directive` after parsing and in `answer_office_request` after the subtree and role checks, before the use case; answer `refused` with the reason. (b) Wire `hold.*` requests and the `hold.changed` event for user clients only. (c) `NotificationEvent::AutoHold` through the notification bridge. (d) Remove holds of deleted agents in the delete request; prune at startup (D10). (e) Run the hold migration in bootstrap.
- **Depends on:** T4, T5.
- **Acceptance:** integration tests through a real agent-call connection: 100 directives in 10 seconds deliver at most 10 and record every refusal with the agent as actor, the limit as reason before the automatic hold and the hold as reason after it (SC-001); a request that breaks both a hold and a limit names the hold (A10); the auto-hold notification is sent while that agent's terminal is focused (MODIFIED 0003/FR-006); with the office on hold every agent-made kind is refused while user typing reaches the terminal and the user's own create, start, stop, delete, and new task succeed (SC-002, PB-1, FR-005); after a server restart with the office on hold, 0 agent-made actions succeed (SC-003); a looping agent gets exactly 1 `auto_hold` notification and is held after the third refusal (SC-004); an agent hold refuses an action of the CEO on the held agent (Q2); deleting a held agent leaves no hold row; an agent connection asking for `hold.set` is refused as not allowed; fmt, clippy, per-crate tests, boundaries.

### T7: Desktop controls (FR-001, FR-003, FR-009, FR-011, PB-4, SC-005)

- **Files:** `D/modules/hierarchy/` or a new `D/modules/hold/` (state, service, components), `D/modules/panel/components/agent-row.svelte`, `D/modules/agent/components/agent-details.svelte`, `D/modules/notification/` (auto-hold text), `D/core/shortcuts/app-shortcuts.ts`, `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/`.
- **Work:** the D11 controls; `hold.list` on connect and `hold.changed` updates; the limits dialog with validation messages; office-level record entries in the limits dialog; the D9 shortcut (default or alternative, per the answer).
- **Depends on:** T6, and the D9 answer for the shortcut step only.
- **Acceptance:** `bun test` for state and service; Playwright with axe on the mock core: hold and release the office and an agent by keyboard, "Office on hold" and "on hold" as text, already-held message, invalid limit refused with the message, the auto-hold notification text, 1 key press holds the office from the panel (SC-005 per D9); `svelte-check`, `eslint`, `prettier --check`.

### T8: Docs and QA (all criteria)

- **Files:** `README.md` (hold, release, limits, shortcut), `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md` (cross-reference to the new refusal reasons), `specs/changes/0007-hold-and-limits/qa-report.md`.
- **Work:** document the user flow and the limits; record the QA rows for SC-001 to SC-005 and the edge cases with evidence; hands-on rows (screen reader pass, 200 percent zoom, a real CLI looping on directives) recorded as BLOCKED until a person runs them, never PASS.
- **Depends on:** T7.
- **Acceptance:** every Traceability row has evidence or a recorded gap; `rg` finds no other project's name.

## Dependencies

T1 → T2 → T4; T1 → T3 → T4; T4 → T5 → T6 → T7 → T8.

- **Parallel:** T2 and T3 touch different crates and can run together.
- **Exclusive:** `Cargo.lock`, the migration ledgers, `src/shared/wire`, and the e2e mock core belong to one running task at a time.
- **Gate:** PR #63 merged and this plan approved by god before T1 code lands on the umbrella.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged). After each commit, `git status --porcelain` lists only untracked sandbox files.

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0007-hold-and-limits/{data-model.md,contracts/}` | `docs(spec-0007): add the hold data model and protocol` |
| 2 | T2 | `src/kernel/`, `M/audit/`, `D/modules/audit/`, compiler-led `match` sites | `feat(audit): record holds with the office as an actor` |
| 3 | T3 | `M/hierarchy/domain/` | `feat(hierarchy): add the hold rule and rate windows` |
| 4 | T4 | `M/hierarchy/application/`, `M/hierarchy/infrastructure/` | `feat(hierarchy): store holds and guard agent actions` |
| 5 | T5 | `M/agent/`, `H/bridges/` | `feat(agent): add the rate limit settings` |
| 6 | T6 | `H/`, `M/notification/` | `feat(server): gate agent calls on holds and limits` |
| 7 | T7 | `D/`, `src/apps/desktop/e2e/` | `feat(desktop): hold the office or an agent and edit limits` |
| 8 | T8 | `README.md`, `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md`, `specs/changes/0007-hold-and-limits/qa-report.md` | `docs(spec-0007): document holds and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **Per phase PR:** CI green on the slim matrix (ubuntu all steps; macOS and Windows Rust tests), Efe's review at the head sha.
- **Security pass:** `secure-engineering` checks on the gate: the agent connection cannot reach `hold.*` or `settings.*`; refusal text stays truncated (`MAX_REFUSAL_TEXT_CHARS`); the user path never calls the gate.
- **Real surface:** a real CLI that loops on `agent-call directive.send`, the screen reader pass, and the 200 percent zoom check are hands-on rows in the QA report.
- **Rollback:** each phase PR is reverted in the umbrella. The hold table is additive; the audit migration copies every row, so the old code reads the rebuilt table except office-level rows (`target_agent_id` NULL), which only this change writes. With no hold set and default limits, a normal team's actions pass as before.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-005 and the edge cases.
2. **Architecture:** `scripts/check-boundaries.sh` passes; no module crate depends on another.
3. **Code quality:** lints clean; `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office created before this change opens after the upgrade with its record intact (T2 test, CI on three OSes).
5. **Scope fidelity:** nothing from Out is in the diff; the user's own actions pass with the office on hold (T6 test).
6. **Readiness:** `/ac:verify` passes on the umbrella head before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T4, T6, T7 | T4(a) office `SetHold`/`ReleaseHold`; T6(b) `hold.*`; T7 one action each |
| FR-002 | T3, T6 | T3 office hold refuses every kind; T6 SC-002 integration test |
| FR-003 | T4, T7 | T4(a) agent hold; T7 details actions |
| FR-004 | T3, T6 | T3 subtree and outside-actor cases; T6 CEO-on-held-agent test |
| FR-005 | T6 | SC-002 test: typing and user actions pass while held |
| FR-006 | T4, T6 | T4 reopen test; T6 SC-003 restart test |
| FR-007 | T3, T6 | T3 boundary tests; T6 SC-001 reasons name the limit |
| FR-008 | T3, T4, T6 | T3 tracker; T4(b) automatic hold and one notify; T6 SC-004 |
| FR-009 | T5, T7 | T5 validation and next-action bridge test; T7 limits dialog |
| FR-010 | T2, T4, T6 | T2 actor and kinds; T4 records per path; T6 record assertions |
| FR-011 | T7 | Playwright text states |
| 0002/FR-009 (modified) | T6 | hold and limit refusals answered, told, and recorded |
| 0003/FR-006 (modified, in part) | T6, T7 | T6(c) `AutoHold` event and the focused-terminal test; T7 notification text |
| PB-1 | T6 | user actions with the office on hold |
| PB-2 | T6 | no CLI stops or terminal closes on hold (process still running) |
| PB-3 | T6, T8 | no CLI configuration path written (existing guard tests still pass) |
| PB-4 | T7 | keyboard Playwright runs with axe |
| PB-5 | T3, T4 | tree rules unchanged; 0002 tests still pass |
| SC-001 | T6, T8 | 100-directive integration run |
| SC-002 | T6, T8 | office-hold integration run |
| SC-003 | T6, T8 | restart integration run |
| SC-004 | T6, T8 | looping-agent run, one notification |
| SC-005 | T7, T8 | Playwright 1-key run per D9 |
| US1, US2, US3 | T3 to T7 | the T6 integration runs and T7 Playwright runs |
