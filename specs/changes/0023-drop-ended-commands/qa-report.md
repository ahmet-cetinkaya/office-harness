# 0023 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `spec/0023-drop-ended-commands` with T1 to T3, `origin/main` 15c8cd3 (0022) merged, and the fixes after the readiness check: no log line on a successful removal or rebuild (A6), and a second SC-003 case for a cleanly stopped database. The Rust gates ran on that tree.
- **Overall status:** automated tests prove every requirement. The criteria with counts (SC-001 to SC-003, SC-007, SC-008) ran with the stated counts of sessions and markers, so they are PASS. SC-004 and SC-006 are the 0017 tests, unchanged and green; SC-004 is also checked in `drop_ended_commands.rs` with its second start. SC-005 ran with 4 agents (SC-004 setup). The timing parts of SC-007 and the real-time bounds that the sweep tests carry are as in 0017.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| NOT RUN | No check ran. The evidence column says why. |

### Scope of the automated evidence

- `tests/drop_ended_commands.rs` runs the server binary as a process, with fake CLIs (Unix shell scripts) that start helpers. Each marker sits in the command line of a helper only. No terminal output shows it (the fake CLIs print `ready`).
- Each byte scan reads the raw bytes of `office.db`, `office.db-wal`, and `office.db-shm` (each file that exists) and looks for each marker. It is taken at the point that the SC states: after the next server start, which the test waits for through the `server started` log line (that line comes after the start cleanup).
- Honesty check of every scan: before the ending, the test waits until each marker is both stored in a stored process identity (SQL `instr` on `command_display`) and found in the raw bytes of the files. A test fails when a marker is missing at that point, so a 0 never comes from a marker that was never stored. SC-003 asserts 30 of 30 markers in the files before the first start of this build.
- The database of SC-003 is built as the 0017 build left it: `secure_delete` off, rows inserted, then dropped (deleted), replaced (updated to another text), and ended. The test keeps the write log of that build: it copies the database file and the `-wal` file before it closes the connection, and puts them back after the close, because the close would checkpoint and delete the log. The copy is the image of a server that stopped without closing its connection. A second case closes the connection instead, which checkpoints the log into the database file, so the freed text sits in the free cells and pages of the file; the close and the later inserts overwrite some of it, so that case asserts at least 1 marker before the start, not 30. The database still holds the tables of 0010 (the test removes the rebuild mark, so the first start of this build runs the rebuild).
- SC-008 replaces the command line of a helper through `exec` (as the 0017 test does), not the command line of the CLI itself.
- The scans do not cover disk blocks outside the three files, swap, or file system journals (spec A4).
- The VACUUM error path is only logged. No test makes the VACUUM fail; the busy checkpoint is tested (FR-006).
- The Rust gates exclude the `office-harness-desktop` crate. This change does not touch the desktop, so the desktop gates are NOT RUN.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `bash scripts/check-boundaries.sh` | workspace | PASS (`boundaries ok`) |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` | workspace | PASS, no warnings |
| `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=4` | workspace | PASS: 1811 passed, 0 failed, 111 result lines, on the tree with `main` 15c8cd3 merged (before the merge, after T3: 1772 passed, 110 result lines) |
| New integration tests, 3 runs | `tests/drop_ended_commands.rs` (5 tests) | PASS: 3 of 3 runs, 5 passed, 0 failed each |
| Desktop gates | `src/apps/desktop` | NOT RUN: no desktop file changed |

### Existing tests that this change changed (T1)

| Test | Change |
|---|---|
| `every_normal_end_of_a_running_session_deletes_its_rows_in_the_same_step` (`src/modules/agent/infrastructure/tests/persistence.rs`) | Replaces `the_end_of_a_running_session_ends_its_rows_in_the_same_step`. The old test expected ended rows after a normal end; the new one expects none, for each kind of normal end. |
| `a_stop_deletes_every_stored_identity_of_the_session` (`tests/session_processes.rs`) | Now asserts that no row remains after a stop (it asserted ended rows). |
| `a_clean_server_restart_deletes_every_stored_identity` (`tests/session_processes.rs`) | Now asserts that no row remains after a clean server restart (it asserted ended rows). |

### Note on T2

D4 (the checkpoint after the start deletes) and D5 (the checkpoint after the rebuild) share one `wal_checkpoint(TRUNCATE)`, run after the `VACUUM` when the rebuild is pending. Two checkpoints would log a busy result twice on the first start. One checkpoint after the rebuild still leaves the write log empty.

## Success criteria

| SC | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `twenty_sessions_that_end_normally_leave_no_identity_and_no_marker_after_the_next_start` (`tests/drop_ended_commands.rs`): 20 sessions, each with a CLI and a helper with a unique marker; 10 stops by the person (`agent.stop`), 5 ends of the CLI (a gate file ends the script), 5 by a clean server stop (`server.restart`). The test reads 0 stored identities of the session right after each stop and each CLI end, and 0 of all 20 at the end. Unit level: `every_normal_end_of_a_running_session_deletes_its_rows_in_the_same_step`; `a_stop_deletes_every_stored_identity_of_the_session`, `a_clean_server_restart_deletes_every_stored_identity` (`tests/session_processes.rs`). |
| SC-002 | PASS (automated) | Same test, after the second start: 0 of 20 markers in the three files. Before the ends: 20 of 20 markers stored and found in the files. |
| SC-003 | PASS (automated) | `a_database_of_the_0017_build_holds_no_marker_after_its_first_start_of_this_build`: 10 markers in ended identities of 10 sessions, 10 in dropped helper identities, 10 in replaced command lines. 30 of 30 found in the files before the first start of this build; after it, 0 of 30, 0 stored identities, 0 `end_leftover` entries (no leftover process ended). `a_cleanly_stopped_database_of_the_0017_build_holds_no_marker_after_its_first_start`: the same rows after a clean stop; at least 1 marker in the database file before the start, 0 after. |
| SC-004 | PASS (automated) | The 0017 test `the_plan_says_for_each_lost_session_what_the_sweep_found` (`tests/server_sweep.rs`) stays green and unchanged. `after_the_resume_no_ended_identity_of_the_lost_sessions_remains_and_the_sweep_entries_keep_their_text` repeats the 4 agents and checks the marks after the sweep and after a second start without a resume (the note for 1 agent, the warning for 2, no mark for 1). |
| SC-005 | PASS (automated) | `after_the_resume_no_ended_identity_of_the_lost_sessions_remains_and_the_sweep_entries_keep_their_text`: `resume.start` for the 4 agents; the 4 lost sessions held identities before it and hold 0 after it; the `end_leftover` entries (actor, agent, result, detail) are the same list before and after. |
| SC-006 | PASS (automated) | The 0017 test `thirty_stubborn_agents_with_helpers_end_by_force_before_the_plan_lists_them` (`tests/server_sweep.rs`) stays green and unchanged (60 processes). |
| SC-007 | PASS (automated) | `ten_helpers_that_end_while_their_session_runs_leave_no_marker_after_the_next_start`: 10 sessions, each with a helper that ends 15 seconds after it starts; all 10 sessions still run when the helpers are dropped; 10 of 10 markers stored and found in the files before the drop; after the clean stop and the next start, 0 of 10 in the three files. |
| SC-008 | PASS (automated) | `ten_replaced_command_lines_leave_no_old_marker_after_the_next_start`: 10 helpers replace themselves by `exec` 12 seconds after they start (the 0017 test uses 7); 10 of 10 old markers stored and found before; each row takes the new text `sleep ...` without the marker; after the next start, 0 of 10 in the three files. |

## Requirements

| Id | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `a_new_session_deletes_the_ended_rows_of_earlier_lost_sessions_and_keeps_open_ones`, `a_new_session_leaves_the_ended_rows_of_another_agents_lost_session` (`persistence.rs`); SC-005 test (the resume stores a new session). |
| FR-002 | PASS (automated) | `a_start_deletes_the_ended_rows_that_the_rules_do_not_keep_and_keeps_the_rest` (`tests/start_maintenance.rs`); SC-003 test. |
| FR-003 | PASS (automated) | The same start test (open rows of a `lost` session and the rows of the last `lost` session stay); `a_new_session_deletes_the_ended_rows_of_earlier_lost_sessions_and_keeps_open_ones`; the SC-004 and SC-005 test (the last `lost` sessions keep identities until the resume). |
| FR-004 | PASS (automated) | `a_start_truncates_the_write_log` (`tests/start_maintenance.rs`); `the_office_connection_zeroes_freed_content` (`tests/startup.rs`); the SC-002, SC-007 and SC-008 byte scans. Not covered: copies in disk blocks outside the three files (A4). |
| FR-005 | PASS (automated) | `a_pending_rebuild_empties_the_free_list_and_writes_the_mark`; `the_rebuild_mark_is_written_once_and_the_next_start_skips_the_rebuild` (the mark is unchanged and the free list is not emptied, so no rebuild ran); the SC-003 byte scans. A successful rebuild writes no log line (A6), so the tests read the free list. |
| FR-006 | PARTIAL (automated) | `a_busy_checkpoint_leaves_no_mark_and_the_next_start_finishes`: one log line, no mark, the next start finishes. The failure of the `VACUUM` itself is only logged and has no test. |
| 0017/FR-008 | PASS (automated) | `every_normal_end_of_a_running_session_deletes_its_rows_in_the_same_step` (stop, CLI end, start failure, sign-in end, clean server stop); `a_session_end_deletes_the_rows_of_its_own_session_only`; the SC-001 test. |
| PB-1 | PASS (automated) | The 0017 sweep tests in `tests/server_sweep.rs` stay green (SC-004, SC-006 rows). |
| PB-2 | PASS (automated) | `the_plan_says_for_each_lost_session_what_the_sweep_found`; the marks of the SC-004 test, also after a second start. |
| PB-3 | PASS (automated) | SC-005: the sweep entries have the same text after the resume. |
| PB-4 | PASS (automated) | `ten_clean_stops_and_ten_normal_stops_end_nothing_and_start_no_agent` (`tests/server_sweep.rs`) stays green. |
| PB-5 | PASS (automated) | The existing recorder tests in `tests/session_processes.rs` stay green (store within 10 seconds, helper kept while it runs, `exec` update in place, helper limit). |
| PB-6 | PASS (automated) | The existing continue tests stay green; the resume of the SC-005 test continues or starts a new session of a `lost` agent. |
