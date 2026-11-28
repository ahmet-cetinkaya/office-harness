# 0013 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0013-bulk-resume` with `main` c07f21c merged (0014 and the specs archive), the tree of the implementation commits (T1 to T5), toolchain 1.100.0 in CI and the local nix clippy 1.99. Every local gate below ran on that merged tree.
- **Overall status:** complete. The automated evidence stands for FR-001 to FR-008, PB-1 to PB-5, SC-002, SC-003, SC-005, and SC-006. SC-001 and SC-004 are PARTIAL: the behavior is proven, the stated counts (30 agents in 60 s, 10 restarts) were not run. Hands-on rows: the real `claude` resume after a SIGKILL and the 200% zoom and 360 px dialog are PASS (agent-run, 2026-11-23, `main` 468ad56); the screen reader row and the macOS and Windows rows are WAIVED (owner).
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (agent-run) | An agent ran the check by hand in the real app. The note says when and on which build. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| WAIVED (owner) | The owner accepted the row without a run. The note says when and why. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs. `tests/bulk_resume.rs` runs an in-process server with a 2-second check period (`ServerOptions::resume_check_period`); the product uses 20 seconds.
- `tests/server_crash.rs` and `tests/server_restart.rs` run the real server binary and kill it with SIGKILL. `server_crash.rs` is Linux only, because it reads process start times from `/proc`.
- The Playwright tests run against a mock core, so they prove the dialog, not the real server.
- The Rust gates exclude the `office-harness-desktop` crate, whose build script needs the prepared sidecar on this machine. CI builds it on all three OSes. This change does not touch that crate.
- Research R3 (the real CLIs at a terminal hang-up) was a hand check on Linux with throwaway homes. It is in `research.md`, not in the automated gates.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `cargo clippy --all-targets -D warnings` | workspace without `office-harness-desktop`; `-A clippy::must_use_candidate` only for a finding that the local clippy 1.99 raises on `main` too (`cli/run.rs`) | PASS (clean) |
| `cargo test` | workspace without `office-harness-desktop`: 1196 passed, 0 failed | PASS |
| `tests/bulk_resume.rs` and `tests/server_crash.rs` | 8 and 1 tests, run 6 times before the merge, once after | PASS (each run) |
| `scripts/check-boundaries.sh --self-test` and the check | repository | PASS (boundaries ok) |
| `bun test src` | desktop: 359 passed, 0 failed | PASS |
| Playwright | desktop, full suite on the mock core, Chromium: 231 passed, 0 failed; `e2e/resume.spec.ts` 10 of them | PASS |
| `svelte-check`, `eslint`, `prettier --check` | desktop: 497 files, 0 errors, 0 warnings; lint and format clean | PASS |
| CI | the PR | In the PR |

## Requirements

