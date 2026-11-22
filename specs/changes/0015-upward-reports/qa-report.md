# 0015 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0015-upward-reports` with `main` merged at 9357aa1 (0012 on `main`): T2 863f8d3, T3 b429a5f, T4 ba98a50, T5 ed3f82a, tests 94eb09d and e88e1e5, desktop 78ec85b, and the T7 commit of this file. Toolchain 1.100.0, on Linux.
- **Overall status:** every requirement and success criterion has automated evidence. SC-001 and SC-006 are proven on a smaller set than the criterion names; the rows say so. macOS and Windows rely on CI.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23 at the T7 commit. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (review) | The criterion is about what the diff does not touch; a read of the diff proves it. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The integration tests in `src/apps/office-harnessd/tests/agent_report.rs` run the real server and the real `office-harnessd agent-call` helper with the credential of a live session. The agents are fake CLIs (shell scripts) that echo typed input, not real agent CLIs.
- The desktop tests run against the mock core of the e2e suite, not the real server.
- No hands-on run of the desktop app with the real server was made for this change.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` | every crate but the desktop shell crate, which does not build on this machine (GTK) | PASS (clean) |
| `cargo test --workspace --exclude office-harness-desktop` | 1,326 tests | PASS (0 failed) |
| `tests/agent_report.rs` | 11 tests, 3 runs in a row | PASS (11 passed each time) |
| `scripts/check-boundaries.sh` | workspace | PASS (boundaries ok) |
| `bun test src` | desktop, 369 tests | PASS |
| `svelte-check`, `eslint .`, `prettier --check .` | desktop | PASS (0 errors) |
| `playwright test` | desktop e2e, 238 tests, 2 workers | PASS |

## Requirements

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `report_length_bounds_hold`, `a_blank_report_is_refused`, `each_wire_kind_reads_back_and_an_unknown_kind_is_refused` (domain); `a_report_at_the_length_bound_passes_and_longer_or_blank_ones_are_refused` (integration); `a_bad_report_kind_text_or_person_flag_is_a_parse_failure` (gateway). |
| FR-002 | PASS (automated) | `a_member_and_a_lead_report_to_their_parent`, `an_agent_without_a_parent_reports_to_the_person`, `a_report_marked_for_the_person_skips_the_parent` (domain); `ten_reports_that_name_another_agent_are_refused_and_recorded`, `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive`, `a_member_marks_a_question_for_the_person_and_its_lead_gets_nothing` (integration). |
| FR-003 | PASS (automated) | `control_and_bidirectional_characters_become_visible_symbols` (domain); `the_ceo_reaches_the_person_with_the_text_in_visible_form`, `a_report_to_a_ready_parent_is_typed_as_one_line_and_sets_no_task_text` (handler); `the_ceo_reaches_needs_you_with_the_text_in_visible_form` (Needs you item and record), `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive` (the parent's typed bytes). |
| FR-004 | PASS (automated) | `a_report_to_a_busy_parent_waits_and_a_second_one_replaces_it_in_place`, `a_report_that_does_not_show_at_the_input_line_is_recorded_as_unsubmitted` (handler); `a_report_waits_while_the_parent_shows_a_question_and_is_typed_after_it`, `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive` (integration, answer `submitted`). |
| FR-005 | PASS (automated) | `a_report_to_a_stopped_parent_is_refused_in_both_records`, `a_waiting_report_of_a_moved_sender_is_withdrawn_untyped` (handler). |
| FR-006 | PASS (automated) | `exactly_the_report_limit_passes_and_the_next_is_refused`, `reports_take_no_directive_slot` (domain); `the_report_past_the_limit_is_refused_with_the_limit_name`, `an_office_hold_refuses_a_report_to_the_person_and_to_a_parent`, `a_hold_on_the_parent_refuses_its_childs_report` (handler); `holds_and_the_reports_limit_refuse_and_the_third_limit_refusal_holds_the_sender` (integration); the setting `limit_reports_per_agent` in `settings.tests.rs`, `rate_limit_source.tests.rs`, and the Limits e2e (`hold.spec.ts`, five limits). |
| FR-007 | PASS (automated) | `the_ceo_reaches_needs_you_with_the_text_in_visible_form` (answer `needs_you`, one `report` notification, the item). |
| FR-008 | PASS (automated) | `a_later_event_keeps_the_report_and_a_second_report_replaces_it` (domain); `a_report_survives_a_resume_and_a_hold_release_but_not_a_stop` (service); `a_report_item_stays_through_work_and_a_wait_and_leaves_when_the_terminal_opens`, `five_reports_for_the_person_add_one_notification_and_show_the_fifth` (integration). |
| FR-009 | PASS (automated) | See SC-010. |
| FR-010 | PASS (automated) | `a_second_report_of_a_sender_replaces_its_waiting_one_in_place_with_its_own_time`, `a_report_never_replaces_the_claimed_head_and_joins_the_end_instead`, `reports_take_at_most_their_share_and_directives_keep_the_rest`, `a_full_queue_refuses_a_new_report_but_a_replacement_still_fits` (queue); `ten_waiting_reports_leave_room_for_a_directive_from_above_and_refuse_an_eleventh`, `a_no_room_refusal_takes_no_slot_of_the_reports_limit` (handler); `ten_child_reports_leave_room_for_the_ceos_directive_and_refuse_an_eleventh` (integration). |
| 0002/FR-008 (modified) | PASS (automated) | `reports_are_an_allowed_type_and_the_refusal_names_them` (gateway); the 0002 contract note. |
| 0003/FR-006 (modified) | PASS (automated) | `a_report_while_an_unread_report_exists_adds_nothing_but_a_bell_still_does` (create handler); `a report stays unread and pings the OS even for the open and focused agent (0015)` (desktop notification store). |
| 0008/FR-002 (modified) | PASS (automated) | `an_item_with_a_report_sends_its_kind_text_and_time` (wire); desktop `needs-you.spec.ts` report rows; e2e `a report row shows the event, the kind, and the text, and names the report as its description`, `a long report is clamped, and the toggle works from the keyboard`. |
| 0008/FR-003 (modified) | PASS (automated) | `a_report_starts_an_item_in_the_waiting_group` (domain). |
| 0008/FR-005 (modified) | PASS (automated) | `a_report_survives_a_resume_and_a_hold_release_but_not_a_stop`, `an_item_without_a_report_still_leaves_on_a_resume`, `opening_the_terminal_or_dismissing_clears_a_report_item` (service). |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated, smaller tree) | `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive`: in a tree of a CEO, two leads, and two members, a member's report reaches only its lead and a lead's report only the CEO; the other lead and member receive no byte. The criterion names 9 agents with 8 reports; the test proves the rule on two levels with 5 agents. |
| SC-002 | PASS (automated) | `ten_reports_that_name_another_agent_are_refused_and_recorded`: 10 frames with `target_agent_id`, `agent_id`, `parent_agent_id`, `team_id`, or `to` naming the CEO or the lead are `invalid`, the lead's record grows by 10, and Needs you stays empty. |
| SC-003 | PASS (automated) | `the_ceo_reaches_needs_you_with_the_text_in_visible_form`: the item is in `needs_you.list` within 2 s of the call. The title count follows the item count, which the 0008 e2e suite proves for any item. |
| SC-004 | PASS (automated) | The Needs you item and the record entry of `the_ceo_reaches_needs_you_with_the_text_in_visible_form` hold U+001B, U+009B, U+202E, and LF only as visible symbols; the parent's typed bytes in `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive` hold `␛`, not U+001B; `a_report_to_a_ready_parent_is_typed_as_one_line_and_sets_no_task_text` types `done?␊yes` as one line. |
| SC-005 | PASS (automated) | `a_report_at_the_length_bound_passes_and_longer_or_blank_ones_are_refused`. |
| SC-006 | PASS (automated, one report) | `a_report_waits_while_the_parent_shows_a_question_and_is_typed_after_it`: nothing reaches the parent while the question shows; the line is typed after it. The criterion names 10 reports; the report takes the same queue and the same question check as a directive, which `a_directive_waits_while_a_question_shows_and_is_typed_after_it` (0012) proves. |
| SC-007 | PASS (automated) | `holds_and_the_reports_limit_refuse_and_the_third_limit_refusal_holds_the_sender`: 5 of 5 refused under the office hold; after release, 5 pass, the next 3 are refused naming the limit, and the sender is on hold. |
| SC-008 | PASS (automated) | `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive`: the lead sends a directive to the id in the heading and the member's terminal receives it. |
| SC-009 | PASS (automated) | `a_report_item_stays_through_work_and_a_wait_and_leaves_when_the_terminal_opens`: the CEO's item stays while it works and then waits, `notification.read_agent` removes it, and a member's report item leaves on dismiss. |
| SC-010 | PASS (automated) | Accepted: the lead's record entry in `reports_reach_only_the_parent_and_the_parent_answers_with_a_directive` and the CEO's in `the_ceo_reaches_needs_you_with_the_text_in_visible_form`. Refused: 10 entries in `ten_reports_that_name_another_agent_are_refused_and_recorded`, both records in `a_report_to_a_stopped_parent_is_refused_in_both_records`. Withdrawn: `a_waiting_report_is_withdrawn_and_recorded_when_its_parent_stops` (sender and parent). Replaced: `a_report_to_a_busy_parent_waits_and_a_second_one_replaces_it_in_place`. Unsubmitted: `a_report_that_does_not_show_at_the_input_line_is_recorded_as_unsubmitted`. |
| SC-011 | PASS (automated) | `ten_child_reports_leave_room_for_the_ceos_directive_and_refuse_an_eleventh`: with 10 reports waiting in a working lead's queue, a second report of one member keeps place 1, the 11th member is refused with the no-room reason, and the CEO's directive waits at place 11. |
| SC-012 | PASS (automated) | `five_reports_for_the_person_add_one_notification_and_show_the_fifth`. |

## Preserved behaviors

| ID | Verdict | Evidence |
|---|---|---|
| PB-1 | PASS (automated) | `agent_call` (22) and `agent_directive` (13) suites stay green. |
| PB-2 | PASS (automated) | The 0012 queue and handler tests and `agent_directive` stay green; reports join the end of the queue or replace their own entry, never jump it (queue tests). |
| PB-3 | PASS (automated) | `a_report_to_a_ready_parent_is_typed_as_one_line_and_sets_no_task_text` and the busy case: no receipt for a submitted report. |
| PB-4 | PASS (automated) | `needs_you` (15) and `notifications` (21) suites and the 0008 e2e specs stay green. |
| PB-5 | PASS (review) | The diff writes no CLI configuration and reads no prompt; delivery reuses the 0012 screen checks. |
| PB-6 | PASS (automated) | `defaults_and_refusal_text_follow_the_table` keeps the four 0007 defaults; `reports_take_no_directive_slot`. |
| PB-7 | PASS (review) | `report.send` answers once; the agent connection publishes nothing to the agent. |

## Deviations

Implementation notes. None of them defers or changes a requirement.

- **Visible form in the use case, not the gateway (PLAN D3).** The plan made the visible form in `agent_call.rs`. The use case (`SendReportHandler`) makes it instead, so the length check and the visible form sit in one place and every receiver gets the same form. The 0014 `visible_text` moved from the server app to the hierarchy domain for that; the subtree read imports it from there.
- **Ports bundle.** `SendReportHandler::new` takes a `ReportPorts` bundle, as `SendDirectiveHandler::new` takes `DirectivePorts`, and holds the directive handler whose queue a report to a parent shares.
- **Holds checked where the queue is.** A report for the person passes the hold gate in `SendReportHandler`; a report to a parent passes it in `SendDirectiveHandler::send_report`, after the room checks, so a no-room refusal takes no rate slot (FR-010).
- **A failed Needs you write does not refuse the report.** The contract's `internal` row names a tree read failure only; a failed notification or item write raises the storage alert, as for every attention event (contract amended).
- **Commits.** T6 tests are two commits (person path, parent path), and the desktop has its own commit.
