# 0008 QA report

## Summary

- **Date:** 2026-11-22.
- **Build under test:** branch `feat/0008-p2-desktop` at the PR #92 r2 commit (the review fixes on top of 43198ad; Phase 2, desktop). Phase 1 (server) is PR #87, run at e62500f (r2).
- **Umbrella gate (2026-11-22):** after the merge of main cbbe2a3 into `feat/0008-needs-you-queue` (3b67a4d): `cargo fmt --check` clean; `scripts/lint-rust.sh` (whole workspace) clean; `cargo test --workspace` 903 passed, 0 failed; `bun run tauri build --debug --no-bundle` ok; `bun test src` 260 passed; svelte-check 0 errors, 0 warnings; eslint and `prettier --check` clean; Playwright 179 passed; `scripts/check-boundaries.sh` ok.
- **Overall status:** the automated evidence stands for every requirement, success criterion, and edge case, with the gaps named in Known gaps. The hands-on rows (screen reader pass, 200 percent zoom, the real window title on KDE, the real jump key on KDE) are BLOCKED: nobody has run them. macOS and Windows hands-on rows are WAIVED (owner, 2026-11-20, the same decision as 0003, 0005, and 0007: CI evidence suffices). No hands-on row is PASS.
- **Status of this file:** the local gate results below were run on this branch on 2026-11-22. The test names were read from the repository on 2026-11-22. The desktop gates (`bun test`, `svelte-check`, `eslint`, `prettier`, Playwright) were run again after the PR #92 review fixes; the Rust and `tauri build` rows in the table below are from a3d4415, and no Rust file changed in those fixes. The umbrella gate line above supersedes them: it ran every gate on the merged umbrella tree (3b67a4d).

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person or an agent-run pass must run the check. Nobody has run it. It is not a PASS. |
| WAIVED | The owner decided that CI evidence suffices. Nobody ran it by hand. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server.
- The window title is set by a Tauri command. The test of its text is a Rust unit test; the invoke is checked in Playwright against a mock.
- The jump key in Playwright is a synthetic key press in Chromium. It does not prove that a desktop environment delivers `Ctrl+Shift+F11` to the webview (see Manual QA).

### Local gates (2026-11-22, Linux, toolchain 1.100.0)

