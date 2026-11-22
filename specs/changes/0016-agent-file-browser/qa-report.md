# 0016 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0016-agent-file-browser` at e2e4d07 (T5 desktop 23a561e, T2 to T4 server e65896b, T6 docs b394077, then main 468ad56 merged with 0013 and the 0002 A5 fix), toolchain 1.100.0, on Linux.
- **Overall status:** every requirement and success criterion has automated evidence. The server side runs against the real server and real git repositories; the desktop side runs the real webview code against the mock core. macOS and Windows rely on CI.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (review) | The criterion is about what the diff does not touch; a read of the diff proves it. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- `tests/file_view.rs` runs the real server with a user client over the authenticated connection, on real folders and real git repositories made by the test. The agents are not started: the file view reads the folder, not the CLI.
- The infrastructure tests run the real system git (`git_reader.tests.rs`, `local_file_view.tests.rs`) and the real file system, including links and a named pipe on Unix (`folder_reader.tests.rs`).
- The Playwright file `e2e/file-view.spec.ts` drives the built webview against the mock core (`e2e/support/mock-core.ts`, `e2e/support/file-fixture.ts`), with axe checks. It proves the UI behavior, not the server answers.
- No hands-on run of the packaged desktop app with a real server was made for this change (see Gaps).

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `scripts/lint-rust.sh` (fmt check, clippy `-D warnings`, all targets) | `agent-domain`, `agent-application`, `agent-infrastructure`, `agent-presentation`, `office-harnessd` | PASS (clean) |
| `cargo test` | `agent-domain` 74, `agent-application` 136, `agent-infrastructure` 165 and persistence 18, `agent-presentation` 36, `office-harnessd` lib 183 | PASS |
| `tests/file_view.rs` | 8 tests, 3 runs in a row | PASS (8 passed each time) |
| Regression | `agent_call` 22, `agent_worktree` 16, `agent_subtree_status` 9, `agent_directive` 8; after the merge of main `file_view` 8, `agent_call` 22, `bulk_resume` 8, `agent-presentation` 39, `agent-application` 151 | PASS |
| `bun test src` | 380 tests after the merge of main, with the new `models/file-view.spec.ts` | PASS |
| `bun run check`, `bun run lint`, `prettier --check` | the desktop app | PASS (0 errors, 0 warnings) |
| Playwright `e2e/file-view.spec.ts` | 23 tests, 2 workers | PASS |
| Playwright full suite | 254 tests, 2 workers, on the merged tree e2e4d07 | PASS |

## Requirements

