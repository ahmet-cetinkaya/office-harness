# 0010 QA report

## Summary

- **Date:** 2026-11-22.
- **Build under test:** branch `feat/0010-agent-worktree`, the commit that adds the PR review fixes (T1 to T5 merged with `main` 2405c5a), toolchain 1.100.0. Every gate below ran on that tree.
- **Overall status:** the automated evidence stands for FR-001 to FR-008, PB-1 to PB-5, and SC-001 to SC-005. FR-009 and the two modified 0009 requirements are covered by the template tests listed below. Rows that need a person, a real CLI, or a real app are BLOCKED. Nobody ran them.
- **Status of this file:** the gate numbers come from runs on 2026-11-22. The CI result of the PR is not in this file. The test gaps of PR #109 were closed on 2026-11-23 (SC-005).

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests run real git on temporary repositories that each test makes. They use fake CLIs, not the real agent CLIs.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server.
- The Rust gates exclude the `office-harness-desktop` crate, which does not build on the Linux machine used here. CI builds it on all three OSes.
- Every git test runs on Linux here. The tests that need a stand-in program, a symbolic link, or file modes are Unix-only; CI runs the others on macOS and Windows.

### Local gates (2026-11-22)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --check` | workspace | PASS |
| `cargo clippy --all-targets -D warnings` | workspace without `office-harness-desktop` | PASS (clean) |
| `cargo test` | workspace without `office-harness-desktop`: 989 passed, 0 failed | PASS |
| `tests/agent_worktree.rs` | 10 tests, run 4 times before the merge, once after | PASS (10 passed each time) |
| git service tests | `agent-infrastructure` `git_` tests: 43 | PASS |
| `scripts/check-boundaries.sh` | repository | PASS (boundaries ok) |
| `bun test src` | desktop: 289 passed, 0 failed | PASS |
| Playwright | desktop, full suite on the mock core, Chromium: 199 passed, 0 failed | PASS |
| `svelte-check`, `eslint`, `prettier --check` | desktop: 465 files, 0 errors, 0 warnings; lint and format clean | PASS |
| CI | the PR | Not run yet |

## Requirements

