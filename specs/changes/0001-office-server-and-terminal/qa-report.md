# 0001 QA report (draft)

## Summary

- **Date:** 2026-11-16.
- **Build under test:** branch `feat/0001-office-server-and-terminal`.
- **Overall status:** `blocked`. The run is complete only when every row is PASS or WAIVED (owner). The Linux hands-on rows are still BLOCKED, so the run is blocked.
- **Owner decision, 2026-11-16:** separate hands-on runs on macOS and Windows are not needed; the CI evidence on Linux, macOS, and Windows is enough. The macOS and Windows hands-on rows are WAIVED (owner), not PASS.
- **Status of this file:** a draft. On 2026-11-19 an agent ran SC-001, SC-003, and SC-007 on Linux/KDE Wayland against the built app and server of `main` at f6dc663, with fake CLIs; those rows say "agent-run". The other Linux hands-on rows still need a person, a real CLI account, a restart, or a participant. No value in this file is an estimate. Raw evidence (screenshots, traces, logs, timing files) is kept outside the repository, in the QA agent's folder of the office hive (`hive/agents/celal-mwhvybh8/qa/evidence/`); repository readers cannot open it. The agent-run rows therefore states its own method, setup, and measured values. Run setup: debug builds of the desktop app and `office-harnessd`, WebDriver through `tauri-driver`, an isolated data and config folder per run with login start off, fake CLIs (shell scripts) in place of agent CLIs, and no change to system settings.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test that runs in CI proves the criterion on that OS. The evidence column names the test. |
| PENDING (Linux run) | A person can run the check on Linux. Nobody has run it yet. |
| WAIVED (owner) | The owner decided that this hands-on check is not needed on this OS. It is not a PASS: nobody ran it. The evidence column names the automated tests that ran on that OS, if any. |
| BLOCKED | The check needs a person on that OS, a real CLI account, a restart, a 60-minute wait, a network capture, or a human participant. It is not done. |

### Scope of the automated evidence

- The automated tests use fake CLIs, not the real `claude`, `codex`, `gemini`, or `opencode`.
- A PASS (automated) row shows that the code does what the criterion says. It does not replace a hands-on run with the real surface.
- CI run 43780798038 at commit 264148efcbff1e7c7d2aee5127be0620f97590fc passed on Linux, macOS, and Windows. It ran every named test; `tests/sessions.rs` runs only on Linux and macOS (`#![cfg(unix)]`). The SC-005 and SC-009 tests moved to `tests/sessions_portable.rs`, which also runs on Windows; CI run 43787191028 at commit beb3d280d02980dad2e40c7011d675770e197aee ran both of them on Linux, macOS, and Windows and passed. A later commit that changes only this file does not change the tested code.

### What a person must run

**On Linux** (the macOS and Windows runs are waived by the owner)

- Install the build from the `release` workflow artifact of this branch.
- SC-001: type 1,000 keystrokes into an agent that runs an echo test CLI, and record the time from the keydown in the webview to the render of the echoed character. Report the 95th percentile.
- SC-002: run the four-CLI task script with `claude`, `codex`, `gemini`, and `opencode`. Record any account type that cannot run `gemini`.
- SC-003: start agents, keep the app closed for 60 minutes, then open it and compare the agents and their output.
- SC-004: restart the machine, log in without opening the app, and record the time until the server runs and what the office holds.
- SC-007: capture network traffic for 1 hour, filtered to the office-harness processes.
- SC-008: a participant who has never used office-harness creates an agent and types into its terminal. Time the run and name the participant in the report.
- SC-006, second part: try to connect from another machine and record that the attempt fails.
- SC-005 with the real CLIs: compare the CLI configuration folders before and after a session (optional, because the automated test covers the criterion with fake CLIs).

