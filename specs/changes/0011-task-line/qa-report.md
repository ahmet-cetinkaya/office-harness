# 0011 QA report

## Summary

- **Date:** 2026-11-22.
- **Build under test:** branch `feat/0011-task-line`. The server gates ran on the tree of 8910f6d (T2 and T3), toolchain 1.100.0. The desktop gates ran on the working tree of T4 on top of 8910f6d (the T4 and T5 commits did not exist yet when they ran).
- **Overall status:** the automated evidence stands. Rows that need a person, a real CLI, or a real app are BLOCKED. Nobody ran them.
- **Owner decision, 2026-11-20:** separate hands-on runs on macOS and Windows are not needed; the CI evidence is enough. The macOS and Windows hands-on rows are WAIVED (owner), not PASS: nobody ran them.
- **Status of this file:** the gate numbers below come from runs on 2026-11-22. The test names were read from the repository on 2026-11-22. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| BLOCKED | A person must run the check. Nobody has run it. It is not a PASS. |
| WAIVED | The owner decided that the check is not needed. Nobody ran it. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server. The mock core was written for this change and follows the contract; it is not the server.
- The Rust gates ran per crate, because the `office-harness-desktop` crate does not build on the Linux machine used here (libdbus-sys). CI builds it on all three OSes.

### Local gates (2026-11-22)

| Gate | Scope | Result |
|---|---|---|
| `cargo clippy -D warnings` | `agent-domain`, `agent-application`, `agent-infrastructure`, `agent-presentation`, `audit-domain`, `hierarchy-application`, `office-harnessd`, at 8910f6d | PASS (clean) |
| `cargo test` | at 8910f6d: `agent-domain` 55, `agent-application` 102, `agent-infrastructure` 58 + 15 persistence, `agent-presentation` 23, `audit-domain` 5, `hierarchy-application` 58, `office-harnessd` lib 130, and every integration binary green | PASS |
| `tests/task_text.rs` | 11 tests, run 4 times, at 8910f6d | PASS (11 passed each time) |
| `scripts/check-boundaries.sh` | at 8910f6d | PASS (boundaries ok) |
| `bun test src` | desktop, T4 working tree: 238 pass, 0 fail (13 are the new `task-text.spec.ts` tests) | PASS |
| Playwright | desktop, full suite, T4 working tree: 169 passed | PASS |
| Playwright, new file | `e2e/task-line.spec.ts`: 10 tests, run 3 times with `--repeat-each=3`: 30 passed | PASS |
| `svelte-check` | desktop, T4 working tree: 0 errors, 0 warnings | PASS |
| `eslint`, `prettier --check` | desktop, T4 working tree | PASS (clean) |
| CI | the PR | Not run yet |

Not run: each commit was not built on its own; the gates cover the trees named above. No `cargo` gate ran on the T4 and T5 changes, because they touch no Rust.

## Requirements

