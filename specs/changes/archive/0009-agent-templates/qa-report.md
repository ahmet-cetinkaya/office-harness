# 0009 QA report

## Summary

- **Date:** 2026-11-22.
- **Build under test:** branch `feat/0009-p2-desktop` (Phase 2, desktop: T6 file commands and T7 templates UI). Phase 1 (server: T2 to T5) is merged as PR #98, head f14aaa2.
- **Phase 2 gate (2026-11-22, Linux, the phase 2 tree with the umbrella and main 2405c5a merged in):** PASS. `cargo fmt --check`, workspace lint, and `scripts/check-boundaries.sh` passed; `cargo test --workspace` passed 1050 tests with 0 failures; `tauri build --debug` passed; `bun test` passed 313 tests; `svelte-check`, `eslint`, and `prettier --check` passed; Playwright passed 208 tests.
- **Overall status:** the automated evidence stands for every requirement, success criterion, and edge case, with the gaps named in Known gaps. The hands-on rows are BLOCKED: nobody has run them (SC-001's 3-minute human run, the real file dialogs on KDE, macOS, and Windows, a screen reader pass, and 200 percent zoom). No hands-on row is PASS.
- **Status of this file:** the test names were read from the repository on 2026-11-22. The per-phase counts in "Local gates" come from the worker reports of Phase 1, T6, and T7; this task ran no test. The Phase 2 gate above is the one run on the merged phase 2 tree; it is not a run of the final umbrella head. The test gaps of PR #109 were closed on 2026-11-23 (the missing project folder and the agent allowlist; both tests were already on main).

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests run on real SQLite and use fake CLIs, not the real agent CLIs.
- The Playwright tests run against a mock core, so they prove the webview screens, not the real server. The system file dialog is not opened in them.
- The file commands are tested as Rust functions on files in a temporary folder. The real dialog, on any OS, is a Manual QA row.
- Fault injection (a failing agent create, a failing team save, a failing undo, a failing read-back) uses test doubles at the ports named in the evidence.

### Local gates

| Gate | Scope | Result |
|---|---|---|
| `cargo test --workspace` | Phase 1, PR #98 head f14aaa2 (worker report): 948 passed | PASS |
| `office-harness-desktop` (Tauri crate) | T6 (worker report): 54 tests passed | PASS |
| `bun test src` | T7 (worker report): 312 passed | PASS |
| Playwright with axe | T7 (worker report): full suite 205 passed, including the 15 tests of `e2e/templates.spec.ts` | PASS |
| SC-001 machine run | 15 agents in 3 create actions, 137 ms against a limit of 180000 ms | PASS |
| Umbrella: `cargo fmt --check`, lint, `cargo test --workspace`, `tauri build`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, Playwright, `scripts/check-boundaries.sh` | the phase 2 tree (umbrella and main 2405c5a merged in) | PASS: 1050 cargo tests, 313 bun tests, 208 Playwright tests, all other checks clean |
| CI matrix of the Phase 2 PR, `/ac:review`, `/ac:verify` on the umbrella head | | open (Final verification of the plan) |

## Requirements

Paths are relative to the repository root. `T` is `src/modules/template`, `H` is `src/apps/office-harnessd`, `D` is `src/apps/desktop`, and `A` is `src/modules/agent`. `HB` is `H/src/bridges/template_create.tests.rs`, `HT` is `H/tests/templates.rs`, `AC` is `H/tests/agent_call.rs`, `TS` is `T/application/src/services/template_service.tests.rs`, `TB` is `T/domain/src/entities/team_body.tests.rs`, `TF` is `T/domain/src/services/template_file.tests.rs`, `TJ` is `T/domain/src/services/body_json.tests.rs`, `TN` is `T/domain/src/value_objects/name_pattern.tests.rs`, `TV` is `T/domain/src/services/save_name.tests.rs`, `TI` is `T/infrastructure/tests/persistence.rs`, `TH` is `T/presentation/src/handlers/template_handlers.tests.rs`, `DF` is `D/src-tauri/src/template_files.tests.rs`, `DC` is `D/src-tauri/src/commands.tests.rs`, and `DE` is `D/e2e/templates.spec.ts`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | Copy from an agent: `TS`, `a_body_copied_from_an_agent_gets_the_default_pattern_and_the_agents_values`; `HT`, `an_agent_template_made_from_an_agent_creates_numbered_agents`. Save rules: `TS`, `a_name_used_ignoring_case_and_a_broken_rule_are_refused`, `saving_with_an_id_replaces_and_an_unknown_id_is_not_found`; `TB`, `an_entry_with_an_empty_command_or_pattern_is_named`; `TJ`, `an_agent_body_round_trips_and_keeps_no_value_apart_from_an_empty_value`. Wire: `TH`, `saving_answers_with_the_template_and_announces_the_full_list`, `a_kind_that_does_not_match_the_body_says_so`; `HT`, `a_kind_that_does_not_match_the_body_is_refused`. Desktop: `DE`, `saves an agent as a template from its details`. The 0010 plan owns the MODIFIED text of FR-001. |
| FR-002 | PASS (automated) | Names: `TN`, `the_number_is_the_lowest_that_makes_the_name_unique_ignoring_case`, `a_pattern_without_a_number_gets_one_only_when_the_name_is_taken`. Bridge: `HB`, `a_project_gives_the_working_folder_when_the_template_has_none`, `a_team_creation_names_every_agent_from_its_pattern`. Server run: `HT`, `an_agent_template_made_from_an_agent_creates_numbered_agents`. |
| FR-003 | PASS (automated) | `TB`, `a_team_needs_a_member_entry_and_each_count_is_at_least_one`, `an_entry_with_an_empty_command_or_pattern_is_named`; `TJ`, `a_team_body_round_trips`. Desktop: `DE`, `builds a team template with a lead and a member entry`. The 0010 plan owns the MODIFIED text of FR-003. |
| FR-004 | PASS (automated) | `HB`, `a_team_creation_names_every_agent_from_its_pattern`; `HT`, `two_teams_from_one_template_get_their_names_from_the_patterns`. Desktop: `DE`, `creates a CEO and then two teams of one lead and six members from the templates`. |
| FR-005 | PASS (automated) | Fault injection at the lead, a member, and the last member: `HB`, `a_team_creation_that_fails_at_any_agent_leaves_nothing_behind`. At the store step and the team save: `a_team_creation_whose_store_step_fails_removes_every_agent_and_the_team`, `a_team_creation_whose_team_save_fails_removes_the_lead`; at the placement of a direct creation: `a_direct_creation_whose_placement_fails_removes_the_agent`. A failing undo: `a_removal_that_fails_in_the_undo_leaves_only_that_agent_and_is_named`, `an_undo_that_leaves_an_agent_behind_raises_an_alert`. Every reason in one answer: `every_pre_check_failure_is_named_in_one_answer_and_nothing_is_created`. Create and Delete entries of a rollback: `each_rolled_back_agent_has_a_create_and_a_delete_entry`. Exact events: `HT`, `a_creation_that_fails_after_a_write_sends_only_the_records_of_its_entries`, `another_client_hears_each_created_agent_once_and_nothing_for_a_failed_creation`. Desktop: `DE`, `a taken team name creates nothing and every reason shows as text`. |
| FR-006 | PASS (automated) | `TS`, `a_team_template_keeps_its_copy_when_an_agent_template_is_edited_or_deleted`; `HT`, `editing_and_deleting_a_template_change_no_agent_or_team`; `DE`, `a template is edited and deleted by keyboard and the agents stay`. |
| FR-007 | PASS (automated) | Names kept, values `null`: `TS`, `an_export_holds_no_environment_value_and_keeps_the_names`; `TF`, `export_writes_every_environment_value_as_null_and_the_file_reads_back`. Write: `DF`, `an_export_writes_the_contents_to_the_picked_path`, `a_failed_write_says_the_file_was_not_written`. Notice before writing: `DE`, `the export notice shows before anything is written`; `a file that cannot be written shows the reason`. |
| FR-008 | PASS (automated) | Nothing saved before confirm: `TS`, `a_preview_names_the_save_names_drops_values_and_saves_nothing`, `a_confirm_saves_every_template_without_values`. Preview: `DE`, `the import preview shows variable names, the values line and an escape symbol, and a cancel saves nothing`, `confirming the import preview saves the templates`. The file text is read by a command that takes no path (see PB-4). |
| FR-009 | PASS (automated) | Each refusal kind: `TF`, `an_unknown_version_unreadable_text_and_a_large_file_are_refused_with_a_reason`, `a_template_that_breaks_a_save_rule_is_named_with_its_reason`, `an_unknown_field_in_the_file_or_a_template_is_named`, `a_template_without_a_name_is_named_by_its_position`; `TB`, `fifty_agents_are_accepted_and_fifty_one_are_refused`; `TS`, `a_refused_file_saves_nothing_and_names_the_template_and_the_reason`. Size at the server: `HT`, `an_import_text_over_256_kib_is_refused_as_invalid`. Size at the file command: `DF`, `a_file_of_exactly_the_cap_is_read`, `a_file_one_byte_over_the_cap_is_refused`. Desktop: `DE`, `an invalid import shows its reason and saves nothing`, `a file that the core refuses shows the core's reason`. |
| FR-010 | PASS (automated) | `TB`, `fifty_agents_are_accepted_and_fifty_one_are_refused`; `TF`, `a_file_of_fifty_agents_is_accepted`. Desktop: `DE`, `a team template over the limit is refused with its reason`. |
| FR-011 | PASS (automated) | Domain: `A/domain/src/value_objects/env_entry.tests.rs`, `no_value_and_an_empty_value_are_different_entries`. Storage: `A/infrastructure/src/persistence/additions_document.tests.rs`, `no_value_and_an_empty_value_stay_different_in_the_stored_text`; `TI`, `no_value_and_an_empty_value_stay_different_after_a_round_trip`; `TJ`, `an_agent_body_round_trips_and_keeps_no_value_apart_from_an_empty_value`. Wire: `A/presentation/src/contracts/additions_body.tests.rs`, `a_null_value_is_no_value_and_an_empty_text_is_a_value`. Creation keeps it: `HB`, `a_value_less_environment_entry_keeps_no_value_in_the_stored_additions`. Import drops values: `HT`, `an_import_of_a_file_with_an_env_value_saves_no_value`. Not passed to the CLI: `A/application/src/commands/start_agent/start_agent_handler.tests.rs`, `a_variable_without_a_value_is_not_passed_and_an_empty_value_is`. Shown: `DE`, `the create dialog shows a variable without a value as no value`. The agent details show it as `NAME: no value`: `DE`, `an agent's details show a variable without a value as no value` (US3 scenario 6). |
| PB-1 | PASS (automated) | `HB`, `a_second_ceo_is_refused_with_the_tree_rule_text`, `a_place_for_a_template_with_no_role_is_a_reason`. |
| PB-2 | PASS (automated) | `HB`, `each_created_agent_has_a_create_record_with_the_user_as_actor`, `each_rolled_back_agent_has_a_create_and_a_delete_entry`. |
| PB-3 | PASS (automated) | `HT`, `template_work_starts_no_agent_and_writes_no_cli_configuration`: saving, creating an agent and a team, and exporting leave every file and folder of the fake CLI homes and of the project folder as they were. Added by ACF-183. |
| PB-4 | PASS (automated) | The file commands take no path: they open the dialog and touch only the picked path. `DF`: `a_file_of_exactly_the_cap_is_read`, `a_file_one_byte_over_the_cap_is_refused`, `a_folder_is_refused_as_not_a_regular_file`, `a_fifo_is_refused_without_being_opened`, `bytes_that_are_not_text_are_refused`, `a_missing_file_names_the_read_failure`, `an_export_writes_the_contents_to_the_picked_path`. The webview gets no `dialog:` or `fs:` permission: `DC`, `the_webview_capability_grants_exactly_the_listed_commands_and_notifications`. |
| PB-5 | PASS (automated) | `DE`: every test works by keyboard, and the views are checked with axe (`expectNoSeriousViolations`), for example `an empty templates view says so and offers save and import`, `the create dialog shows a variable without a value as no value`. The screen reader pass is BLOCKED. |
| A1 to A10 | PASS (automated) | A1: `HT`, `template_work_starts_no_agent_and_writes_no_cli_configuration` (every agent made from a template is `stopped`). A10: `HT`, `template_changes_write_no_action_record` (save, edit, export, delete, and import write no row; a create from a template then writes one). Both added by ACF-183. A2: `TN` tests above. A3: `TV`, `a_taken_name_gets_the_lowest_free_number_ignoring_case`. A4: `HB`, `a_team_lead_entry_with_the_ceo_role_is_created_as_a_lead`. A5: `TS`, `a_team_template_keeps_its_copy_when_an_agent_template_is_edited_or_deleted`. A6: `TS`, `an_export_holds_no_environment_value_and_keeps_the_names`; `HT`, `an_import_of_a_file_with_an_env_value_saves_no_value`. A7: `TB`, `fifty_agents_are_accepted_and_fifty_one_are_refused`; `DF` cap tests. A8: `TI`, `templates_survive_closing_and_reopening_the_database`. A9: `AC`, `an_agent_may_not_ask_for_a_type_outside_the_allowlist` (it asks for each of the ten `template.*` types, and every answer is `refused`). |
| SC-001 | PARTIAL (automated) | Machine run: `DE`, `SC-001: a 15-agent tree from two templates in three create actions` builds a CEO and two teams of a lead and 6 members from two templates in 3 create actions, by keyboard, on the mock core: 15 agents, 3 create actions, 137 ms against a limit of 180000 ms. The run writes `sc-001-timing.json`; the copy is kept outside the repository in the QA agent's folder, `qa/evidence/sc-001-timing.json`. The 3-minute human run is BLOCKED (Manual QA). |
| SC-002 | PASS (automated) | `HT`, `export_then_import_into_a_fresh_office_keeps_everything_but_env_values`; `TF`, `export_writes_every_environment_value_as_null_and_the_file_reads_back`; `TS`, `an_export_holds_no_environment_value_and_keeps_the_names`. |
| SC-003 | PASS (automated) | One test for each kind of FR-009: `TF`, `an_unknown_version_unreadable_text_and_a_large_file_are_refused_with_a_reason`, `a_template_that_breaks_a_save_rule_is_named_with_its_reason`, `a_file_of_fifty_agents_is_accepted` (the 50 edge); `TB`, `fifty_agents_are_accepted_and_fifty_one_are_refused`; `TS`, `a_refused_file_saves_nothing_and_names_the_template_and_the_reason`; `TI`, `insert_all_saves_every_template_or_none`; `HT`, `an_import_text_over_256_kib_is_refused_as_invalid`. |
| SC-004 | PASS (automated) | `HT`, `editing_and_deleting_a_template_change_no_agent_or_team`; `TS`, `a_team_template_keeps_its_copy_when_an_agent_template_is_edited_or_deleted`; `DE`, `a template is edited and deleted by keyboard and the agents stay`. |
| SC-005 | PASS (automated) | `HB`, `a_team_creation_that_fails_at_any_agent_leaves_nothing_behind` (lead, first member, last member), `a_team_creation_whose_store_step_fails_removes_every_agent_and_the_team`, `a_team_creation_whose_team_save_fails_removes_the_lead`, `a_taken_team_name_alone_is_a_conflict_and_creates_nothing`; `HT`, `a_taken_team_name_creates_nothing`. A failing undo leaves only the agent that could not be removed and names it (`a_removal_that_fails_in_the_undo_leaves_only_that_agent_and_is_named`); that case does not leave 0 agents by design and is reported, not hidden. |
| US1, US2, US3 | PASS (automated) | The `HT` runs of FR-001, FR-002, FR-004 to FR-009, the `DE` runs, and the rows above. US3 scenario 6 is proven by `DE`, `an agent's details show a variable without a value as no value`. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| No template exists (Empty) | PASS (automated) | `DE`, `an empty templates view says so and offers save and import` ("No template exists yet.", "New template", "Import templates"). |
| An imported template has the name of an existing template (Boundary) | PASS (automated) | `TV`, `a_taken_name_gets_the_lowest_free_number_ignoring_case`; `TS`, `a_preview_names_the_save_names_drops_values_and_saves_nothing`. |
| A name pattern has no `{n}` and the name exists (Boundary) | PASS (automated) | `TN`, `a_pattern_without_a_number_gets_one_only_when_the_name_is_taken`. |
| A lead entry has role CEO, or a member entry has role team lead (Invalid) | PASS (automated) | Lead entry with role CEO: `HB`, `a_team_lead_entry_with_the_ceo_role_is_created_as_a_lead`. Member entry with role team lead, created as a member: `HB`, `a_member_entry_with_the_team_lead_role_is_created_as_a_member`. |
| An agent template with role CEO is used when a CEO exists (Invalid) | PASS (automated) | `HB`, `a_second_ceo_is_refused_with_the_tree_rule_text`. |
| The template's working folder, or the project's folder, does not exist (Invalid) | PASS (automated) | Template folder, named before anything is created: `HB`, `a_missing_working_folder_is_named_before_anything_is_created`. Project folder, named before anything is created: `HB`, `a_project_whose_folder_is_missing_is_named_before_anything_is_created`. |
| No working folder and no project (Invalid) | PASS (automated) | `HB`, `no_folder_and_no_project_needs_a_working_folder`. |
| An imported prompt or command holds control characters or escape codes (Invalid) | PASS (automated) | `DE`, `the import preview shows variable names, the values line and an escape symbol, and a cancel saves nothing` (ESC shows as a visible symbol); `D/src/modules/template/models/template-text.spec.ts`. |
| A file larger than 256 KiB, or a team template with 51 agents (Boundary) | PASS (automated) | See FR-009: `DF` (exactly 256 KiB read, 256 KiB + 1 refused), `HT`, `an_import_text_over_256_kib_is_refused_as_invalid`, `TB`, `fifty_agents_are_accepted_and_fifty_one_are_refused`. |
| An agent template used in a team template is deleted (Concurrent) | PASS (automated) | `TS`, `a_team_template_keeps_its_copy_when_an_agent_template_is_edited_or_deleted`. |
| Two creations from one template at the same moment (Concurrent) | PASS (automated) | `HB`, `two_concurrent_creations_from_one_template_get_different_names` (real SQLite). |
| The file cannot be written on export (Failure) | PASS (automated) | `DF`, `a_failed_write_says_the_file_was_not_written`; `DE`, `a file that cannot be written shows the reason`. No template changes: the export reads the templates only. |
| A flag or startup prompt holds a secret (Failure) | PASS (automated) | `DE`, `the export notice shows before anything is written`. The text is the one in the README. |
| A name pattern with `{team}` is used outside a team creation (Invalid) | PASS (automated) | Refused with the pattern named: `HB`, `a_team_pattern_on_a_direct_creation_without_a_team_names_the_pattern`. Replaced by the team of the place: `HB`, `a_member_template_is_placed_under_a_lead_with_the_team_name_filled_in`; `TN`, `the_team_name_replaces_the_team_token_and_stays_text`, `a_team_pattern_without_a_team_gives_no_name`. |

## Traceability check

Every row of the Traceability table in `PLAN.md` has evidence or a recorded gap.

| Plan row | Verdict | Where |
|---|---|---|
| FR-001 to FR-011 | evidence for each | Requirements table above. |
| PB-1 to PB-5 | evidence | Requirements table above. |
| A1 to A10 | evidence | Requirements table above. |
| SC-001 | machine evidence; human run BLOCKED | Requirements table and Manual QA. |
| SC-002 to SC-005 | evidence | Requirements table above. |
| US1, US2, US3 | evidence, including US3 scenario 6 | Requirements table above. |

## Plan deviations

None recorded by T8. The T6 crate facts below are from the T6 worker. After this report, 0010 superseded the D2 rule that a body with `own_worktree` is refused (see the plan, D9), and ACF-183 added the tests of PB-3, A1, and A10 named above.

## Known gaps (open only)

1. **No hands-on pass** of the rows in Manual QA.
2. **The CI matrix of the Phase 2 PR, `/ac:review`, and `/ac:verify` on the umbrella head** are not run in this file. The full local gate ran once, on the merged phase 2 tree (see the Summary and "Local gates").

## Manual QA

No person has run any row. The rows stay BLOCKED for the owner.

| Check | Verdict | Note |
|---|---|---|
| SC-001, the human run: build the 15-agent tree from two templates in under 3 minutes and at most 3 create actions | BLOCKED | The scripted run above is machine evidence on a mock core. A person has not timed it. |
| The real system file dialog on KDE Plasma, for export and for import | BLOCKED | The Rust tests use files in a temporary folder; Playwright uses a mock. |
| The real system file dialog on macOS | BLOCKED | Same. |
| The real system file dialog on Windows | BLOCKED | Same. |
| Screen reader pass: the templates view, the create dialog, the import preview in its fixed order, and the refusal reasons | BLOCKED | The automated checks are axe and text only. The spoken output is unproven. |
| 200 percent zoom: the templates view, the editor, the create dialog, and the import preview | BLOCKED | Not run by hand. |

## T6: crate facts for `tauri-plugin-dialog` (plan D6, conditions d and e)

Date: 2026-11-22. Toolchain 1.100.0. Each fact comes from `cargo metadata --format-version 1 --locked` or `cargo tree --locked`, not from the plan.

| Fact | Plan said | Found |
|---|---|---|
| Version in `Cargo.lock` | `2.8.4` | `2.8.4` |
| License | `Apache-2.0 OR MIT` | `Apache-2.0 OR MIT` |
| `tauri` requirement | `^2.12` | `^2.12` (normal and build use; a dev use has the same range) |
| Normal dependency `rfd` | `^0.17` | `^0.17`, locked at `0.17.2` |
| Normal dependency `tauri-plugin-fs` | `^2.7.1` | `^2.7.1`, locked at `2.7.1` |
| Source | official Tauri org | registry `crates.io`, repository `https://github.com/tauri-apps/plugins-workspace` |
| `tauri` versions in the graph | one | one: `2.12.5` (`cargo tree -i tauri` lists `tauri v2.12.5`; `cargo tree -d` lists no `tauri`) |
| Pin | `=2.8.4` in the root `[workspace.dependencies]` | `tauri-plugin-dialog = "=2.8.4"`, next to `tauri = "=2.12.5"` |

`cargo tree --locked -p tauri-plugin-dialog -e normal --depth 1`:

```
tauri-plugin-dialog v2.8.4
├── log v0.4.35
├── raw-window-handle v0.6.2
├── rfd v0.17.2
├── serde v1.0.229
├── serde_json v1.0.154
├── tauri v2.12.5
├── tauri-plugin-fs v2.7.1
├── thiserror v2.0.21
└── url v2.5.8
```

### Crates the change adds to `Cargo.lock`

`git diff Cargo.lock` adds exactly three `[[package]]` entries and one dependency line on `office-harness-desktop`. Every other crate in the plugin's tree was already locked.

| Crate | Version | License (from `cargo metadata`) | With AGPL-3.0-or-later |
|---|---|---|---|
| `tauri-plugin-dialog` | 2.8.4 | `Apache-2.0 OR MIT` | compatible: both choices are permissive and one-way compatible |
| `tauri-plugin-fs` | 2.7.1 | `Apache-2.0 OR MIT` | compatible |
| `rfd` | 0.17.2 | `MIT` | compatible |

The webview gets no `dialog:` or `fs:` permission. `capabilities/default.json` holds only the `allow-*` entries of `WEBVIEW_COMMANDS` and `notification:default`; a Rust test in `commands.tests.rs` asserts this.