## SC-001: Typed characters appear within 100 ms (p95)

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-001 | Linux | 1,000 keystrokes with an echo test CLI; p95 of keydown to xterm.js render | p95 58 ms, p50 52 ms, max 60 ms (DOM updated in the page); next animation frame p95 74 ms, max 76 ms | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) | Real app and server, an echo fake CLI in raw mode. 1,000 keys sent through WebDriver at 50 ms spacing; a keydown listener timestamped each key (performance.now) and a MutationObserver on the terminal rows timestamped the echoed character; 1000 of 1000 matched. Also 100 keys at 300 ms spacing: p50 10 ms, max 20 ms. Pass is p95 at most 100 ms. Excluded: OS input latency (the keys enter the page through the driver, not the keyboard), and the build was a debug build with the DOM renderer. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0001-SC-001/`. |
| SC-001 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-001 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-002: A full task in each of the four CLIs

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-002 | Linux | Four-CLI task script: start, prompt, approve a tool use with the keyboard, Ctrl+C, continue a previous session | Not measured | BLOCKED | Needs the real CLI accounts and a person at the keyboard. `gemini` may not run on every account type; record it. |
| SC-002 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-002 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-003: Agents keep running while the app is closed for 60 minutes

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-003 | Linux | Close the app for 60 minutes, then reopen | App closed 2 h 53 min; 5 of 5 agents still ran; 5 of 5 terminals showed output written while the app was closed | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) | Real server and debug app of `main` at f6dc663, 5 fake agents that write a numbered line every 20 s. App closed at 19:48:43 (agents at line 2 or 3). At 22:42, `agent.list` showed all 5, and their side logs were at line 180. On reopen, each terminal showed its latest 20 visible lines (162 to 182), all from the closed period. Not covered: real agent CLIs. Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0001-SC-003/`. |
| SC-003 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-003 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-004: The server runs within 60 seconds after a machine restart and login

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-004 | Linux | Restart the machine, log in, do not open the app, time the server start, then list the office | Not measured | BLOCKED | Needs a machine restart. The tests `definitions_and_settings_survive_a_restart` and `running_agents_become_interrupted_at_start_and_none_starts_by_itself` in `src/apps/office-harnessd/tests/startup.rs` cover the stored office and the `interrupted` status only, not the login start or the 60-second limit. |
| SC-004 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-004 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-005: 0 files changed in a CLI's configuration

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-005 | Linux | Create, start, continue, edit additions, stop, and delete an agent for each of the four CLI names, with fake CLIs that write nothing. Compare size, modification time, and content hash of the home folder and the XDG folders before and after. | 0 files changed (per the test assertion, CI run 43787191028) | PASS (automated) | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration`. The file runs on Linux, macOS, and Windows. It uses fake CLIs, which matches the criterion's CLI that writes nothing itself. |
| SC-005 | macOS | Same as Linux, by hand or with the Unix test | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. `tests/sessions_portable.rs` test `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration` passed on this OS in CI run 43790227411. |
| SC-005 | Windows | Same test as Linux, with fake `.cmd` CLIs and a fixed environment: `USERPROFILE`, `APPDATA`, and `LOCALAPPDATA` point into a temporary fake home. | 0 files changed (per the test assertion, CI run 43787191028) | PASS (automated) | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration`. It uses fake CLIs, not the real ones. |

## SC-006: Connections without a valid token are refused, and none from another machine succeeds

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-006 | Linux | Token: wrong proof, missing proof, late proof, wrong first frame, old token after regeneration. Remote: loopback-only bind, refusal of a peer that is not loopback. | Refused in every automated case (per the test assertions, CI run 43787191028) | PASS (automated) | `src/apps/office-harnessd/tests/access_gate.rs`, tests `a_wrong_proof_is_closed_with_4401_and_nothing_else_is_sent`, `a_missing_proof_is_closed_with_4401_after_the_timeout`, `a_late_proof_is_refused`, `any_other_first_frame_is_closed_with_4401_and_gets_no_answer`, `a_proof_computed_for_another_port_is_refused`, `after_regeneration_the_old_token_fails_and_its_open_sockets_close`, `an_upgrade_with_a_wrong_host_or_any_origin_is_refused`, `the_socket_is_bound_to_loopback_only`, `a_peer_that_is_not_loopback_is_refused_and_a_loopback_peer_is_served`. Limit: these tests prove the loopback bind and the peer check. They do not make a connection from a second machine. |
| SC-006 | macOS | Same attempts by hand, plus one from another machine | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. The `tests/access_gate.rs` tests passed on this OS in CI run 43790227411. |
| SC-006 | Windows | Same attempts by hand, plus one from another machine | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. The `tests/access_gate.rs` tests passed on this OS in CI run 43790227411. |