| Id | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `folder_reader.tests.rs`: `a_listing_has_folders_first_then_names_by_code_point`, `more_than_a_thousand_entries_are_counted_in_more`, `the_git_folder_is_never_listed_in_any_case`, `link_targets_are_classified_from_the_root`; `file_view.rs` `every_request_answers_with_its_contract_fields`; e2e "lists folders first and follows the tree keyboard pattern". |
| FR-002 | PASS (automated) | `a_file_reads_as_text_with_its_size`, `an_empty_file_is_text_with_size_zero`, `the_size_cap_is_exact`, `a_zero_byte_in_the_prefix_makes_a_file_binary`, `a_folder_is_not_a_regular_file`; e2e "shows a text file with line numbers, read only", "shows the size and the limit of a file that is too large", "says that a binary file is not shown". |
| FR-003 | PASS (automated) | `a_path_that_leaves_the_root_is_refused_in_every_way`, `the_git_data_is_refused_even_through_a_link`, `a_link_that_stays_inside_is_followed_through_two_links`, `the_old_path_of_a_rename_goes_through_the_same_parse_and_containment`; the path form tests in `file_rules.tests.rs`; `file_view.rs` `no_path_outside_the_root_is_ever_answered` (SC-001). |
| FR-004 | PASS (automated) | `a_secret_looking_file_gives_no_bytes_without_a_reveal`, `a_plainly_named_link_to_a_secret_looking_file_is_secret`, `a_secret_looking_pipe_answers_secret_without_being_opened`, the three rename tests and `a_secret_untracked_file_is_not_read_without_a_reveal` in `local_file_view.tests.rs` (review condition C1); `file_view.rs` `secret_looking_files_send_no_content_until_one_reveal`, `a_renamed_secret_file_is_secret_in_the_changes_and_in_the_commit`; e2e "shows a secret file once and hides it again on the next open", "hides a revealed file on a tab switch and on refresh", "treats a rename of a secret-looking file as secret until revealed". |
| FR-005 | PASS (automated) | `the_changes_of_the_root_list_every_kind_and_skip_ignored_files`, `a_subfolder_root_lists_its_own_paths_without_the_outside_ones`, `more_than_a_thousand_changes_are_counted_in_more`, `the_diff_holds_staged_and_unstaged_changes_without_git_headers`, `a_staged_rename_diffs_through_both_paths`, `an_untracked_file_diff_is_built_from_its_bytes`, `a_binary_change_is_binary_and_a_huge_diff_is_too_large`; e2e "shows a diff, and says when a diff is too large or binary". |
| FR-006 | PASS (automated) | `the_history_pages_by_fifty_and_counts_only_commits_inside_the_root`, `a_commit_shows_its_message_and_its_files_against_the_first_parent`, `a_root_commit_shows_every_file_as_added_and_diffs_against_nothing`, `a_merge_commit_is_compared_with_its_first_parent`, `an_id_that_is_not_a_commit_of_the_repository_is_invalid`; `file_view.rs` `bad_requests_get_the_codes_of_the_contract` (`HEAD`, a short id, `--output=x`, upper case); e2e "lists 50 commits, then 100, then all 120, and opens a commit and its diff". |
| FR-007 | PASS (automated) | `local_file_view.tests.rs` `a_whole_session_runs_no_program_and_writes_nothing` (SC-004) with its control `the_marker_programs_do_run_when_git_is_not_hardened`. |
| FR-008 | PASS (automated) | e2e "shows a loading text while the office answers", "shows an empty folder", "says that the folder is not in a git repository", "says that there is no change and no commit", "shows each failure with its reason and a Refresh action"; `a_missing_root_is_a_failure_that_says_why`, `without_git_every_git_answer_says_so`. |
| FR-009 | PASS (automated) | bun `models/file-view.spec.ts` (`visibleLine`: tab kept, ESC, DEL, C1, U+202E, CR at line end; base64 with invalid UTF-8); e2e "shows a file as text, with control characters as symbols", "shows a commit message as text, with control characters as symbols". |

## Success criteria

| Id | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `file_view.rs` `no_path_outside_the_root_is_ever_answered`: 24 refused requests (parent parts, absolute paths, a link to a file outside, a link to a folder outside, a link inside a linked folder, a chain that leaves the root, and the git data directly and through a link), and no answer holds a byte of the outside files. |
| SC-002 | PASS (automated) | `the_size_cap_is_exact` (524,288 bytes shown, 524,289 `too_large` with 0 bytes); `a_binary_change_is_binary_and_a_huge_diff_is_too_large`. |
| SC-003 | PASS (automated) | `file_view.rs` `secret_looking_files_send_no_content_until_one_reveal` (10 secret-looking changed files: 0 content bytes in the tree, reads, changes, and diffs; one reveal gives exactly that file) and `a_renamed_secret_file_is_secret_in_the_changes_and_in_the_commit`; the UI side in the e2e reveal tests. |
| SC-004 | PASS (automated) | `a_whole_session_runs_no_program_and_writes_nothing`: a repository that names a hook, `diff.external`, a textconv driver, a clean filter (required), a smudge filter, `core.fsmonitor`, and `log.showSignature` with `gpg.program` on a signed commit; after listing, reading, changes, diffs, history, and commits, 0 marker files exist, and the snapshot of every path, its bytes, and its modification time in the folder and its git data is unchanged. The control test shows the markers fire without the hardening. |
| SC-005 | PASS (automated) | `the_changes_of_the_root_list_every_kind_and_skip_ignored_files` and `a_subfolder_root_lists_its_own_paths_without_the_outside_ones` (modified, staged new, deleted, renamed, untracked inside; one modified outside, not listed). |
| SC-006 | PASS (automated) | `the_history_pages_by_fifty_and_counts_only_commits_inside_the_root` (120 commits in the root, 10 outside: 50, 100, 120, none of the 10); the e2e paging test. |
| SC-007 | PASS (automated) | e2e: one test per row of the Visible states table, the keyboard tests ("opens from the details by keyboard...", "moves between the parts with the arrow keys, Home, and End", "lists folders first and follows the tree keyboard pattern"), and axe checks. |
| SC-008 | PASS (automated) | e2e "shows a file as text, with control characters as symbols" and "shows a commit message as text...": `<script>` and `<img>` show as text, no script runs, no image request is made, U+001B shows as `␛` and U+202E as `‮`. |
| SC-009 | PASS (automated) | `file_view.rs` `a_large_folder_lists_reads_and_reports_changes_in_time`: in a folder of 50,000 files, a folder of 1,000 entries lists within 2 s, a 512 KiB file reads within 2 s, and 1,000 changed files list within 5 s. |

