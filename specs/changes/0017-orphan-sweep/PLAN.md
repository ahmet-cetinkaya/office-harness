---
status: draft
spec: specs/changes/0017-orphan-sweep/SPEC.md
commits: per-task
---

# Plan: Orphan sweep after an unclean server stop (0017)

## Summary

While a session runs, the server stores the identity of its CLI process and of each process that descends from it: the process id, the process start time, and the command line, plus the machine's boot time. After an unclean stop, the next server start finds the `lost` sessions, binds its listener at once, and runs the sweep in the background. The sweep reads each stored identity again. It ends a process only when all values still match, first with a request and then by force after the grace period. It records one action record entry for each stored identity. Until the sweep ends, the resume dialog shows "Ending processes left from the last run", and the user's starts wait. After the sweep, the 0013 warning names only the agents for which the office could not prove that nothing is left.

**What you get:**
- A process inspection port in the terminal module with one adapter for Linux, macOS, and Windows (D1, D2).
- A table of stored process identities per session, private as the office database is (D3).
- A recorder that stores the CLI identity at spawn and the helper identities with a 5-second scan (D4).
- The sweep at server start, with a gate that holds the user's starting requests and the resume plan until it ends (D5 to D7).
- A sweep finding per `lost` session in `resume.plan`, a sweep state for the desktop, and the modified 0013/FR-008 marks in the dialog (D8, D9).

**Why this approach:**
- A process id alone is not an identity: ids are reused. Process id plus start time plus command line plus boot time names one process run, on every OS.
- The listener binds before the sweep, so the desktop app can show the sweep state (FR-007) and a slow sweep never looks like a dead server.
- The recorder scans the session trees that the stop code already walks (`process_tree.rs`), so helpers are known before an unclean stop takes the tree away.

**What it will not do:** end a process that has no stored identity, sweep without a `lost` session, end helpers after a normal end of a session, or sweep other machines (spec Out of scope).

**Shape of the work:** 6 tasks in one PR (`feat/0017-orphan-sweep`), one commit per task. **Effort:** M.

**Main risks:**
- **Ending a wrong process.** Every value must match, the match is checked again just before each end request (FR-003), and the end request goes to the process id only, never to a process group. A test proves each non-match reason (SC-002).
- **Start time precision.** macOS and Windows give whole seconds, so a reused process id with the same command line within the same second would match. The command line and the boot time must also match, and the office started both processes, so the risk is accepted and recorded in `research.md`. Linux uses exact clock ticks (D2).
- **Windows.** A detached process has no console, so the polite request does not exist. The sweep ends it by force at once and records "ended by force" (D6).

**Blockers:** none. The new dependency `sysinfo` (D1) is part of this plan's review gate.

## Goal

