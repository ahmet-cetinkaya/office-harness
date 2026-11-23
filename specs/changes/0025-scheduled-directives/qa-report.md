# 0025 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0025-scheduled-directives`, one commit per task: T1 84117a6 (kernel actor, schedule domain), T2 757814a (application, store, wire handlers), T3 1343726 (hierarchy) and 4dd1929 (cap setting, actor label, notification without an agent), T4 be19d6f (host), T5 1ca6dec (desktop).
- **Overall status:** every requirement and success criterion has automated evidence. Four are PARTIAL (automated): SC-002 and SC-003 prove the daylight saving rules in the domain tests with fixed zones and one wire test of a zone change, not on a real machine; SC-005 proves the grace rule but not the wait for a running sweep; SC-014 and SC-015 run on a smaller scale than the criterion names. No hands-on run with the real server and a real agent CLI was made.
- **Status of this file:** the gate numbers are from the local runs of the task commits. The CI result of the pull request is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | An automated test proves the rule, but on a smaller scale or a shorter clock than the criterion names. The evidence column says what differs. |
| GAP | No test proves the criterion. |
| BLOCKED | The check was not run. It is never counted as a pass. |

### Scope of the automated evidence

- The wire tests in `src/apps/office-harnessd/tests/schedules.rs` run the real server with fake CLIs and a fake clock and zone (`ServerOptions`), with a tick of 50 ms. The agent calls use the real `office-harnessd agent-call` helper.
- The domain tests (`schedule-domain`) run the due-time rules with fixed zones (`America/New_York`, `Europe/Berlin`).
- The desktop tests run against the mock core of the e2e suite (`e2e/schedules.spec.ts`), not the real server.
- Not run: Windows and macOS, and the re-read of the system zone by `jiff` at run time (plan R1).

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `cargo clippy ... -D warnings` | schedule crates, hierarchy, agent, audit, notification, wire, `office-harnessd` | PASS (the local `must_use_candidate` at `cli/run.rs:18` is older than this change and is not in the diff) |
| `cargo test -p office-harnessd` | 711 tests, 28 in `tests/schedules.rs` | PASS |
| `cargo test` for the touched crates | kernel, schedule-*, hierarchy-*, agent-*, audit-*, notification-*, wire | PASS |
| `scripts/check-boundaries.sh` | workspace | PASS |
| `bun run check`, `lint`, `format:check`, `bun test src` | desktop, 512 unit tests | PASS |
| `playwright test --workers=2` | desktop e2e, 325 tests | PASS |
| `cargo check` of `office-harness-desktop` | desktop shell crate | BLOCKED (system library `libdbus-1` missing on this machine); the change to `api_error.rs`, `session.rs` is checked by `rustfmt` only |

## Success criteria

| Criterion | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `a_once_schedule_types_its_text_at_the_due_time_and_names_itself_in_the_record` |
| SC-002 | PARTIAL (automated) | domain: `spring_forward_runs_a_skipped_time_once_after_the_gap`, `fall_back_runs_a_repeated_time_once_at_its_first_occurrence`, `berlin_changes_follow_the_same_rules`; wire: `a_daily_schedule_follows_a_change_of_the_time_zone`. Not run over 10 real days. |
| SC-003 | PARTIAL (automated) | domain: `every_two_hours_keeps_two_elapsed_hours_across_both_changes` |
| SC-004 | PASS (automated) | `a_server_stop_over_due_times_makes_no_late_run_and_gives_one_missed_entry_and_one_notice` |
| SC-005 | PARTIAL (automated) | `a_run_started_90_seconds_late_runs_and_one_started_150_seconds_late_is_missed` (clock jump); the wait for a running sweep has no wire test |
| SC-006 | PASS (automated) | `a_clock_set_back_after_a_run_does_not_run_that_due_time_again`, `a_clock_set_three_hours_forward_makes_one_missed_entry_with_the_count_3_and_no_run` |
| SC-007 | PASS (automated) | `under_an_office_hold_each_due_run_is_refused_recorded_and_never_made_later`, `under_a_hold_on_the_target_each_due_run_is_refused_with_the_hold` |
| SC-008 | PASS (automated) | `the_fourth_schedule_for_one_target_in_a_minute_is_refused_with_the_limit_and_its_value`; the leg that a parent's directive is accepted: hierarchy unit tests of `guard_scheduled` |
| SC-009 | PASS (automated) | `an_asleep_target_is_woken_by_the_schedule_and_then_gets_the_directive` |
| SC-010 | PASS (automated) | `a_stopped_target_gives_skipped_runs_one_notice_and_a_new_one_after_a_success` |
| SC-011 | PASS (automated) | `deleting_the_target_switches_the_schedule_off_at_once_with_one_notice_and_no_run`; also `a_subtree_delete_switches_off_the_schedule_of_a_deleted_descendant`, `the_end_of_a_temp_that_is_the_target_switches_the_schedule_off` |
| SC-012 | PASS (automated) | `run_now_twice_waits_the_first_and_skips_the_second_for_the_previous_run` |
| SC-013 | PASS (automated) | `the_cap_refuses_the_next_save_with_its_value_and_a_cap_of_200_takes_200` |
| SC-014 | PARTIAL (automated) | `ten_schedules_and_their_history_survive_a_restart_and_the_ones_that_are_on_run`: 10 schedules and 1 restart, not 5 restarts and a machine restart |
| SC-015 | PARTIAL (automated) | `twelve_schedules_due_in_the_same_minute_for_agents_that_wait_all_run`: 12 agents, not 50 |
| SC-016 | PASS (automated) | `an_agent_that_asks_for_any_schedule_request_is_refused_and_recorded_and_nothing_changes` |
| SC-017 | PASS (automated) | e2e `make, switch off, edit, switch on, Run now, cancel and confirm a delete`; each person action adds one entry: `each_save_switch_run_now_and_delete_of_the_person_adds_one_entry_with_the_person` |
| SC-018 | PASS (automated) | `a_delete_of_a_schedule_whose_directive_waits_leaves_the_directive_to_be_typed` |
| SC-019 | PASS (automated) | `an_unsubmitted_directive_is_recorded_once_in_the_targets_record_and_notified`, `a_waiting_directive_whose_target_stops_is_withdrawn_and_recorded_once` |

## Requirements

Every FR has at least one test above. Extra evidence: FR-001 and FR-002 `a_save_that_breaks_rules_names_each_field_and_rule_in_the_error`, `a_cap_outside_1_to_200_is_refused_and_the_old_value_stays`; FR-008 the open-run rule `an_open_run_at_a_server_start_is_failed_with_the_reason_that_the_server_stopped`; FR-013 and the visible states: the 17 tests of `e2e/schedules.spec.ts` (empty state, row text, off reason, history, field errors, cap, 360 px width, axe scan).

## Gaps and notes

- The 10-minute queue expiry of a waiting scheduled directive, and a wake in progress at a server stop, have no wire test; the stop case of 0012/FR-005 is proven by `a_waiting_directive_whose_target_stops_is_withdrawn_and_recorded_once`.
- The 200% zoom of the schedules view is not tested; the 360 px width is.
- Windows and macOS are covered only by CI.