Paths are relative to the repository root. `H` is `src/apps/office-harnessd`, `M` is `src/modules`, `D` is `src/apps/desktop`. Server end-to-end tests are in `H/tests/bulk_resume.rs` unless named otherwise; desktop end-to-end tests are in `D/e2e/resume.spec.ts`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | Desktop: `the button shows the count of interrupted agents and is absent at zero`, `no interrupted agent, no button`. |
| FR-002 | PASS (automated) | Server: `a_cli_that_cannot_continue_starts_a_new_session_at_once_with_the_reason` (plan entry: `can_continue`, `reason`, `held`, `after_unclean_stop`); `H/src/server/resume_order.tests.rs` (plan order). Desktop: `the list shows each agent with its plan text and checks them all`; `D/src/modules/resume/state/resume-controller.spec.ts` (held agents unchecked, nothing starts before Confirm). |
| FR-003 | PASS (automated) | Server: `an_interrupted_agent_continues_its_last_session_and_the_result_is_continued`, `a_cli_that_cannot_continue_starts_a_new_session_at_once_with_the_reason`, `members_start_before_their_lead_and_the_lead_before_the_ceo`, `an_agent_that_is_no_longer_interrupted_is_skipped`. Unit: `M/agent/application/src/commands/resume_agent/resume_agent_handler.tests.rs` (continue, new session, conflict when not interrupted, unknown agent). Desktop: `unchecking one and confirming sends the others, with live results`. |
| FR-004 | PASS (automated) | Server: `a_refused_continue_gets_one_new_session_recorded_with_the_office_and_its_reason`. Unit: `the_fall_back_starts_a_new_session_after_a_continue_that_ended_by_itself`, `the_fall_back_is_a_conflict_after_a_stop`, `the_fall_back_is_a_conflict_when_a_newer_session_exists`, `the_reason_of_a_continue_that_did_not_start_carries_the_text`. |
| FR-005 | PASS (automated) | Server: `an_agent_that_fails_twice_is_started_twice_held_and_notified_once` (2 starts after the first run, `set_by: office` with the reason, 1 `resume_hold` notification). Unit: `M/hierarchy/application/.../hold_service.tests.rs` `an_office_hold_for_a_failed_resume_stores_the_reason_journals_and_notifies_once`, `an_agent_already_held_gets_no_second_hold_or_notification`, `a_failed_write_of_an_office_hold_leaves_memory_and_the_ports_untouched`; `H/src/bridges/notification_bridge.tests.rs` `a_resume_hold_row_carries_the_reason_as_its_detail_and_reaches_the_clients`; the Needs-you clear on release `a_hold_release_clears_an_auto_hold_or_resume_hold_item_only`. |
| FR-006 | PASS (automated) | Server: every `bulk_resume.rs` test checks the `resume.result` sequence (`checking`, then `continued`, `new_session`, `held`, or `skipped`). Unit: `H/src/server/resume_run.tests.rs`. Desktop: `unchecking one and confirming sends the others, with live results`; `D/src/modules/resume/models/resume.spec.ts`. |
| FR-007 | PASS (automated) | Server: `a_refused_continue_gets_one_new_session_recorded_with_the_office_and_its_reason` (office start entry with `detail`), `a_cli_that_cannot_continue_starts_a_new_session_at_once_with_the_reason` (user start entry with `detail`). Unit: `an_interrupted_agent_continues_its_last_session_and_the_record_has_no_detail`; audit `a_detail_reads_back_and_an_entry_from_before_the_detail_column_reads_as_none`, `a_detail_is_written_as_text`. Desktop: `the reason of a new session shows in the agent's action record`. |
| FR-008 | PASS (automated) | Server: `H/tests/server_crash.rs` (after a SIGKILL, `resume.plan` marks all 3 agents); `H/tests/server_restart.rs` `a_session_left_open_by_an_unclean_stop_ends_as_lost_at_the_next_start`; `M/agent/infrastructure/tests/persistence.rs` `sessions_still_open_at_the_next_start_become_lost`. Desktop: `an unclean stop marks the agent, warns, and holds Confirm until acknowledged`, `no mark, no warning`. |
| PB-1 | PASS (automated) | `H/tests/server_process.rs` `after_an_unclean_stop_running_agents_become_interrupted_and_none_starts`; `H/tests/server_crash.rs` (all agents `interrupted` after the restart, none started). |
| PB-2 | PASS (automated) | `H/tests/sessions.rs` (all 9 pass unchanged); `a_lost_session_is_continued_like_an_interrupted_one`. |
| PB-3 | PASS (automated) | `H/tests/agent_hold.rs` (all 10 pass unchanged); `agent.resume` is a user request outside the agent allowlist (`H/tests/agent_call.rs` `an_agent_may_not_ask_for_a_type_outside_the_allowlist` covers `resume.plan`, `resume.start`, `agent.resume`). |
| PB-4 | PASS (automated) | The change writes no CLI file; the fake CLIs run with a fake home. The existing configuration-folder tests pass unchanged. |
| PB-5 | PASS (automated) | FR-004, FR-005, and FR-007 evidence: continue, start, and hold entries with their actor. |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PARTIAL (automated) | Two activations (open, Confirm) and three after an unclean stop (with the acknowledgement): the desktop tests above. Starting is proven for up to 4 agents (`members_start_before_their_lead_and_the_lead_before_the_ceo`). 30 agents within 60 seconds was not run. |
| SC-002 | PASS (automated) | `a_refused_continue_gets_one_new_session_recorded_with_the_office_and_its_reason`: one office start entry with a reason, a running new session. The run takes about 2 check periods. |
| SC-003 | PASS (automated) | `an_agent_that_fails_twice_is_started_twice_held_and_notified_once`. |
| SC-004 | PARTIAL (automated) | One server start after a SIGKILL in each of `server_process.rs`, `server_crash.rs`, and `server_restart.rs` starts no agent. Ten restarts in a row were not run. |
| SC-005 | PASS (automated) | `members_start_before_their_lead_and_the_lead_before_the_ceo`; `resume_order.tests.rs`. |
| SC-006 | PASS (automated) | `server_crash.rs` (all 3 marked after a SIGKILL); `no mark, no warning` and `server_restart.rs` (a clean stop gives `interrupted`, so no mark); the desktop test keeps Confirm unavailable until the acknowledgement. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| No interrupted agent | PASS (automated) | `no interrupted agent, no button`. |
| Nothing checked | PASS (automated) | `Confirm is unavailable with nothing checked and says so`. |
| Agent changed after the list opened | PASS (automated) | `an_agent_that_is_no_longer_interrupted_is_skipped`. |
| Two clients confirm at once | PASS (automated) | The still-interrupted check runs with the start under the status order (`an_agent_that_is_not_interrupted_is_a_conflict_and_nothing_starts`). |
| The CLI cannot continue | PASS (automated) | `a_cli_that_cannot_continue_starts_a_new_session_at_once_with_the_reason`. |
| The continue does not start | PASS (automated) | `a_command_that_cannot_start_is_a_success_with_a_failure_that_the_record_keeps`, `the_reason_of_a_continue_that_did_not_start_carries_the_text`, `resume_run.tests.rs` `an_answer_with_a_spawn_error_is_a_start_that_did_not_run`. |
| A user stop inside the check period | PASS (automated) | `a_user_stop_inside_the_check_period_ends_the_watch_without_a_new_session`. |
| Already on hold | PASS (automated) | `an_agent_already_held_gets_no_second_hold_or_notification`. |
| The hold cannot be stored | PASS (automated) | `a_failed_write_of_an_office_hold_leaves_memory_and_the_ports_untouched`; the run then reports the failure (`ResumeContext::hold_failed`). |
| Invalid `resume.start` | PASS (automated) | `an_empty_or_unknown_field_request_is_refused`; `resume_requests.tests.rs`. |
| Escape and close while running | PASS (automated) | `Escape closes the dialog while the list still loads or runs`. |
| Keyboard only | PASS (automated) | `the keyboard alone opens, checks, confirms, and closes`. |

