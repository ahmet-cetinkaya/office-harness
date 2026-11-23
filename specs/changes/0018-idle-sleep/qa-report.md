# 0018 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0018-idle-sleep` at HEAD, with `origin/main` 15b18e9 merged; the local nix clippy and rustc 1.99; CI uses toolchain 1.100.0. The Rust and desktop gates ran on the merged tree.
- **Overall status:** the automated evidence stands for the sleep, the wake, the fallback and the hold, the refusals, and the setting rules. Most criteria with counts (SC-001 to SC-006, SC-008, SC-009) are proven in behavior with fewer runs than the stated counts, so they are PARTIAL. The hands-on check with a real `claude` CLI (put an agent to sleep, then wake it with Enter) was not run. It is BLOCKED.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- `tests/idle_sleep.rs` runs an in-process server behind a real socket with fake CLIs (Unix shell scripts). It shortens the time: one idle minute lasts 100 ms, the sleeper checks every 50 ms, and the check period of a wake is 2 s. The times of the criteria (60 seconds to begin, 10 seconds to wake) are therefore not measured in real time.
- The unit tests run the handlers with fakes for the terminal, the gate, and the clock. The directive and report tests run the hierarchy handlers with fake targets.
- The Playwright tests run against a mock core, so they prove what the desktop shows and sends, not the real server.
- No test runs a real CLI.
- The Rust gates exclude the `office-harness-desktop` crate, whose build script needs the prepared sidecar on this machine. CI builds it on all three OSes. This change does not touch that crate.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --check` | workspace | PASS |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -D warnings` | with `-A clippy::must_use_candidate` only for a finding that the local clippy 1.99 raises on `main` too (`cli/run.rs`) | PASS (clean) |
| `cargo test --no-fail-fast` | workspace without `office-harness-desktop`, merged tree: 1518 passed, 0 failed, measured before `a_wake_under_a_hold_continues_the_session_and_the_hold_stays` was added (see the next row) | PASS |
| `tests/idle_sleep.rs` | 15 tests | PASS |
| `scripts/check-boundaries.sh` | repository | PASS (boundaries ok) |
| `bun test src` | desktop: 415 passed, 0 failed | PASS |
| Playwright | desktop, full suite on the mock core, Chromium: 278 passed, 0 failed | PASS |
| `svelte-check`, `eslint`, `prettier --check` | desktop: 523 files, 0 errors, 0 warnings; lint and format clean | PASS |
| Hands-on: a real `claude` agent put to sleep and woken with Enter | not run | BLOCKED |
| CI | the PR | In the PR |

## Success criteria

| SC | Verdict | Evidence |
|---|---|---|
| SC-001 | PARTIAL (automated) | `with_idle_sleep_off_no_idle_agent_is_put_to_sleep` (`tests/idle_sleep.rs`): with the setting never turned on, an idle agent stays `waiting_for_input` over the watch time. 1 agent, not 10; the watch is 1.5 s (15 idle minutes of the test clock), not 24 hours. The test does not count office starts or record entries. |
| SC-002 | PARTIAL (automated) | `an_idle_agent_nobody_needs_is_put_to_sleep_with_one_record_and_no_notification`: the agent shows `asleep` after the idle time. 1 agent, not 10; the bound of 6 minutes 10 seconds is not measured in real time. |
| SC-003 | PARTIAL (automated) | Each condition alone keeps the agent awake: `an_item_in_needs_you_keeps_the_agent_awake` (b), `a_hold_keeps_the_agent_awake` (c), `a_waiting_directive_keeps_its_target_awake` (`tests/agent_directive.rs`, d), `unsent_person_text_keeps_the_agent_awake` (e), `a_running_child_keeps_its_parent_awake` (`tests/agent_directive.rs`, f). The last check for (a) and for a reason of the gate: `an_agent_that_does_not_wait_for_input_conflicts_without_a_record_or_a_stop` and `a_reason_of_the_gate_conflicts_without_a_record_or_a_stop` (`sleep_agent_handler.tests.rs`). Gaps: 1 run each, not 3 times the idle time with repeats; a waiting report (d) has no test of its own (it shares the queue count with a directive); pending office text has no test of its own; the 10 tries of (a) were not run. |
| SC-004 | PARTIAL (automated) | `turning_idle_sleep_on_restarts_the_count_for_long_idle_agents` (the agent does not sleep in half the idle time after the setting was turned on, then sleeps); `turning_the_setting_on_or_changing_it_restarts_the_count` (`idle_sleep_run.tests.rs`, also for a change of the value). 1 agent, not 10; the window is half of 5 test minutes, not 30 minutes. |
| SC-005 | PARTIAL (automated) | `a_wake_continues_the_last_session_and_records_one_wake_by_the_person`: `wake.start` continues the same session, with the results `checking` then `continued`. Desktop: `Enter in the focused terminal view wakes the agent`, `Wake in the details wakes the agent` (mock core). 1 wake, not 20; the 10 seconds are not timed. The wake with a real CLI is BLOCKED (see the hands-on row of the gates). |
| SC-006 | PARTIAL (automated) | `an_idle_agent_nobody_needs_is_put_to_sleep_with_one_record_and_no_notification` (one `sleep` entry with actor `office`, no `stop` entry, the notification list and the Needs you list unchanged) and `a_wake_continues_the_last_session_and_records_one_wake_by_the_person` (one `wake` entry with actor `user`). 1 cycle, not 20. |
| SC-007 | PASS (automated) | `a_wake_that_fails_twice_starts_the_agent_twice_and_holds_it_with_one_notification`: the results `checking`, `checking`, `held`; the reason starts with "the wake failed"; exactly 1 `resume_hold` notification names the agent. The fake CLI ends at once on every start. |
| SC-008 | PARTIAL (automated) | Clean restart: `an_asleep_agent_stays_asleep_after_a_server_restart_and_is_not_resumable` (`asleep` after the restart, `resume.plan` lists no agent, no new session). Crash: `only_running_agents_become_interrupted_and_none_starts` (`tests/persistence.rs` of the agent infrastructure) runs the sweep of the next start over every status, and an `asleep` agent keeps `asleep`. Gaps: 1 restart, not 10; no test kills the server binary with SIGKILL while an agent is asleep (the sweep runs on the stored statuses). |
| SC-009 | PARTIAL (automated) | `a_directive_to_an_asleep_member_is_refused_as_asleep_and_recorded` (`tests/agent_directive.rs`, real server: the answer says "it is asleep", the failure is in the record). Reports: `a_report_to_an_asleep_parent_is_refused_as_asleep_in_both_records` (`send_report_handler.tests.rs`, unit only). Gaps: 1 directive, not 100%; a report to an asleep agent has no test on the real server. |

