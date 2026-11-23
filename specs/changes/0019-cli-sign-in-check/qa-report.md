# 0019 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0019-cli-sign-in-check` with `main` bdb4d20 merged (88bff7f), the implementation commits (Rust, desktop, docs), the local nix clippy 1.99 and rustc 1.99; CI uses toolchain 1.100.0. The Rust gates ran again on the merged tree; the desktop gates ran before the merge, and the merge changed no desktop file.
- **Overall status:** the automated evidence stands for FR-001 to FR-011 and PB-1 to PB-6. SC-002, SC-003, SC-005, SC-008, and SC-010 are proven in behavior with fewer runs than the stated counts, so they are PARTIAL. SC-007 (real CLIs) is BLOCKED: the answers of the real status commands were checked by hand (research R1), the start with a real CLI was not run.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- `tests/sign_in_check.rs` runs an in-process server behind a real socket with fake CLIs named `claude` and `gemini` (Unix shell scripts). Every other fake CLI of the test suite answers a status command as signed in (`FakeClis::add`).
- The application tests (`session_launcher.tests.rs`) use a fake status command runner; the runner tests (`status_command.tests.rs`) run stand-in programs on Unix and a batch file on Windows.
- The Playwright tests run against a mock core, so they prove the session list text, not the real server.
- The Rust gates exclude the `office-harness-desktop` crate, whose build script needs the prepared sidecar on this machine. CI builds it on all three OSes. This change does not touch that crate.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `cargo clippy --all-targets -D warnings` | workspace without `office-harness-desktop`; `-A clippy::must_use_candidate` only for a finding that the local clippy 1.99 raises on `main` too (`cli/run.rs`) | PASS (clean) |
| `cargo test --no-fail-fast` | workspace without `office-harness-desktop`, merged tree: 1466 passed, 0 failed | PASS |
| `tests/sign_in_check.rs` | 8 tests | PASS |
| `scripts/check-boundaries.sh` | repository | PASS (boundaries ok) |
| `bun test src` | desktop: 393 passed, 0 failed | PASS |
| Playwright | desktop, full suite on the mock core, Chromium: 263 passed, 0 failed | PASS |
| `svelte-check`, `eslint`, `prettier --check` | desktop: 514 files, 0 errors, 0 warnings; lint and format clean | PASS |
| CI | the PR | In the PR |

## Success criteria

