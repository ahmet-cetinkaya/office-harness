---
status: completed
spec: specs/changes/archive/0022-waiting-item-sleep/SPEC.md
commits: per-task
---

# Plan: Idle agents sleep past their waiting item, and one wait gives one item (0022)

## Summary

The server learns to tell a prompted change of an agent's status from an unprompted one, and it uses that one fact in three places.

**What you get:**
- A `WaitTracker` in the server. It hears every wait end: person input, office text, session start and end. It keeps the start of the current stretch of `working`, and it classifies each change between `working` and `waiting for input` as prompted or unprompted.
- The notification bridge asks the tracker at each status change:
  - An unprompted change adds no notification and no item, and it does not clear the item.
  - A timer removes the item when a stretch of `working` passes the work bound.
- A Needs-you item gets a stored flag `waits_only`. It stays true while the item has recorded only changes to `waiting for input` and holds no report.
- The idle sleep gate changes in three ways:
  - A plain waiting item no longer blocks a sleep. An item is plain when it is `waits_only`, the screen can be read, and the 0012 question check finds no open question.
  - Idle-since counts from the last prompted change.
  - A failed sleep allows a new try only after a prompted change.
- The temp idle end of 0020 keeps the old rule: any item stops it.

**Why this approach:**
- All wait ends are already seen in the server process: `PersonInput` and the status bridge see them, and the terminal host sees session starts. One tracker beside `PersonInput` keeps the rule in one place, so the notification and the sleep code cannot disagree.
- The tracker measures the work bound from the stored start of the stretch, not by polling (Efe, spec r2). A one-shot timer, armed with that start, only removes the item. The classification at the return reads the stored start again.
- The status machine and the terminal module do not change (spec Context, PB-5). The fix is on the consumer side.