## SC-007: 0 outside network connections in 1 hour

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-007 | Linux | 1-hour network capture filtered to the office-harness processes | 0 outside connections in 2 h 56 min | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) | `strace -f -e trace=network` on the WebDriver launcher and all its children (the desktop app, the `office-harnessd` server it started, and the fake CLIs) from 19:47 to 22:43, plus a second 6-minute app run. Every IPv4 connect or send address was 127.0.0.1; there was no IPv6 address; the other lines were netlink route reads, which open no connection. 363 `ss -tunap` samples over the same period show no socket of the server, the app, or WebKit to an outside address. Not covered: real agent CLIs (the spec does not count them). Evidence: `hive/agents/celal-mwhvybh8/qa/evidence/0001-SC-007/`. |
| SC-007 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-007 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-008: First use within 2 minutes without editing a file

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-008 | Linux | Timed first use by a participant who has never used office-harness | Not measured | BLOCKED | Needs a human participant. The participant's name goes here when the run happens: (not recorded). |
| SC-008 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |
| SC-008 | Windows | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. |

## SC-009: An additions edit applies to the next session only

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-009 | Linux | Edit flags, environment variables, and the startup prompt of a running agent. Check that the running session keeps the old values and the next session gets the new ones. | Next session has the new values; running session unchanged (per the test assertions, CI run 43787191028) | PASS (automated) | Server: `src/apps/office-harnessd/tests/sessions_portable.rs`, test `flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one`. Webview: `src/apps/desktop/e2e/office.spec.ts`, test `a running agent's additions edit applies to the next session, and another client's save refreshes the editor`. The webview test runs against a mock core, so it covers the screen only. |
| SC-009 | macOS | Same as Linux | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. `tests/sessions_portable.rs` test `flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one` passed on this OS in CI run 43790227411. |
| SC-009 | Windows | Same test as Linux, with fake `.cmd` CLIs. A batch file cannot record typed input, so the test proves the startup prompt by its echo in the terminal output. | Next session has the new values; running session unchanged (per the test assertions, CI run 43787191028) | PASS (automated) | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one`. It uses fake CLIs, not the real ones. |

## Windows stop of an agent

The automated tests found that a stop of a CLI that starts through a `.cmd` file waited the full grace period (`stop_grace_ms`, 10 seconds by default). The server now ends such a stop quickly: it lets its agents react to the interrupt key, it starts an npm shim's interpreter directly, and it answers the "Terminate batch job (Y/N)?" question of `cmd.exe` only when that question appears after the interrupt key. The automated tests start the server inside the test process, so they do not prove the case where the desktop app starts the server with no console. That case is the hands-on row below.

| Check | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| Fast stop | Windows | Start the server from the desktop app. Start an agent with an npm-installed CLI, stop it, and record the stop time. Check that the server log has no warning that the agents cannot react to the interrupt key. Pass means the stop takes less than the 10 second grace. | Not measured | WAIVED (owner) | Waived by owner 2026-11-16, CI evidence on 3 OSes. `tests/sessions_windows.rs` tests `an_npm_shim_starts_its_interpreter_directly_and_a_stop_is_fast` and `a_batch_file_that_is_not_an_npm_shim_is_stopped_by_answering_its_question` passed on Windows with the default grace in CI run 43927333809, with the server inside the test process. |