| SC | Verdict | Evidence |
|---|---|---|
| SC-001 | PARTIAL (automated) | `a_start_of_a_cli_that_is_not_signed_in_fails_with_the_reason_and_starts_nothing` and `a_continue_of_a_cli_that_is_not_signed_in_fails_with_the_reason` (application), `a_start_of_a_cli_that_is_not_signed_in_fails_with_one_visible_reason` (server): `failed`, the never-started session with the reason, the failure in the action record, no terminal start, no credential asked for. A new task of an agent that does not run and an agent-made start take the same `prepare` path and the same gateway check; they were not run as separate tests, and the count of 12 was not run. |
| SC-002 | PARTIAL (automated) | `a_hanging_status_command_blocks_no_stop_and_the_start_goes_on`: the start answers within 6 s, its entry notes the timeout, and the status command's child is gone 1 s after the deadline; `a_timeout_ends_the_command_and_its_children` (runner). 1 start, not 5. |
| SC-003 | PARTIAL (automated) | `a_sign_in_screen_in_the_first_seconds_ends_the_cli_as_not_signed_in`: `failed`, session `not-signed-in: <reason>`, one `stop` entry with actor `office`, one `failed` notification. The test waits up to 6 s, not 3 s; 1 start, not 5. |
| SC-004 | PASS (automated) | `sign_in_words_after_the_watch_period_end_nothing` (words after the period) and `a_key_typed_into_an_agent_stops_only_its_own_watch` (a person's key stops that agent's watch, and only that one: L1). |
| SC-005 | PARTIAL (automated) | `a_start_of_a_cli_that_is_not_signed_in_fails_with_one_visible_reason`: a key-shaped line is in no answer, session, action entry, or the server log; the quoted line shows ESC and U+202E as visible symbols. `a_signed_in_cli_starts_and_its_status_output_shows_nowhere`. `a_quoted_line_is_visible_and_capped` (200 characters). The output was a few lines, not 10 KiB; `at_most_the_cap_is_kept` covers 1 MiB at the runner. |
| SC-006 | PARTIAL (automated) | `a_check_leaves_the_home_folder_unchanged` (`tests/sign_in_check.rs`): the file list, sizes, and change times of the fake home folder are equal before and after a check. 1 check, and the home folder only: the office's data folder changes by design when a start fails (database, log), so it is not compared. |
| SC-007 | BLOCKED | Research R1 ran the real status commands by hand: `claude auth status` exit 0 signed in, exit 1 with `Not logged in` in an empty home; `codex login status` exit 1 with `Not logged in`. A start of an agent with the real CLIs through the office was not run. |
| SC-008 | PARTIAL (automated) | `a_hanging_status_command_blocks_no_stop_and_the_start_goes_on`: a stop of another running agent ends within 1 s while the check hangs. 1 stop, not 5. |
| SC-009 | PARTIAL (automated) | `own_arguments_flags_or_a_key_variable_skip_the_check` (application: own arguments, flags, key variable in the additions) and `own_arguments_or_a_key_variable_skip_the_check` (server): 0 status runs, the agents start. A key variable in the login environment (`StatusCommandBridge`) has no test. |
| SC-010 | PARTIAL (automated) | `a_startup_prompt_with_the_words_turns_the_watch_off`; the directive case: `a_directive_with_the_sign_in_words_in_the_first_seconds_ends_no_session` (`tests/agent_directive.rs`: the sign-in words are echoed on the screen of a `gemini` agent, and it stays `waiting_for_input` for 3 s). 2 sessions, not 6. |

## Requirements

| Id | Evidence |
|---|---|
| FR-001 | `the_status_command_runs_with_the_program_folder_and_additions_only`, `a_cli_without_a_status_command_runs_no_check`, `claude_and_codex_have_a_status_command_and_gemini_only_screen_words`. |
| FR-002 | SC-001 rows; `a_new_task_on_a_running_agent_is_refused_and_the_running_session_goes_on`, `a_continue_on_a_running_agent_is_refused_and_the_running_session_goes_on`. |
| FR-003 | SC-002 rows. |
| FR-004 | `a_check_without_an_answer_lets_the_start_go_on_with_a_note`, `an_agent_that_changed_after_the_check_ahead_starts_with_a_note`, the answer table in `sign_in_check.tests.rs`. |
| FR-005 | SC-003 and SC-004 rows; `a_persons_stop_between_the_mark_and_the_exit_keeps_stopped_and_adds_no_entry` and `a_sign_in_stop_that_ends_a_working_run_records_one_office_stop` (`agent_status_bridge.tests.rs`). |
| FR-006 | `the_command_gets_its_arguments_the_folder_and_exactly_the_environment`, `the_input_is_empty`, `at_most_the_cap_is_kept`; the server test checks that no `OFFICE_HARNESS_` name reaches the status command. |
| FR-007 | `the_reason_names_the_cli_how_it_was_found_and_what_to_do`, `a_quoted_line_is_visible_and_capped`. |
| FR-008 | SC-005 row; `the_no_answer_note_says_why_without_output`. |
| FR-009 | SC-008 row; `an_answer_checked_ahead_is_used_without_a_second_run`; the server test counts one status run per start. |
| FR-010 | SC-009 row; `an_empty_key_variable_does_not_skip_the_check`. |
| FR-011 | SC-010 row. |
| PB-4 | Not run as its own test: a not-signed-in start is a start that never ran (`spawn-error`), which the 0013 fallback already reads as "does not start" (`tests/bulk_resume.rs`). |

## Deviations from the plan

- D10: no server-only request. The watch marks the run and stops the child; the status bridge ends the session at that run's exit (agreed in the spec review r2).
- The session list shows `not signed in. <reason>` and `never started: <text>` (D11); the action record shows the failure with the existing "Failed:" text.
