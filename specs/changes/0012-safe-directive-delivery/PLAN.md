---
status: draft
spec: specs/changes/0012-safe-directive-delivery/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-23 together with the spec, for one review of both. -->

# Plan: Safe directive delivery (0012)

## Summary

A directive no longer goes into the terminal at once. The send handler checks it as before, then either types it now (the target waits for input, no person typed there in the last 30 s, no open question on the screen, nothing waits before it) or puts it in the target's queue. One server thread types waiting directives when their target is ready, and withdraws the ones that time out, lose their target, or meet a hold or a tree change.

**What you get:**
- A per-target queue inside the hierarchy send handler, with a fast path that keeps today's behavior when the target is ready.
- A readiness answer from the target bridge: status, person input in the last 30 s, and a screen check.
- A pure screen recogniser for open questions in the terminal domain, with unit tests over sample screens.
- `directive.send` answers `{"delivery": "typed"}` or `{"delivery": "waiting", "position": n}`.
- Withdrawals and the queue-full refusal in the sender's and the target's action records.

**Why this approach:**
- The send handler already owns the order of terminal writes (`write_order`, `send_directive_handler.rs:27-29`), the journal, and the receipt, so the queue lives next to them and the bridges stay thin.
- The server already keeps a vt100 copy of every screen (`screen_emulator.rs`), so the question check needs no CLI change (spec PB-4).
- The person-input stamp sits in the one route that carries user keys (`terminal.input`, `server/terminal_requests.rs:98`), so a directive's own write never counts as person input.

**What it will not do:** a queue view, a withdraw request, hooks, persistence across a server restart (spec Out of scope).

**Shape of the work:** 4 tasks in one PR, one commit per task. **Effort:** S to M.

**Main risks:**
- **Existing directive tests** type at once into a member that is still `working`. With the gate they wait for `waiting_for_input`. T4 starts those offices with a short quiet period and waits for the member's `waiting_for_input` before sending, so they keep testing the fast path.
- **False question matches** delay a directive up to the 10-minute limit. The recogniser looks only at the bottom 15 non-empty rows and uses tight markers (D4); the unit tests include idle screens of each CLI shape that must not match.

**Blockers:** none.

## Goal

Deliver spec 0012: FR-001 to FR-005 and the MODIFIED 0002/FR-010, with PB-1 to PB-5 kept.

## Context

- **Repository:** `main` at 6e3d197.
- **Seams** (read at 6e3d197):
  - Send handler: `src/modules/hierarchy/application/src/commands/send_directive/send_directive_handler.rs` (`handle` 59-122, `write_and_receive` 126-134). Ports: `src/modules/hierarchy/application/src/common/ports/directive_ports.rs`.
  - Bridges: `src/apps/office-harnessd/src/bridges/directive_bridges.rs` (`DirectiveTargetBridge` 38-67, `DirectiveWriterBridge` 72-89, journal 155-218). Wiring: `src/apps/office-harnessd/src/bootstrap/hierarchy_services.rs:144-158`.
  - Agent-call route: `src/apps/office-harnessd/src/server/agent_call.rs` (`answer_directive` 183-217 answers `Value::Null`; `recorded_and_published` 144-151).
  - Terminal: `TerminalRuntime` (`src/modules/terminal/application/src/common/interfaces/terminal_runtime.rs`), `PortableTerminalRuntime`, `TerminalSession` (core lock holds the `ScreenEmulator`), `ScreenEmulator::last_line` / `cursor_line` read the vt100 screen.
  - User input route: `src/apps/office-harnessd/src/server/terminal_requests.rs` (`INPUT` at 24, dispatched at 98).
  - Holds: `HoldService::guard` counts the rate window; the pure `hold_refusal(holds, rows, caller, target)` checks holds only (`hold_service.rs`).
  - Clocks: `SteadyClock` implements `HoldClock` (`src/apps/office-harnessd/src/bridges/system_clock.rs:50-58`).
  - Integration tests: `src/apps/office-harnessd/tests/agent_directive.rs`, `agent_hold.rs`, `task_text.rs`, `task_order.rs`, `agent_call.rs`; the office binds with an optional quiet period (`tests/support/running_office.rs:39-75`).

## Decisions