Paths are relative to the repository root. `H` is `src/apps/office-harnessd`, `M` is `src/modules`, `D` is `src/apps/desktop`. End-to-end server tests are in `H/tests/agent_worktree.rs`; git service tests are in `M/agent/infrastructure/src/services/git_worktrees.tests.rs` (`GW`) and `git_runner.tests.rs` (`GR`).

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `the_choice_is_offered_only_inside_a_repository`; `GW`: `info_in_a_repository_names_the_branch_and_the_free_branch_name`, `info_in_a_subfolder_reads_the_prefix_also_through_a_symbolic_link`, `info_when_detached_gives_the_full_commit_id`, `info_outside_a_repository_or_without_git_is_not_in_a_repository`, `without_git_on_the_login_path_the_folder_is_not_in_a_repository`. Desktop: `D/e2e/agent-worktree.spec.ts`: `the choice is not offered outside a repository`, `inside a repository the choice is off and shows base, branch name, and the note`. |
| FR-002 | PASS (automated) | `two_agents_change_the_same_file_only_in_their_own_worktrees`; `GW`: `create_and_remove_leave_nothing_and_the_git_configuration_unchanged`, `a_subfolder_create_starts_the_agent_in_the_same_subfolder`; `create_agent_handler.tests.rs`: `with_the_choice_the_agent_works_in_the_worktree_subfolder`. |
| FR-003 | PASS (automated) | `two_agents_change_the_same_file_only_in_their_own_worktrees` (the record carries `worktree`); `M/agent/infrastructure/tests/persistence.rs`: `a_worktree_survives_a_reopen`. Desktop: `D/e2e/agent-worktree.spec.ts`: `turning the choice on sends the base and the branch name, and the details show the rows`. |
| FR-004 | PASS (automated) | `outside_a_repository_the_create_is_refused_and_makes_nothing`; `GW`: `an_unwritable_worktrees_folder_fails_the_create_and_leaves_nothing`, `a_checkout_that_times_out_leaves_nothing`, `a_subfolder_untracked_in_the_base_refuses_the_create_and_leaves_nothing`, `a_base_outside_the_listed_branches_is_refused_before_git_runs`, `the_data_folder_inside_the_repository_is_refused`; `create_agent_handler.tests.rs`: `an_insert_failure_discards_the_worktree_and_the_branch`, `a_create_the_record_refuses_discards_the_agent_and_the_worktree`, `a_failed_additions_save_discards_the_worktree`, `a_worktree_that_cannot_be_made_creates_no_agent`. A role create whose placement fails after the create: covered at the use-case level (`AgentCreatorBridge::remove` discards; `delete_agent_handler.tests.rs`, `discard_removes_without_asking`), no end-to-end test, because the placement is checked before the create. |
| FR-005 | PASS (automated) | `a_subtree_delete_asks_once_and_a_stale_member_stops_it_before_any_delete`; `keep_and_own_commits_and_a_gone_repository_keep_and_name_what_stays`; `M/hierarchy/application/src/commands/subtree/subtree_handler.tests.rs`: `a_stale_worktree_on_one_member_refuses_before_any_delete`. Desktop: `D/e2e/agent-worktree.spec.ts`: `a delete asks keep or remove, keep is focused, and keeping lists the kept branch and folder`. |
| FR-006 | PASS (automated) | `changes_are_removed_only_with_the_hash_the_user_confirmed`; `a_file_the_running_agent_writes_at_its_stop_refuses_the_removal`; `GW`: `the_status_counts_every_untracked_file_and_skips_ignored_ones`, `the_hash_changes_when_one_file_is_reverted_and_another_added`, `a_worktree_that_became_dirty_is_refused_by_the_plain_remove` (also with `status.showUntrackedFiles=no`). Desktop: `D/e2e/agent-worktree.spec.ts`: `removing a worktree with changes asks again with the counts, and cancel keeps it`. The "confirm again" answer is covered by unit tests only (`worktree.spec.ts`, `office-controller.spec.ts`). |
| FR-007 | PASS (automated) | `ten_clean_worktrees_removed_leave_no_folder_record_or_branch`; `keep_and_own_commits_and_a_gone_repository_keep_and_name_what_stays`; `GW`: `own_commits_count_what_the_base_lacks_and_a_missing_base_keeps_the_branch`, `a_branch_checked_out_elsewhere_is_not_deleted`, `own_commits_that_cannot_be_counted_keep_the_branch`, `a_branch_that_moved_since_its_count_is_not_deleted`; `delete_agent_handler.tests.rs`: `a_branch_with_own_commits_or_without_its_base_is_kept`; a delete that fails part way names what went: `subtree_handler.tests.rs`: `a_failing_delete_deletes_no_agent_above_the_failed_one`, `delete_agent_handler.tests.rs`: `a_record_that_fails_after_the_removal_names_what_went`. |
| FR-008 | PASS (automated) | `a_start_after_the_worktree_folder_is_gone_is_refused`; `a_continue_after_the_worktree_folder_is_gone_is_refused_and_starts_nothing` (`H/tests/agent_worktree.rs`); `start_agent_handler.tests.rs`: `a_worktree_with_a_broken_git_link_refuses_the_start`, `a_deleted_worktree_folder_refuses_the_start`. |
| FR-009 | PASS (automated) | `H/bridges/template_create.tests.rs`: `a_team_whose_members_choose_an_own_worktree_gets_one_branch_and_worktree_each` (1 lead, 6 members, one branch and folder each), `an_own_worktree_choice_outside_a_repository_refuses_the_whole_creation_and_leaves_nothing`, `a_failure_at_the_last_member_removes_every_worktree_and_branch`, `an_agent_template_with_the_choice_creates_an_agent_in_its_own_worktree`; `H/tests/agent_worktree.rs`: `a_team_template_with_the_choice_on_the_members_makes_one_worktree_per_member`, `a_team_template_with_the_choice_outside_a_repository_creates_nothing`. Desktop: `D/e2e/templates.spec.ts`: `a team whose members choose an own worktree outside a repository creates nothing and says why`. |
| 0009/FR-001, 0009/FR-003 (modified) | PASS (automated) | `M/template/domain` `body_json.tests.rs`: `the_own_worktree_choice_defaults_to_off_and_round_trips_in_every_entry`, `an_own_worktree_choice_that_is_not_true_or_false_is_refused`; `template_service.tests.rs`: `a_body_copied_from_an_agent_with_its_own_worktree_keeps_the_choice`, `the_own_worktree_choice_survives_an_export_and_an_import`; `H/tests/templates.rs`: `export_then_import_into_a_fresh_office_keeps_everything_but_env_values` (0009 SC-002, the choice included); `H/tests/agent_worktree.rs`: `a_template_made_from_an_agent_with_its_own_worktree_keeps_the_choice_and_the_repository_folder`. Desktop: `bun test` `template.spec.ts`, `template-form.spec.ts`, `template-text.spec.ts`; `D/e2e/templates.spec.ts`: `a member entry's own worktree box is set by keyboard, saved with the template, and shown before a create`, `the import preview shows the own worktree choice` (both with axe). |
| PB-1 | PASS (automated) | `create_agent_handler.tests.rs`: `without_the_choice_no_worktree_is_made`; `the_choice_is_offered_only_inside_a_repository` (default create makes no worktree). |
| PB-2 | PASS (automated) | `GW`: `create_and_remove_leave_nothing_and_the_git_configuration_unchanged` (`.git/config` byte-identical), `neither_a_hook_file_nor_a_configured_hook_runs`; `two_agents_change_the_same_file_only_in_their_own_worktrees` (the repository folder shows 0 changes). No test reads a CLI's configuration; the change adds no such code path. |
| PB-3 | PASS (automated) | `a_subtree_delete_asks_once_and_a_stale_member_stops_it_before_any_delete` (the descendant count is still required); the 0002 tests in `H/tests/agent_subtree.rs` pass unchanged. |
| PB-4 | PASS (automated) | `each_create_and_delete_is_recorded_with_its_actor`. |
| PB-5 | PASS (automated) | `a_worktree_survives_a_reopen`; `H/tests/migrations.rs`: `an_office_from_before_own_worktrees_keeps_every_agent_without_one`. |
| A7 | PASS (automated) | `H/tests/agent_call.rs`: `an_unsupported_command_or_field_is_refused_and_recorded` (an agent's `own_worktree` is refused with `invalid`). |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `two_agents_change_the_same_file_only_in_their_own_worktrees`. |
| SC-002 | PASS (automated) | `ten_clean_worktrees_removed_leave_no_folder_record_or_branch`. |
| SC-003 | PASS (automated) | `changes_are_removed_only_with_the_hash_the_user_confirmed`; `a_file_the_running_agent_writes_at_its_stop_refuses_the_removal`. |
| SC-004 | PASS (automated) | `the_choice_is_offered_only_inside_a_repository`; `outside_a_repository_the_create_is_refused_and_makes_nothing`. |
| SC-005 | PASS (automated) | `a_worktree_survives_a_reopen` proves the columns survive a close and reopen of the database. `a_restart_of_the_server_keeps_every_branch_base_and_worktree_folder` (`H/tests/agent_worktree.rs`) stops the whole server and starts a new one on the same data folder: three agents with own worktrees list the same branch, base, folder, and working folder, an agent without one still lists none, and `agent.worktree_status` still reads each worktree. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| Folder not in a repository | PASS (automated) | `the_choice_is_offered_only_inside_a_repository`. |
| Detached checkout | PASS (automated) | `GW`: `info_when_detached_gives_the_full_commit_id`. |
| Branch name exists | PASS (automated) | `GW`: `a_taken_branch_name_moves_to_the_next_free_one`; domain: `a_taken_name_gets_the_lowest_free_suffix`. |
| Subfolder | PASS (automated) | `GW`: `a_subfolder_create_starts_the_agent_in_the_same_subfolder`, `a_subfolder_that_is_a_link_out_of_the_worktree_in_the_base_is_refused`. |
| Folder inside a linked worktree | PASS (automated) | `GW`: `info_in_a_linked_worktree_names_the_main_repository`. |
| Disk full or unwritable | PASS (automated) | `GW`: `an_unwritable_worktrees_folder_fails_the_create_and_leaves_nothing`. |
| Worktree folder deleted by hand | PASS (automated) | `a_start_after_the_worktree_folder_is_gone_is_refused` (start refused; remove clears the record). |
| Repository removed or moved | PASS (automated) | `keep_and_own_commits_and_a_gone_repository_keep_and_name_what_stays` (kept and named, the agent deleted). |
| Agent running at delete | PASS (automated) | `a_file_the_running_agent_writes_at_its_stop_refuses_the_removal`. |
| Lead and members, some with changes | PASS (automated) | `a_subtree_delete_asks_once_and_a_stale_member_stops_it_before_any_delete`. Desktop: `D/e2e/agent-worktree.spec.ts`: `a subtree delete shows one question listing every worktree`. |
| Branch checked out elsewhere | PASS (automated) | `GW`: `a_branch_checked_out_elsewhere_is_not_deleted`. |
| Base branch gone | PASS (automated) | `GW`: `own_commits_count_what_the_base_lacks_and_a_missing_base_keeps_the_branch`; `a_branch_with_own_commits_or_without_its_base_is_kept`. |
| Own worktree on an existing agent | PASS (automated) | No request offers it; `agent.create` is the only request that takes `own_worktree`. |
| Server unreachable or token refused | PASS (automated) | Unchanged 0001 states; the 0001 tests pass. |

## Plan deviations

Recorded in the spec's Deviations section and in the plan's decisions; none changes a requirement.

- **Branch name (spec A3):** `office-harness/<slug>`, then `-2`, `-3`, with no short id (PLAN D3, accepted in Efe's plan review r2). This is a planned deviation, not a fail.
- **Hooks off (PLAN D1):** the null device (`/dev/null`, `NUL`) as `core.hooksPath`, not the `.no-hooks` folder, plus the hooks that git's configuration names.
- **Status hash (PLAN D6):** the hash includes the size and modification time of each changed file.
- **Branch delete (PLAN D6):** `git update-ref -d` with the counted commit, not `git branch -D`; `branch -D` stays only in the undo of a create.
- **Subtree delete (PLAN D6):** one stale member refuses the whole subtree and deletes nobody, not "keep that member, delete the rest".
- **Delete answer:** `agent.delete` answers `{deleted_branches, kept_branches, kept_folders}` for every delete.

## Security checks (plan "Security pass")

| Check | Evidence |
|---|---|
| No shell; arguments as a list; paths after `--` | `GR`: `hooks_and_the_folder_come_first_and_arguments_stay_a_list`. |
| Base and branch name rules | `GW`: `a_base_outside_the_listed_branches_is_refused_before_git_runs`, `a_branch_named_like_a_commit_id_is_refused_as_base`, `a_branch_name_without_the_prefix_is_refused`; domain slug tests. |
| Clean environment, no prompt | `GR`: `the_child_environment_drops_git_and_askpass_variables`, `the_server_environment_cannot_point_git_elsewhere`. |
| Bounded output, cleaned stderr | `GR`: `more_than_one_mebibyte_of_output_fails`, `the_reason_loses_url_credentials_and_is_cut`, `a_failure_keeps_the_cleaned_reason`. |
| Timeout ends the tree | `GR`: `a_timeout_ends_git_and_its_children`, `a_child_that_keeps_the_pipe_after_git_exits_is_ended_at_the_deadline`. |
| Removal stays in the worktrees folder | `GW`: `a_folder_outside_the_worktrees_folder_is_never_removed` (also the worktrees folder, a repository's folder, and a folder below a worktree), `a_relative_worktrees_folder_is_made_absolute`. |
| No upward repository search | `GW`: `a_worktree_without_its_git_link_reads_as_unreachable`. |
| Hooks off, also hooks named in git's configuration | `GW`: `neither_a_hook_file_nor_a_configured_hook_runs` (fails when the configured-hook switch is removed); `GR`: `hooks_and_the_folder_come_first_and_arguments_stay_a_list`. |
| Control and bidirectional characters out of git's error text | `GR`: `the_reason_loses_control_and_bidirectional_format_characters`. |
| Independent reviews | Two read-only advisor reviews (git service; use cases and wiring), a review of the stopped duplicate's git service, and the PR gate review. Their findings are fixed or listed under Gaps. |

## Hands-on checks

| Check | Verdict |
|---|---|
| Real repositories on macOS and Windows | BLOCKED |
| A real CLI working in a worktree | BLOCKED |
| Screen reader pass of the delete question and the second confirmation | BLOCKED |
| 200 percent zoom of the create form and the delete question | BLOCKED |

## Gaps

- The Windows timeout path (`taskkill /T /F`) has no test; the Unix path does. On Windows a helper whose parent already exited can outlive the timeout; only Unix ends the whole process group.
- Accepted LOW review findings, not changed in this PR: a helper that keeps stderr open after git exits 0 reads as a timeout; a failed undo is not reported; two creates with the same name at once answer `internal` for the second, not `conflict`.
- Fixed after this report: the status read ran the repository's clean filters and submodule configuration, now off (PLAN D1; PR #104 and #107); the start check did not confirm that the worktree's git folder belongs to the stored repository, now it does (PLAN D7, `worktree_link.rs`).
- Changed after this report (ACF-183): a single delete now reads a stopped agent's worktree once, and a subtree delete reads each member at most three times (PLAN D6, lock scope); a data folder inside a linked worktree of the repository is refused as one inside the repository. Tests added by ACF-183 (commit 6d93302, read from the repository on 2026-11-22; not run for this file): FR-004, the data folder inside a linked worktree: `GW`, `the_data_folder_inside_a_linked_worktree_is_refused`. FR-005 to FR-007, in `M/agent/application/src/commands/delete_agent/delete_agent_handler.tests.rs` (as changed by 222ffb1, PR #108, which removed the first test of this change, `stop_and_check_reads_nothing_for_an_agent_that_was_already_stopped`): `stop_and_check_reads_a_stopped_agent_again_and_refuses_a_change` (a stopped member is read again and a change is refused) and `a_clean_confirmed_worktree_is_removed_without_force_with_its_branch` (a single delete reads the worktree once).
- The desktop e2e does not cover the "confirm again" re-ask or the branch-name `conflict` refresh; unit tests cover the first.
- The Windows path comparisons in the git tests run only in CI.
