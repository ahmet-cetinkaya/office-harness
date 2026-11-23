# 0022 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `spec/0022-waiting-item-sleep`, with `origin/main` 5272d6f included. The Rust gates ran on the final tree. Two hands-on runs: the first at 449535e (T3), and the 30-minute run at 6a6e890, which holds every code change of this PR.
- **Overall status:** automated tests prove every requirement. Criteria with counts are PARTIAL where the tests ran fewer rounds or shorter times than stated. The 30-minute hands-on run with a real signed-in `claude` passed SC-004: exactly 1 notification from the reply to 30 min 16 s after the prompted wait, and a sleep 311.4 s after that wait, with no dismiss and no terminal view open.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| PASS (hands-on) | An agent ran the check on the real surface. The evidence column says what ran. |

### Scope of the automated evidence

- `tests/needs_you_waits.rs` and `tests/idle_sleep.rs` run an in-process server behind a real socket with fake CLIs (Unix shell scripts). They shorten the times: a 400 ms quiet period and a 1 s work bound in `needs_you_waits.rs`, and a 100 ms idle minute in `idle_sleep.rs`.
- The unit tests run the wait tracker, the notification bridge, the sleep gate and the sleeper with fakes for the clock, the terminal and the stores.
- The Rust gates exclude the `office-harness-desktop` crate. This change does not touch the desktop, so the desktop gates are NOT RUN.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `bash scripts/check-boundaries.sh` | workspace | PASS (`boundaries ok`) |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` | workspace | PASS, no warnings |
| `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=4` | workspace | PASS: 1796 passed, 0 failed, 109 result lines. One earlier full run at 6a6e890 had 1 failure in `tests/temps.rs` (`a_temp_with_a_needs_you_item_is_not_ended_and_ends_one_idle_limit_after_the_dismiss`, 0 `temp_ended` notifications at its read). The test reads the list once, with no wait. It passed 5 of 5 runs alone, and the next full run was green. Untouched by this PR |
| New integration tests, 3 runs | `tests/needs_you_waits.rs` (7 tests), `tests/idle_sleep.rs` (20 tests) | PASS: 3 of 3 runs green each |
| Desktop gates | `src/apps/desktop` | NOT RUN: no desktop file changed |

## Criteria

| Criterion | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `an_idle_agent_with_a_plain_waiting_item_sleeps_and_its_item_stays_with_its_wait_time`; hands-on: the item stayed with its `first_event_at` after the sleep (`0022-needs-you-asleep.json`) |
| FR-002 | PASS (automated) | `a_waiting_item_keeps_the_agent_awake_while_its_screen_cannot_be_read` |
| 0003/FR-006 (c) | PASS (automated) | `a_redrawing_cli_gives_one_notification_and_one_item_with_one_wait_time`; `an_unprompted_wait_adds_no_notification_and_leaves_the_item_alone` |
| 0003/SC-004 | PASS (automated) | the existing quiet-period notification tests stay green; `input_into_an_agent_with_an_item_removes_it_and_the_return_gives_a_new_one` |
| 0008/FR-001 | PASS (automated) | `a_redrawing_cli_gives_one_notification_and_one_item_with_one_wait_time` (1 item, `first_event_at` unchanged); `a_dismissed_item_stays_gone_while_the_cli_redraws` |
| 0008/FR-005 | PASS (automated) | `input_into_an_agent_with_an_item_removes_it_and_the_return_gives_a_new_one`; `input_while_a_redraw_shows_working_removes_the_item_and_the_return_gives_a_new_one` (the item leaves within 300 ms of the input, under the 1 s bound); `the_item_leaves_at_the_bound_while_the_agent_still_works`; `an_unprompted_working_leaves_the_item_alone`; `a_working_that_follows_starting_is_not_judged_and_clears_the_item`; `a_wake_removes_the_item_of_a_sleeping_agent_at_starting` |
| 0018/FR-002 | PASS (automated) | the SC-001, SC-002 and SC-006 rows; `a_redrawing_cli_sleeps_one_idle_time_after_its_prompted_wait` (idle since, A7) |
| 0018/FR-009 | PASS (automated) | `a_failed_sleep_bars_the_agent_until_a_prompted_wait_after_the_failure` (unit). A failing stop cannot be made with the fake CLIs, so no integration test |
| 0018 Terms Idle, Idle since | PASS (automated) | the wait tracker unit tests (`the_first_wait_of_a_run_is_prompted`, `a_redraw_is_unprompted_both_ways`, `a_person_input_makes_the_next_change_prompted`, `office_text_makes_the_next_change_prompted`, `a_session_start_resets_the_state_and_counts_as_a_wait_end`, `a_session_end_resets_the_state_and_counts_as_a_wait_end`, `the_last_prompted_wait_moves_only_on_prompted_waits`, `each_wait_end_tells_the_listener_of_every_clone`); `a_redrawing_cli_sleeps_one_idle_time_after_its_prompted_wait` |
| SC-001 | PARTIAL (automated) | 1 agent with a shortened idle minute sleeps and keeps its item. The count of 10 agents is not run |
| SC-002 | PASS (automated) | unit: `an_item_that_recorded_more_than_waits_keeps_the_agent_awake` (bell, report, auto hold, resume hold), `a_waiting_item_keeps_the_agent_awake_while_the_screen_shows_an_open_question` (the three 0012 shapes); integration: `an_item_that_recorded_a_bell_keeps_the_agent_awake`, `a_waiting_item_keeps_the_agent_awake_while_the_screen_shows_a_question` |
| SC-003 | PARTIAL (automated) | `a_redrawing_cli_gives_one_notification_and_one_item_with_one_wait_time` with a shortened interval and duration. The 20 minutes are not run |
| SC-004 | PASS (hands-on) | the 30-minute run at 6a6e890: 1 notification from the reply to 30 min 16 s after the prompted wait; sleep 311.4 s after that wait; 0 dismisses, 0 terminal views (see Hands-on run) |
| SC-005 | PARTIAL (automated) | `input_into_an_agent_with_an_item_removes_it_and_the_return_gives_a_new_one` runs 1 round, not 10 |
| SC-006 | PASS (automated) | `a_waiting_item_keeps_the_agent_awake_while_its_screen_cannot_be_read` makes 10 checks, 0 pass |
| SC-007 | PARTIAL (automated) | `work_past_the_bound_after_a_dismiss_gives_one_item_and_one_notification_at_the_return`: 1 round, with a dismiss in place of an open and a 1 s bound in place of 2 minutes of work. The no-sleep-before-5-minutes part follows from idle since (`a_redrawing_cli_sleeps_one_idle_time_after_its_prompted_wait`) |
| PB-1 | PASS (automated) | the SC-002 tests, plus the existing hold, queue, text and child tests in `tests/idle_sleep.rs` |
| PB-2 | PASS (automated) | the existing open and dismiss tests stay green; `a_dismissed_item_stays_gone_while_the_cli_redraws` |
| PB-3 | PASS (automated) | `input_into_an_agent_with_an_item_removes_it_and_the_return_gives_a_new_one`; `input_while_a_redraw_shows_working_removes_the_item_and_the_return_gives_a_new_one` |
| PB-4 | PASS (automated) | `a_bell_after_the_wait_still_makes_a_bell_item_that_a_later_redraw_leaves_alone`; the existing exit and report tests stay green |
| PB-5 | PASS (automated) | no change to the status detection; the existing status tests stay green |
| PB-6 | PASS (automated) | the existing `with_idle_sleep_off_no_idle_agent_is_put_to_sleep` and the no-notification tests stay green |
| PB-7 | PASS (automated) | the existing `a_stop_that_fails_records_the_failure_and_keeps_the_status` (`sleep_agent_handler.tests.rs`: 1 failed `sleep` entry, the status stays) stays green; `a_failed_sleep_bars_the_agent_until_a_prompted_wait_after_the_failure` |
| PB-8 | PASS (review) | the question check reads `runtime.screen` only; the diff writes no CLI configuration |
| A9 (temp idle end) | PASS (automated) | `the_idle_end_of_a_temp_stops_for_a_plain_waiting_item_too`; the existing `a_temp_with_a_needs_you_item_is_not_ended_and_ends_one_idle_limit_after_the_dismiss` stays green |

## Changed existing tests

- `notification_bridge.tests.rs`: `interrupted_keeps_an_item_and_stopped_working_and_starting_clear_it` and `working_after_waiting_for_input_clears_the_item_and_tells_the_clients` now call `wait_end` before the `working` frame. Under 0008/FR-005 as modified, a `working` after a wait with no wait end is a redraw and keeps the item.
- `tests/idle_sleep.rs`: `an_item_in_needs_you_keeps_the_agent_awake` is removed. A plain waiting item no longer keeps an agent awake (FR-001). The SC-001, bell and question tests replace it.
- `idle_sleep_gate.tests.rs`: `no_agent_sleeps_before_the_gate_is_connected` reads `last_prompted_wait` in place of the removed `quiet_for`.
- `idle_sleep_run.tests.rs`: `a_failed_sleep_bars_the_agent_until_it_was_active_again` is rewritten for 0018/FR-009 as modified and renamed `a_failed_sleep_bars_the_agent_until_a_prompted_wait_after_the_failure`.

## Hands-on run

Two runs, with the same steps. The first, at 449535e, stopped after the wake, about 5 minutes after the prompt. The code gate asked for the full 30 minutes of SC-004, so a second run at 6a6e890 kept the agent asleep until 30 minutes after the prompt before it read the notification list and woke the agent. SC-004 rests on the second run.

- **Setup:** the real app (debug build) on Linux under `tauri-driver`, an isolated data folder, and a real signed-in `claude` started with `--permission-mode default` in a folder it already trusts. No sign-in, trust or permission screen was answered.
- **Steps:** start the agent, type one prompt ("Reply with the single word ok."), set the idle time to 5 minutes, then leave the agent alone: no dismiss, no input, and no terminal view open (0 `.xterm` views at every 5 s tick). In the second run, read the notification list every 30 s while the agent sleeps, until 30 minutes after the prompted wait. Then focus the agent's row and press Enter in its terminal view.
- **Observed in the 30-minute run (6a6e890):**
  - The reply came, and the CLI then redrew its screen; the status changed between `working` and `waiting for input` during the first 20 s. None of these changes added a notification or changed the item.
  - Notifications for the agent up to the wake, read 1816 s (30 min 16 s) after the prompted wait: 1 at the first wait after the start, before the prompt was typed, and exactly 1 at the prompted wait after the reply. SC-004 counts from the reply, so the count is 1.
  - The sleep entry in the action record is 311.4 s after the prompted wait, within 6 min 10 s. The agent showed `asleep` at the 5 s tick 315.5 s after it, and stayed `asleep` until the wake. Its Needs you item stayed, with the same `first_event_at` (FR-001).
  - Enter woke the agent. The wake result went "Waking", "Checking", then "Continued" (same session), and the agent showed `waiting for input`.
  - The old item had left when the agent was woken. Focusing the agent's row marked its notifications read 1.3 s before the wake, so the item left through the read, not at `starting`. The removal at `starting` is proven by `a_wake_removes_the_item_of_a_sleeping_agent_at_starting`. The first wait of the woken session then added 1 new item and 1 notification, as a prompted wait does.
- **First run (449535e):** the same counts up to its wake (1 notification after the reply, sleep entry 311.6 s after the prompted wait), with the same read-before-wake note.
- **Evidence:** `qa/evidence/b-0022-head/thirty/` (the 30-minute run) and `qa/evidence/b-0022-head/first/` in the lead's agent folder (driver `qa/r22.py`), with the session ids masked: `steps.json`, `0022-notifications-before-wake.json`, `0022-notifications.json`, `0022-needs-you-asleep.json`, `0022-needs-you-after-wake.json`, `0018-06-actions.json`.
