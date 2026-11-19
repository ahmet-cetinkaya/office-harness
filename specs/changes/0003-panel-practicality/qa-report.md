# 0003 QA report

## Summary

- **Date:** 2026-11-19.
- **Build under test:** `main` at 50e5805 (merge of PR #77). The change landed in these merged PRs: #32 (T1 and T2: last output line, bell), #39 (T3: notification module), #41 (T4: last view, shortcut validation), #46 (T5: panel view), #49 (T6: shortcuts and switcher), #51 (T6b: name matcher), #59 (T7: notifications, OS delivery, restored last view), the review follow-ups #44, #47, and #51, the scene-aware reads in #74, and the read fixes in #77 (ACF-157).
- **Overall status:** the automated evidence stands. The hands-on checks have not run, so no row that needs a person is a PASS. Some rows are PARTIAL, and PB-1 is a GAP.
- **Status of this file:** the test names were read from the repository at 50e5805 on 2026-11-19. This task ran no test, build, or browser: it is a docs task. The run results below come from the PR reviews and from the local runs that the ticket reports.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap, and Known gaps states what is not. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server. The OS notification call is a test double in `notification-store.spec.ts`; no test sends a real OS notification.
- Playwright runs on Ubuntu only in CI (since #57). The Rust tests and the webview tests run on Linux, macOS, and Windows.
- `src/apps/office-harnessd/tests/notifications.rs` runs on all three OSes. The bell test `a_bell_from_the_child_becomes_one_bell_event` first failed on Windows in the review of #32; it is in the suite that CI runs green on all three OSes at the merged PRs below.

### Run evidence

| Run | Scope | Result |
|---|---|---|
| Local, Celal, 2026-11-19 | Desktop `bun test src` on the #77/#78 trees | 224 pass, 0 fail |
| Local, Celal, 2026-11-19 | Desktop Playwright, full suite, on the #77/#78 trees | 157 to 159 passed, 0 failed |
| Local, Celal, 2026-11-19 | Desktop `svelte-check`, on the #77/#78 trees | 0 errors |
| CI, merges to `main` | Linux, macOS, Windows | green (reported on the ticket; this file did not read a run for 50e5805 itself) |
| CI 44537846067 | PR #77 at 0c83c65 (reviewed PASS) | green on 3 OSes; Ubuntu Playwright 157 of 157 |
| CI 44525304129, attempt 2 | PR #74 at bcda0f9 (reviewed PASS) | green on 3 OSes; Ubuntu Playwright 139 of 139 |
| CI 44476421286 | PR #59 at 7c95488 (reviewed PASS, round 2) | green on 3 OSes; Ubuntu Playwright 120 passed; 81 Rust `test result: ok` lines |
| CI 44332593441 | PR #51 at 24aaec6 (reviewed PASS) | green on 3 OSes |
| CI 44295781460 | PR #46 at 436f62b (reviewed PASS, round 3) | green on 3 OSes |
| CI 44289760736 | PR #47 at 39aa885 (reviewed PASS) | green on 3 OSes |
| CI 44270860775 | PR #44 at e431a4b (reviewed PASS) | green on 3 OSes |
| CI 44234801473 | PR #39 at 68c25f0 (reviewed PASS) | green on 3 OSes |

Not run for this file: any Rust gate. The local runs above did not include `cargo`. Final review rounds: PR #32 PASS in round 2 (`office-harness-pr32-r2.md`); PR #41 PASS on code in round 3 (`office-harness-pr41-r3.md`), with the CI gate left to the merge; PR #49 PASS on code with the CI gate open on a macOS flake from `main` (`office-harness-pr49.md`). All three merged (heads 8eba211, a44d029, 87a596a); this file did not read the CI runs that closed the #41 and #49 gates.

## Requirements

Paths below are relative to the repository root. `H` is `src/apps/office-harnessd`, `N` is `src/modules/notification`, `T` is `src/modules/terminal/infrastructure`, `A` is `src/modules/agent`, and `D` is `src/apps/desktop`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `D/e2e/panel.spec.ts`: `a click opens the agent's terminal and clears its unread row (FR-009)`, `the last view restores the project, the agent, and its terminal (FR-008)`. The terminal opens in the panel's own page in every e2e test; no test opens a second window, and the app has one window. `D/e2e/office-scene.spec.ts`: `a click embeds the terminal with focus, no second window`. |
| FR-002 | PASS (automated) | `D/e2e/panel.spec.ts`: `All projects lists the agents of every project and the agent without one`, `a row names the agent, its project, its status, and its role and team`, `the last output line shows on the row`. `D/src/modules/panel/models/panel.spec.ts`: `all projects shows the agents of every project and the no-project agents`, `the project name comes from the agent's project, or not at all`. Server line: `T/src/services/screen_emulator.tests.rs`: `the_last_line_is_the_last_non_empty_line_without_codes`, `a_carriage_return_redraw_counts_once_in_the_last_line`, `a_line_wrapped_across_rows_reads_as_one_line`, `an_empty_screen_has_no_last_line`, `the_last_line_follows_the_alternate_screen`. `T/tests/terminal_runtime.rs`: `the_last_line_is_reported_when_it_changes_and_throttled_after`. |
| FR-003 | PASS (automated) | `D/e2e/panel.spec.ts`: `a row names the agent, its project, its status, and its role and team`, `a row without a role and team leaves them out of the name`. `D/src/modules/panel/models/panel.spec.ts`: `the hierarchy gives the role, and the team name for a team agent`, `the hierarchy line reads as text, or is empty outside the hierarchy`. |
| FR-004 | PASS (automated) | Click: `D/e2e/panel.spec.ts`, `a shortcut reaches a live terminal with output within one second (SC-002)` (the test clicks a row first), `a click opens the agent's terminal and clears its unread row (FR-009)`. Shortcut: `shortcut 1 opens the first agent's terminal`, the table test `shortcut ${position} opens the agent at position ${position}` (each of 1 to 9), `agent 10 is reached through the switcher, by no key`. Switcher: `the switcher shortcut opens the dialog with focus in the filter`, `the filter narrows the list and a choice opens that terminal`. Table: `D/src/core/shortcuts/app-shortcuts.spec.ts`, `each agent combination maps to its own position`, `position 10 and above have no direct shortcut (A1)`. |
| FR-005 | PASS (automated) | `D/e2e/panel.spec.ts`: `one project shows only its own agents, and All projects brings the rest back`. `D/src/modules/panel/models/panel.spec.ts`: `one project shows only its own agents`, `a no-project agent never shows under one project`. |
| FR-006 | PARTIAL (automated) | Exit and fail: `H/src/bridges/notification_bridge.tests.rs`, `each_notifying_status_creates_one_row_with_the_copied_name`, `the_four_notifying_statuses_are_read_from_a_frame`. Waiting for input: `H/tests/agent_protocol.rs`, `the_status_follows_starting_working_waiting_and_exited_or_failed`; `T/tests/terminal_runtime.rs`, `output_after_a_quiet_period_is_reported_once_for_the_change_not_for_each_chunk`. Bell: `T/src/services/screen_emulator.tests.rs`, `a_bell_outside_an_osc_string_is_counted_and_taken_once`, `every_bell_of_a_repeated_ring_is_counted`, `a_bell_that_ends_a_window_title_is_not_a_bell`, `a_bell_split_across_reads_is_counted_once_when_it_completes`; `T/tests/terminal_runtime.rs`, `a_bell_from_the_child_becomes_one_bell_event`; `H/src/bridges/notification_bridge.tests.rs`, `a_bell_of_the_current_run_creates_the_notification`, `a_late_bell_of_an_earlier_run_creates_nothing`. Exception (a): `N/application/src/commands/create_notification/create_notification_handler.tests.rs`, `a_second_bell_while_the_first_is_unread_adds_nothing`, `a_bell_after_the_unread_one_was_read_creates_a_new_one`, `the_bells_of_two_agents_do_not_coalesce`; `H/src/bridges/notification_bridge.tests.rs`, `a_second_bell_while_the_first_is_unread_is_coalesced`. Exception (b): `D/src/modules/notification/state/notification-store.spec.ts`, `an event for the open and focused agent shows read, with no OS ping (D5)`, `an unfocused window still pings the OS for the selected agent (D5)`; `D/e2e/office-scene.spec.ts`, `an event of the open desk's agent creates no unread row (FR-006 b, LOW-2)`, `a new event of the panel-selected agent stays unread while the scene is open (FR-006 b)`. Gap: the OS notification itself is a test double, so no test proves that the OS shows it (Manual QA). |
| FR-007 | PASS (automated) | `H/tests/notifications.rs`: `events_without_a_client_list_as_unread_after_a_client_connects`, `a_new_event_reaches_a_connected_client_as_notification_created`. `N/infrastructure/tests/persistence.rs`: `the_listing_is_unread_first_then_newest_first`. `D/e2e/panel.spec.ts`: `the unread count shows as text, and the dialog lists the rows (FR-007)`. |
| FR-008 | PASS (automated) | Server setting: `A/domain/src/value_objects/settings.tests.rs`, `a_last_view_of_the_documented_shape_stores_and_reads_back`, `an_empty_last_view_clears_the_stored_view`, `a_last_view_past_one_kib_is_refused`, `a_last_view_that_is_not_the_documented_shape_is_refused`. `A/application/src/commands/set_setting/set_setting_handler.tests.rs`, `a_last_view_of_the_documented_shape_is_saved_and_returned`. Model: `D/src/modules/notification/models/last-view.spec.ts`, `a present project and agent restore as they were`, `a missing project falls back to all projects`, `a deleted agent is named and restores no selection (US3)`. Screen: `D/e2e/panel.spec.ts`, `the last view restores the project, the agent, and its terminal (FR-008)`, `the app writes the last view back as the user moves (D6)`, `returning from the scene does not re-run the restore (FR-008, D6)`. |
| FR-009 | PASS (automated) | `N/application/src/queries/list_notifications/list_notifications_handler.tests.rs`: `read_agent_marks_only_that_agents_rows`, `reading_an_unknown_agent_marks_nothing_and_fails_not`. `H/tests/notifications.rs`: `a_read_marks_one_row_and_read_agent_marks_one_agents_rows`. `D/src/modules/notification/state/notification-store.spec.ts`: `opening a notification reads it (FR-009)`, `opening an agent's terminal reads all of that agent's rows (FR-009)`. `D/e2e/panel.spec.ts`: `a click opens the agent's terminal and clears its unread row (FR-009)`, `opening a terminal by any route reads that agent's rows (FR-009)`. |
| 0001/FR-017 (modified) | PASS (automated) | `D/e2e/terminal.spec.ts`: `the focus shortcut leaves the terminal and is the only key held back`, `a configured shortcut replaces the default`. `D/e2e/panel.spec.ts`: `a key that is no table combination reaches the CLI`. `D/src/core/shortcuts/app-shortcuts.spec.ts`: `no entry equals the focus shortcut`, `a key with the wrong modifiers matches nothing`. `D/e2e/panel.spec.ts` presses the keys with a terminal focused: `shortcut 1 opens the first agent's terminal`, `shortcut <n> opens the agent at position <n>` (positions 2 to 9), and `the switcher shortcut opens the dialog with focus in the filter`. |
| PB-1 | GAP | No 0003 test closes the app and checks that an agent lives and records. The 0001 tests own the keep-running rule. This change adds no code that stops an agent on close: the notification module and the last view only read and store. |
| PB-2 | PARTIAL (automated) | The notification rows live in the office database and reach only connected clients: `H/tests/notifications.rs`, `a_new_event_reaches_a_connected_client_as_notification_created`. No test captures network traffic or shows that the OS call stays on the machine; the network check is a 0001 row. |
| PB-3 | PARTIAL (automated) | `D/e2e/panel.spec.ts`: `keyboard focus is visible on a row at both widths`, `Tab stays inside the dialog and Escape returns focus`, the shortcut and switcher tests of FR-004, `the notification dialog is axe clean at both widths`. Not proven: no test opens the notification dialog or the project switcher by keyboard; the tests click the dialog open and use `selectOption` for the project. See Known gaps. |
| PB-4 | PASS (automated) | `D/e2e/panel.spec.ts`: `a row names the agent, its project, its status, and its role and team` (status in the accessible name), `an agent whose stop request is open shows Stopping`. Axe at both widths: `expectNoSeriousViolationsAtBothWidths` in `All projects lists...`, `the empty office keeps its offer at both widths`, `the notification dialog is axe clean at both widths`. |
| PB-5 | PASS (automated) | `D/e2e/app.spec.ts`: `the connecting state tells the user that the app is connecting`, `offers to start the server and reaches the office when it starts`, `lists the steps to give the app a valid token`, `a state change from the core replaces the view without a reload`. The 0001 states render before the panel in `the connected state shows the office shell`. |
| SC-001 | PARTIAL (automated) | Status change in place: `D/e2e/panel.spec.ts`, `a status and a line change update the row in place and keep the selection and focus`. Server throttle: `T/tests/terminal_runtime.rs`, `the_last_line_is_reported_when_it_changes_and_throttled_after` (the report interval is 1 second, `LINE_REPORT_EVERY`). Gap: no test runs 30 agents in 3 projects and measures 2 seconds. Hardware time is BLOCKED (Manual QA). |
| SC-002 | PASS (automated) | `D/e2e/panel.spec.ts`: `a shortcut reaches a live terminal with output within one second (SC-002)` (asserts less than 1,000 ms against the mock core). Real server latency is BLOCKED (Manual QA). |
| SC-003 | PASS (automated) | `D/e2e/panel.spec.ts`: `one project shows only its own agents, and All projects brings the rest back`. The project switcher is one control and one action per choice. |
| SC-004 | PARTIAL (automated) | Each of the four events creates its row without a client wait: see FR-006. No test measures the 10 seconds from the event to the OS notification. Hands-on timing is BLOCKED (Manual QA). |
| SC-005 | PASS (automated) | `H/tests/notifications.rs`, `events_without_a_client_list_as_unread_after_a_client_connects`: 3 agents change status while no client is connected (wait, exit, fail, plus the user's stop that records nothing), and a client that connects later lists exactly 3 unread rows. |
| SC-006 | PARTIAL (automated) | `D/e2e/panel.spec.ts`, `the last view restores the project, the agent, and its terminal (FR-008)`. No test times the restore against 3 seconds. Hands-on timing is BLOCKED (Manual QA). |
| SC-007 | BLOCKED | A timed human task with 30 agents (find the waiting agent and type within 10 seconds of the notification). No automated test can stand in for it. |
| US1, US2, US3 | PARTIAL (automated) | The tests of FR-001 to FR-009 above. US1 scenario 1 and 4: `panel.spec.ts` `All projects ...` and `one project ...`. US2 scenario 7: `H/src/bridges/notification_bridge.tests.rs`, `stopped_and_interrupted_create_nothing`; `H/tests/notifications.rs`. US3 scenario 2: `D/e2e/panel.spec.ts`, `a last view whose agent is gone says so and opens nothing (US3)`. Not proven: US2 rests on FR-006, which is PARTIAL (the OS notification is a test double). |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| A project has no agents (Empty) | PASS (automated) | `D/e2e/panel.spec.ts`: `an empty project says so and offers to create an agent`, `the agent form opens with the empty project's project chosen`, `the empty office keeps its offer at both widths`. `D/src/modules/panel/models/panel.spec.ts`: `an empty project is named, an empty choice under all projects is not`. |
| An agent produced no output yet (Empty) | PASS (automated) | `T/src/services/screen_emulator.tests.rs`, `an_empty_screen_has_no_last_line`. |
| An agent has no project (Empty) | PASS (automated) | `D/e2e/panel.spec.ts`, `All projects lists the agents of every project and the agent without one`. `panel.spec.ts` (unit), `a no-project agent never shows under one project`. |
| The last output line is long or has color and cursor codes (Boundary) | PARTIAL (automated) | Codes: `T/src/services/screen_emulator.tests.rs`, `the_last_line_is_the_last_non_empty_line_without_codes`. The server cuts the line at 200 characters (`LAST_LINE_CHARS`). No test checks the cut, and no test checks that the row shortens the text to fit. |
| More than nine agents (Boundary) | PASS (automated) | `D/e2e/panel.spec.ts`, `agent 10 is reached through the switcher, by no key`. `D/src/core/shortcuts/app-shortcuts.spec.ts`, `position 10 and above have no direct shortcut (A1)`. |
| Many agents finish at the same moment (Boundary) | PASS (automated) | `N/application/.../create_notification_handler.tests.rs`, `every_non_bell_event_creates_its_own_row`. `N/infrastructure/tests/persistence.rs`, `rows_of_one_millisecond_keep_their_insertion_order_newest_first`. `H/tests/notifications.rs`, `events_without_a_client_list_as_unread_after_a_client_connects` (three events in one burst). |
| The focus shortcut is set to a key that an agent or switcher shortcut uses (Invalid) | PASS (automated) | `A/domain/src/value_objects/settings.tests.rs`: `a_focus_shortcut_that_a_fixed_shortcut_takes_names_the_conflict`, `a_reserved_shortcut_is_refused_in_every_spelling_the_desktop_matches`, `each_reserved_shortcut_normalizes_to_its_canonical_form`, `a_reserved_shortcut_with_an_extra_modifier_is_not_the_same_combination`. `A/application/.../set_setting_handler.tests.rs`, `a_shortcut_of_the_fixed_table_is_a_conflict`. `D/src/core/shortcuts/app-shortcuts.spec.ts`, `the table matches the combinations the server reserves`. |
| The agent or the project of the last view was deleted (Invalid) | PASS (automated) | `D/src/modules/notification/models/last-view.spec.ts`: `a missing project falls back to all projects`, `a deleted agent is named and restores no selection (US3)`. `D/e2e/panel.spec.ts`, `a last view whose agent is gone says so and opens nothing (US3)`. |
| A notification refers to a deleted agent (Invalid) | PASS (automated) | `D/e2e/panel.spec.ts`, `a row of a deleted agent says so, reads, and opens nothing (edge case)`. `H/src/bridges/notification_bridge.tests.rs`: `a_notification_outlives_a_deleted_agent_with_its_copied_name`, `a_notification_of_an_already_deleted_agent_says_it_was_deleted`. `N/infrastructure/tests/persistence.rs`, `a_row_stays_listed_when_no_agent_row_exists_anywhere`. |
| One agent rings the bell again while its bell notification is unread (Boundary) | PASS (automated) | See FR-006 exception (a). |
| An agent changes status while the user reads the list (Concurrent) | PASS (automated) | `D/e2e/panel.spec.ts`, `a status and a line change update the row in place and keep the selection and focus`. |
| The OS blocks notifications for the desktop app (Failure) | PASS (automated) | `D/src/modules/notification/state/notification-store.spec.ts`, `a denied OS permission stays silent, and the list still works (D7)`. The permission result is a test double; a real OS block is BLOCKED (Manual QA). |
| The app cannot reach the server or its token is rejected (Unauthorized) | PASS (automated) | See PB-5. |

## Known gaps (open only)

1. **No test runs 30 agents.** SC-001 and SC-007 name 30 agents in 3 projects. The e2e suite proves the in-place update on a few agents, and the server proves the 1-second line throttle.
2. **No test times a notification against 10 seconds (SC-004) or a restore against 3 seconds (SC-006).**
3. **The OS notification is never sent in a test.** `notification-store.spec.ts` uses a double for the call, and the Tauri plugin is wired in `D/src-tauri/src/main.rs` without a test of its own.
4. **No test checks the cut of a long last output line.** The server cuts at 200 characters; the row's shortening to fit has no test.
5. **PB-1 has no 0003 test.** The 0001 tests own the keep-running rule.
6. **No test opens the notification dialog or the project switcher by keyboard** (PB-3). The tests click the dialog open and use `selectOption` for the project.
7. **The shortcut defaults are unverified on KDE Plasma and on macOS in media-key mode** (see the approved Clarifications of 2026-11-19 and `research.md` R1). No test covers either desktop.
8. **The CI runs that closed the #41 and #49 gates were not read for this report.** Both PRs were reviewed PASS on code and merged.
9. **Playwright runs on Ubuntu only in CI.** The e2e suite is not run on macOS or Windows.

## Manual QA

Nobody has run these checks. Each stays BLOCKED until a person runs it.

| Check | Verdict | Note |
|---|---|---|
| SC-007: with 30 agents, find the agent that waits and type into it within 10 seconds of the notification | BLOCKED | Needs a person and a stopwatch. |
| SC-001 on real hardware: 30 agents in 3 projects, status and last line within 2 seconds | BLOCKED | Needs a person and 30 running agents. |
| SC-004: the OS notification within 10 seconds of exit, fail, bell, and waiting for input | BLOCKED | Needs a person on the running app. |
| SC-006: the panel shows the last view within 3 seconds of opening | BLOCKED | Needs a person with a timer on the running app. |
| OS notification delivery on Linux, macOS, and Windows, and the blocked-permission case | BLOCKED | The tests use a double for the OS call. |
| Shortcut defaults on KDE Plasma, and on macOS with and without the Fn setting | BLOCKED | Needs a person on each desktop (approved Clarifications, 2026-11-19). |
| Screen reader pass: row names, unread count, and the notification announcement | BLOCKED | Needs a person with a screen reader. |
| 200 percent zoom: the panel, the notification dialog, and the switcher | BLOCKED | The e2e suite runs at the default zoom. |
