---
status: approved
spec: specs/changes/0017-orphan-sweep/SPEC.md
commits: per-task
---

# Plan: Orphan sweep after an unclean server stop (0017)

## Summary

While a session runs, the server stores the identity of its CLI process and of each process that descends from it: the process id, the process start time, and a hash and a capped display text of the command line, plus the machine run (boot id or boot time). After an unclean stop, the next server start finds the `lost` sessions, binds its listener at once, and runs the sweep in the background. The sweep opens a handle to each stored process (a pidfd on Linux, a process handle on Windows), reads the identity on that handle, and ends the process on that same handle only when all values still match, first with a request and then by force after the grace period. On macOS, and on a Linux kernel before 5.3, no handle exists: it reads again and then ends by process id. It records one action record entry for each stored identity. Until the sweep ends, the resume dialog shows "Ending processes left from the last run", and the user's starts wait. After the sweep, the 0013 warning names only the agents for which the office could not prove that nothing is left.

**What you get:**
- A process inspection port in the terminal module with one adapter for Linux, macOS, and Windows (D1, D2).
- A table of stored process identities per session, private as the office database is (D3).
- A recorder that stores the CLI identity at spawn and the helper identities with a 5-second scan (D4).
- The sweep at server start, with a gate that holds the user's starting requests and the resume plan until it ends (D5 to D7).
- A sweep finding per `lost` session in `resume.plan`, a sweep state for the desktop, and the modified 0013/FR-008 marks in the dialog (D8, D9).

**Why this approach:**
- A process id alone is not an identity: ids are reused. Process id plus start time plus command line plus machine run names one process run, on every OS. Ending through a handle keeps an id that is reused after the check from being ended.
- The listener binds before the sweep, so the desktop app can show the sweep state (FR-007) and a slow sweep never looks like a dead server.
- The recorder scans the session trees that the stop code already walks (`process_tree.rs`), so helpers are known before an unclean stop takes the tree away.

**What it will not do:** end a process that has no stored identity, sweep without a `lost` session, end helpers after a normal end of a session, or sweep other machines (spec Out of scope).

