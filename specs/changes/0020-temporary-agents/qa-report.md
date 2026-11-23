# 0020 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0020-temporary-agents` with `main` merged at fa83dff (0021 on `main`) in c05eb62: T1 15e5bd2, T2 a49099e, T4 992c49c, T3 31b6449, T5 5960859, the test timing fix f55d083, the review r3 N2 fix d6c9275, and the commits of this file. Toolchain 1.100.0, on Linux.
- **Overall status:** every requirement and success criterion has automated evidence. FR-004 and SC-005 are PARTIAL (automated): their tests run the idle limit on a 300 ms minute and a 50 ms watcher tick, not the real minutes that SC-005 names. SC-009 ends a temp from the keyboard on the mock core only. The hands-on run with a real agent CLI was not made (BLOCKED). macOS and Windows rely on CI.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23 on the merge commit. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (review) | The criterion is about what the diff does not touch; a read of the diff proves it. |
| PARTIAL (automated) | An automated test proves the rule, but on a smaller scale or a shorter clock than the criterion names. The evidence column says what differs. |
| GAP | No test proves the criterion. The evidence column says why. |
| BLOCKED | The check was not run. It is never counted as a pass. |

### Scope of the automated evidence

- The integration tests in `src/apps/office-harnessd/tests/temps.rs` (named "wire" below) run the real server, real git repositories, and the real `office-harnessd agent-call` helper with the credential of a live session. The agents are fake CLIs (shell scripts) that echo typed input, not real agent CLIs.
- The wire tests set the temp watcher's tick to 50 ms and one idle-limit minute to 300 ms through `ServerOptions`, so a 5-minute limit runs in 1.5 s, well above the 400 ms quiet period of the test office. The rules are the same; only the clock is shorter.
- The desktop tests run against the mock core of the e2e suite (`e2e/temps.spec.ts`, named "e2e" below), not the real server.
- No hands-on run of the desktop app with the real server and a real agent CLI was made for this change.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` | every crate but the desktop shell crate, which does not build on this machine (GTK) | PASS (clean) |
| `cargo test --workspace --exclude office-harness-desktop` | 105 suites, 1,616 tests | PASS (0 failed) |
| `tests/temps.rs` | 17 tests, 5 runs in a row, 3 of them beside a full workspace run | PASS (17 passed each time) |
| `scripts/check-boundaries.sh` | workspace | PASS (boundaries ok) |
| `bun test src` | desktop, 445 tests | PASS |
| `svelte-check`, `eslint .`, `prettier --check .` | desktop | PASS (0 errors) |
| `playwright test` | desktop e2e, 301 tests, 2 workers | PASS (301 passed) |

## Requirements

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | Wire `the_person_creates_a_temp_that_starts_in_its_own_worktree_and_gets_its_objective` (the person's temp starts, its objective is typed first, record `create`, `start`, `directive`); `a_lead_creates_three_member_temps_and_each_done_report_reaches_the_lead_and_ends_its_temp` (an agent's temp); `the_create_rules_of_a_temp_and_the_temp_requests_refuse_what_they_must` (a blank or control-character objective, an objective without temp, temp without an objective). Unit tests of `Objective` (1 and 8,192 characters accepted; blank, 8,193, ESC, DEL, U+009B refused). E2e: the create form sends `temp` and `objective` and blocks a blank objective or a line break. |
| FR-002 | PASS (automated) | Wire `the_person_creates_a_temp_that_starts_in_its_own_worktree_and_gets_its_objective` (own branch and worktree in a repository); `the_create_rules_of_a_temp_and_the_temp_requests_refuse_what_they_must` (`own_worktree` on an agent that is not a temp is refused). E2e: own worktree is on for a temp in a repository, and the person can turn it off. |
| FR-003 | PASS (automated) | Wire `the_cap_refuses_the_next_temp_for_an_agent_and_the_person_and_holds_nobody`. Unit tests: the settings take 1 and 100 and refuse 0, 101, and text with the stored value kept; the 9th temp at the default cap is refused; two parallel creates at the cap minus one give one temp. E2e: the limits dialog refuses a cap of 0 and 101 and saves a valid one. |
| FR-004 | PARTIAL (automated) | Wire `a_silent_temp_ends_for_the_idle_limit_and_a_busy_one_and_one_under_a_hold_stay` and `a_temp_with_a_needs_you_item_is_not_ended_and_ends_one_idle_limit_after_the_dismiss`. Unit tests in `temp_run.tests.rs` (quiet-since moment, the latest of output and the last blocked moment). E2e: the idle limit refuses 4 and 1441. The idle limit runs on a 300 ms minute and a 50 ms tick, so the "within 60 seconds" bound is proven only on that clock. |
| FR-005 | PASS (automated) | Done: `a_lead_creates_three_member_temps_...`. Idle: `a_silent_temp_ends_...`. Exit: `a_temp_whose_cli_exits_with_code_1_ends_and_its_parent_hears_of_it`. Stop: `a_stop_by_the_parent_or_the_person_ends_the_temp_naming_the_actor`. End by the person: `five_entries_survive_a_restart_...`. Delete: `deleting_one_temp_leaves_a_finished_entry_naming_the_person`, `deleting_a_lead_ends_its_three_temps_and_keeps_their_worktrees`. Parent ended: `a_lead_temps_done_report_ends_its_two_member_temps_first`. The status order: `new_tasks_and_continued_sessions_on_running_temps_end_no_temp` and `a_temp_that_ends_refuses_a_new_task_and_a_second_end`; making `temp.end_check` ignore the status made the first one fail. |
| FR-006 | PASS (automated) | Wire `the_keep_rule_keeps_a_worktree_with_changes_or_commits_and_removes_a_clean_one`, `a_lead_temps_done_report_ends_its_two_member_temps_first` (children end first), and every end test reads one finished entry and one `end` entry in the record with the actor and the reason. |
| FR-007 | PASS (automated) | Wire `a_silent_temp_ends_...` (the end notice in the running parent's terminal, and a `temp_ended` notification when no parent hears of it), `a_temp_whose_cli_exits_with_code_1_ends_and_its_parent_hears_of_it`. Unit test of `end_notice_line`. Desktop unit test: a `temp_ended` notification names the reason. |
| FR-008 | PASS (automated) | Wire `five_entries_survive_a_restart_a_dismiss_removes_one_and_an_interrupted_temp_stays` (the dismiss also adds one `dismiss` entry to the ended temp's record). Unit tests: entries list newest first, survive a reopen, and a dismiss removes one row. E2e: Dismiss sends `temp.dismiss` and the entry leaves the list. |
| FR-009 | PASS (automated) | Wire `temp_list_shows_ten_temps_at_several_places_with_their_creator_and_parent`. E2e: every field of both lists, the empty texts, a kept folder and branch, End with Cancel and Escape ending nothing, keyboard alone with a visible focus, 360 px without horizontal scroll, an axe scan with no serious or critical violation, the details "Temp" row, and the stop confirmation of a temp. |
| 0015/FR-005 (MODIFIED) | PASS (automated) | Wire `a_done_report_waiting_in_a_working_parents_queue_is_typed_after_the_temp_ended`. Hierarchy unit tests of reports from a temp that stay after the temp leaves the tree. |
| PB-1 | PASS (automated) | The whole workspace suite of 0001 to 0021 passes unchanged, and the e2e create form sends neither temp field for an agent that is not a temp. |
| PB-2 | PASS (automated) | Wire `the_create_rules_of_a_temp_and_the_temp_requests_refuse_what_they_must` (the 0002 rules refuse a temp create where they refuse any create); the 0002 hierarchy suite passes unchanged. |
| PB-3 | PASS (automated) | Wire `the_cap_refuses_the_next_temp_...` (cap refusals put nobody on hold); the 0007 suite passes unchanged. |
| PB-4 | PASS (automated) | Wire `deleting_a_lead_ends_its_three_temps_and_keeps_their_worktrees`; the 0010 delete suite passes unchanged. |
| PB-5 | PASS (automated) | Wire `the_person_creates_a_temp_...` (the objective is a queue entry typed under 0012) and `a_lead_creates_three_member_temps_...` (reports under 0015). |
| PB-6 | PASS (automated) | Wire `five_entries_survive_a_restart_a_dismiss_removes_one_and_an_interrupted_temp_stays` (an `interrupted` temp is not ended after a restart). |
| PB-7 | PASS (review) | The diff writes no file in a CLI's configuration. |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | Wire `the_person_creates_a_temp_that_starts_in_its_own_worktree_and_gets_its_objective`. |
| SC-002 | PASS (automated) | Wire `a_lead_creates_three_member_temps_and_each_done_report_reaches_the_lead_and_ends_its_temp`. |
| SC-003 | PASS (automated) | Wire `the_keep_rule_keeps_a_worktree_with_changes_or_commits_and_removes_a_clean_one` (4 temps: clean, change, commit, both). |
| SC-004 | PASS (automated) | Wire `the_cap_refuses_the_next_temp_for_an_agent_and_the_person_and_holds_nobody` (cap 3; 6 refusals of one agent and none on hold). |
| SC-005 | PARTIAL (automated) | Wire `a_silent_temp_ends_...` and `a_temp_with_a_needs_you_item_is_not_ended_...` with an idle limit of 5 minutes on a 300 ms minute and a 50 ms tick, not the 6, 15, and 10 real minutes that the criterion names. The busy, held, and Needs you temps run for more than 3 idle limits and are not ended. |
| SC-006 | PASS (automated) | Wire `deleting_a_lead_ends_its_three_temps_and_keeps_their_worktrees`. |
| SC-007 | PASS (automated) | Wire `a_lead_temps_done_report_ends_its_two_member_temps_first` (each member's `end` entry is earlier than the lead's). |
| SC-008 | PASS (automated) | Wire `five_entries_survive_a_restart_a_dismiss_removes_one_and_an_interrupted_temp_stays`. |
| SC-009 | PASS (automated) | Wire `temp_list_shows_ten_temps_at_several_places_with_their_creator_and_parent`; e2e "it works by keyboard alone, with a visible focus" ends a temp from the keyboard, on the mock core, not the real server. |
| SC-010 | PASS (automated) | Wire `a_done_report_waiting_in_a_working_parents_queue_is_typed_after_the_temp_ended`. |
| SC-011 | PASS (automated) | Wire `a_temp_whose_cli_exits_with_code_1_ends_and_its_parent_hears_of_it`. |
| SC-012 | PASS (automated) | Wire `new_tasks_and_continued_sessions_on_running_temps_end_no_temp` (10 new tasks and 10 continued sessions, 0 ends). |

## Hands-on check

| Check | Verdict | Note |
|---|---|---|
| A real `claude` temp that reports `done`, in the desktop app with the real server | BLOCKED | Not run. The fake CLIs of the wire tests stand in for it. |

## Known follow-up

- **Idle end outside the status order (final review r3, N1):** the watcher claims a temp for an idle end without `temp.end_check`, so a new task that holds the status order at that moment can see its fresh session stopped. The window is milliseconds. Routing idle ends through `temp.end_check` closes it.

## Deviations from the plan

- **Dismiss record:** the plan's D11 did not name it; the spec's data flow does. A dismiss adds a `dismiss` entry (actor `user`, detail the entry id) to the ended temp's record, and the contract says so.
- **Refused from the role path:** an agent's role-path create returns the cap refusal as `refused`, through a new `Refused` kind of the hierarchy creator error, instead of `internal`.
- **Creating guard:** a new temp reads `stopped` until its start takes the status order, so the watcher skips a round while a temp create runs.
- **One settings store for a create:** the merge with 0021 keeps one required settings store in the create handler; 0021's optional one and its builder are gone, and the dependency-link switch reads the same store.