Paths are relative to the repository root. `H` is `src/apps/office-harnessd`, `D` is `src/apps/desktop`. Server tests are in `H/tests/task_text.rs` unless a path says otherwise.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `D/src/modules/agent/models/task-text.spec.ts`: `taskLine` (first line, exactly 80 graphemes whole, 81 cut with an ellipsis, an emoji as one grapheme), `taskAge` (0, 59, 60, 125 minutes), `taskRowText`. `D/e2e/task-line.spec.ts`: `a pushed task shows in the row and in its accessible name`, `a long task stops at 80 characters with an ellipsis`. |
| FR-002 | PASS (automated) | `a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status`; `a_refused_directive_changes_no_task_text`. |
| FR-003 | PASS (automated) | Server: `the_user_sets_edits_and_clears_the_task_text_and_every_client_gets_it`, `a_task_text_of_the_limit_is_accepted_and_one_past_it_is_refused`, `an_agent_connection_may_not_set_a_task_text`. Desktop: `D/e2e/task-line.spec.ts`, `a task is set, edited, and cleared by keyboard` (the record shows the entry), `8,193 characters show the refusal and keep the editor open`, `Cancel leaves the task as it was`. |
| FR-004 | PASS (automated) | `a_new_task_clears_the_text_and_start_and_continue_keep_it`. Desktop: `D/e2e/task-line.spec.ts`, `a new task clears the line`. |
| FR-005 | PASS (automated) | `D/e2e/task-line.spec.ts`: `the details show the full text with control codes as symbols` (the second line shows in the details); `a task is set, edited, and cleared by keyboard`. |
| FR-006 | PASS (automated) | `D/src/modules/agent/models/task-text.spec.ts`, `visibleText` (ESC, BEL, DEL, a C1 code, U+202E and the other bidirectional format characters). `D/e2e/task-line.spec.ts`: `an escape code shows as visible symbols on one line`, `the details show the full text with control codes as symbols`. Server: `a_control_character_is_stored_as_it_was_entered`. |
| FR-007 | PASS (automated) | `a_member_that_prints_output_keeps_its_task_text`. `D/e2e/task-line.spec.ts`: `the task event keeps the last line and the status`. The 5-minute run with a real CLI is BLOCKED (SC-002). |
| 0003/FR-002 (modified) | PASS (automated) | `D/e2e/task-line.spec.ts`, `a pushed task shows in the row and in its accessible name`. |
| PB-1 | PASS (automated) | `a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status` (the terminal types exactly the directive and Enter). |
| PB-2 | PASS (automated) | `a_new_task_clears_the_text_and_start_and_continue_keep_it` (start and continue keep the text). |
| PB-3 | GAP | No test reads or writes a CLI prompt, title, or configuration. The change adds no such code path; the evidence is the diff, not a test. |
| PB-4 | PASS (automated) | `D/e2e/task-line.spec.ts`, `the task event keeps the last line and the status`; the existing panel, office, and scene suites pass unchanged in the full run of 169. |
| PB-5 | PASS (automated) | `D/e2e/task-line.spec.ts`, `a task is set, edited, and cleared by keyboard`, `Cancel leaves the task as it was`, and `8,193 characters show the refusal and keep the editor open`: Set, Edit, Save, Cancel, and Clear all run by keyboard with visible focus returned to the Edit control, each state with axe (no serious or critical violation). |
| A1, A3 | PASS (automated) | `task-text.spec.ts`: `taskAge` and `taskLine`. |
| A2 | PASS (automated) | `the_user_sets_edits_and_clears_the_task_text_and_every_client_gets_it` (the terminal gets nothing from `agent.task.set`, as the test asserts). |
| A4 | PASS (automated) | `a_user_edit_racing_a_directive_leaves_both_in_the_record` and `a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status` (one `directive` entry, no `task_text` entry for a directive). |
| SC-001 | PASS (automated) | `a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status` (the event within the test's wait). Row: `D/e2e/task-line.spec.ts`, `a pushed task shows in the row and in its accessible name`. A real app run is BLOCKED. |
| SC-002 | PARTIAL (automated) | Machine part: `a_member_that_prints_output_keeps_its_task_text`. The 5-minute run with a real CLI that changes its title is BLOCKED (Manual QA). |
| SC-003 | PASS (automated) | `D/e2e/task-line.spec.ts`, `an escape code shows as visible symbols on one line`: one line, the ESC symbol visible, the raw ESC absent from the row text. |
| SC-004 | PASS (automated) | `a_new_task_clears_the_text_and_start_and_continue_keep_it`; the row clears in `D/e2e/task-line.spec.ts`, `a new task clears the line`. |
| SC-005 | PASS (automated) | `the_task_text_and_its_time_survive_a_restart_and_go_with_a_deleted_agent`; the repository and migration tests of `agent-infrastructure` (58 + 15 persistence, green). |
| US1, US2 | PASS (automated) | The server tests and the `D/e2e/task-line.spec.ts` tests above. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| An agent never got a directive and the user set nothing (Empty) | PASS (automated) | `D/e2e/task-line.spec.ts`, `a task is set, edited, and cleared by keyboard` starts with `No task text.` and no row line; the other suites show rows without a task line. |
| Empty or whitespace text clears (Empty) | PASS (automated) | `the_user_sets_edits_and_clears_the_task_text_and_every_client_gets_it`; desktop: the clear step of `a task is set, edited, and cleared by keyboard`. |
| 8,192 characters accepted, 8,193 refused (Boundary) | PASS (automated) | `a_task_text_of_the_limit_is_accepted_and_one_past_it_is_refused`; desktop: `8,193 characters show the refusal and keep the editor open`. |
| The first line is exactly 80 characters (Boundary) | PASS (automated) | `task-text.spec.ts`, `shows exactly 80 graphemes whole`. |
| Escape codes or control characters in the text (Invalid) | PASS (automated) | See FR-006. |
| Two directives at once (Concurrent) | PASS (automated) | `directives_at_once_leave_the_text_that_was_written_last` (3 directives, because the per-target limit is 3 per 60 s). |
| A refused directive (Invalid) | PASS (automated) | `a_refused_directive_changes_no_task_text`. |
| A user edit while a directive arrives (Concurrent) | PASS (automated) | `a_user_edit_racing_a_directive_leaves_both_in_the_record`. |
| The agent is deleted (Boundary) | PASS (automated) | `the_task_text_and_its_time_survive_a_restart_and_go_with_a_deleted_agent`. |
| The text holds sensitive text and the screen is shared (Failure) | GAP | Out of scope by the spec: the office shows it like any other row text. |
| The app cannot reach the server or its token is rejected (Unauthorized) | PASS (automated) | The 0001 states are unchanged; the existing e2e suites cover them and pass in the full run. No 0011 test is specific to it. |

## Known gaps (open only)

1. **The mock core is not the server.** The desktop tests prove the webview against the contract's shapes. The real server and the real app were not run together by an agent here.
2. **No unit test covers `withTask` and the controller's `saveTask` on their own.** The Playwright tests cover them through the screen.
3. **The age ticks once a minute and is proven only at its first reading** ("0 min ago"). No test waits a minute.
4. **PB-3 has no test** (see the row above).
5. **The security pass** of the plan (`secure-engineering`, light) is not recorded here. The server enforces the length cap (`a_task_text_of_the_limit_is_accepted_and_one_past_it_is_refused`), refuses the agent connection (`an_agent_connection_may_not_set_a_task_text`), and the display never applies control codes (FR-006 rows); the review itself was not run in this task.

## Plan deviations

1. T2 and T3 landed in one commit (8910f6d), because they share the bridge and fake files.
2. `AgentStore::update_task` takes `Option<&TaskText>`, not `Option<TaskText>`.
3. The concurrency test sends 3 directives, not more, because the per-target limit is 3 per 60 seconds.
4. One more `AgentStore` fake needed `update_task`: the agent presentation handler tests.
5. T4: the action record label for `task_text` is "Changed the task text" (the plan said "task text"), in the style of the other labels. The helper adds `taskRowText` (age and line in one function) beside `visibleText`, `taskLine`, and `taskAge`. The `\p{Cc}` property class replaces three hand-written ranges for C0, DEL, and C1 in `visibleText`, because the linter refuses control characters in a regular expression.
6. T4: the line feed is kept by `visibleText` (it only breaks the line); `taskLine` cuts at it, and the details wrap it.

## Manual QA

No hands-on row has been run. All rows stay BLOCKED until a person runs them.

| Check | Verdict | Note |
|---|---|---|
| Screen reader pass: the task line in the row name, the Task section, Edit, Clear, and the refusal message | BLOCKED | Nobody has run it. The row name carries the task line (proven by the e2e name check); the spoken output is unproven. |
| 200 percent zoom: the row with a task line and the Task section with its editor | BLOCKED | Nobody has run it. |
| A real agent CLI that prints output and changes its title for 5 minutes keeps its task line (SC-002) | BLOCKED | Nobody has run it. |
| The task line in the real app on Linux, with a real directive between two agents | BLOCKED | Nobody has run it. |
| The same on macOS and Windows | WAIVED (owner, 2026-11-20) | CI evidence on the three OSes suffices; nobody ran a hands-on pass. |