## Plan deviations

- **`fall_back` answers like `resume`.** It returns `continued: false` and the reason, the text of the record detail without its `new session: ` prefix, so the dialog shows the same reason as the record.
- **The start-up mark keeps its name.** `SessionRepository::mark_running_as_interrupted` now writes `lost`; only its doc changed.
- **The desktop controller lives in the panel store.** It reads `resume.result` while the dialog is closed, and it keeps a result that arrives before the `resume.start` answer, which the real server can send.
- **Results can come before the answer.** The server spawns the run before it answers `resume.start`. The contract allows it, and the desktop and the tests handle it.

## Hands-on checks

| Check | Verdict | Note |
|---|---|---|
| A signed-in `claude` agent resumed after a SIGKILL of the server, in the real app | PASS (agent-run) | Run 2026-11-23 on Linux, app and server from `main` 468ad56, clean data folder. The CLI answered one prompt; the server got a SIGKILL and was started again from the app. The panel showed "Resume interrupted (1)". The dialog listed the agent with "Continues the session started ...", the unclean-stop mark, the warning, and the acknowledgement; Confirm stayed unavailable until the tick. After Confirm the result read "Checking: continuing a session", then "Continued" at 20.6 s. Asked in the resumed terminal, the CLI named its earlier reply, so its context was kept. The old session ended with "lost". |
| The warning and the results announced by a screen reader | WAIVED (owner) | The dialog uses `role="status"` and `aria-describedby`; nobody listened with a screen reader. WAIVED under the owner's waiver of person-only rows, accepted 2026-11-23. |
| 200% zoom and 360 px width of the dialog | PASS (agent-run) | Run 2026-11-23 on Linux, `main` 468ad56, app at device scale 2 (GDK_SCALE=2), three interrupted agents with long names after an unclean stop. At 960 CSS px (maximized) and at 360 CSS px: no sideways scroll, 0 elements past the right edge, 0 cut text. Long names wrap; the dialog scrolls up and down to the warning, the acknowledgement, and Confirm. |
| macOS and Windows | WAIVED (owner) | CI runs the automated tests on both; nobody used the dialog there. WAIVED per the owner decision of 2026-11-20 that CI evidence is enough. |

## Gaps

- The office does not find or end CLI processes left over after an unclean stop (spec A5, research R1 and R3). A follow-up change can store each session's process id, start time, and command and end only the processes whose three values still match.
- A CLI that refuses a continue but keeps running is not detected (spec A4).

## Updates

- 2026-11-23: PB-6 said the focused-terminal exception does not apply to the `resume_hold` notification, but the desktop exempted only `auto_hold`: a hold after a failed resume was marked read on arrival while its agent's terminal was open and the window focused. One predicate, `isNeverReadOnArrival`, now lists both kinds. New tests: `a failed resume's hold stays unread and pings the OS even for the open and focused agent (0013 PB-6)` (unit, and a `resume_hold` push in the existing read-on-arrival test), and `a hold after a failed resume stays unread while its terminal is open (0013 PB-6)` (e2e, `hold.spec.ts`). Each unit test fails without the change.
