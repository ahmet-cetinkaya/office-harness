# 0007 QA report

## Summary

- **Date:** 2026-11-19.
- **Build under test:** branch `feat/0007-p2-desktop`. All local gates ran on the tree of 855b01b (Phase 1, the #71 and #73 review fixes, T7 and T8); the later commits change docs only. Phase 1 (PR #71) was reviewed PASS at 32ad2ff with CI green.
- **Overall status:** the automated evidence stands. An agent ran the hands-on rows that need no person on Linux/KDE Wayland on 2026-11-19 (see Manual QA): the pre-upgrade restore is PASS (agent-run), and the others are PARTIAL (fake CLI, Linux only, no screen reader). Rows that need a person, a real CLI, macOS, or Windows are not PASS.
- **Status of this file:** the local gate results below were run on this branch on 2026-11-19. The test names were read from the repository on 2026-11-19. This task ran no test, build, or browser: it is a docs task.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap, and Known gaps states what is not. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server.
- The integration tests in `src/apps/office-harnessd/tests/agent_hold.rs` run on Linux and macOS (they use the unix agent-call bridge), as the 0002 integration tests do.
- The local Rust gates ran per crate, because the `office-harness-desktop` crate does not build on the Linux machine used here (libdbus-sys). CI builds it on all three OSes.

### Local gates (2026-11-19)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --check` | Rust, at 855b01b | PASS |
| `cargo clippy -D warnings` | `agent-domain`, `hierarchy-application`, `office-harnessd`, at 855b01b | PASS |
| `cargo test` | `office-harnessd` and `hierarchy-application`: 23 suites ok, including `agent_hold` (10 tests) and the wall-clock-step test, at 855b01b | PASS |
| `scripts/check-boundaries.sh` | at 855b01b | PASS |
| `bun test src` | desktop, at 855b01b: 224 pass, 0 fail | PASS |
| Playwright | desktop, full suite, at 855b01b: 148 passed | PASS |
| `svelte-check` | desktop, at 855b01b: 0 errors | PASS |
| `eslint`, `prettier --check` | desktop, at 855b01b | PASS (clean) |
| CI | Phase 1, PR #71, at 32ad2ff | PASS (green; reviewed PASS) |

Not run: the Phase 2 PR has no CI result in this file. Each commit was not built on its own; the gates cover the tree of 855b01b.

## Requirements

Paths below are relative to the repository root. `H` is `src/apps/office-harnessd`, `HD` is `src/modules/hierarchy/domain/src/services`, `HA` is `src/modules/hierarchy/application/src/commands/hold/hold_service.tests.rs`, and `D` is `src/apps/desktop`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | Server: `HA`, `setting_and_releasing_the_office_hold_journals_each_change`. Wire: `H/src/server/hold_requests.tests.rs`, `only_the_four_hold_requests_are_accepted`. Desktop: `D/e2e/hold.spec.ts`, `the office is held and released by keyboard, and says so as text`; `D/src/modules/hold/state/hold-controller.spec.ts`, `the office hold sends no agent id and the agent hold sends one`. |
| FR-002 | PASS (automated) | `HD/hold.tests.rs`: `office_hold_wins_over_agent_holds`. `H/tests/agent_hold.rs`: `an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types`, `a_directive_to_a_non_descendant_while_the_office_is_held_gets_the_tree_reason`. `HA`: `an_office_hold_refusal_beats_the_limit_and_journals_nothing_itself`. |
| FR-003 | PASS (automated) | `HA`: `holding_twice_is_a_conflict_with_the_scope_message`. `H/tests/agent_hold.rs`: `an_agent_hold_refuses_an_action_on_the_held_agent_and_a_double_hold_conflicts`. `D/e2e/hold.spec.ts`: `an agent is held and released by keyboard; row and details say on hold`. |
| FR-004 | PASS (automated) | `HD/hold.tests.rs`: `agent_hold_refuses_the_agent_and_its_descendant_as_caller`, `agent_hold_refuses_actions_on_the_agent_and_its_descendant`, `agent_hold_refuses_an_ancestor_acting_on_the_held_agent`, `agent_hold_leaves_unrelated_agents_alone`, `an_agent_placed_under_a_held_lead_later_is_covered` (A2). `HA`: `an_agent_hold_covers_a_held_target_and_a_held_ancestor`. `H/tests/agent_hold.rs`: `an_agent_hold_refuses_an_action_on_the_held_agent_and_a_double_hold_conflicts`. |
| FR-005 | PASS (automated) | `H/tests/agent_hold.rs`, `an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types`: the user's five actions succeed and `terminal.input` succeeds while the office is on hold. The same test, for the user path never calling the gate. See Known gaps for the CLI process check. |
| FR-006 | PASS (automated) | `src/modules/hierarchy/infrastructure/src/persistence/repositories/hold_repository_tests.rs`: `holds_survive_closing_and_reopening_the_database`. `H/tests/agent_hold.rs`: `a_hold_set_before_a_restart_still_refuses_every_agent_action_after_it`. |
| FR-007 | PASS (automated) | `HD/rate_windows.tests.rs`: `exactly_the_directive_limit_passes_and_the_next_is_refused`, `exactly_the_target_limit_passes_and_the_next_is_refused`, `exactly_the_create_limit_passes_and_the_next_is_refused`, `exactly_the_lifecycle_limit_passes_and_the_next_is_refused`, `a_slot_frees_after_sixty_seconds`, `defaults_and_refusal_text_follow_the_table`. `HA`: `the_exact_limit_passes_and_the_next_action_is_refused`. `H/tests/agent_hold.rs`: `a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded`. |
| FR-008 | PASS (automated) | `HD/auto_hold_tracker.tests.rs`: `the_third_refusal_within_a_minute_fires_and_a_clear_restarts_the_count`, `refusals_spread_over_more_than_a_minute_do_not_fire`. `HA`: `the_third_limit_refusal_holds_the_caller_once_and_notifies_once`, `an_agent_already_held_gets_no_second_hold_or_notification`, `hold_refusals_neither_count_in_windows_nor_feed_the_tracker`. `H/tests/agent_hold.rs`: `a_looping_agent_is_limited_then_held_and_the_owner_is_told_exactly_once`. |
| FR-009 | PASS (automated) | `src/modules/agent/domain/src/value_objects/settings.tests.rs`: `the_limit_defaults_follow_the_plan`, `a_limit_takes_every_whole_number_from_one_to_a_thousand`, `a_limit_outside_that_range_or_not_a_whole_number_is_refused`. `H/src/bridges/rate_limit_source.tests.rs`: `a_changed_setting_applies_to_the_next_call`. `HA`: `limits_are_read_on_every_call`. `D/e2e/hold.spec.ts`: `the dialog shows the four limits with their defaults and saves new values`. |
| FR-010 | PASS (automated) | `H/src/bridges/hold_bridges.tests.rs`: `a_user_hold_of_an_agent_is_recorded_with_the_user_as_actor_and_the_agent_as_target`, `the_office_hold_has_no_target`, `an_automatic_hold_is_recorded_with_the_office_as_actor`, `a_release_is_recorded_with_the_user_as_actor`. Audit: `src/modules/audit/domain/src/action_record.tests.rs`, `hold_and_release_have_their_wire_names`; `src/modules/audit/infrastructure/src/persistence/action_repository.tests.rs`, `an_office_actor_and_a_missing_target_survive_the_round_trip`, `the_nullable_target_migration_keeps_every_row`; `src/modules/audit/presentation/src/mappings/action_entry.tests.rs`, `an_office_level_entry_has_the_office_actor_and_a_null_agent`. Refusals with the agent as actor: `H/tests/agent_hold.rs`, `a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded`. |
| FR-011 | PASS (automated) | `D/e2e/hold.spec.ts`: `an office that was on hold before the app opened shows the hold`, `the hold change is announced in the live region`; agent row and details: `an agent is held and released by keyboard; row and details say on hold` (accessible name `Ada, working, on hold`). Model: `D/src/modules/hold/models/hold.spec.ts`, `holding and releasing the office are announced`. The screen reader pass is a separate row (Manual QA). |
| 0002/FR-009 (modified) | PASS (automated) | `H/tests/agent_hold.rs`: `a_directive_to_a_non_descendant_while_the_office_is_held_gets_the_tree_reason` (the 0002 reason comes first, A10), `directives_to_a_stopped_member_are_refused_by_the_tree_and_never_hold_the_lead`, `a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded` (answered, told, recorded). |
| 0003/FR-006 (modified, in part) | PARTIAL (automated) | Event and detail: `H/src/bridges/notification_bridge.tests.rs`, `an_auto_hold_row_carries_the_limit_name_as_its_detail_and_reaches_the_clients`; `src/modules/notification/infrastructure/tests/persistence.rs`, `the_detail_of_an_auto_hold_reads_back_and_an_old_row_has_none`; `src/modules/notification/presentation/src/mappings/notification_json.tests.rs`, `an_auto_hold_carries_its_detail`. One notification: `H/tests/agent_hold.rs`, `a_looping_agent_is_limited_then_held_and_the_owner_is_told_exactly_once`. Text: `D/src/modules/notification/models/notification.spec.ts`, `an automatic hold names the limit from the detail (0007 D7)`. Exception (b) does not apply: `D/e2e/hold.spec.ts`, `an automatic hold shows its reason, and the notification names the agent and the limit (FR-008)` (the terminal is open and the row stays unread). Gap: no test checks that the OS notification is sent while the terminal is focused. |
| PB-1 | PASS (automated) | `H/tests/agent_hold.rs`, `an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types`; `an_agent_connection_asking_for_a_hold_is_refused_as_not_allowed` (the agent side cannot reach `hold.*`). |
| PB-2 | PARTIAL (automated) | The same test: `terminal.attach` and `terminal.input` succeed on a held office. No test asserts that the CLI process is still alive after the hold; no code path in the hold stops one. |
| PB-3 | GAP | No new test. A hold writes only to `office.db` (`hierarchy_holds`) and the action record. The 0001 guard tests are not part of this report's runs. |
| PB-4 | PASS (automated) | `D/e2e/hold.spec.ts`: the office hold, the agent hold, both releases, and the limits dialog by keyboard, each with axe (`expectNoSeriousViolations`); `Escape closes the dialog and focus returns to the Limits control`. |
| PB-5 | PASS (automated) | The 0002 suites pass unchanged in `cargo test` of `office-harnessd` (21 suites) and `hierarchy-application` (46 tests). `H/tests/agent_hold.rs`, `directives_to_a_stopped_member_are_refused_by_the_tree_and_never_hold_the_lead`, shows a 0002 refusal still wins. |
| SC-001 | PASS (automated) | `H/tests/agent_hold.rs`, `a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded`: at most 10 reach the terminal; the first refusals name the limit and the rest name the hold. A real CLI run is BLOCKED (Manual QA). |
| SC-002 | PASS (automated) | `H/tests/agent_hold.rs`, `an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types`. |
| SC-003 | PASS (automated) | `H/tests/agent_hold.rs`, `a_hold_set_before_a_restart_still_refuses_every_agent_action_after_it`. |
| SC-004 | PASS (automated) | `H/tests/agent_hold.rs`, `a_looping_agent_is_limited_then_held_and_the_owner_is_told_exactly_once`. |
| SC-005 | PASS (automated) | `D/e2e/hold.spec.ts`, `one key press, Ctrl+Shift+H, holds the office from the panel (SC-005)`. The key also holds from a focused terminal (D9 = b, see the spec's Deviations): `Ctrl+Shift+H holds the office while focus is in a terminal (D9 = b)`. |
| US1, US2, US3 | PASS (automated) | The integration tests of SC-001 to SC-004 and `FR-003`/`FR-004` above, and the three `describe` groups in `D/e2e/hold.spec.ts` (`office hold`, `agent hold`, `limits`). |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| Hold the office when no agent runs (Empty) | PARTIAL (automated) | `HA`, `setting_and_releasing_the_office_hold_journals_each_change` holds with no running session. No test starts an agent afterward and sees the refusal; `an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types` covers the refusal with running agents. |
| Hold an agent already held, release one not held (Invalid) | PASS (automated) | `HA`: `holding_twice_is_a_conflict_with_the_scope_message`, `releasing_what_is_not_held_is_a_conflict`. `D/e2e/hold.spec.ts`: `the server's already-held message reaches the user`, `an agent's already-held message reaches the user`. |
| A limit of 0, a negative number, or text (Invalid) | PASS (automated) | `settings.tests.rs`, `a_limit_outside_that_range_or_not_a_whole_number_is_refused`. `D/src/modules/hold/models/hold.spec.ts`, `zero, negatives, text, fractions, and big numbers are refused with the server's words`. `D/e2e/hold.spec.ts`: `a value that is not a limit is refused with the message, and nothing is saved`, `the server's refusal of a limit is shown as sent`. |
| Exactly the limit passes, the next is refused (Boundary) | PASS (automated) | The four `exactly_the_..._limit_passes_and_the_next_is_refused` tests in `HD/rate_windows.tests.rs`; `HA`, `the_exact_limit_passes_and_the_next_action_is_refused`. |
| A directive arrives while the target is held (Concurrent) | GAP | The gate runs before the terminal write, so a directive is whole or refused. No test races a hold against a directive. The 0002 race suites cover the write path. |
| Office release while an agent is held automatically (Concurrent) | PASS (automated) | `HA`, `releasing_the_office_keeps_an_automatic_agent_hold`. |
| Delete an agent that is on hold (Concurrent) | PASS (automated) | `H/tests/agent_hold.rs`, `deleting_a_held_agent_leaves_no_hold`. `HA`: `forget_agents_drops_only_those_agents_holds`, `a_failed_forget_keeps_the_holds`. Repository: `delete_agent_holds_removes_the_named_agents_and_keeps_the_rest`. |
| A held agent's parent is deleted with its subtree (Concurrent) | PARTIAL (automated) | Startup pruning: `HA`, `prune_drops_holds_of_agents_that_no_longer_exist`. No test deletes a subtree that contains holds through the wire. |
| An agent with no role acts (Unauthorized) | PASS (automated) | The 0002 refusal for a caller outside the hierarchy comes before the gate (A10); the 0002 suites pass unchanged. No 0007 test is specific to it. |
| The hold state cannot be stored (Failure) | PASS (automated) | `HA`: `a_failed_write_leaves_the_holds_and_the_journal_unchanged`, `a_failed_automatic_hold_write_is_reported_and_retried_on_the_next_refusal`. `H/src/server/hold_requests.tests.rs`, `errors_map_to_the_contract_codes_and_a_conflict_keeps_its_text_alone`. |
| The server restarts within a 60-second window (Failure) | PARTIAL (automated) | Holds stay: see FR-006. Rate counts are memory only by design (data model); no test restarts the server and checks that the counts start at 0. |

## Known gaps (open only)

1. **`hold.changed` events can arrive out of order under concurrent changes.** Clients replace their state on each event, so a late event can show an older state until the next change or a resync. `D/src/modules/hold/state/hold-controller.spec.ts`, `an older list answer never overwrites a newer event`, covers the list answer only.
2. **No test deletes a subtree that has holds.** Only the delete of one held agent and the startup prune have tests.
3. **No test asserts that a CLI process survives a hold.** Typing into the terminal is tested (PB-2).
4. **No test shows the OS notification of an automatic hold while that agent's terminal is focused.**
5. **No test races a hold against a directive,** and no test restarts the server to check that rate counts start at 0.
6. **LOW-6, accepted: an audit rebuild resets the `AUTOINCREMENT` mark.** The effect is harmless, because no code deletes action rows.
7. **PB-3 has no 0007 test** (see the row above).

## Manual QA

Rows marked agent-run were run by an agent on Linux/KDE Wayland with fake CLIs on 2026-11-19, against the built app and server of `main` at f6dc663. The other rows stay BLOCKED until a person runs them.

Raw evidence (screenshots, traces, logs, timing files) is kept outside the repository, in the QA agent's folder of the office hive (`hive/agents/celal-mwhvybh8/qa/evidence/`); repository readers cannot open it. Each row therefore states its own method, setup, and measured values. Run setup: debug builds of the desktop app and `office-harnessd`, WebDriver through `tauri-driver`, an isolated data and config folder per run with login start off, fake CLIs (shell scripts) in place of agent CLIs, and no change to system settings.

| Check | Verdict | Note |
|---|---|---|
| Screen reader pass: the hold state, the release actions, and the live region announcement | PARTIAL (agent-run, Linux/KDE Wayland, 2026-11-19) | AT-SPI tree: the header landmark "Office hold" holds the button "Hold office", which becomes "Release office" while held; an agent row reads "ceo, waiting for input, on hold, CEO, ..."; the details offer "Hold agent" and "Release agent"; the Office record is a dialog with list items such as "Put on hold by user. Succeeded". The DOM live regions (polite) carried "Office on hold", "Office released from hold", "ceo on hold", and "ceo released from hold". No announcement event reached the AT-SPI event stream, so the announcement itself is unproven, and so is the spoken output. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/a11y/`, `hive/agents/celal-mwhvybh8/qa/evidence/0007-hold-by-hand/`. |
| 200 percent zoom: the panel header, the Limits dialog, and the Office record list | PARTIAL (agent-run, Linux/KDE Wayland, 2026-11-19) | Emulated by halving the window with WebDriver (550x360 CSS px for a 1100x720 window), not by the webview zoom. Panel header, Limits dialog, and Office record: no horizontal overflow, 0 elements past the right edge, screenshots show no cut text; the Limits dialog scrolls inside itself. Unproven: the real zoom control and a person's reading check. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/zoom200/`. |
| A real agent CLI that loops on `office-harnessd agent-call directive.send` | PARTIAL (agent-run, Linux/KDE Wayland, 2026-11-19) | A fake bash CLI, not a real agent CLI, ran as a CEO and sent 100 directives in a loop through `agent-call directive.send` to a team lead. 3 reached the target (the per-target limit of 3 per 60 seconds is lower than 10); directives 4 to 6 were refused with the limit message; the automatic hold came after the third refusal; directives 7 to 100 were refused with the hold message. Exactly 1 auto_hold notification row and exactly 1 desktop Notify call ("ceo was put on hold after 3 refusals by the limit ..."). The action record holds 100 agent rows (3 ok, 3 limit, 94 hold) and 1 office hold row. Unproven: a real CLI. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0007-looping-cli/`. |
| Hold and release by hand in the running app on Linux, macOS, and Windows | PARTIAL (agent-run, Linux/KDE Wayland, 2026-11-19) | Linux only, in the running app: Hold office showed "Office on hold" and the button became Release office; Release office returned the state; the Office record listed both entries; the agent hold and release from the details worked (row says on hold, button toggles, live region text). The office and agent holds each gave the right entries. macOS and Windows stay BLOCKED. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0007-hold-by-hand/`. |
| Restore of a pre-upgrade `office.db` copy with the previous program | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) | The previous program (commit 62f0bd6) made an office (2 projects, 4 agents, a setting) in an isolated folder and was stopped; the file was copied. The new program started on the folder and upgraded it (audit, notification, and hierarchy ledgers 1 to 2). The previous program then refused the upgraded database: exit code 1, "the database ledger audit_schema_version is at version 2, newer than the 1 that this server knows". With the copy put back as office.db, the previous program started and listed the same agents, projects, and setting as before. Limit: a small office with no sessions, and both programs were started by hand, not from an installer. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0007-pre-upgrade/`. |
