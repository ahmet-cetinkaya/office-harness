---
status: draft
spec: specs/changes/0014-subtree-status/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-23 with the spec, for one review of both. Code starts only after the spec review gate passes. -->

# Plan: A parent reads the state of its descendants (0014)

## Summary

A running agent gets one new call, `agent.subtree`, on the connection it already uses for directives. The answer lists its descendants in tree order with name, parent, role, status, last output time, a Needs you flag, a hold flag, and the task line. It never carries terminal content. The read takes no fields, so it cannot point anywhere but at the caller's own subtree.

**What you get:**
- `agent.subtree` on the agent connection; the allow list grows from 6 to 7 types.
- A tree-order walk of the descendants in the hierarchy domain.
- A "quiet for" reading on the terminal runtime, which gives the last output time without any output.
- An entry builder that writes each field by name, so a later field on the agent object cannot leak.
- Integration tests through the real helper for SC-001 to SC-007, FR-004, FR-005, and PB-3.

**Why this approach:** every input already exists on the server: the tree snapshot (`TreeQuery::store_snapshot`), the agents (`list_agents`), the task text (0011), Needs you (`NeedsYouUseCases::list`), and the holds (`HoldService::list` with `hold_refusal`). Only the last output time is new, and the terminal session already keeps it as `last_output: Instant` (`M/terminal/infrastructure/src/services/terminal_session.rs:68`).

**What it will not do:** the upward report channel, events to agents, terminal content, a rate limit for reads, any desktop change (spec Out of scope).

**Shape of the work:** 5 tasks, one PR, one commit per task. **Effort:** S.

**Main risks:**
- **Leaking terminal content.** The builder copies named fields only and never calls `agent_json` (which carries `last_line` after the gateway fills it). A test prints a marker in every descendant and checks every answer (SC-002).
- **Reading outside the subtree.** The walk starts at the caller on one snapshot and the request takes no fields; a test reads from all 9 agents of a two-branch tree (SC-001) and tries to name a sibling (SC-006).

**Decisions made for you:** the request name, the wire shape, the walk order, the task line cut, the quiet reading, and that reads skip holds, limits, and the record (spec A2, A3).

**Blockers:** none. The spec review gate passes first. The build slot rule applies to every build or test run.

## Goal

Deliver spec 0014: a running agent reads the state of its own descendants, and nothing else, with no terminal content.

## Context

Paths: `H` = `src/apps/office-harnessd`, `M` = `src/modules`. Read on `main` at 6e3d197.

- **Agent connection:** `H/src/server/agent_call.rs`. `ALLOWED_REQUESTS` (line 27) has 6 types; `answer_request` (line 86) dispatches them; `NOT_ALLOWED` (line 37) is the refusal text; `journal_refusal` records a refusal with the caller as actor and publishes it; `smuggled_field` refuses `actor` and `caller` on every allowed type; `truncated` cuts at a character count and adds `…`.
- **Gateway:** `H/src/server/agent_requests.rs`. `AgentGateway` holds the agent handlers (`use_cases().list_agents`), the `runtime: Arc<dyn TerminalRuntime>`, the tree query `hierarchy: Arc<TreeQuery>`, and `needs_you: NeedsYouBridge` (which implements `NeedsYouUseCases::list`). `ServerState.hold_service` is the `HoldService`.
- **Tree:** `M/hierarchy/domain/src/services/tree_rules.rs`. `TreeRow {agent_id, name, role, parent_agent_id, team_id}`; `descendants_of` is deepest-first (for stop and delete), so the read needs its own parent-first walk.
- **Holds:** `M/hierarchy/domain/src/services/hold.rs`, `hold_refusal(holds, rows, caller, None)` is the 0007/FR-004 coverage; `HoldService::list()` returns `HoldsView {office, agents}`.
- **Task text:** `agent_domain::value_objects::TaskText` with `text()` and `set_at()` (0011).
- **Terminal:** `TerminalRuntime` (`M/terminal/application/src/common/interfaces`) is implemented by `PortableTerminalRuntime` and by five test fakes (`H/src/bridges/typed_input.tests.rs`, `agent_status_bridge.tests.rs`, `notification_bridge.tests.rs`, `directive_bridges.tests.rs`, `M/terminal/presentation/src/handlers/terminal_handlers.tests.rs`).
- **Clock:** `H/src/bridges/system_clock.rs`, `SystemClock` implements the agent `Clock` (`now() -> Timestamp`).
- **Tests:** `H/tests/agent_directive.rs` shows the pattern: fake CLIs that dump their environment, `place()` through `PlaceAgentHandler`, `call_helper()` running the real binary with a credential, `actions_of()`.
- **Docs:** `README.md` "Calling the office from an agent" (line 262 on) still says only `agent.whoami` is answered; the 0002 contract lists the allowed types; `H/tests/agent_hold.rs:500` checks the start of `NOT_ALLOWED`.