Deliver spec 0017: leftovers of `lost` sessions are ended before the user can resume, only exact matches are ended, every result is recorded, and the 0013 warning names only the agents that may still have a live CLI.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` b768b63 on 2026-11-23.

- **Spawn:** `spawn_in_pty` (`M/terminal/infrastructure/src/services/pty_host.rs:35-61`), called from `portable_terminal_runtime.rs:106`; the pid is `pty.child.process_id()` (`terminal_session.rs:156`), in memory only.
- **Session row:** inserted before the spawn (`M/agent/application/src/common/session_launcher.rs:267`), terminal started at `:269`.
- **Process tree:** `M/terminal/infrastructure/src/services/process_tree.rs`: unix `descendants` with `ps -A -o pid= -o ppid=` (`:34-42`), `request_end` (killpg SIGTERM), `force_end`; Windows `taskkill /T /F`. Stop: `terminal_session.rs:267-300`.
- **Sessions table:** `M/agent/infrastructure/src/persistence/migrations/0003_additions_and_sessions.sql:13-24`; newest agent migration `0007_agent_worktree.sql`; `SessionRepository::mark_running_as_interrupted` writes `lost` (`session_repository.rs:189-197`).
- **Start sequence:** `H/main.rs:48-56` (`bootstrap::start`, `Server::bind`, `serve_until_stopped`); `H/bootstrap/services.rs:104-113` marks agents `interrupted` and sessions `lost` before the bind.
- **Resume plan:** `H/server/resume_requests.rs:83-132`; `after_unclean_stop` comes from `exit_result == "lost"` (`:116-120`).
- **Action record:** `ActionJournal::record_detailed` (`M/agent/application/src/common/action_journal.rs:42-58`); office actor as in `H/server/resume_run.rs:204-208`; `detail` column from audit `0003_detail.sql`.
- **Tests:** `H/../tests/server_crash.rs` (Linux; `ignores-hangup` stand-in, `ProcessIdentity` from `/proc/<pid>/stat`), `server_restart.rs`, `server_process.rs`.
- **Crates:** `nix` 0.28 and `windows-sys` are in the workspace; no crate reads process start time or command line on all three OS.

## Decisions

- **D1 Process inspection crate.** Use `sysinfo` 0.40.1 (MIT, MSRV 1.95, released 2026-10-30, the newest release at least two weeks old on 2026-11-23) with `default-features = false, features = ["system"]`, only to read processes (`research.md`). It reads the process id, start time, command line, parent, status, and boot time on Linux, macOS, and Windows. Writing three readers by hand (procfs, libproc and sysctl, Toolhelp and the process environment block) is more code with more OS risk. Ending a process does not use `sysinfo`: its Windows path has no request step and its Unix path does not report the OS error, so the adapter ends processes with `nix` (Unix) and `windows-sys` (Windows), both already in the workspace (D6). If T1 finds that a same-user command line cannot be read on macOS or Windows in CI, T1 stops and the plan goes back to the gate.
- **D2 Identity and match.** `ProcessIdentity { pid: u32, started_at: u64, command: String, boot_time: u64 }`.
  - `started_at`: on Linux the raw start time in clock ticks after boot (`start_time_raw`), which is exact; on macOS and Windows the start time in whole seconds that the kernel stores, which is stable between reads. The adapter is the only writer and reader, so the unit never crosses a boundary.
  - `boot_time` in seconds; two boot times are the same machine run when they differ by at most 2 seconds, because Linux derives it from the clock and it can move by a second (`research.md`). A restart of the machine moves it by much more.
  - `command`: the arguments joined with a single space, as the OS reports them (spec A4). An empty command (not readable) is never a match.
  - A process in the zombie state counts as not running.
  - A stored identity matches a running process when the boot time is the same and the process with that id has the same start time and command. The result of a non-match names the first value that differs, in this order: not readable, machine restarted, not running, other start time, other command line (FR-005).
- **D3 Storage.** Agent migration `0008_session_processes.sql`: table `agent_session_processes (id INTEGER PRIMARY KEY, session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE, pid INTEGER NOT NULL, started_at INTEGER NOT NULL, command TEXT NOT NULL, boot_time INTEGER NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('cli','helper')), state TEXT NOT NULL CHECK (state IN ('open','ended')), UNIQUE (session_id, pid, started_at))`. The table lives in the office database, whose file is private to the owner (0001/FR-009), so FR-001's privacy needs no new code; a test checks the mode on Unix. Repository: `insert_open`, `end_for_session`, `end_one`, `open_for_lost_sessions`.
- **D4 Recorder.**
  - The terminal runtime reads the CLI identity right after the spawn and returns it with the started terminal; the launcher stores it before it publishes the session as started (FR-001). A failed read stores nothing and logs it; that session later counts as "no complete stored process identities" (0013/FR-008 modified).
  - A recorder task scans every 5 seconds (below the 10 seconds of FR-002): for each running session it walks the descendants with the existing tree walk and stores each new identity as `helper`; for each stored open `helper`, it marks it `ended` when it no longer matches. A helper that left the tree stays stored while it matches (FR-002).
  - When a session ends while the server runs, `end_for_session` runs in the same step that ends the session row (FR-008). A clean server stop ends every session first, so nothing stays `open`.
- **D5 Sweep timing.** `bootstrap::start` still marks `lost` before the bind. After the bind, `serve_until_stopped` starts the sweep task when `open_for_lost_sessions` is not empty. A `SweepGate` (a `tokio::sync::watch<SweepState>`) holds `running` until the sweep ends, then `done`.
- **D6 Sweep run.**
  - For each stored identity, in parallel (spec A5): read the process; on a non-match, record `left` with the D2 reason and mark the identity `ended` in one database transaction (FR-005, FR-009).
  - On a match: read again, then send the request to end to that process id only (Unix `nix::sys::signal::kill` with SIGTERM to the pid, not the group). Wait the stop grace period, checking every 100 ms. If it still matches, read again and force it (Unix SIGKILL to the pid; Windows `OpenProcess` and `TerminateProcess` through `windows-sys`). On Windows a detached process has no console for a request, so the request step is skipped and the result is "ended by force".
  - Result `ended`, `ended by force`, or `could not end` (still matching after the force, or the OS refused, with the OS error text). The action record entry and the `ended` mark are written in one transaction (FR-004, FR-006, FR-009). The audit and agent tables share the office database; T3 confirms this, and if they do not, T3 writes the mark first and the entry second and records the order as a Deviation.
  - Action `end_leftover`, actor office, detail `"<result>: pid <pid>, session <id>, <command>"` or `"left: <reason>: pid ..."`.
  - One notification `leftover_kept` per sweep that has a `could not end` result, naming the agents (FR-006, spec A8).
- **D7 Waiting requests** (FR-007). `agent.start`, `agent.new_task`, `session.continue`, `agent.resume`, and `resume.start` await the gate before they take the status order. `resume.plan` answers `{sweeping: true, entries: []}` while the gate is `running`. A server stop during the sweep stops the wait with the stopping refusal that 0001 already has.
- **D8 Sweep finding** (0013/FR-008 modified). `resume.plan` entries replace the boolean `after_unclean_stop` with `unclean_stop: null | {ended: n} | {may_run: reason}`. `may_run` covers: a `could not end` or `not readable` result, or no stored identity for a `lost` session. `ended: n` counts `ended` and `ended by force`. A `lost` session whose identities were all `left` (not running, machine restarted, other start time, other command line) and with none ended gives `null`. The old boolean stays in the answer as `true` exactly when `may_run` is set, so an older desktop app keeps its warning.
- **D9 Desktop.**
  - `office.sweep` event `{state: "running" | "done"}`, and the same state in the hello answer; the panel shows "Ending processes left from the last run" in a `role="status"` region while `running`.
  - The resume dialog shows the same text in place of the list while `sweeping`, then loads the plan when the event says `done`.
  - The dialog marks: "Ended N processes left from the last run" for `ended`, the 0013 mark and warning for `may_run`, nothing for `null`. The acknowledgement rule applies only to `may_run`.
  - The action record shows the new action text "ended a leftover process" with the detail.

## Scope

**In:** D1 to D9, `contracts/sweep-protocol.md` (the `resume.plan` change, the `office.sweep` event, the hello field, and the new action and notification), README notes, unit, integration, and e2e tests, `qa-report.md`, and the Turkish HTML report.

**Out:** the spec's Out of scope list.

## Tasks

### T1 Process inspection port and adapter (D1, D2)

- Port `ProcessInspector` in `M/terminal/application` (`read(pid) -> Option<ProcessIdentity>`, `boot_time`, `descendants(pid)`, `request_end(pid)`, `force_end(pid)`); adapter in `M/terminal/infrastructure`.
- **Acceptance:** unit tests on each OS in CI: the identity of a spawned child reads the same twice; it matches itself; it does not match after the child ends; a child with the same command started later has another start time; `request_end` and `force_end` end only that pid (a sibling with the same command keeps running). `cargo test -p terminal-infrastructure` passes. The boundary script passes.

### T2 Storage and recorder (D3, D4) - depends on T1

- Migration, repository, launcher change, recorder task, `end_for_session` in the session end path.
- **Acceptance:** repository unit tests (insert, unique, end one, end all of a session, open identities of `lost` sessions only); migration test counts updated; an integration test with a fake CLI that starts a helper and detaches it shows both identities stored within 10 seconds, the helper kept after it leaves the tree, and every identity `ended` after a normal stop and after a clean `server.restart` (SC-006 part). The office database file mode test on Unix passes.

### T3 Sweep and gate (D5 to D7) - depends on T2

- `SweepGate`, the sweep task, the waiting requests, the `leftover_kept` notification, the action text.
- **Acceptance:** `tests/server_sweep.rs` (Unix with the `ignores-hangup` stand-in and a detached helper; Windows with a detached stand-in) proves:
  - after a forced kill of the server, every leftover ends before `resume.plan` answers with entries, with one `end_leftover` entry each (SC-001 at 30 agents with 1 helper each, SC-004);
  - no end for a process with the same command and another start time, a stored id now used by another command, and a process the office did not start (SC-002);
  - the list answers within 30 seconds with 30 `lost` sessions (SC-003);
  - a stand-in that ignores SIGTERM is `ended by force`; a process the office cannot end gives `could not end` and one notification (FR-006);
  - a start, a new task, and a continue sent during the sweep run after it (FR-007);
  - a forced kill of the server during the sweep, then a start: every leftover ends and each identity has exactly one result entry (SC-007);
  - 10 clean stops and 10 normal stops end 0 processes and start 0 agents (SC-006).

### T4 Resume plan finding (D8) - depends on T3

- `resume_requests.rs` builds `unclean_stop` from the sweep results; the old boolean kept for older apps.
- **Acceptance:** integration test with 4 agents (one leftover ended, one that cannot be ended, one CLI that ended on the hang-up, one session with no stored identity): `ended: 1`, `may_run`, `null`, `may_run` (SC-005 server side). The 0013 `bulk_resume` and `server_crash` tests still pass, updated to the new field.

### T5 Desktop (D9) - depends on T4 for the contract only

- Sweep state service, panel status text, dialog sweeping state and marks, action record text, mock-core.
- **Acceptance:** Vitest for the model (finding to mark and warning) and the sweep state; Playwright `e2e/sweep.spec.ts`: the panel text while running and gone after `done`, the dialog text in place of the list, the three marks, Confirm gated only by `may_run`, the action record entry; `bun run check`, `lint`, `format:check`, `test`, and Playwright pass.

### T6 Docs and evidence - depends on T1 to T5

- `contracts/sweep-protocol.md`; the 0013 contract and the 0001 data model amended where `resume.plan` and the action list are described; README section; `qa-report.md` with each FR, SC, and PB; Deviations if any.
- **Acceptance:** each FR, SC, and PB has a qa row with evidence. The no-other-product grep of the diff is clean.

## Traceability

| Spec item | Tasks | Evidence |
|---|---|---|
| FR-001 | T2 | recorder test, file mode test |
| FR-002 | T2 | detached helper test |
| FR-003 | T1, T3 | `server_sweep.rs` leftovers and re-check |
| FR-004 | T3 | one entry per ended leftover |
| FR-005 | T1, T3 | non-match reasons test |
| FR-006 | T3 | could not end and one notification |
| FR-007 | T3, T5 | waiting requests test, e2e panel and dialog text |
| FR-008 | T2 | ended after a stop and a clean restart |
| FR-009 | T3 | kill during the sweep test |
| 0013/FR-008 (modified) | T4, T5 | four-agent test, e2e marks |
| PB-1 | T3 | no agent starts after the sweep (`server_process.rs` and SC-006) |
| PB-2 | T3 | the sweep writes only the office database; review of the diff |
| PB-3 | T2 | existing stop tests pass unchanged |
| PB-4 | T1, T3 | SC-002 test |
| PB-5 | T3, T4 | `bulk_resume.rs` passes after the sweep |
| PB-6 | T4 | a `lost` session continues after the sweep |
| SC-001 to SC-007 | T3, T4, T5 | as named in the task acceptance |

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright.
- The repository's boundary check script.
- CI green on Linux, macOS, and Windows.
- Hands-on: a real CLI agent and its helper (the `opencode` service) ended by the sweep after a SIGKILL of the server, in the real app. Otherwise the qa row stays BLOCKED, never PASS.