**Shape of the work:** 6 tasks in one PR (#126, branch `spec/0017-orphan-sweep`), one commit per task. **Effort:** M.

**Main risks:**
- **Ending a wrong process.** Every value must match, the match is checked again just before each end request (FR-003), and the end goes through the verified handle where the OS has one (Linux 5.3 and later, Windows), never to a process group. A test proves each non-match reason (SC-002) and that the port has no end call by process id outside the handle (T1).
- **macOS and old kernels.** No process handle exists on macOS or on Linux before 5.3. There the sweep reads again and then calls `kill(pid)`. A process that exits and whose id is reused in that short time can be ended wrongly. The start time there is whole seconds, so the same command within the same second would also match. The risk is accepted for these systems only and recorded in `research.md` (spec A10). Linux with a pidfd uses exact clock ticks and Windows uses the 100 ns creation time (D2).
- **Windows.** A detached process has no console, so the polite request does not exist. The sweep ends it by force at once, on the verified handle, and records "ended by force" (D6).
- **Agent-chosen command lines.** A command line can be very long and can hold control characters. The match uses its hash; the store keeps a capped display text; the action record shows it in visible form; open helper rows are capped at 200 per session (D2, D3, D4, D6).

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
- **Crates:** `nix` 0.28 (`default-features = false`), `windows-sys` 0.61.2 (in `private-fs` and `console-control`), and `sha2` (workspace) are in the workspace; `libc` is a dependency of `nix` but not a direct one. No crate reads process start time or command line on all three OS.

## Decisions

- **D1 Process inspection and ending.** Use `sysinfo` 0.40.1 (MIT, MSRV 1.95, released 2026-10-30, the newest release at least two weeks old on 2026-11-23) with `default-features = false, features = ["system"]`, only to read (`research.md`). Each piece has one job:
  - `sysinfo`: the process list and parent links for `descendants` on all three OS; the start time and command line on Linux and macOS; the boot time on macOS and Windows. On Windows it is not used for the identity of one process (see below), because the identity must come from the handle that ends the process.
  - Linux: `/proc/sys/kernel/random/boot_id` for the machine run; `libc::syscall` with `SYS_pidfd_open` and `SYS_pidfd_send_signal` for the handle (`nix` 0.28 has no wrapper; T1 adds `libc` as a direct dependency of the adapter crate and checks the wrapper first); `nix::poll` on the pidfd; `nix::sys::signal::kill` only for the old-kernel fallback.
  - Windows: `windows-sys` 0.61.2 with `Win32_Foundation`, `Win32_System_Threading` (`OpenProcess`, `GetProcessTimes`, `TerminateProcess`, `GetExitCodeProcess`), and `Wdk_System_Threading` (`NtQueryInformationProcess` with `ProcessCommandLineInformation`, class 60, Windows 8.1 and later). All of them run on one handle opened with `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE`. T1 confirms the feature names; if the class 60 call fails, the identity is "not readable" and the process is never ended.
  - macOS: `sysinfo` to read and `nix::sys::signal::kill` to end (no handle).
  - Hash: `sha2` (workspace), SHA-256 of the whole command line.
  The recorder and the sweep use the same reader for a given OS, so the stored and the live command line have the same form. If T1 finds that a same-user command line cannot be read on macOS or Windows in CI, T1 stops and the plan goes back to the gate.
- **D2 Identity, handle, and match.** `ProcessIdentity { pid: u32, started_at: u64, command_hash: [u8; 32], command_display: String, boot: String }`, and a port type `ProcessHandle` (D1) that holds the process and offers `identity()`, `request_end()`, and `force_end()`.
  - `started_at`: Linux, the raw start time in clock ticks after boot (`start_time_raw`), exact. Windows, the `GetProcessTimes` creation time in 100 ns units (FILETIME as `u64`), stored at full precision. macOS, whole seconds, which is stable between reads. The adapter is the only writer and reader, so the unit never crosses a boundary.
  - `boot`: Linux, the text of `/proc/sys/kernel/random/boot_id`, compared for equality (no tolerance). macOS and Windows, the boot time in whole seconds as text; two values are the same machine run when they differ by at most 2 seconds.
  - `command_hash`: SHA-256 of the command line as the reader returns it (arguments joined with a single space on Linux and macOS; the raw command line text on Windows). `command_display`: the first 1,000 characters of the same text, kept raw; the visible form is made when it is shown (D6, D9). The full line is not stored (spec FR-001, M2). An empty command line (not readable) is never a match.
  - A process in the zombie state counts as not running.
  - Match: the process has the same start time and the same command hash, and the machine run is the same. The result of a non-match names the first value that differs, in this order: not readable; not running; on Linux a different `boot_id` gives "machine restarted"; other start time; other command line; and on macOS and Windows, when pid, start time, and command all match but the boot times differ by more than 2 seconds, "boot time changed". The last case is not ended and counts as `may_run` (D8), because a clock correction can move the boot time (spec Match, FR-005). The "not running" and "other start time" checks come before the boot check on macOS and Windows, so a real restart usually gives no warning there.
- **D3 Storage.** Agent migration `0008_session_processes.sql`: table `agent_session_processes (id INTEGER PRIMARY KEY, session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE, pid INTEGER NOT NULL, started_at INTEGER NOT NULL, command_hash BLOB NOT NULL, command_display TEXT NOT NULL CHECK (length(command_display) <= 1000), boot TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('cli','helper')), state TEXT NOT NULL CHECK (state IN ('open','ended')), UNIQUE (session_id, pid, started_at))`. The table lives in the office database, whose file is private to the owner (0001/FR-009), so FR-001's privacy needs no new code; a test checks the mode on Unix. `started_at` as `INTEGER` holds a Linux tick count, a macOS second, or a Windows FILETIME; a FILETIME is below 2^63, so SQLite's signed 64-bit integer holds it (a test stores one). The unique key stays `(session_id, pid, started_at)`: an exec keeps all three, so the recorder updates the row in place (D4). Repository: `insert_open`, `update_command`, `drop_helper`, `count_open_helpers`, `end_for_session`, `end_one`, `open_for_lost_sessions`.
- **D4 Recorder.**
  - The terminal runtime reads the CLI identity right after the spawn and returns it with the started terminal; the launcher stores it before it publishes the session as started (FR-001). A failed read stores nothing and logs it; that session later counts as "no complete stored process identities" (0013/FR-008 modified).
  - A recorder task scans every 5 seconds (below the 10 seconds of FR-002 and FR-011): for each running session it lists the descendants with the port and stores each new identity as `helper`. A helper that left the tree stays stored while it matches (FR-002).
  - For each stored open row of a running session, the recorder opens a handle and reads the identity. Same pid and start time, same command hash: nothing. Same pid and start time, another command hash (exec in place, `sh -c 'exec node ...'`, npm and npx wrappers): `update_command` stores the new hash and display text, and the row stays open (FR-011). Another start time or no process: a helper row is dropped (`drop_helper`, FR-002); a CLI row is left for the session end (FR-008). Dropping a helper keeps the table small: only open rows of running sessions and the rows of ended sessions (at most 201 per session) stay.
  - Helper limit (FR-010): the constant `MAX_OPEN_HELPERS_PER_SESSION = 200`. Before an insert, the recorder checks `count_open_helpers`. At the limit it stores no new helper of that session, writes one action record entry (action `helper_limit`, actor office, detail names the limit) and one log line, and sets a flag for the session in memory that stops the entry from repeating. After a server restart the count is checked again, and at the limit the entry is written once more. At a sweep, a session whose stored open helpers are at the limit has the finding `may_run` (D8); the recorder never removes a stored helper to make room.
  - When a session ends while the server runs, `end_for_session` runs in the same step that ends the session row (FR-008). A clean server stop ends every session first, so nothing stays `open`.
- **D5 Sweep timing.** `bootstrap::start` still marks `lost` before the bind. After the bind, `serve_until_stopped` starts the sweep task when `open_for_lost_sessions` is not empty. A `SweepGate` (a `tokio::sync::watch<SweepState>`) holds `running` until the sweep ends, then `done`.
- **D6 Sweep run.**
  - For each stored identity, in parallel (spec A5): open a handle for the pid, read the identity on it, and compare. On Linux the order is: `pidfd_open`, read, then a non-blocking poll of the pidfd. If the pidfd shows that the process has exited, the result is "not running", because the read may have seen a new process that took the id. On Windows the handle is opened once and the creation time and the command line come from it. On a non-match, record `left` with the D2 reason and mark the identity `ended` (FR-005, FR-009).
  - On a match, end through the handle (spec A10 for the exceptions):
    - Linux 5.3 and later: `pidfd_send_signal` with SIGTERM to the pidfd, then wait the stop grace period, checking every 100 ms. If the pidfd shows the process has not exited, read the identity again on the same pidfd, and if it still matches send SIGKILL through the same pidfd. A pidfd never points to another process, so an id that is reused is never signaled. `ESRCH` on the pidfd means the process has gone: "ended".
    - Windows: the polite request does not exist for a detached process, so the request step is skipped. Compare the creation time and the command line on the handle, then `TerminateProcess` on the same handle, wait for it with `WaitForSingleObject` up to the grace period, and read the exit state with `GetExitCodeProcess`. The result is "ended by force".
    - macOS, and Linux where `pidfd_open` returns `ENOSYS`: read again, then `kill(pid, SIGTERM)` to the pid only, never to the group; after the grace period, read again and `kill(pid, SIGKILL)`. This is the residual risk of spec A10. The adapter logs once at start that it uses this path.
  - Result `ended`, `ended by force`, or `could not end` (still matching after the force, or the OS refused, with the OS error text). The mark and the action record entry are written in one database transaction (FR-004, FR-006, FR-009). The audit and agent tables share the office database; T3 confirms this.
  - Idempotency key (spec FR-009), used only if T3 finds that the two writes cannot share one transaction: the entry detail starts with `identity <row id>:`. The sweep writes the entry first and the mark second. At the start of a sweep for an identity, it looks up an entry with that key. If one exists, it only marks the identity `ended`, and it ends and records nothing. A crash between the end and the entry gives one "left: not running" entry later. A crash between the entry and the mark gives the key lookup. Either way there is at most one entry and, with the lookup, exactly one. T3 records the case as a Deviation.
  - Action `end_leftover`, actor office, detail `"<result>: pid <pid>, session <id>, <command in visible form>"` or `"left: <reason>: pid ..."`. Visible form: the display text with every control, escape, and bidirectional-control character written as `\u{..}`, then, if the stored line had 1,000 characters or more, an ellipsis and the first 8 hex characters of the hash.
  - One notification `leftover_kept` per sweep that has a `could not end` result, naming the agents (FR-006, spec A8).
- **D7 Waiting requests** (FR-007). `agent.start`, `agent.new_task`, `session.continue`, `agent.resume`, and `resume.start` await the gate before they take the status order. `resume.plan` answers `{sweeping: true, entries: []}` while the gate is `running`. A server stop during the sweep stops the wait with the stopping refusal that 0001 already has.
- **D8 Sweep finding** (0013/FR-008 modified). `resume.plan` entries replace the boolean `after_unclean_stop` with `unclean_stop: null | {ended: n} | {may_run: reason}`. `may_run` covers: a `could not end`, `not readable`, or `boot time changed` result, no stored identity for a `lost` session, or a session with open helpers at the helper limit (D4). `ended: n` counts `ended` and `ended by force`. A `lost` session whose identities were all `left` (not running, machine restarted, other start time, other command line) and with none ended gives `null`. The old boolean stays in the answer as `true` exactly when `may_run` is set, so an older desktop app keeps its warning.
- **D9 Desktop.**
  - `office.sweep` event `{state: "running" | "done"}`, and the same state in the hello answer; the panel shows "Ending processes left from the last run" in a `role="status"` region while `running`.
  - The resume dialog shows the same text in place of the list while `sweeping`, then loads the plan when the event says `done`.
  - The dialog marks: "Ended N processes left from the last run" for `ended`, the 0013 mark and warning for `may_run`, nothing for `null`. The acknowledgement rule applies only to `may_run`.
  - The action record shows the new action text "ended a leftover process" with the detail, and the action text "stopped storing helpers" for `helper_limit`. The desktop escapes control and bidirectional-control characters again in the detail before it shows it, and renders it as text only.

## Scope

**In:** D1 to D9, `contracts/sweep-protocol.md` (the `resume.plan` change, the `office.sweep` event, the hello field, and the new action and notification), README notes, unit, integration, and e2e tests, `qa-report.md`, and the Turkish HTML report.

**Out:** the spec's Out of scope list.

## Tasks

### T1 Process inspection port and adapter (D1, D2)

- Port `ProcessInspector` in `M/terminal/application` (`open(pid) -> Option<ProcessHandle>`, `boot`, `descendants(pid)`); `ProcessHandle` offers `identity()`, `request_end()`, and `force_end()` and no call that takes a pid. Adapter in `M/terminal/infrastructure`: pidfd (Linux), process handle (Windows), pid-based handle (macOS and the old-kernel fallback).
- **Acceptance:** unit tests on each OS in CI: the identity of a spawned child reads the same twice; it matches itself; it does not match after the child ends; a child with the same command started later has another start time; `request_end` and `force_end` end only that process (a sibling with the same command keeps running). `started_at` on Windows has 100 ns precision (a value that is not a multiple of 10,000,000 is accepted) and a Windows FILETIME round-trips through the database integer.
- **End goes to the verified handle (M1):** on Linux 5.3 and later and on Windows, a handle opened on a child that then exits and is reaped returns "gone" from `request_end` and `force_end`, and signals no other process; a test starts a sibling after the exit and shows it unharmed. The port has no end function that takes a pid, which a compile check on the trait shows. A test forces the old-kernel path (the adapter has a constructor that treats `pidfd_open` as `ENOSYS`) and shows it still ends only the matching child and logs the fallback.
- The command hash is the same for the same line and differs for a line that differs after character 1,000 (M2).
- `cargo test -p terminal-infrastructure` passes. The boundary script passes. A command-line read that fails on macOS or Windows in CI stops the plan (D1).

### T2 Storage and recorder (D3, D4) - depends on T1

- Migration, repository, launcher change, recorder task, `end_for_session` in the session end path.
- **Acceptance:** repository unit tests (insert, unique, `update_command`, `drop_helper`, end one, end all of a session, open identities of `lost` sessions only, the 1,000-character check); migration test counts updated; an integration test with a fake CLI that starts a helper and detaches it shows both identities stored within 10 seconds, the helper kept after it leaves the tree, and every identity `ended` after a normal stop and after a clean `server.restart` (SC-006 part). The office database file mode test on Unix passes.
- **Exec (M3):** a fake CLI whose child replaces itself with another program (`exec`) while the session runs: the stored row for that pid and start time takes the new hash and display text, is not marked ended, and no insert fails on the unique key; after a forced kill of the server the sweep ends it (SC-009).
- **Helper limit (M2, SC-008):** a fake CLI that starts 500 long-lived helpers leaves 200 open helper rows, one `helper_limit` entry, and after a forced kill of the server the finding is `may_run`; short-lived helpers are dropped and do not count.
- **Command text (M2):** a helper with a 2 MB argument and control and bidirectional characters stores a 1,000-character display text and the hash of the whole line; the action record detail has no raw control character.

### T3 Sweep and gate (D5 to D7) - depends on T2

- `SweepGate`, the sweep task, the waiting requests, the `leftover_kept` notification, the action text, the handle-based end (D6), and the check that the audit and agent tables share one transaction (D6, FR-009).
- **Acceptance:** `tests/server_sweep.rs` (Unix with the `ignores-hangup` stand-in and a detached helper; Windows with a detached stand-in) proves:
  - after a forced kill of the server, every leftover ends before `resume.plan` answers with entries, with one `end_leftover` entry each (SC-001 at 30 agents with 1 helper each, SC-004);
  - no end for a process with the same command and another start time, a stored id now used by another command, and a process the office did not start (SC-002);
  - the list answers within 30 seconds with 30 `lost` sessions (SC-003);
  - a stand-in that ignores SIGTERM is `ended by force`; a process the office cannot end gives `could not end` and one notification (FR-006);
  - a start, a new task, and a continue sent during the sweep run after it (FR-007);
  - a forced kill of the server during the sweep, then a start: every leftover ends and each identity has exactly one result entry (SC-007);
  - 10 clean stops and 10 normal stops end 0 processes and start 0 agents (SC-006);
  - the sweep ends through the handle (M1): the port is the only end path in the sweep code, and a fake `ProcessInspector` test shows that a handle that reports "gone" after the last check gets no end call by pid;
  - the boot rules (L1): a changed `boot_id` gives `machine restarted`; with a fake macOS-style inspector, a boot time that differs by 3 seconds with all else equal gives `left: boot time changed` and `may_run`, and the same with another start time gives `other start time` and `null`;
  - the idempotency key route (L2) when the single transaction is not possible: a crash injected between the entry and the mark, then a second sweep, gives exactly one entry for the identity.

### T4 Resume plan finding (D8) - depends on T3

- `resume_requests.rs` builds `unclean_stop` from the sweep results; the old boolean kept for older apps.
- **Acceptance:** integration test with 4 agents (one leftover ended, one that cannot be ended, one CLI that ended on the hang-up, one session with no stored identity): `ended: 1`, `may_run`, `null`, `may_run` (SC-005 server side). A fifth session at the helper limit gives `may_run`. The 0013 `bulk_resume` and `server_crash` tests still pass, updated to the new field.

### T5 Desktop (D9) - depends on T4 for the contract only

- Sweep state service, panel status text, dialog sweeping state and marks, action record text, mock-core.
- **Acceptance:** Vitest for the model (finding to mark and warning) and the sweep state; Playwright `e2e/sweep.spec.ts`: the panel text while running and gone after `done`, the dialog text in place of the list, the three marks, Confirm gated only by `may_run`, the action record entry, with a control character in the command shown as an escape and a long command cut with an ellipsis and the hash start; `bun run check`, `lint`, `format:check`, `test`, and Playwright pass.

### T6 Docs and evidence - depends on T1 to T5

- `contracts/sweep-protocol.md`; the 0013 contract and the 0001 data model amended where `resume.plan` and the action list are described; README section; `qa-report.md` with each FR, SC, and PB; Deviations if any.
- **Acceptance:** each FR, SC, and PB has a qa row with evidence. The no-other-product grep of the diff is clean.

## Traceability

| Spec item | Tasks | Evidence |
|---|---|---|
| FR-001 | T1, T2 | recorder test, file mode test, hash and display text test |
| FR-002 | T2 | detached helper test |
| FR-003 | T1, T3 | `server_sweep.rs` leftovers and re-check, end-through-handle test |
| FR-004 | T3, T5 | one entry per ended leftover, visible form in the entry and the e2e |
| FR-005 | T1, T3 | non-match reasons test, boot rules test |
| FR-006 | T3 | could not end and one notification |
| FR-007 | T3, T5 | waiting requests test, e2e panel and dialog text |
| FR-008 | T2 | ended after a stop and a clean restart |
| FR-009 | T3 | kill during the sweep test, idempotency key test |
| FR-010 | T2, T4 | helper limit test, fifth-session finding |
| FR-011 | T2 | exec update test |
| 0013/FR-008 (modified) | T3, T4, T5 | four-agent test, boot rules test, e2e marks |
| PB-1 | T3 | no agent starts after the sweep (`server_process.rs` and SC-006) |
| PB-2 | T3 | the sweep writes only the office database; review of the diff |
| PB-3 | T2 | existing stop tests pass unchanged |
| PB-4 | T1, T3 | SC-002 test |
| PB-5 | T3, T4 | `bulk_resume.rs` passes after the sweep |
| PB-6 | T4 | a `lost` session continues after the sweep |
| SC-001 to SC-007 | T3, T4, T5 | as named in the task acceptance |
| SC-008 | T2, T4 | helper limit test |
| SC-009 | T2, T3 | exec then forced kill test |

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright.
- The repository's boundary check script.
- CI green on Linux, macOS, and Windows.
- Hands-on: a real CLI agent and its helper (the `opencode` service) ended by the sweep after a SIGKILL of the server, in the real app. Otherwise the qa row stays BLOCKED, never PASS.

## Execution notes

- **T1, Linux reads `/proc` directly.** `sysinfo` 0.40.1 keeps the raw start ticks private and gives whole seconds only, which D2 rules out on Linux. Linux reads `/proc/<pid>/stat` (start ticks, state) and `/proc/<pid>/cmdline`; `sysinfo` still lists descendants on every OS and reads the identity on macOS.
- **T1, `ProcessHandle::wait_for_exit(timeout)`.** D6 checks every 100 ms for an exit; on Linux it polls the pidfd and on Windows it waits on the handle, so no wait goes by process id. The Windows handle therefore also asks for `SYNCHRONIZE`.
- **T1, end results.** `request_end` and `force_end` answer `Delivered`, `Gone`, or `Unsupported` (Windows has no request), or the OS error text.
- **T1, the match is in the identity type.** `ProcessIdentity::mismatch` gives the D2 reasons in the D2 order; the sweep (T3) only applies it. The visible form of the command line stays with T3 and T5 (D6, D9); T1 stores the capped raw display text and the hash.
- **T1, unsafe code.** `terminal-infrastructure` denies unsafe code; only the pidfd and Windows handle modules allow it, each with its reason.
- **For T2.** On Linux the command line can be empty for a moment right after a spawn, which reads as "not readable". The recorder retries the first read briefly before it gives up.

## Review log

r1 (Efe, fa44fb0): M1 to M3, L1, L2 resolved as below. r2 (Efe, 350ff07): PASS for coding; N1 (macOS FR-011 residual under A10) is checked in the code review.

- M1: handle-based end in D1 (crates and APIs), D2 (`ProcessHandle`, 100 ns Windows start time), D6 (pidfd, Windows handle, macOS and old-kernel fallback), Summary and Main risks; tests in T1 and T3; `research.md` accepted risk is now macOS and old Linux only.
- M2: D2 and D3 (hash plus capped display text), D4 (helper limit of 200), D6 (visible form), D9; spec FR-001, FR-004 to FR-006, FR-010, key entities, edge cases, SC-004 and SC-008; tests in T2 and T5.
- M3: D4 (`update_command`, unique key kept) and D3; spec FR-011, A2, A4, edge case and SC-009; test in T2.
- L1: D2 (`boot_id` on Linux, `boot time changed` as `may_run` elsewhere), D8, spec Match, FR-005, edge cases, and `research.md`; test in T3.
- L2: D6 (idempotency key route when one transaction is not possible), T3 test; FR-009 is unchanged.