## Requirements

| Id | Verdict | Evidence |
|---|---|---|
| FR-001 | PARTIAL (automated) | `the_setting_takes_0_and_5_to_1440_minutes_only` (server: 4, 1441, 2.5, and text are refused with the contract text; the stored value stays `0`); `idle_sleep_takes_zero_or_five_to_fourteen_forty_minutes` and `idle_sleep_outside_that_range_is_refused_and_keeps_the_stored_value` (`settings.tests.rs`); desktop: `a bad value is refused before anything is sent`, `a good value is stored as text, off as 0`, `it reads off as 30 minutes`; e2e `it is off by default and turning it on stores 30`, `4 and 1441 are refused and the previous value stays`. `an_asleep_agent_stays_asleep_after_a_server_restart_and_is_not_resumable` reads the stored value 5 again after a server restart. Gap: the failure to save has the refusal test only. |
| FR-002 | PARTIAL (automated) | SC-003 row. Gaps: the 60 seconds to begin are not timed in real time (the knob is 15 s; the tests run a 50 ms tick); a waiting report and pending office text have no test of their own. |
| FR-003 | PARTIAL (automated) | `an_idle_agent_nobody_needs_is_put_to_sleep_with_one_record_and_no_notification` (one `sleep` entry, actor `office`, detail "idle for 5 minutes", no `stop` entry, no notification, no Needs you item, the session ended); `a_waiting_agent_sleeps_with_one_office_entry_and_its_session_ends` (`sleep_agent_handler.tests.rs`); `a_wake_continues_the_last_session_and_records_one_wake_by_the_person` (the kept session is the one that the wake continues); `the_sleep_and_wake_kinds_have_their_wire_names` (`action_record.tests.rs`). Gap: no test shows that the lines the CLI printed while it ended are in the recent output; the desktop test `its terminal view shows the last output, the asleep text, and Wake` runs on the mock core. |
| FR-004 | PARTIAL (automated) | `an_asleep_agent_stays_asleep_after_a_server_restart_and_is_not_resumable`; `an_asleep_agent_stops_without_a_terminal_call_and_adds_a_stop_entry` (`stop_agent_handler.tests.rs`); `an_asleep_agent_is_not_running_and_takes_only_a_start_or_a_stop` (`status_machine.tests.rs`). Gaps: no test turns idle sleep off while an agent is asleep; the delete, the new task, the continue of a session, and the agent-made start of an asleep agent have no test of their own. |
| FR-005 | PASS (automated) | `a_directive_to_an_asleep_member_is_refused_as_asleep_and_recorded`; `an_asleep_target_is_refused_as_asleep_and_journaled` and `a_target_that_went_to_sleep_withdraws_every_waiting_directive_as_asleep` (`send_directive_handler.tests.rs`); `a_report_to_an_asleep_parent_is_refused_as_asleep_in_both_records` and `a_waiting_report_of_a_parent_that_went_to_sleep_is_withdrawn_as_asleep` (`send_report_handler.tests.rs`); `only_an_asleep_member_is_asleep_and_it_is_not_running` and `a_running_member_going_to_sleep_is_asleep_and_not_running` (`directive_bridges.tests.rs`, the going-to-sleep set). |
| FR-006 | PARTIAL (automated) | e2e on the mock core: `Enter in the focused terminal view wakes the agent`, `other keys and a paste reach no CLI and do not wake`, `Wake in the details wakes the agent`, `Wake in the scene desk terminal wakes the agent`. Gap: no test wakes an agent while the office or the agent is on hold. |
| FR-007 | PASS (automated) | `a_wake_continues_the_last_session_and_records_one_wake_by_the_person` (one `wake` entry, actor `user`, no detail); `a_wake_that_fails_twice_starts_the_agent_twice_and_holds_it_with_one_notification`; `a_wake_with_a_cli_that_cannot_continue_starts_a_new_session_with_the_reason` and `an_asleep_agent_wakes_into_its_last_session_and_the_record_is_a_wake_without_detail` (`resume_agent_handler.tests.rs`); `a_wake_of_an_agent_that_is_not_asleep_starts_nothing`, `an_agent_that_is_not_asleep_cannot_be_woken_and_nothing_starts`, `two_wakes_at_once_start_the_agent_once`. |
| FR-008 | PASS (automated) | Server: `a_wake_result_has_its_own_event_and_the_hold_names_the_wake` (`resume_run.tests.rs`) and the results in the SC-005 and SC-007 tests. Desktop: `a wake shows Waking, then Checking, then Continued`, `a CLI that cannot continue starts a new session with the reason`, `a failed wake puts the agent on hold with the reason`, `a wake of an agent that is no longer asleep says so`, `a new sleep clears the last result` (e2e, mock core); `each kind reads as text` and `the conflict answer reads as no longer asleep` (`sleep.spec.ts`, `sleep-controller.spec.ts`). |
| FR-009 | PASS (automated) | `a_stop_that_fails_records_the_failure_and_keeps_the_status` (`sleep_agent_handler.tests.rs`: one failed `sleep` entry, the status stays); `a_failed_sleep_bars_the_agent_until_it_was_active_again` (`idle_sleep_run.tests.rs`: no new try until the agent was active and idle again). |
| 0001/FR-014 (MODIFIED) | PASS (automated) | `only_a_waiting_agent_goes_to_sleep` and `an_asleep_agent_is_not_running_and_takes_only_a_start_or_a_stop` (`status_machine.tests.rs`); e2e `shows asleep in the list and the scene`. |

