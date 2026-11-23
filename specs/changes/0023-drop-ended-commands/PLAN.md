---
status: approved
spec: specs/changes/0023-drop-ended-commands/SPEC.md
commits: per-task
---

# Plan: Keep no command lines of ended sessions (0023)

<!-- Pre-approved by god on 2026-11-23: a plan that follows the spec and Efe's two notes from spec review r2. -->

## Summary

After this change, the office deletes the stored process identities of a session that ends normally. It deletes them in the same step that ends the session. It also clears the database file so that no deleted command line text stays readable in freed space or in the write log.

**What you get:**
- **Secure delete.** `PRAGMA secure_delete = ON` on every office connection. SQLite then zeroes the freed cells and pages of every delete and update. This covers a dropped helper, an `exec` replacement, a deleted agent and the removals below.
- **Removal at a normal session end.** The session end deletes the session's identities instead of marking them ended (0017/FR-008 modified).
- **Removal when a new session is stored.** Storing a new session deletes the ended identities of the agent's earlier `lost` sessions (FR-001).
- **Cleanup at each server start.** After open sessions are marked `lost`, a cleanup deletes the leftover rows (FR-002). Then `PRAGMA wal_checkpoint(TRUNCATE)` runs. A busy result counts as not finished: the server logs it and tries again at the next start (FR-004, FR-006).
- **One rebuild at the first start of this build.** `VACUUM` runs once, followed by a TRUNCATE checkpoint. A stored mark records that this finished, so a failed or busy run is tried again (FR-005, FR-006, Efe r2).

**Why this approach:**
- Secure delete is one pragma, with negligible cost at this database size, and it covers every way a text leaves the store, including the running-session cases of M1.
- The checkpoint at start is what makes "by the next start" true. Old pages stay in the write log until the log is reset.
- `VACUUM` runs once because its cost grows with the database. Only space freed before secure delete was on needs it (spec A9).

