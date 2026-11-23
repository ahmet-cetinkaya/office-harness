# 0021 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0021-worktree-dependencies` at HEAD, on `origin/main` 7d1e85d (no newer main commit at the time of the run). Local toolchain 1.100.0, as in CI.
- **Overall status:** automated tests and one hands-on run prove the links, the refusals, the removal, the setting, and the create form note on Linux. The criteria that name counts (SC-003, SC-004, SC-006, SC-007) are proven in behavior with fewer runs than the stated counts, so they are PARTIAL. SC-006 (Windows) has a unit test that CI runs on Windows.
- **Status of this file:** the gate numbers come from local runs on 2026-11-23. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (hands-on) | A person or an agent ran the check on the real surface. The evidence column says what ran. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |

### Scope of the evidence

- `git_worktrees_dependencies.tests.rs` runs `GitWorktrees` with the real git in temporary folders. The link tests are Unix only; one test runs only on Windows.
- `tests/agent_worktree.rs` runs an in-process server behind a real socket, with the real git and fake CLIs.
- The Playwright tests run against a mock core, so they prove what the desktop shows and sends, not the real server.
- The hands-on run used the real server, a snapshot of this repository with its packages installed by `bun install`, and the real `bun` and Vite. It ran no agent CLI; it ran the commands an agent would run in the worktree.

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --check` | workspace | PASS |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -D warnings` | workspace | PASS (clean) |
| `cargo test --no-fail-fast` | workspace without `office-harness-desktop`: 1531 passed, 0 failed | PASS |
| `scripts/check-boundaries.sh` | repository | PASS (boundaries ok) |
| `bun test src` | desktop: 421 passed, 0 failed | PASS |
| Playwright | desktop, full suite on the mock core, Chromium: 282 passed, 0 failed | PASS |
| `svelte-check`, `eslint`, `prettier --check` | desktop: 531 files, 0 errors, 0 warnings; lint and format clean | PASS |
| Hands-on: a new worktree of this repository builds and tests without an install | see below | PASS |
| CI | the PR | In the PR |

### Hands-on run (2026-11-23)

A snapshot of this branch, committed as a new repository with `bun install --frozen-lockfile` run in it, served as the base checkout. A real server created an agent with own worktree there.

| Step | Result |
|---|---|
| `agent.worktree_info` | `shares_dependencies: true` |
| `git status --porcelain --untracked-files=all` in the new worktree | empty |
| `bun test src` in the worktree's `src/apps/desktop`, no install before it | 421 passed, 0 failed |
| `bun run build` (Vite) in the worktree's `src/apps/desktop` | passed; the worktree's `node_modules/.vite` is not a link |
| The checkout's `src/apps/desktop/node_modules/.vite` (with a marker file made before the create) | unchanged (review r1 N1) |
| `git status` in the worktree after the test and the build | empty |
| `agent.worktree_status` | `changed_files: 0` |
| `agent.delete` with remove | ok; the branch deleted, the worktree folder gone |
| The checkout's `node_modules`, `src/apps/desktop/node_modules`, and `.vite` after the removal | unchanged (names and times) |

## Requirements

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `settings.tests.rs`: `defaults_follow_the_data_model` (on), `worktree_dependency_links_take_on_or_off`, `worktree_dependency_links_refuse_other_text_and_keep_the_stored_value`. Wire: `with_the_setting_off_a_new_worktree_gets_no_links`. Keeping the setting across restarts uses the existing settings rows, which `idle_sleep` and 0001 tests cover. Desktop: `worktree-dependencies.spec.ts` (on by default, off by keyboard, Escape and Cancel store nothing, a refusal shows the reason and keeps the dialog, axe), `worktree-dependencies-controller.spec.ts`. |
| FR-002 | PASS (automated) | `packages_are_linked_at_the_top_and_beside_each_tracked_package_file` (rules `node_modules/` and `node_modules`); wire `a_new_worktree_shares_the_installed_packages_and_never_counts_them`; hands-on run. Template creates use the same `WorktreeCreate` path as 0010 FR-009. |
| FR-003 | PASS (automated) | (a) `nothing_is_linked_while_the_setting_is_off`; (b) `no_link_is_made_on_windows` (CI, Windows); (c, d, e, f) `a_place_without_an_ignored_free_folder_gets_nothing`. |
| FR-004 | PASS (automated) | `links_that_lead_outside_the_base_checkout_are_refused_and_logged`: a package link to an outside folder, a broken package link, and a dependency folder that is itself a link get no link and one log line each; `pkg` is still linked. |
| FR-005 | PASS (automated) | `git status` empty and `status().changed_files == 0` in `packages_are_linked_at_the_top_and_beside_each_tracked_package_file`; `staging_everything_stages_no_link`; wire `changed_files: 0`. The git configuration test of 0010 (`create_and_remove_leave_nothing_and_the_git_configuration_unchanged`) still passes. |
| FR-006 | PASS (automated) | `remove_and_discard_unlink_first_and_leave_the_base_checkout_unchanged`; `the_agents_own_link_and_files_are_left_to_the_removal_and_never_followed`. The links are found from the worktree itself, so a restart changes nothing. |
| FR-007 | PASS (automated) | `a_place_that_cannot_be_linked_is_logged_and_the_create_goes_on` (an unreadable base folder). |
| FR-008 | PASS (automated) | Wire: `agent.worktree_info` answers `shares_dependencies` true, then false after the setting is off. Desktop: `the create form note says whether the packages are shared`. |
| PB-1 to PB-7 | PASS (automated) | The 0010 suites (`agent_worktree.rs`, `git_worktrees.tests.rs`, desktop `agent-worktree.spec.ts`) pass unchanged, except one expected field: the `agent.worktree_info` answer now has `shares_dependencies`. |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (hands-on) | `bun test src` passed in a new worktree of this repository with 0 installs (421 tests). |
| SC-002 | PASS (automated) | `git status` empty and 0 changed files after the create (unit, wire, hands-on). |
| SC-003 | PARTIAL (automated) | 2 removals and 1 undo per unit test, 1 removal in the wire test and the hands-on run, each with the base packages compared before and after. Not 10 removals with 5 after a restart; the links are found from disk, so a restart does not change the removal. |
| SC-004 | PARTIAL (automated) | 1 create with the setting off (unit and wire), not 10. |
| SC-005 | PASS (automated) | `links_that_lead_outside_the_base_checkout_are_refused_and_logged`. |
| SC-006 | PARTIAL (automated) | `no_link_is_made_on_windows`, 1 create, runs in CI on Windows. |
| SC-007 | PARTIAL (automated) | A failed place is proven with an unreadable folder, not with a file system without links. |