## Preserved behavior

| Id | Verdict | Evidence |
|---|---|---|
| PB-1 | PASS (automated) | `an_asleep_agent_stays_asleep_after_a_server_restart_and_is_not_resumable`: after the restart only one session exists. `a_wake_of_an_agent_that_is_not_asleep_starts_nothing`. The fallback follows only the wake (`a_wake_that_fails_twice_starts_the_agent_twice_and_holds_it_with_one_notification`). |
| PB-2 | PASS (automated) | `with_idle_sleep_off_no_idle_agent_is_put_to_sleep`; `the_setting_off_gives_no_rule_and_the_first_value_counts_from_the_start` (`idle_sleep_run.tests.rs`). |
| PB-3 | PASS (automated) | `a_hold_keeps_the_agent_awake` (a hold keeps the agent awake); `a_wake_under_a_hold_continues_the_session_and_the_hold_stays` (the wake is carried out, the results are `checking` then `continued`, and the agent hold is still listed). |
| PB-4 | PARTIAL (automated) | `an_asleep_agent_stops_without_a_terminal_call_and_adds_a_stop_entry`; the whole suite of the workspace passes with the new status and the new actions. No new test runs the new task, the continue, or the delete on an asleep agent. |
| PB-5 | PARTIAL (automated) | e2e `other keys and a paste reach no CLI and do not wake` (the asleep view only). The input to a running CLI has no new test; the suites `terminal_protocol.rs` and `person_input.tests.rs` pass unchanged for their old cases. |
| PB-6 | PARTIAL (automated) | One `resume_hold` notification names the agent (`a_wake_that_fails_twice_starts_the_agent_twice_and_holds_it_with_one_notification`); the text "was put on hold: <detail>" (`notification.spec.ts`). The focused-terminal exception for this notification has no new test. |
| PB-7 | PARTIAL (automated) | e2e `shows asleep in the list and the scene` shows the status as text. Gap: no test reads `asleep` in the `agent.subtree` entries (0014). |
| PB-8 | GAP | No test. The change only ends a CLI and continues or starts it through the existing paths; it adds no write to a CLI's configuration. |

## Deviations from the plan

- The notes "Amended by 0018" in the 0001 documents are not written, because those files are in `specs/changes/archive/`. The contract names them by path.
- The deviations of the spec are in its `## Deviations` section.