- **D1 Queue home.** `DirectiveQueue` in `send_directive/directive_queue.rs`: one `Mutex<HashMap<AgentId, TargetQueue>>` plus a `Condvar`. `TargetQueue { waiting: VecDeque<WaitingDirective>, last_typed: Option<Timestamp> }`; `WaitingDirective { sender, target, text, accepted_at }`. The handler owns it. Lock order: queue, then `write_order`, then the receipt's task lock; nothing that takes the queue lock runs under the others.
- **D2 Fast path and acceptance** (FR-001, FR-002, FR-004, PB-1, PB-2). `handle` keeps every check in its order: snapshot, descendant, text, running, then the new queue-full check (before the hold guard, so a full queue never counts toward a rate limit, 0007 A10), then the hold guard. Under the queue lock: when the target's queue is empty and readiness is `Ready` and the settle gap (D8) has passed, type now and answer `Typed`; else push and answer `Waiting { position }` (1-based). Both paths journal the accepted directive once, as today (`journal.record(sender, target, text, None)`). The answer type is `DirectiveOutcome { Typed, Waiting { position: usize } }`.
- **D3 Readiness port** (FR-002, FR-003, FR-005). `DirectiveTarget` gains `fn readiness(&self, agent_id) -> Result<Readiness, String>` with `Readiness { Ready, Busy(BusyReason), Gone }` and `BusyReason { Working, PersonTyping, OpenQuestion }`. The bridge reads, in order: the stored status (`Gone` when not running, `Busy(Working)` unless `waiting_for_input`), the person-input stamp (D5), then the screen (D4).
- **D4 Question recogniser** (FR-003). `terminal_domain::services::open_question::shows_open_question(screen: &str) -> bool`, pure. It looks at the bottom 15 non-empty rows, lower-cased, and matches when a row:
  - contains a yes-or-no choice: `(y/n)`, `[y/n]`, `(yes/no)`, `[yes/no]`;
  - starts, after spaces, with a selection mark (`❯ › ▶ ► → ● >`) followed by a number and `.` or `)`;
  - contains a permission phrase: `do you want to`, `do you trust`, `allow once`, `allow always`, `allow this`, `approve this`, `permission required`, `requires permission`, `press enter to confirm`, `press enter to continue`.
  `TerminalRuntime` gains `fn screen_text(&self, agent_id) -> Option<String>` (the visible rows, no scrollback; `None` when not running), with a default of `None` so test fakes stay unchanged; `PortableTerminalRuntime` reads it through `TerminalSession` from `ScreenEmulator::screen_text`.
- **D5 Person input** (FR-002, PB-5). `PersonInput` in `bridges/person_input.rs`: a `Mutex<HashMap<AgentId, Instant>>` with `stamp(agent)` and `typed_within(agent, window) -> bool`. `TerminalRequests::input` stamps before it forwards the bytes, and forwards them unchanged. The directive writer does not stamp. Window: `PERSON_TYPING_PAUSE = 30 s`.
- **D6 Delivery thread.** `bootstrap/directive_delivery.rs` spawns one thread: it waits on the queue's `Condvar` while every queue is empty, else every 250 ms runs `handler.deliver_waiting()` inside `recorded_and_published` (made `pub(crate)`), so withdrawal entries reach clients as `action.recorded`. `deliver_waiting` visits each target once: it withdraws expired heads (`accepted_at` older than 10 min), withdraws the whole queue on `Gone`, re-checks the head's tree place (`check_descendant` on a fresh snapshot) and holds (`HoldService::refusal(sender, target, rows) -> Option<String>`, a new non-counting wrapper of `hold_refusal`), and types the head when `Ready` and the settle gap passed.
- **D7 Withdrawal record** (FR-005, SC-004). `journal.record(sender, target, text, Some("withdrawn before typing: <reason>"))`; the bridge writes it in the sender's and the target's records like any refusal. Reasons: `not typed within 10 minutes`, `<name> is no longer running`, `<name> is no longer a descendant of the sender`, and the hold refusal text from 0007.
- **D8 Settle gap** (FR-001). After a directive is typed, the next one for the same target waits at least `SETTLE_AFTER_TYPING = 2 s`, so the target's status has moved off `waiting_for_input` (its echo and its turn) before the gate reads it again.
- **D9 Clock.** A new port `DirectiveClock { fn steady_now(&self) -> Timestamp }`, implemented by `SteadyClock`; the unit tests drive a fake clock through the 10-minute limit and the settle gap.
- **D10 Answer and contract** (FR-004). `answer_directive` answers `{"delivery":"typed"}` or `{"delivery":"waiting","position":n}`. The queue-full refusal is `refused` with `the queue of <name> is full: 20 directives wait`. `contracts/hierarchy-protocol.md` of 0002 gets an "Amended by 0012" note for the result and the new refusal; 0002 SPEC gets the note under FR-010 and the edge case.
- **D11 Constants.** `QUEUE_LIMIT = 20`, `WAIT_LIMIT = 10 min`, `PERSON_TYPING_PAUSE = 30 s`, `SETTLE_AFTER_TYPING = 2 s`, `DELIVERY_POLL = 250 ms`, `QUESTION_ROWS = 15`. Fixed (spec A8).