**Delivery:** one PR (#136, with the spec), 3 tasks, committed per task.

## Context

- Writers and readers are cited in the spec's Context at main 5272d6f:
  - `session_process_repository.rs` (insert, the `exec` update, helper drop, end, finding);
  - `session_repository.rs:179-221` (session end, and `lost` at start);
  - `bootstrap/services.rs:123-124` (the `lost` marking);
  - `orphan_sweep.rs`, `leftover_sweep.rs`, `leftover_ledger.rs`, `resume_requests.rs` and `unclean_stop.rs`.
- Connection pragmas: `src/apps/office-harnessd/src/bootstrap/database.rs:12-14` (`journal_mode = WAL`, `foreign_keys`, `busy_timeout`). Every connection is opened there.
- Agent migrations end at `0009_agent_temp.sql`. The next agent migration is `0010`.

## Decisions

- **D1: Secure delete.** Add `PRAGMA secure_delete = ON;` to the connection pragmas in `database.rs`. Every pooled connection gets it.
- **D2: 0017/FR-008.**
  - In the session-end transaction (`session_repository.rs:179-203`), replace the "mark open identities ended" call (`session_process_repository.rs:226-232`) with a delete of all identities of that session.
  - The `lost` marking at start (`:213-221`) stays as it is, so open identities of `lost` sessions survive for the sweep.
- **D3: FR-001.** In the transaction that stores a new session, delete the ended identities of the same agent's earlier sessions that ended as `lost`.
- **D4: FR-002, FR-004 and FR-006 at each start.**
  - In `bootstrap/services.rs`, right after the `lost` marking and before the sweep, delete in one transaction:
    - every identity of a session that did not end as `lost`;
    - every ended identity of a `lost` session that is not its agent's last session.
  - Then run `PRAGMA wal_checkpoint(TRUNCATE)` and read its result row. A busy value of 1, or an error, means not finished: the server writes one log line with the reason and goes on.
  - The next start repeats both steps, because they run at every start.
- **D5: FR-005, the one-time rebuild.**
  - Migration `0010_agent_maintenance.sql` adds `agent_maintenance (task TEXT PRIMARY KEY, done_at INTEGER NOT NULL)`.
  - At start, after D4: when no row `freed_text_cleared` exists, run `VACUUM`, then `PRAGMA wal_checkpoint(TRUNCATE)`. Insert the row only when both finished, which means no error and no busy result.
  - Otherwise, write the reason to the log and try again at the next start (Efe r2).
  - `VACUUM` needs no open transaction and no other connection writing, so it runs before the server takes requests.
  - As built (T2): D4 and D5 share one TRUNCATE checkpoint, run after the `VACUUM` when the rebuild is pending. Two checkpoints would log a busy result twice on the first start; one checkpoint after the rebuild still leaves the write log empty, because the rebuild's old pages stay in the log until it is truncated.
- **D6: No other change.** The sweep, the match, the finding, the action record text and the wire protocol stay as they are (PB-1 to PB-6).
- **D7: Constitution check.**
  - The removals stay inside the agent module's persistence, and the start sequence is in the server bootstrap, so no module boundary changes.
  - The new migration is additive.
  - The spec and the plan ship in this PR.

## Scope

**In:** D1 to D5, the tests, the qa-report, and a README note about secure delete if the README describes the database.

**Out:**
- disk blocks outside the database files, swap, and journals (spec A8);
- other stored data;
- the text of running sessions.

## Tasks

### T1: Secure delete, and removal at a session end and at a new session (D1, D2, D3)
- **Files:** `bootstrap/database.rs`, `session_repository.rs`, `session_process_repository.rs` and their tests.
- **Acceptance:**
  - Unit or repository tests show:
    - a stop, a CLI end, a start failure, a sign-in end and a clean server stop each leave 0 identities of the ended session (SC-001, 0017/FR-008);
    - a new session removes the ended identities of earlier `lost` sessions and keeps open ones (FR-001, FR-003);
    - `secure_delete` reads 1 on a pooled connection.
  - The existing 0017 tests stay green. The tests that expected ended rows after a normal end are updated to the new rule, and each update is named in the report.
- **Depends on:** none.

### T2: Start cleanup, checkpoint, and the one-time rebuild (D4, D5)
- **Files:** `bootstrap/services.rs`, a small maintenance module next to it if needed, the agent migration `0010` and its repository, and tests.
- **Acceptance:**
  - An integration test restarts the server and checks each of these:
    - the normally ended rows and the non-last `lost` ended rows are gone after the start (FR-002);
    - the rows of a last `lost` session and the open `lost` rows are kept (FR-003);
    - the `-wal` file is empty or truncated after the start;
    - the mark row exists after the first start, and a second start does not run `VACUUM` again;
    - a forced busy checkpoint (a second connection holding a read transaction) leaves no mark and writes one log line, and the next start finishes (FR-006).
- **Depends on:** T1.

### T3: Byte-scan evidence and the QA report
- **Files:** integration tests in `src/apps/office-harnessd/tests/` (a new `drop_ended_commands.rs`), and `specs/changes/0023-drop-ended-commands/qa-report.md`.
- **Acceptance:**
  - Each byte scan covers the `office.db`, `-wal` and `-shm` files and finds 0 markers:
    - SC-001 and SC-002: 20 sessions, each with a unique marker;
    - SC-007: 10 helpers that end mid-session;
    - SC-008: 10 `exec` replacements with the marker in the old command line.
  - SC-003: a database prepared as the 0017 build left it (secure delete off; rows inserted, then dropped, replaced and ended) holds 30 markers before the first start of this build and 0 after it.
  - SC-004 and SC-006: the 0017 sweep tests stay green.
  - SC-005: after a resume of the `lost` agents, 0 ended identities remain and the sweep entries keep their text.
  - Reduced counts are PARTIAL with the reason.
- **Depends on:** T1 and T2.

## Traceability

| Criterion | Task | Evidence |
|---|---|---|
| FR-001 | T1 | new-session removal test |
| FR-002 | T2 | start cleanup test |
| FR-003 | T1, T2 | keep tests (open `lost`, last `lost`) |
| FR-004 | T2, T3 | checkpoint test; SC-002, SC-007 and SC-008 byte scans |
| FR-005 | T2, T3 | mark test; SC-003 byte scan |
| FR-006 | T2 | busy checkpoint test (log line, no mark, retry) |
| 0017/FR-008 | T1 | session-end removal tests for every end kind |
| SC-001 to SC-008 | T1 to T3 | as listed in the tasks |
| PB-1, PB-2 | T3 | 0017 sweep and resume.plan tests stay green, plus SC-004 |
| PB-3 | T3 | SC-005 checks the entry text |
| PB-4 | T3 | the existing 0017/SC-006 test stays green |
| PB-5 | T1 | the existing recorder, helper and `exec` tests stay green |
| PB-6 | T3 | the existing continue tests stay green |

## QA and evidence

- Gates, run by the lead on the final tree:
  - `cargo fmt --all --check` and `bash scripts/check-boundaries.sh`;
  - `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings`;
  - `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=4`, with the pass and fail counts summed from the result lines.
- The new integration tests run 3 times.
- No desktop file changes, so the desktop gates are NOT RUN, with that reason.
- No real CLI is needed: the stand-in CLIs and helpers carry the markers.

## Final verification

Run `/ac:verify` against this plan. Then CI must be green on the 3 OSes, and Efe's code gate runs at the exact head.

## Result

T1 to T3 are done, one signed commit for each task. T2 runs one TRUNCATE checkpoint for D4 and D5 (see D5). The readiness check found two log lines on success, which A6 rules out; they are removed, and the rebuild tests read the free list instead. It also asked for the case of a cleanly stopped 0017 database, which a second SC-003 test now covers. `main` 15c8cd3 (0022) is merged. The gates pass on the final tree: 1811 tests passed, 0 failed. The byte scans ran with the stated counts and found 0 markers in `office.db`, `-wal` and `-shm`; SC-003 found 30 of 30 markers before the first start of this build and 0 after it. FR-006 is PARTIAL: the busy checkpoint is tested, and a failed `VACUUM` is only logged. No spec deviation.