## Decisions

- **D1 Request:** `agent.subtree`, agent connection only, no fields. On the user connection the type reaches the agent handlers as an unknown `agent.*` type and is `invalid`, as today.
- **D2 Walk:** a new `descendants_in_tree_order(tree, agent_id) -> Vec<&TreeRow>` beside `descendants_of`: depth first, parent before children, children in row order, each agent once (a hand-made cycle ends the walk), the agent itself excluded.
- **D3 Last output time:** `TerminalRuntime::quiet_for(agent_id) -> Option<Duration>`, the time since the running child's last output, or since its start when it wrote nothing, `None` when it does not run. No default body: every runtime answers; the five fakes return `None`. The gateway turns it into Unix ms with `SystemClock`: `now - quiet`.
- **D4 Entry builder:** `H/src/server/subtree_status.rs` with a pure `subtree_entries(rows, caller, agents, waiting, holds, last_output_at) -> Vec<Value>` and `task_line(text) -> String`. Each entry is a `json!` of the nine contract fields, nothing copied from `agent_json`. Descendants that the tree names but the agent store does not hold are skipped (a delete between the two reads).
- **D5 Task line:** the text up to the first `\r` or `\n`, cut by `truncated` at 80 characters; `…` is added also when lines follow, so a shortened text always shows it (0011/FR-001). Characters are Unicode scalar values (spec A9).
- **D6 Gateway method:** `AgentGateway::subtree_status(&self, caller, holds: &[Hold]) -> Result<Value, String>` reads the snapshot, the agents, and the Needs you list, then calls the builder. `answer_subtree` in `agent_call.rs` refuses any field (`invalid`, recorded), takes `state.hold_service.list()`, runs the method on the blocking pool, and answers `internal` with a recorded refusal on a read failure. A success writes no record entry and publishes no event.
- **D7 Holds and limits:** the read never calls `guard_office_request` (spec A2).
- **D8 Refusal text:** `NOT_ALLOWED` becomes "an agent may only create, start, stop, give new tasks to, send directives to, and read the state of its own descendants"; the `agent_hold.rs` check keeps its prefix.
- **D9 Constitution check:** AGPL header on new files; no new dependency; module boundaries kept (the builder lives in the server app, which already joins the modules); no code from any other project.

## Scope

- **In:** D1 to D8, tests, the 0002 contract amendment, README, `qa-report.md`, the spec report page.
- **Out:** the spec's Out of scope list; the desktop app.

## Tasks

### T1: Spec, contract, plan (no product code)

SPEC.md, `checklists/requirements.md`, `contracts/subtree-protocol.md`, this plan. **Accept:** the spec review gate passes; no product file changes.

### T2: Tree walk and quiet reading (D2, D3)

`descendants_in_tree_order` with unit tests (two branches, row order kept, caller excluded, a cycle ends). `quiet_for` on the trait, `TerminalSession::quiet_for`, `PortableTerminalRuntime`, five fakes; a runtime test that the reading grows while the child is silent, drops after output, and is `None` after exit. **Accept:** `cargo test -p hierarchy-domain -p terminal-infrastructure` and the office-harnessd unit tests pass; clippy clean.

### T3: Entry builder and gateway (D1, D4 to D8; FR-001 to FR-005)