**Delivery:** one PR (#135, with the spec), 4 tasks, committed per task.

## Goal

Ship every requirement of spec 0022 with automated evidence. Then repeat the 0018 hands-on run with a real `claude`, where SC-004 expects one notification and a sleep without a dismiss.

## Context

- Status detection: `terminal_session.rs:425-472` (the quiet monitor), `agent_status_bridge.rs:78-126,152-168`, and `status_machine.rs:18-21`. None of these change.
- Notification and Needs you:
  - `notification_bridge.rs:134-158` (`apply_frame` clears first, then records) and `:246-300` (`record`, `notify_then_queue`).
  - `needs_you_bridge.rs:80-91`, `needs_you_service.rs:36-106`, and `needs_you_item.rs:73-113` (`record_event`, `record_report`).
  - `clear_reason.rs:8-34` (`Resumed` keeps a report item).
  - Migrations are in `src/modules/notification/infrastructure/src/persistence/migrations/0001-0004`.
- Wait ends:
  - `person_input.rs:47-57` (`stamp`, from `terminal_requests.rs:185`), `:70-72` (`directive_typed`, from `directive_bridges.rs:201`), `:85-87` (`session_started`, from `terminal_host.rs:178`) and `:91-96` (`office_text_left`, from `directive_bridges.rs:212` and `typed_input.rs:43`).
  - A session end has no hook. It is seen at exit in `agent_status_bridge.rs:97-107`.
- Idle sleep:
  - `idle_sleep_gate.rs:139-237`: `check`, `without_activity`, `check_attention`, `check_input_line` and `check_tree`.
  - `idle_sleep_run.rs:32-182`: `RoundState`, `bars` and `round`.
- Question check: `open_question.rs:42-48` `shows_open_question(&ScreenView)`. `runtime.screen(agent)` returns `None` when the screen cannot be read (`directive_bridges.rs:85-89`).
- Temps: `temp_run.rs:191-207` uses `gate.keeps_from_idle_end`, which uses `check_attention`. Out of scope (A9), so the temp path keeps the strict check.
- Tests:
  - `tests/needs_you.rs` (status injected by `publish_status`), `tests/idle_sleep.rs`, and `tests/support/fake_cli.rs` (`add`, `print_then_wait`, `open_with_quiet_period`).
  - The unit test files beside each source: `*.tests.rs`.

## Decisions

- **D1: WaitTracker.** It lives in `H/bridges/wait_tracker.rs`, where `H` is `src/apps/office-harnessd/src`. It is shared through `Arc` the way `PersonInput` is.
  - Per-agent state: `ended_since_wait: bool`, `working_since: Option<Instant>`, `last_prompted_wait: Option<Instant>`, `run: Option<RunId>`.
  - API:
    - `wait_end(agent)` sets `ended_since_wait = true`.
    - `session_started(agent, run)` and `session_ended(agent)` reset the state and count as a wait end.
    - `classify(agent, StatusChange) -> Change::{Prompted, Unprompted}`.
    - `working_since(agent)` and `last_prompted_wait(agent)`.
  - Rules:
    - A change to `working` is prompted when `ended_since_wait` is true. It always stores `working_since = now`.
    - A change to `waiting for input` is prompted when `ended_since_wait` is true, when `now - working_since > work_bound`, or when it is the first wait of the run. A prompted wait sets `last_prompted_wait = now` and `ended_since_wait = false`.
    - Unprompted changes leave both values alone.
- **D2: Wait-end hooks.** Each one calls the tracker in the same place as the `PersonInput` call:
  - `PersonInput::stamp`, `directive_typed`, `office_text_left` and the startup prompt (`typed_input.rs`);
  - `session_started` (`terminal_host.rs:178`);
  - the exit path of `AgentStatusBridge` (`session_ended`).
  - A sleep and a stop end the session through the same exit path.
  - The stamp comes before the status event that it causes, so a person input that arrives with an output burst reads as prompted (spec edge row "same moment").
- **D3: Work bound.** `work_bound = quiet_period + WORK_BOUND_EXTRA`, where `WORK_BOUND_EXTRA` is 10 s.
  - The quiet period is the startup value that the terminals use.
  - `ServerOptions` gets `work_bound_extra` so that tests can shorten it, as `minute` is shortened for idle sleep.
  - Measured from the stored `working_since` (Efe, spec r2).
- **D4: The item leaves at the bound.**
  - On an unprompted change to `working`, the notification bridge arms one `tokio` sleep until `working_since + work_bound`, carrying the stretch's `working_since`.
  - When it fires, the item is removed with `ClearReason::Resumed`, but only if the tracker still shows the same `working_since` and the agent still shows `working`. `Resumed` keeps a report item (0015).
  - At the bound, the tracker sets `ended_since_wait = true`, so the return reads as prompted even if the timer is late.
- **D5: Notification bridge.**
  - `apply_frame` asks `tracker.classify` for `working` and `waiting_for_input` frames of the current run.
  - Unprompted `working`: no clear, and arm the D4 timer.
  - Unprompted `waiting_for_input`: no notification, no `needs_you.record`, and no change to an existing item.
  - All other frames behave as today (PB-4).
  - The classification happens once per frame. The status bridge publishes each frame once.
- **D6: `waits_only` on the item.**
  - Domain: `NeedsYouItem` gets `waits_only: bool`.
  - `record_event` sets it to `kind == WaitingForInput` for a new item, and to `waits_only && kind == WaitingForInput` for an existing item.
  - `record_report` sets it to false.
  - Persistence: migration `0005_needs_you_waits_only.sql` adds `waits_only INTEGER NOT NULL DEFAULT 0`. Items stored before the change block as before.
  - The wire item does not change, because no client needs the flag.
- **D7: Sleep gate.**
  - Plain-item test for sleeps: `check_attention_for_sleep` passes when the agent has no item, or when its item is `waits_only`, `runtime.screen(agent)` is `Some`, and `!shows_open_question(screen)`. `None` blocks (FR-002).
  - `keeps_from_idle_end` keeps the strict `check_attention` (A9).
  - `without_activity`: `since_waiting` becomes `last_prompted_wait.elapsed()` from the tracker (0 when there is none), not the runtime output quiet. `since_input` stays.
  - Condition (a) is still the stored status `waiting_for_input` at both checks (`round` and the ordered sleep step).
- **D8: FR-009.**
  - `RoundState::bars` changes from "active since the failure" to "the tracker's `last_prompted_wait` is after `failed_at`".
  - The idle time is then checked from that moment by the normal check.
- **D9: Constitution check.**
  - The tracker lives in the server app, so no module boundary changes (`scripts/check-boundaries.sh`).
  - Nothing is written to a CLI configuration (PB-8). The new migration is additive.
  - The spec and the plan ship in the same PR.

## Scope

**In:** D1 to D8, the tests, the 0008 needs-you contract note (no wire change), the qa-report, and the hands-on rerun with a real `claude` (SC-004).

**Out:**
- the temp idle end (A9);
- any desktop change, because the desktop shows items and statuses as the server sends them;
- status detection (PB-5);
- a measured screen-redraw length.

## Tasks

### T1: WaitTracker and the wait-end hooks (D1, D2, D3)
- **Files:**
  - new `H/bridges/wait_tracker.rs` with `wait_tracker.tests.rs`;
  - `H/bridges/mod.rs`, `person_input.rs` call sites, `directive_bridges.rs`, `typed_input.rs`, `terminal_host.rs` and `agent_status_bridge.rs`;
  - `H/server/server_options.rs` and the bootstrap wiring.
- **Acceptance:**
  - Unit tests cover:
    - first wait prompted;
    - redraw unprompted both ways;
    - person input makes the next change prompted;
    - office text makes it prompted;
    - session start and end reset the state;
    - a stretch longer than the bound makes the return prompted;
    - a stretch shorter than the bound leaves it unprompted;
    - `last_prompted_wait` moves only on prompted waits.
  - Existing tests stay green.
- **Depends on:** none.

### T2: Notification and Needs you dedupe, plus the item at the bound (D4, D5, D6)
- **Files:**
  - `notification_bridge.rs` with tests;
  - `needs_you_item.rs` with tests;
  - the notification repository and the new migration 0005;
  - the `needs_you_service` if needed.
- **Acceptance:**
  - Unit tests:
    - `waits_only` stays true through waits and turns false on a bell, a hold or a report;
    - a stored item round-trips the flag;
    - existing rows read as false.
  - Integration tests in `tests/needs_you.rs` (fake CLIs, short quiet period, short work bound):
    - a CLI that answers once and then prints a line on its own at intervals gives 1 notification and 1 item with an unchanged wait time (SC-003);
    - a directive into an agent with an item removes it at `working`, and the return gives 1 new item and 1 notification (SC-005, PB-3);
    - a stretch of `working` past the bound after the item was opened gives 1 item and 1 notification at the return (SC-007, US2 scenario 9);
    - the item leaves at the bound while the agent still works (0008/FR-005 modified);
    - a bell, an exit and a report behave as before (PB-4);
    - a dismissed item stays gone through a redraw (US2 scenario 2).
- **Depends on:** T1.

### T3: Sleep gate, idle-since and FR-009 (D7, D8)
- **Files:** `idle_sleep_gate.rs` with tests, `idle_sleep_run.rs` with tests, and `tests/idle_sleep.rs`.
- **Acceptance:**
  - An idle agent with a plain waiting item is put to sleep, and its item stays in Needs you with its wait time (SC-001, FR-001).
  - Each blocker alone keeps the agent awake (SC-002):
    - an item that recorded a bell;
    - a report item;
    - an auto hold or resume hold item;
    - a waiting item while the screen shows each of the three 0012 question shapes.
  - An unreadable screen keeps the agent awake: a unit test with a fake runtime that returns `None` (SC-006, FR-002).
  - A redrawing CLI still sleeps one idle time after its prompted wait (A7).
  - A wake removes the item at `starting` (US1 scenario 7).
  - After a failed sleep, there is no new try until a prompted wait plus the idle time (0018/FR-009 modified).
  - The temp idle end still stops for any item: an existing `temps.rs` test stays green, plus one new test with a waiting item on a temp (A9).
- **Depends on:** T1 and T2.

### T4: Docs, QA report, hands-on rerun
- **Files:**
  - `specs/changes/archive/0022-waiting-item-sleep/qa-report.md` (new);
  - a note in the 0008 needs-you contract that an unprompted change adds no item (no wire change);
  - the PLAN Result.
- **Acceptance:**
  - Every FR, SC and PB in the Traceability table has evidence.
  - Hands-on in the real app with a real signed-in `claude`, idle time 5 minutes, no dismiss and no terminal open: 1 `waiting for input` notification, and `asleep` within 6 min 10 s (SC-004). Then Enter wakes it into the same session, and the item leaves at `starting`.
  - Counts that were not reached are PARTIAL.
- **Depends on:** T1 to T3.

## Traceability

| Criterion | Task | Evidence |
|---|---|---|
| FR-001 | T3 | SC-001 test: the item stays after the sleep |
| FR-002 | T3 | unit test: an unreadable screen blocks |
| 0003/FR-006 (c) | T2 | SC-003 test: 1 notification |
| 0003/SC-004 | T2 | existing `a_wait_is_announced_after_the_quiet_period...` stays green, plus the SC-005 test |
| 0008/FR-001 | T2 | SC-003 test: 1 item, wait time unchanged |
| 0008/FR-005 | T2 | the directive test, the bound test, and the redraw test (item kept) |
| 0018/FR-002 | T3 | SC-001, SC-002, SC-006 tests; idle-since test (A7) |
| 0018/FR-009 | T3 | failed-sleep retry test |
| 0018 Terms Idle, Idle since | T1, T3 | tracker unit tests and the idle-since test |
| SC-001 | T3 | integration test (1 agent with a shortened minute; the count of 10 is PARTIAL unless run) |
| SC-002 | T3 | one test per blocker |
| SC-003 | T2 | integration test with a shortened interval and duration (PARTIAL for the 20 minutes) |
| SC-004 | T4 | hands-on with a real `claude` |
| SC-005 | T2 | integration test (rounds as stated, or PARTIAL) |
| SC-006 | T3 | unit test, 10 checks |
| SC-007 | T2 | integration test with a short bound |
| PB-1 | T3 | SC-002 tests, plus the existing hold, queue, text and child tests |
| PB-2 | T2 | existing open and dismiss tests stay green |
| PB-3 | T2 | directive test |
| PB-4 | T2 | bell, exit and report tests |
| PB-5 | T1 | no change to the status code; existing status tests stay green |
| PB-6 | T3 | existing `with_idle_sleep_off...` and the no-notification test stay green |
| PB-7 | T3 | the failed-sleep test checks the status and 1 entry |
| PB-8 | T3 | the question check reads `runtime.screen` only; no configuration write (review) |

## QA and evidence

- Gates, run by the lead on the final tree:
  - `cargo fmt --all --check`, `bash scripts/check-boundaries.sh`;
  - `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings`;
  - `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=4`, with pass and fail counts summed from the result lines.
- The new integration tests run 3 times.
- No desktop file changes, so the desktop gates are NOT RUN, with that reason.
- Hands-on evidence goes to `qa/evidence/b-0022-head/` (agent folder), with the session ids masked.

## Final verification

Run `/ac:verify` against this plan. Every Traceability row needs its evidence, and CI must be green on the 3 OSes. Then Efe's code gate runs at the exact head.

## Result

T1 to T4 are done, one signed commit for each task. T4 is the 0008 contract note ("Amended by 0022", no wire change), `qa-report.md`, and this section. T4 also makes the SC-006 unit test run 10 checks.

The readiness check found that input reaching an agent while it shows `working` did not remove its item (0008/FR-005 as modified). A fix commit adds a wait-end listener to the tracker: the notification bridge removes the item when the agent shows `working`. The gates pass on the final tree: 1796 tests passed, 0 failed. Code gate r3 asked for two more changes: a person's input is now stamped on the blocking pool, because its wait end may read and write the store; and SC-004 got the full 30-minute run. That run with a real `claude` at 6a6e890 passed SC-004: exactly 1 notification from the reply to 30 min 16 s after the prompted wait, a sleep 311.4 s after that wait, 0 dismisses and 0 terminal views open, and Enter woke it into the same session. SC-001, SC-003, SC-005 and SC-007 are PARTIAL, with fewer rounds or shorter times than stated; `qa-report.md` names each gap. No spec deviation.
