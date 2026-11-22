# 0012 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0012-safe-directive-delivery`, the merge of `main` 468ad56 into 0c52856 (merge 510cddf) with the fixes of the final review (r5) on top, toolchain 1.100.0. The full gates below ran on that tree; the three repeated runs of the directive tests ran on 82b8934 and after the merge of 1d6edb6, as each row says.
- **Overall status:** automated evidence stands for FR-001 to FR-005, the modified 0002/FR-010, and PB-1 to PB-5. The success criteria that name a count are proven with fewer directives than the count, so they are PARTIAL. Rows that need the real agent CLIs are BLOCKED. Nobody ran them.
- **Status of this file:** the gate numbers come from runs on 2026-11-23. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |

### Scope of the automated evidence

- The server tests use fake CLIs (shell scripts in a real terminal), not the real agent CLIs. The question shapes they show are written for the tests.
- The screen checks are unit-tested on sample screens written for the tests, not on screens captured from the real CLIs.
- The Rust gates exclude the `office-harness-desktop` crate, which does not build on the Linux machine used here. No desktop file changed.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --check` | workspace | PASS |
| `cargo clippy --all-targets -D warnings` | workspace without `office-harness-desktop` | PASS (clean) |
| `cargo test` | workspace without `office-harness-desktop`: 1273 passed, 0 failed | PASS |
| `tests/agent_hold.rs` | 10 tests, run 3 times after the merge of 1d6edb6 (the sender-record check from `main` leaves withdrawals out, as the target count does) | PASS (each time) |
| `tests/agent_directive.rs`, `tests/task_text.rs` | 13 and 14 tests, run 3 times in a row on 82b8934, once more in the merged workspace run | PASS (each time) |
| `scripts/check-boundaries.sh` | repository | PASS (boundaries ok) |
| Desktop checks (`bun test`, Playwright, `svelte-check`) | no desktop file changed | Not run |
| CI | the PR at dc9aa61 and 92d7a4c | dc9aa61: macOS failed one test (BSD `stty raw` turns echo off; the fake CLIs now ask for `stty raw echo`). 92d7a4c: Windows passed; Ubuntu and macOS failed the 100-directive hold test, because `main` had added a sender-record count that the 0012 withdrawals change; fixed in the merge. CI on the new head is not in this file. |

## Requirements

Paths are relative to the repository root. `H` is `src/apps/office-harnessd`, `M` is `src/modules`. `SDH` is `M/hierarchy/application/src/commands/send_directive/send_directive_handler.tests.rs`, `DQ` is `directive_queue.tests.rs` beside it, `DB` is `H/src/bridges/directive_bridges.tests.rs`, `PI` is `H/src/bridges/person_input.tests.rs`, `OQ` and `IL` are `M/terminal/domain/src/services/open_question.tests.rs` and `input_line.tests.rs`, and `AD` is `H/tests/agent_directive.rs`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `SDH`: `a_busy_target_queues_the_directive_and_types_nothing`, `ten_waiting_directives_are_typed_once_each_in_the_order_sent`, `the_next_directive_waits_the_settle_gap_after_one_was_typed`, `directives_sent_at_once_are_received_in_the_order_they_were_written`; `DQ`: `places_count_from_one_in_the_order_of_arrival`, `a_claimed_head_is_not_claimed_twice`, `the_next_delivery_waits_the_settle_gap`; `H/tests/task_text.rs`: `directives_at_once_leave_the_text_that_was_written_last` (three real directives, typed one after another). |
| FR-002 | PASS (automated) | `DB`: `an_idle_member_with_a_clear_screen_is_ready`, `a_working_or_starting_member_is_busy`, `an_open_question_on_the_screen_keeps_the_member_busy`, `a_person_who_typed_keeps_the_member_busy`, `office_text_that_waits_for_the_person_keeps_the_member_busy`; `PI`: all seven tests; `ID` (`M/terminal/domain/src/services/input_draft.tests.rs`): all eight tests, with `a_lone_escape_holds_the_line_until_a_later_enter`; `OQ`: all eight tests; `AD`: `a_directive_waits_while_a_question_shows_and_is_typed_after_it`, `a_half_typed_line_of_a_person_holds_a_directive_and_nothing_is_typed`, `a_startup_prompt_that_waits_for_the_person_holds_directives`. |
| FR-003 | PASS (automated) | `IL`: all nine tests; `DB`: `the_writer_pastes_the_text_and_presses_enter_once_it_shows`, `a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directives`; `SDH`: `a_directive_whose_text_does_not_show_is_left_unsubmitted`, `a_waiting_directive_whose_text_does_not_show_is_recorded_unsubmitted`; `AD`: `a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directive` (the member receives the text and no Enter; both records carry the reason). |
| FR-004 | PASS (automated) | `SDH`: `a_busy_target_queues_the_directive_and_types_nothing` (places 1 and 2), `a_full_queue_refuses_the_next_directive_and_records_it`; `AD`: the answers `submitted`, `waiting` with `position: 1`, and `unsubmitted` in the tests above. |
| FR-005 | PASS (automated) | `SDH`: `a_directive_that_waits_ten_minutes_is_withdrawn_untyped`, `a_target_that_stopped_withdraws_every_waiting_directive`, `a_target_moved_out_of_the_subtree_withdraws_the_waiting_directive`, `a_server_stop_withdraws_every_waiting_directive`, `hold_guard::a_hold_set_while_a_directive_waits_withdraws_it_and_counts_nothing`; `AD`: `a_waiting_directive_is_withdrawn_and_recorded_when_its_target_stops` (both records, with the text and the reason); `H/tests/agent_hold.rs`: `a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded` (the automatic hold withdraws the waiting directives). A normal stop closes the queue: `SDH`: `a_directive_sent_after_a_server_stop_is_refused_and_recorded` (refused as `the office server is stopping`, nothing typed), `a_head_claimed_before_a_server_stop_is_not_typed` (one withdrawal entry, no write); `DQ`: `a_closed_queue_takes_in_and_hands_out_nothing`. The normal-stop path at the server is wired in `H/src/bootstrap/interrupt_agents.rs`; no end-to-end test stops a server with directives waiting. |
| 0002/FR-010 (modified) | PASS (automated) | `AD`: `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter` (the member reads exactly the text and one Enter, answer `submitted`), `a_directive_waits_while_a_question_shows_and_is_typed_after_it`. |
| PB-1 | PASS (automated) | Every 0002 and 0007 refusal test in `SDH`, `AD`, `H/tests/agent_hold.rs`, and `H/tests/agent_call.rs` passes unchanged. |
| PB-2 | PASS (automated) | `SDH`: `a_busy_target_queues_the_directive_and_types_nothing` (one entry per accepted directive); `AD`: `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter` (one `ok` entry, actor the lead). |
| PB-3 | PASS (automated) | `SDH`: `a_directive_whose_text_does_not_show_is_left_unsubmitted` and `a_waiting_directive_whose_text_does_not_show_is_recorded_unsubmitted` (no receipt); `H/tests/task_text.rs`: `a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status`, `directives_at_once_leave_the_text_that_was_written_last`. |
| PB-4 | PASS (automated) | The checks read only the server's screen copy (`TerminalRuntime::screen`); no code path writes a CLI file. `H/tests/task_text.rs`: `the_task_text_neither_reads_a_cli_title_nor_writes_a_cli_configuration` passes unchanged. |
| PB-5 | PASS (automated) | `H/src/server/terminal_requests.rs` stamps and then forwards the bytes unchanged; `H/tests/terminal_protocol.rs` and `H/tests/agent_protocol.rs` input tests pass unchanged. |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PARTIAL (automated) | `AD`: `a_directive_waits_while_a_question_shows_and_is_typed_after_it` proves 0 bytes reached the member while the question showed, for one directive; `OQ` covers each question shape. 50 directives in a row were not run. |
| SC-002 | PARTIAL (automated) | `AD`: `a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directive` proves no Enter for one directive; `IL`: `a_question_that_does_not_echo_shows_nothing`. 20 directives were not run. |
| SC-003 | PARTIAL (automated) | `PI`: `a_person_holds_directives_for_the_typing_pause` (held 1 ms before 30 s, free at 30 s); `AD`: `a_half_typed_line_of_a_person_holds_a_directive_and_nothing_is_typed` (one directive). 20 directives over a real 30-second pause were not run. |
| SC-004 | PASS (automated) | `SDH`: `ten_waiting_directives_are_typed_once_each_in_the_order_sent`. |
| SC-005 | PASS (automated) | `SDH`: every withdrawal and unsubmitted test checks the entry, the text, and both agents; `AD`: `a_waiting_directive_is_withdrawn_and_recorded_when_its_target_stops`, `a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directive`. |
| SC-006 | PARTIAL (automated) | `AD`: `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter` answers `submitted` from the call itself; no test measures the 2-second bound. |
| SC-007 | PARTIAL (automated) | `AD`: `a_startup_prompt_that_waits_for_the_person_holds_directives` and the second directive in `a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directive` (one directive each). 10 directives were not run. |
| SC-008 | PARTIAL (automated) | `PI`: `an_unsent_line_holds_directives_past_the_pause_until_it_is_sent_or_cleared` (held at ten times the pause, free after a line-clearing key); `AD`: `a_half_typed_line_of_a_person_holds_a_directive_and_nothing_is_typed` (the member receives only the person's `hi`). 10 directives were not run. |

## Considered, not built

A cross-check against the reference behavior (see the pull request) named these habits of its delivery queue. This change covers or answers each:

| Habit | In 0012 |
|---|---|
| Wait until the CLI is idle | Covered: the target must wait for input (FR-002). |
| Hold while the person has an unsent line | Covered: unsent person text holds delivery until it is sent or cleared (A5, SC-008). |
| Never send Escape | Covered: the office writes only the paste and one Enter; no test path writes Escape, and A5 states it. |
| A start-up grace before the first delivery | Not built: a new session is `starting`, then `working`, and reaches `waiting for input` only after the quiet period, which already acts as a grace; a startup prompt also holds the queue (A11). |
| About 4.5 seconds between deliveries | Not built as a fixed gap: the next delivery waits 2 seconds (plan D8) and then for `waiting for input` again, which follows a full quiet period after the previous turn. |

## Checks a person must run

| Check | Verdict | What to do |
|---|---|---|
| Real CLI permission prompts | BLOCKED | Start claude, codex, gemini, and opencode agents; make each show its tool-permission question; send a directive from the lead. Expected: the answer is `waiting` and nothing is typed until the question is answered. |
| Real CLI input lines | BLOCKED | With each of those CLIs idle at its prompt, send a short and a long (over 1,000 characters) directive. Expected: `submitted` for both; for a CLI that shows a paste placeholder, the placeholder counts. |
| Real CLI trust question at start | BLOCKED | Start a CLI in a folder it has not seen, so it asks whether to trust the folder, and send a directive. Expected: `waiting` until the question is answered. |