## Tasks

### T1 — Screen recogniser and screen text (terminal module)

- `open_question.rs` + `open_question.tests.rs` in `terminal_domain::services` (D4). Tests: each marker kind matches; an idle prompt row (`> `, a shell `$ `), a numbered list in agent output without a selection mark, and a question older than the bottom 15 rows do not match.
- `ScreenEmulator::screen_text`, `TerminalSession::screen_text`, `TerminalRuntime::screen_text` (default `None`), `PortableTerminalRuntime` override. One emulator test.
- **Done when:** terminal crates' tests and clippy pass.

### T2 — Queue in the send handler (hierarchy module)

- Ports (D3, D9), `DirectiveQueue` (D1), `handle` returning `DirectiveOutcome` (D2), `deliver_waiting` (D6), `HoldService::refusal` (D6), withdrawal journaling (D7), settle gap (D8).
- Unit tests with fakes: fast path typed; waiting while `Working`, `PersonTyping`, `OpenQuestion`, each typing nothing; order of 10 kept (SC-003); position numbers; 21st refused and journaled; head expires at 10 min and is journaled (fake clock); `Gone` withdraws all; a hold and a tree change withdraw the head; the settle gap delays the second; the accepted directive journaled once on both paths (PB-2); the receipt only when typed (PB-3); rules and refusals unchanged (existing tests stay green, PB-1).
- **Done when:** hierarchy crates' tests and clippy pass.

### T3 — Bridges, input stamp, delivery thread, answer (server)

- `PersonInput` (D5) with unit tests; stamp in `TerminalRequests::input`.
- `DirectiveTargetBridge::readiness` (D3) with unit tests over status, stamp, and a fake runtime's screen text.
- `SteadyClock` implements `DirectiveClock`; wiring in `hierarchy_services.rs`; delivery thread (D6); `answer_directive` result (D10).
- **Done when:** `office-harnessd` builds, its unit tests and clippy pass.

### T4 — Integration tests and documents

- `tests/agent_directive.rs`: existing tests bind with a short quiet period (300 ms) and wait for the member's `waiting_for_input` before sending; the answer checks `delivery: "typed"`.
- New integration tests in `tests/agent_directive.rs`:
  - a member shows `Do you want to proceed? (y/n)` and stays quiet; the directive answers `waiting`; nothing is typed while the question shows; the fake CLI then clears the screen and reads, and receives exactly the text and Enter (SC-001, FR-003);
  - person input through `terminal.input` and then a directive: the answer is `waiting` (FR-002);
  - a waiting directive and a stop of the target: the sender's and the target's records show `withdrawn before typing` with the text (FR-005, SC-004).
- Documents: 0002 SPEC note under FR-010 and its edge case, 0002 contract note (D10), 0012 `qa-report.md` (FR and SC rows with evidence), spec status to `in_progress` after approval, and the Deviations section if anything differs.
- **Done when:** the workspace gate passes (fmt, clippy, all Rust tests, boundaries script), and the desktop checks pass unchanged (no desktop file changes; `bun run check` only if a shared contract type moved).

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` with `CARGO_BUILD_JOBS=4` and `--test-threads=4`, `RUSTFLAGS='-Clinker-features=-lld'` locally.
- Boundaries script as CI runs it.
- No desktop or UI change, so no Playwright run is needed; CI runs the full matrix.

## Rollback

One PR, no migration, no stored data. Reverting the PR restores direct typing.