`subtree_status.rs` with unit tests: exact field set per entry (no `last_line`, `command`, `working_folder`, `machine_id`, `worktree`, `additions`), parent-first order, `held` through an ancestor and through the office hold, `needs_you`, `task` null and set, `task_line` at 80 and 81 characters, with a second line, with `\r\n`. `AgentGateway::subtree_status`, `answer_subtree`, the allow list, `NOT_ALLOWED`. **Accept:** unit tests pass; `ALLOWED_REQUESTS` has 7 entries.

### T4: Integration tests (SC-001 to SC-007, FR-004, FR-005, PB-3)

New `H/tests/agent_subtree_status.rs` with the real server and helper:
- SC-001: CEO, 2 leads, 6 members, all running, placed parent first (the CEO before its leads, so every lead stores the CEO as parent; spec A10); each of the 9 reads; each answer equals its descendant set exactly.
- SC-002: every member prints `marker-7Q`; no answer contains it, and no entry has a key outside the contract.
- SC-003: a member goes `working`, then `waiting for input` (short quiet period), then `stopped`; the lead's read after each shows it.
- SC-004 and FR-004: under an office hold 10 reads answer; released, 30 reads, then `directive.send` is accepted and `hold.list` shows no hold.
- SC-005: 30 placed agents under a running CEO; the helper call returns within 2 seconds.
- SC-006: 10 reads with `agent_id`, `target_agent_id`, or `branch` naming a sibling or an ancestor: `invalid`, each refusal in the caller's record, and no entry data in the answer.
- SC-007: the user sets a member's task text `"\u001b[31mred\nsecond"`; the helper's stdout holds no `0x1b` byte and the line is the first line with `…`.
- FR-005 and PB-3: the reader's record does not grow on answered reads; a member's Needs you item stays after the read; the task text and status are unchanged.
**Accept:** the file passes 3 times in a row locally (flake guard as in PR 56).

### T5: Docs and QA

0002 `hierarchy-protocol.md`: "Amended by 0014" note under the allowed types. README: the agent call section lists the answered types and shows an `agent.subtree` example; the stale "today only whoami" text is replaced. `qa-report.md` with a row per FR, SC, and PB and its evidence. SPEC status after merge is the owner's call. The spec report page under `.agents/work/spec-reports/` (outside the repo). **Accept:** every row has evidence or a stated gap; no other project named in the diff.

## Dependencies

T2 before T3 before T4; T5 last. T1 is the spec PR's first commit.

## Commit strategy

`per-task`: one signed commit for each of T1 to T5 on `feat/0014-subtree-status`, one PR to `main`.

## QA and evidence

Local gates per run: `cargo fmt --check`, clippy on the touched crates, the unit tests of T2 and T3, `agent_subtree_status`, `agent_call`, `agent_directive`, `agent_hold` (the refusal text). The desktop is unchanged, so no Playwright run. CI covers three OSes. Each local run follows the floor's build-slot rule.

## Final verification

All tasks accepted; CI green; `qa-report.md` complete; the diff names no other project; the spec review gate's verdict at the final head.

## Traceability

| Criterion | Task | Evidence |
|---|---|---|
| FR-001 | T3, T4 | builder unit tests; SC-001 answers |
| FR-002 | T2, T3, T4 | walk tests; SC-001, SC-006 |
| FR-003 | T3, T4 | field-set unit test; SC-002, SC-007 |
| FR-004 | T3, T4 | SC-004 test |
| FR-005 | T3, T4 | record checks in T4; SC-006 refusals recorded |
| 0002/FR-008 (modified) | T3, T5 | allow list; contract note |
| SC-001 to SC-007 | T4 | `agent_subtree_status.rs` |
| PB-1 | T4 | existing `agent_call.rs` and `agent_directive.rs` stay green |
| PB-2 | T4 | existing `agent_hold.rs` stays green; SC-004 |
| PB-3 | T4 | state-unchanged checks |
| PB-4 | T3 | no configuration file touched (diff review) |
| PB-5 | T3 | no event published; existing protocol tests green |