## Preserved behavior

| Id | Verdict | Evidence |
|---|---|---|
| PB-1 | PASS (automated) | `file_view.rs` `the_agent_connection_refuses_a_files_request_and_records_it`; `ALLOWED_REQUESTS` of `agent_call.rs` is unchanged (diff). |
| PB-2 | PASS (automated) | SC-004 snapshot; no CLI configuration path is touched (diff review). |
| PB-3 | PASS (review) | The six requests are served only after the authenticated user connection (0001 auth tests stay green); the agent connection does not offer them (PB-1). |
| PB-4 | PASS (automated) | `file_view.rs` `reading_changes_no_status_publishes_no_event_and_records_no_action` (a second client sees no event; the action record and the agent's status do not change). |
| PB-5 | PASS (automated) | Full Playwright suite 254 passed on the merged tree. Two existing tests now scroll the terminal into view first (Deviations). |

## Deviations from the plan

- **One server commit for T2 to T4.** The folder reader and the git reader use the port types of the application layer, so a commit per task would not build alone. They are one commit, e65896b.
- **Filter switches reused.** The 0010 function that empties filter drivers lists the configuration with `config --list --name-only -z` instead of `config --get-regexp`. It now also empties `smudge`, because `git diff` ran a configured smudge filter in the marker test. The 0010 worktree status and remove runs get the same switch, which changes nothing for them.
- **Submodule work trees.** Every status and diff passes `--ignore-submodules=dirty`, so git never runs inside a submodule, whose own filters the switches do not cover. A submodule shows as changed only when its recorded commit changed.
- **Root commit.** A commit without a parent is compared with the empty tree from `hash-object -t tree --stdin`, in both the file list and the diff, instead of `diff-tree --root`. The result is the same.
- **Renames across the root.** git pairs renames only inside the pathspec of the root, so in a subfolder root a file moved across the root's edge shows as added or deleted, never as a rename with an outside old path. The guard that drops an outside old path stays but cannot be reached by a test.
- **Commit id check.** The full-hex check runs in the application handler (the 0010 rule `is_commit_id`); a tree id or an unknown full id is refused by the git reader.
- **Pure rename.** A rename or a mode change without content change has no hunk, so its diff answers `none`, also after a reveal.
- **Read time.** `read_at` comes from the system clock in the infrastructure layer, not from the `Clock` port. The History tab shows the desktop's load time, because the history answer has no `read_at`.
- **Commit answer budget.** The commit's file list is budgeted after the size of the escaped message, so a 64 KiB message cannot push the answer past the frame.
- **Existing e2e tests.** The "Browse files" button can wrap the agent's action row and put the terminal below the 720 px fold of the test window, where xterm pauses rendering. `terminal.spec.ts` and the panel SC-002 test scroll the terminal into view first; no product code changed for this.
- **Reflow test.** At 640 px the panel behind the dialog already scrolls sideways before this change; the test checks that the dialog itself stays inside the viewport and does not scroll sideways.

## Gaps and residual risk

- **No hands-on run of the packaged app.** The desktop flow is proven against the mock core and the server against real repositories, but the two were not run together in the packaged app on a real agent for this change.
- **Unix-only open flags.** `O_NONBLOCK` and `O_NOFOLLOW` apply on Unix only. On Windows a named pipe cannot be swapped in for a file the same way, and the containment check still runs first.
- **Race and hard links.** An agent that swaps a folder for a link between the check and the read, or that adds a hard link to an outside file, can still make the view show that file (spec Residual risk).
- **`.git` on Windows.** The `.git` refusal compares ASCII case only; NTFS short names and trailing dots are not covered.
- **Large commit message.** A message over the git runner's 1 MiB output limit answers `internal` instead of a shortened message.
- **git version.** The marker control test depends on the local git starting the hook and the monitor when not hardened; another git version could fail the control before the hardening.