| Gate | Scope | Result |
|---|---|---|
| `cargo test -p agent-domain -p agent-infrastructure -p office-harnessd` | at a3d4415: 396 passed, 0 failed | PASS |
| `cargo clippy` | `agent-domain`, `agent-infrastructure`: lint clean | PASS |
| `office-harness-desktop` (Tauri crate) | at a3d4415: lint clean; 45 tests passed | PASS |
| `bun run tauri build --debug --no-bundle` | desktop, at a3d4415 | PASS (succeeded) |
| `bun test src` | desktop, after the PR #92 review fixes: 259 passed, 0 failed | PASS |
| `svelte-check` | desktop: 0 errors, 0 warnings | PASS |
| `eslint`, `prettier --check` | desktop | PASS (clean) |
| Playwright (system chromium, 4 workers) | desktop, full suite after the PR #92 review fixes: 179 passed, 0 failed, including the 20 tests of `e2e/needs-you.spec.ts` | PASS |
| Phase 1 (PR #87 r2, e62500f) | 327 tests passed on the 4 notification crates and `office-harnessd`; `tests/needs_you.rs` and `tests/notifications.rs` passed 3 times | PASS |

One Playwright test was corrected during the run. The five-press jump test expected keyboard focus in the terminal of the exited agent Ed. The terminal of a non-running agent is read-only and does not take focus (0001 terminal view, `terminal-view.svelte`, about line 211). The test now asserts that the terminal is shown, and asserts focus only for running agents. See SC-002.

`scripts/check-boundaries.sh` printed "boundaries ok" on a3d4415 (2026-11-22). Not run in this file: the CI matrix of the Phase 2 PR, `/ac:review`, and `/ac:verify` on the umbrella head. They stay open for the Final verification of the plan.

## Requirements

Paths are relative to the repository root. `N` is `src/modules/notification`, `H` is `src/apps/office-harnessd`, `D` is `src/apps/desktop`, and `A` is `src/modules/agent`. `NS` is `N/application/src/commands/needs_you/needs_you_service.tests.rs`, `NI` is `N/domain/src/entities/needs_you_item.tests.rs`, `NB` is `H/src/bridges/notification_bridge.tests.rs`, `HB` is `H/src/bridges/hold_bridges.tests.rs`, `NY` is `H/tests/needs_you.rs`, `NE` is `D/e2e/needs-you.spec.ts`, `NM` is `D/src/modules/needs-you/models/needs-you.spec.ts`, `NC` is `D/src/modules/needs-you/state/needs-you-controller.spec.ts`, and `AS` is `D/src/core/shortcuts/app-shortcuts.spec.ts`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | Record before coalescing: `NB`, `a_coalesced_bell_updates_the_item_and_creates_no_second_notification`, `a_second_bell_while_the_first_is_unread_is_coalesced`, `the_item_follows_the_notification_event_with_its_origin`, `a_coalesced_bell_after_a_clear_makes_an_item_with_a_null_origin_on_the_wire`. One item per agent, kept wait time: `NI`, `a_second_event_keeps_the_wait_time_and_changes_the_latest`, `an_agent_without_an_item_gets_a_new_one_with_its_origin_at_the_event_time`; `N/infrastructure/tests/persistence.rs`, `saving_a_second_item_of_one_agent_replaces_the_first`. Server: `NY`, `a_bell_makes_a_bell_item`. Desktop, on-screen agent: `NE`, `an event for the on-screen agent in the focused window leaves no item, title, or count announcement`; `NC`, `an item whose origin was read on arrival never shows or counts`, `a coalesced item with no origin always shows`. |
| FR-002 | PASS (automated) | Detail per kind: `NB`, `the_detail_of_each_kind_is_the_one_the_contract_names`; `NY`, `a_failed_command_shows_the_text_the_status_frame_carries`. Mapping: `N/presentation/src/mappings/notification_json.tests.rs`, `an_item_names_its_origin_notification_or_sends_null`. Row text: `NM`, `a waiting agent shows the quiet period in seconds`, `a failed agent shows the server's text as sent`, `the other kinds are named`, `the accessible name holds name, project, event, and wait time`, `an agent without a project has no project part`, `counts from the first event, in the largest fitting unit`; `NE`, `shows name, project, event, and wait time as text, in the server's order`. |
| FR-003 | PASS (automated) | Order: `NI`, `failed_comes_before_waiting_and_waiting_before_exited`, `waiting_bell_and_auto_hold_share_one_group_ordered_by_wait`, `the_longest_wait_comes_first_inside_a_group`, `equal_waits_are_ordered_by_agent_name`, `a_waiting_agent_that_fails_moves_to_the_failed_group_and_keeps_its_wait`. Server run: `NY`, `thirty_agents_with_five_events_list_exactly_those_five_in_order`. Desktop keeps the server's order: `NM`, `the items of an answer keep the server's order`; `NC`, `load reads needs_you.list in the server's order and announces nothing`. |
| FR-004 | PASS (automated) | Read clears: `NS`, `reading_the_origin_notification_clears_the_item`; `NY`, `reading_the_origin_notification_clears_the_item_and_tells_the_clients`, `read_agent_clears_the_item_and_tells_the_clients`; `NB`, `a_read_right_after_notification_created_finds_the_item_and_clears_it`. Dismiss: `NS`, `a_dismissed_agent_starts_a_new_item_at_the_new_time`, `dismissing_an_agent_without_an_item_is_not_found`; `NY`, `a_dismiss_answers_the_rest_and_marks_nothing_read`, `dismiss_refuses_an_agent_without_an_item_a_bad_id_and_an_unknown_field`. Removal by origin and agent: `N/infrastructure/tests/persistence.rs`, `an_item_is_removed_by_agent_and_by_origin`. Desktop: `NC`, `a dismiss removes the item at once and sends the agent id`, `a refused dismiss reads the list again and returns the refusal`, `a dismiss of an item that is already gone is no failure`; `NE`, `a dismiss by keyboard removes the item and leaves the agent's status`. Opening a terminal by any route reads the rows (0003/FR-009), which clears the item: `D/e2e/panel.spec.ts`, `opening a terminal by any route reads that agent's rows (FR-009)`. |
| FR-005 | PASS (automated) | `NS`, `a_status_change_clears_the_item`, `a_hold_release_clears_an_auto_hold_item_only`, `forgetting_missing_agents_keeps_the_existing_ones`. `NB`, `interrupted_keeps_an_item_and_stopped_working_and_starting_clear_it`, `working_after_waiting_for_input_clears_the_item_and_tells_the_clients`, `a_deleted_agent_keeps_its_notification_but_gets_no_item`. `HB`, `releasing_the_agent_hold_clears_its_auto_hold_item_and_tells_the_clients`, `releasing_an_agent_hold_keeps_an_item_of_another_kind`, `the_office_hold_release_keeps_the_item_of_an_automatic_agent_hold`. `NY`, `working_clears_the_item_and_interrupted_keeps_it`, `deleting_the_agent_clears_its_item`. |
| FR-006 | PASS (automated) | `N/infrastructure/tests/persistence.rs`, `needs_you_items_survive_closing_and_reopening_the_database`. `NY`, `events_while_no_client_is_connected_list_with_their_first_event_times`, `items_survive_a_server_restart_and_the_interrupted_mark`. |
| FR-007 | PASS (automated) | Table entry: `AS`, `holds twelve entries: agents 1 to 9, the switcher, the jump, the office hold`, `holds twelve distinct combinations`, `the jump is bound for the default focus shortcut`, `the table matches the combinations the server reserves`. Five presses: `NE`, `five presses from a focused terminal open the five terminals in order and empty the list`. Controller: `NC`, `the top press takes the first item at once, so a second press takes the next`. |
| FR-008 | PASS (automated) | `NE`, `says that no agent needs the user when the list is empty`, `with an empty list the view stays and the live region says so`. `NM`, `says the count, or that the list is empty`. |
| FR-009 | PASS (automated) | Count beside the heading: `NE`, `scrolls when more items than fit exist, and the title shows the full count`. Title text: `D/src-tauri/src/commands.tests.rs`, `the_title_carries_the_count_only_when_an_agent_needs_the_user`. Title invokes in both views: `NE`, `follows the dismissals in the panel`, `follows the jumps in the scene`. Announcement: `NE`, `a count change is announced once and a new item is not announced twice`; `NC`, `a changed event replaces the list and announces the count change once`. The real title on KDE is a Manual QA row. |
| FR-010 | PASS (automated) | `NE`, `in the scene it opens the top desk and leaves the panel's choices alone`, `follows the jumps in the scene`. |
| 0001/FR-017 (modified) | PASS (automated) | `AS`, `every kind of entry leaves a focused terminal, the jump too`, `the jump entry is unbound and the other entries stay bound`; `D/e2e/panel.spec.ts`, `a key that is no table combination reaches the CLI` (the free key is `Control+Alt+F11`, see Plan deviations). `NE`, `five presses from a focused terminal ...`. |
| PB-1 | PASS (automated) | The 0003 notification tests pass unchanged in the 396-test run (`N/application/.../read_notification_handler.tests.rs`, `N/presentation/src/handlers/notification_handlers.tests.rs`, `H/tests/notifications.rs`, `NB`). A coalesced bell adds no notification: `NB`, `a_coalesced_bell_updates_the_item_and_creates_no_second_notification`. No test asserts directly that Needs you adds no OS notification. |
| PB-2 | PARTIAL (automated) | No new detector: `NB`, `the_four_notifying_statuses_are_read_from_a_frame` and `stopped_and_interrupted_and_other_types_record_nothing` show the items come from the existing status and bell events. No test asserts that no CLI configuration path is written; no code path in this change writes one. |
| PB-3 | PASS (automated) | `NB`, `stopped_and_interrupted_add_no_item`; `NY`, `working_clears_the_item_and_interrupted_keeps_it`. |
| PB-4 | PASS (automated) | `NE`: the list, an empty list, and the collision line are checked with axe (`expectNoSeriousViolations`) in `shows name, project, event, and wait time as text, in the server's order`, `says that no agent needs the user when the list is empty`, and `a stored focus shortcut equal to the jump key keeps the key and unbinds the jump`. Keyboard: `a dismiss by keyboard removes the item and leaves the agent's status`, `the dismiss button shows a focus ring`, the five-press test. Text, not color: `NM`, `the accessible name holds name, project, event, and wait time`. |
| PB-5 | PARTIAL (automated) | The requests are local wire requests on the existing connection; no new outbound code was written for this change. No new test asserts it, and the existing guards of 0001 are not part of this report's runs. |
| A1 and the collision edge cases | PASS (automated) | Server stores the jump key as a focus shortcut: `A/domain/src/value_objects/settings.tests.rs`, `a_stored_jump_shortcut_is_kept_in_every_spelling`, `a_stored_value_loads_as_a_new_value_does_except_the_jump_shortcut`, `a_reserved_shortcut_is_refused_in_every_spelling_the_desktop_matches`, `each_reserved_shortcut_normalizes_to_its_canonical_form`; `A/application/.../set_setting_handler.tests.rs`, `a_shortcut_of_the_fixed_table_is_a_conflict`. Desktop: `D/src/core/shortcuts/focus-shortcut.spec.ts`, `a stored jump key is kept, in every spelling`; `AS`, `the jump entry is unbound and the other entries stay bound`, `the jump is bound for the default focus shortcut`, `no entry equals the focus shortcut`. Collision line and behavior: `NE`, `a stored focus shortcut equal to the jump key keeps the key and unbinds the jump`, `the collision line is absent with the default focus shortcut`. The line text is the one in the README. |
| SC-001 | PASS (automated) | Server: `NY`, `thirty_agents_with_five_events_list_exactly_those_five_in_order` (30 agents, 5 items, the order of FR-003). Desktop: `NE`, `shows name, project, event, and wait time as text, in the server's order`; the jump shows the top agent's terminal with focus in the five-press test. The 1 second bound is tested for the first jump: `NE`, `five presses from a focused terminal ...` waits at most `FIRST_JUMP_BUDGET_MS` (1 s) for it, on the mock core. A real-CLI run is BLOCKED (Manual QA). |
| SC-002 | PASS (automated) | `NE`, `five presses from a focused terminal open the five terminals in order and empty the list`. Corrected during the 2026-11-22 run: the first version expected focus in the terminal of the exited agent Ed. The terminal of a non-running agent is read-only and takes no focus (0001 terminal view, `terminal-view.svelte`, about line 211). Per Efe's D5 clarification (2026-11-22), the jump now puts focus on the terminal view heading ("Terminal, Ed") for an agent that is not running; the test asserts that for Ed and focus in the terminal for the running agents. See Plan deviations 7. |
| SC-003 | PASS (automated) | Server: `NY`, `a_change_reaches_a_connected_client_within_two_seconds`, `a_wait_is_announced_after_the_quiet_period_and_within_two_seconds_of_it`. Desktop title: `NE`, `follows the dismissals in the panel`, `follows the jumps in the scene`. |
| SC-004 | PASS (automated) | `NY`, `events_while_no_client_is_connected_list_with_their_first_event_times` (three events with no client; all three listed with their first event times). The 3 second bound after the app opens is not timed by a test. |
| SC-005 | PASS (automated) | `NY`, `an_agent_that_keeps_writing_and_rings_no_bell_gets_no_item`. |
| US1, US2, US3 | PASS (automated) | The `NY` runs of FR-001 to FR-006 and SC-001, SC-003 to SC-005, and the Playwright runs in `NE` (`list`, `jump`, and `title` tests above). |
| ACF-160 | PASS (automated) | The always-true `isTerminalShortcut` field is removed from the desktop shortcut table. `AS`, `every kind of entry leaves a focused terminal, the jump too`; the `bun test` and `svelte-check` runs are clean. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| No agent waits (Empty) | PASS (automated) | `NE`, `says that no agent needs the user when the list is empty`, `with an empty list the view stays and the live region says so`. |
| Event for the agent whose terminal is open in the focused window (Boundary) | PASS (automated) | `NE`, `an event for the on-screen agent in the focused window leaves no item, title, or count announcement`; `NC`, `an item whose origin was read on arrival never shows or counts`. |
| An agent with an item rings the bell many times (Boundary) | PASS (automated) | `NB`, `a_coalesced_bell_updates_the_item_and_creates_no_second_notification`; `NI`, `a_second_event_keeps_the_wait_time_and_changes_the_latest`. |
| Two items with the same wait time (Boundary) | PASS (automated) | `NI`, `equal_waits_are_ordered_by_agent_name`. |
| More items than fit (Boundary) | PASS (automated) | `NE`, `scrolls when more items than fit exist, and the title shows the full count`. |
| The top agent is deleted at the press (Concurrent) | PASS (automated) | `NE`, `an agent deleted at the press is skipped for the next item`; `NC`, `the top press skips an agent that no longer exists`. |
| Two fast presses before the server answers (Concurrent) | PASS (automated) | `NE`, `two fast presses before the server answers open the first and then the second item`; `NC`, `the top press takes the first item at once, so a second press takes the next`. |
| An agent changes back to `working` before the user looks (Concurrent) | PASS (automated) | `NB`, `working_after_waiting_for_input_clears_the_item_and_tells_the_clients`; `NY`, `working_clears_the_item_and_interrupted_keeps_it`. |
| The server restarts while items exist (Failure) | PASS (automated) | `NY`, `items_survive_a_server_restart_and_the_interrupted_mark`. |
| The OS does not show the window title (Failure) | PARTIAL (automated) | The count beside Needs you is tested (FR-009). No test removes the title bar. |
| The project switcher hides the item's project (Boundary) | PASS (automated) | `NE`, `opens an agent of a project that the choice hides, as the switcher does`. |
| The server cannot be reached or the token is rejected (Unauthorized) | PARTIAL (automated) | The 0001 states are unchanged; no 0008 test targets them. A refused dismiss shows its message and reads the list again: `NC`, `a refused dismiss reads the list again and returns the refusal`. |
| An agent connection asks for `needs_you.*` (Unauthorized) | PASS (automated) | `H/tests/agent_call.rs`, `an_agent_may_not_ask_for_a_type_outside_the_allowlist` (`needs_you.list` and `needs_you.dismiss` answer `refused`). |
| A bad `agent_id` in `needs_you.dismiss` (Invalid) | PASS (automated) | `NY`, `dismiss_refuses_an_agent_without_an_item_a_bad_id_and_an_unknown_field`. |
| A database made before this change (Migration) | PASS (automated) | `N/infrastructure/tests/persistence.rs`, `a_database_at_notification_0002_upgrades_with_every_row_intact`. CI on three OSes is open. |
| A stored focus shortcut equals the jump key (Collision) | PASS (automated) | See A1 above. |
| A hold by the office releases (Concurrent) | PASS (automated) | `HB`, `releasing_the_agent_hold_clears_its_auto_hold_item_and_tells_the_clients`. |

## Plan deviations

1. **`AgentNameLookup` port.** Added so the notification module reads an agent's name without depending on the agent module. Accepted by Efe.
2. **T3 and T4 in one commit.** Accepted by Efe.
3. **MEDIUM-2, contract text.** The `detail` of a `failed` item is the status text of the server, not a text built from the exit code in the desktop. Decided by god; the contract says so.
4. **The free key in the e2e test is `Control+Alt+F11`,** because `Ctrl+Shift+F12` is the default focus shortcut. `D/e2e/panel.spec.ts`, `a key that is no table combination reaches the CLI`.
5. **`focus-shortcut.ts` moved to `D/src/core/shortcuts/`** so the shared state can load the focus shortcut once at app start. A change of the setting applies at the next app start.
6. **A dismiss that answers `not_found` is a success in the desktop,** because the item is already gone.
7. **Jump focus for an agent that is not running** (D5 clarification by Efe, 2026-11-22): focus lands on the terminal view heading, not in the read-only terminal. The heading's accessible name now names the agent ("Terminal, <agent name>"), and the terminal section keeps the name "Terminal". `NE`, `five presses from a focused terminal ...` asserts it for the exited agent Ed.

## Known gaps (open only)

1. **The 3 second (SC-004) bound is not timed by a test.** The 1 second bound of SC-001 is tested on the mock core only, and the server bound of 2 seconds (SC-003) is tested.
2. **PB-2 and PB-5 have no test of their own.** They rest on code reading and on the existing 0001 guards.
3. **No test shows the window title with a missing title bar** (tiling layout).
4. **No hands-on pass** of the rows in Manual QA.

## Manual QA

An agent ran the first row in the real app and server on 2026-11-22 (Linux, KDE Plasma Wayland, umbrella tree after the merge of main cbbe2a3). No person has run any row; the person-only rows stay BLOCKED for the owner.

| Check | Verdict | Note |
|---|---|---|
| Real app and server, keyboard only, with fake CLIs: four agents fail (exit 3), go quiet, ring the bell, and exit; then the app is closed and opened again; then the jump key is pressed four times | PASS (agent) | Through WebDriver key actions and the office protocol. The server order was failed, then the bell and the quiet agent by first event, then exited; the failed detail was "the command ended with exit code 3" and the exited detail null (FR-002, FR-003). The section showed "Needs you (4)" with the four rows and their wait times before and after the app restart (FR-006). The script pressed the key, waited 1.0 s, then read the focus; each read took 1.03 to 1.07 s after the press including the WebDriver round trips, and each found the focus already on its target (SC-001 asks for 1 s; the run shows the target was reached within that 1.0 s wait, not a finer time): focus on the heading "Terminal, Fa" and "Terminal, Ex" for the agents that do not run, and in the terminals of Be and Qu for the running ones (FR-007, D5 clarification). After four jumps `needs_you.list` was empty and the section said "No agent needs you" (FR-004, FR-008). The native window title was not read (see the next rows). The bell row text was not seen in the real app: by the time the section was read, the bell agent had gone quiet, so its latest event was waiting for input; the bell text is covered by Playwright only. Raw evidence (`run.json`, `after-jumps.png`) is kept outside the repository in the QA agent's folder `qa/evidence/0008-needs-you/`. |
| Screen reader pass: Needs you, the count announcement, dismiss, and the jump | BLOCKED | The automated checks are axe and the live-region text only. The spoken output is unproven. |
| 200 percent zoom: the Needs you section, the collision line, and the scroll of a long list | BLOCKED | Only the two panel widths of the Playwright run are checked. |
| The real window title on KDE Plasma, in the panel and in the scene | BLOCKED | The title text is a Rust unit test and the invoke is mocked in Playwright. |
| The real jump key, `Ctrl+Shift+F11`, on KDE Plasma (a global binding of the desktop could take it) | BLOCKED | Playwright sends a synthetic key. If KDE binds the key, the `Cmd`/`Ctrl+Alt` fallback of the 0003 spec applies; it is not shipped. |
| A real agent CLI that goes quiet, rings the bell, fails, and exits, then the jump | BLOCKED | The agent run above uses fake CLIs. |
| The jump key on macOS (with `Fn`) and the window title on macOS | WAIVED | Owner, 2026-11-20: CI evidence suffices, as for 0003, 0005, and 0007. |
| The jump key and the window title on Windows | WAIVED | Same decision. |
