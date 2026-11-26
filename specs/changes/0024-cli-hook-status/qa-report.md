# 0024 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0024-cli-hook-status` on `origin/main` 7c23699. The Rust gates ran at e8da8ca. The hands-on runs used a real, signed-in `claude` 2.1.322 through the office server.
- **Overall status:** Automated tests cover the requirements. SC-002 passed on the real CLI. Nine criteria are PARTIAL because their full counts were not run: SC-001, SC-003, SC-004, SC-005, SC-006, SC-007, SC-009, SC-011, and SC-014. Each row says what ran and what did not. The CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| PASS (hands-on) | A person or agent ran the check on the real surface. The evidence column says what ran. |

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `bash scripts/check-boundaries.sh` | workspace | PASS (`boundaries ok`) |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` (clippy 1.99 here; the pinned 1.100 is not installed. CI runs 1.100 and found one finding in a test file at 564393d, fixed after it) | workspace | PASS for everything this change adds. One finding of 1.99 in `cli/run.rs:18` (`must_use_candidate`) is not in a file this change touches. |
| `cargo test --workspace --exclude office-harness-desktop` | workspace | PASS: 1884 passed, 0 failed (the desktop crate needs system libraries that this machine lacks; CI covers it) |
| `bun test src`, `eslint`, `svelte-check`, `prettier --check`, the boundaries spec | desktop | PASS: 483 tests, no lint, type, or format findings |
| Playwright `e2e/hook-status.spec.ts`, and the `worktree-dependencies`, `needs-you`, and `panel` specs | desktop | PASS: 7 and 62 tests |

### Scope of the automated evidence

- `tests/hook_reports.rs` posts raw HTTP requests to a running server and checks each refusal and the caps.
- `tests/hook_status.rs` drives the status of a running agent with the reports that the hooks of `claude` send. The CLI is a shell script; the reports are posted by the test.
- `tests/hook_launch.rs` starts a fake `claude` (a script that answers `--version`) and reads the arguments and the environment it got. It also checks every route of the cleanup.
- Unit tests cover `HookSessions` (every row of the plan's event table), the settings file text, the version check, the secret store, and the launcher's decision.
- `tests/temps.rs` has one test for the idle limit of a temp. `tests/hook_status.rs` has one for the idle sleep.
- `tests/hands_on_claude.rs` is the hands-on run. It is ignored by default because it needs the owner's account.

## Hands-on runs with a real `claude` 2.1.322

The run starts one agent in a new folder, accepts the trust dialog, and sends prompts. Times are seconds from the start of the run and come from the `agent.status` and `hook.changed` events of the server.

| Step | What happened | Result |
|---|---|---|
| Trust dialog | The CLI waits for the folder to be trusted. No report arrives. The agent shows `waiting for input` after the quiet period, with the hook state `estimated` and the reason `no report has arrived yet`. | PASS (A1: hooks are held back until the folder is trusted; the fallback works) |
| Turn 1 ("reply with ok") | `working` at the prompt (22.39 s, less than 0.1 s after the keys), `waiting for input` with `turn_stopped` at 23.18 s, at the moment of the stop report. The hook state became `exact` at the first report. | PASS for SC-001 (real CLI, 3 runs) |
| Turn 2 (create a file, default permission mode) | `waiting for input` with the event `permission_asked` 1.4 s after the prompt, with one item and one notification. The hook state showed `ask_open`. Enter ended the ask and the status went to `working` at once. | PASS for SC-003 (status, item, notification) |
| Turn 3 (an essay, then Escape) | The CLI sent no stop report. The state turned `late` after one quiet period (5.3 s after Escape), with the reason `no report for a stalled turn`, and the agent showed `waiting for input`. | PASS (A1: Escape sends no stop report; FR-014) |
| `/compact` | `compacting` showed while it ran (7.3 to 8.5 s). After it ended, the agent returned to `waiting for input` at once with `turn_stopped`. The first run of this step found that the late state kept an open turn from the Escape step, so the agent waited for the quiet period. The fix resets the open work when a session turns late. | PASS after the fix (`hook_sessions` unit test and a rerun) |
| A project hook of the person (`UserPromptSubmit` that appends a line to a file) | It ran for every prompt, alongside the office hooks: 4 lines in the file for the 4 prompts sent (3 prompts and `/compact`). | PASS for FR-003, SC-006 (project hook; a user-level hook was not added) |
| A dead endpoint (`claude -p` with a hook to a closed port) | The turn took 4.0 and 4.8 s with the hook against 4.1 and 4.2 s without it, and the answer was the same. A refused connection fails at once. | PARTIAL for SC-007: a hanging endpoint was not tried. The hook timeout of 2 s is in the file. |
| 30-minute idle run (turn, then no input) | One notification for the turn. Then 0 status changes and 0 notifications for 30 minutes. | PASS for SC-002 |

## Success criteria

| Id | Verdict | Evidence |
|---|---|---|
| SC-001 | PARTIAL (hands-on, automated) | Three turns of a real `claude`: `working` within 0.1 s of the prompt, `waiting for input` at the stop report. Automated: `hook_status.rs` `a_turn_works_from_its_start_to_its_stop_within_a_second_of_each_report`. 20 rounds were not run.  Covered: 3 of 20 turns on the real CLI. |
| SC-002 | PASS (hands-on, automated) | A 30-minute run of the real `claude` 2.1.322 (`a_real_claude_idles_with_no_status_change_and_one_notification`, 1854 s): one notification for the turn, then 0 status changes and 0 notifications in 30 minutes of idle. The automated redraw test (`output_of_a_waiting_exact_session_makes_no_change_and_no_notification`) shows 0 changes in 3.5 s of redraws. |
| SC-003 | PARTIAL (hands-on, automated) | One real ask. Automated: the item and notification test, `an_open_ask_keeps_an_idle_agent_awake_even_after_its_item_is_dismissed`, and the directive readiness unit test (50 directives were not sent).  Covered: 1 of 10 asks on the real CLI. Not run: 50 directives during asks. |
| SC-004 | PARTIAL (automated) | `hook_reports.rs`: wrong secret, no secret, another session, an ended session (`every_refusal_is_one_answer_and_changes_nothing`), a body of 1 MB, a browser `Origin`, a wrong `Host`. A request from another machine could not be sent from this one; the loopback check is the `require_loopback_peer` layer of 0001.  Covered: each refusal with one 404, the caps, the browser origin. Not run: a request from another machine. |
| SC-005 | PARTIAL (automated) | `hook_launch.rs`: exit, failure, stop, server stop. Start failure: `terminal_host.rs` calls `end` on a failed spawn (no test forces a spawn failure with a file). Crash restart: `status_hook_files` unit test for the sweep.  Covered: exit, failure, stop, server stop, and the startup sweep. Not run: idle sleep as an end route, and a spawn failure with a real file. |
| SC-006 | PARTIAL (hands-on) | The project hook ran for every prompt next to the office hooks. A fake-CLI test of the user's settings is not possible: the fake CLI does not read settings.  Covered: one project hook on 4 turns. Not run: a user-level hook, a run with and without the office hooks, a comparison of the settings. |
| SC-007 | PARTIAL (hands-on) | See the table above. |
| SC-008 | PASS (automated) | `another_cli_gets_no_file_no_option_and_no_secret`; the 0001 status tests did not change. |
| SC-009 | PARTIAL (automated) | `the_cli_folders_are_not_touched` compares the fake home and XDG folders before and after a start. The real `~/.claude` was not compared.  Covered: the fake home and XDG folders. Not run: the real `~/.claude`. |
| SC-010 | PASS (automated) | The unit test `a_body_keeps_only_its_kind_and_the_helper_flag`, and `a_valid_report_reaches_the_sink_with_only_its_kind` (the log holds neither the prompt text nor the secret). The database is searched for the secret in `hook_launch.rs`; the prompt text never reaches the database because the server keeps no body. |
| SC-011 | PARTIAL (hands-on, automated) | One Escape on the real CLI: late after one quiet period. Automated: `a_lost_stop_turns_the_session_late_and_the_quiet_period_decides_again`. 10 rounds were not run.  Covered: 1 of 10 interrupts on the real CLI. |
| SC-012 | PASS (automated) | `the_switch_off_and_an_old_version_give_a_session_without_the_file`. |
| SC-013 | PASS (automated) | `an_open_permission_ask_keeps_a_temp_from_the_idle_end_after_its_item_is_dismissed`. |
| SC-014 | PARTIAL (automated) | `a_claude_session_gets_the_file_and_the_secret_and_loses_both_at_its_stop` searches the database and the log. Views, notifications, and action record entries never take the secret (it is in no wire type).  Covered: one session at a time. Not run: 10 sessions. |

## Not run

- A hanging endpoint (SC-007).
- A user-level hook and a managed policy with `allowManagedHooksOnly` (A1).
- A helper sub-agent of the CLI: the settings file subscribes to the six events of the main conversation only, so the turn events of a helper are never reported. A permission ask inside a helper would carry `agent_id`, and the server drops it (`reports_of_a_helper_sub_agent_change_nothing`), but this was not seen with a real helper.
- The minimum version 2.1.295 of the version check: taken from the CLI's documentation. Only 2.1.322 was run.
- macOS and Windows: the 3-OS CI is the evidence (owner rule of 2026-11-16).
