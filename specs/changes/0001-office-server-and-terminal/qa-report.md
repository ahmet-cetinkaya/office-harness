# 0001 QA report (draft)

## Summary

- **Date:** 2026-11-16.
- **Build under test:** branch `feat/0001-p5-ship`.
- **Overall status:** `blocked`. The run is complete only when every row is PASS. Most rows below are BLOCKED, so the run is blocked.
- **Status of this file:** a draft. No hands-on run has happened yet. Every measured value is empty until a person runs the check. No value in this file is an estimate.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test that runs in CI proves the criterion on that OS. The evidence column names the test. |
| PENDING (Linux run) | A person can run the check on Linux. Nobody has run it yet. |
| BLOCKED | The check needs a person on that OS, a real CLI account, a restart, a 60-minute wait, a network capture, or a human participant. It is not done. |

### Scope of the automated evidence

- The automated tests use fake CLIs, not the real `claude`, `codex`, `gemini`, or `opencode`.
- A PASS (automated) row shows that the code does what the criterion says. It does not replace a hands-on run with the real surface.
- CI run 43780798038 at commit 264148efcbff1e7c7d2aee5127be0620f97590fc passed on Linux, macOS, and Windows. It ran every named test; `tests/sessions.rs` runs only on Linux and macOS (`#![cfg(unix)]`). The SC-005 and SC-009 tests moved to `tests/sessions_portable.rs`, which also runs on Windows; the Windows rows below wait for a green CI run of that change. A later commit that changes only this file does not change the tested code.

### What a person must run

**On Linux, macOS, and Windows**

- Install the build from the `release` workflow artifact of this branch.
- SC-001: type 1,000 keystrokes into an agent that runs an echo test CLI, and record the time from the keydown in the webview to the render of the echoed character. Report the 95th percentile.
- SC-002: run the four-CLI task script with `claude`, `codex`, `gemini`, and `opencode`. Record any account type that cannot run `gemini`.
- SC-003: start agents, keep the app closed for 60 minutes, then open it and compare the agents and their output.
- SC-004: restart the machine, log in without opening the app, and record the time until the server runs and what the office holds.
- SC-007: capture network traffic for 1 hour, filtered to the office-harness processes.
- SC-008: a participant who has never used office-harness creates an agent and types into its terminal. Time the run and name the participant in the report.

**On macOS and Windows only**

- Every row of SC-006 (there is no hands-on or automated evidence for these OSes yet), and the macOS rows of SC-005 and SC-009. The Windows rows of SC-005 and SC-009 have automated evidence once CI is green.

**On Linux only**

- SC-006, second part: try to connect from another machine and record that the attempt fails.
- SC-005 with the real CLIs: compare the CLI configuration folders before and after a session (optional, because the automated test covers the criterion with fake CLIs).

## SC-001: Typed characters appear within 100 ms (p95)

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-001 | Linux | 1,000 keystrokes with an echo test CLI; p95 of keydown to xterm.js render | Not measured | BLOCKED | No automated latency test exists. Needs a person to run the measurement. Pass means p95 at most 100 ms. |
| SC-001 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. |
| SC-001 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. |

## SC-002: A full task in each of the four CLIs

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-002 | Linux | Four-CLI task script: start, prompt, approve a tool use with the keyboard, Ctrl+C, continue a previous session | Not measured | BLOCKED | Needs the real CLI accounts and a person at the keyboard. `gemini` may not run on every account type; record it. |
| SC-002 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. Needs the real CLI accounts. |
| SC-002 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. Needs the real CLI accounts. |

## SC-003: Agents keep running while the app is closed for 60 minutes

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-003 | Linux | Close the app for 60 minutes, then reopen | Not measured | BLOCKED | Needs a 60-minute wait with real or test agents. The automated test `definitions_and_settings_survive_a_restart` in `src/apps/office-harnessd/tests/startup.rs` covers a server restart, not this criterion. |
| SC-003 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. |
| SC-003 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. |

## SC-004: The server runs within 60 seconds after a machine restart and login

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-004 | Linux | Restart the machine, log in, do not open the app, time the server start, then list the office | Not measured | BLOCKED | Needs a machine restart. The tests `definitions_and_settings_survive_a_restart` and `running_agents_become_interrupted_at_start_and_none_starts_by_itself` in `src/apps/office-harnessd/tests/startup.rs` cover the stored office and the `interrupted` status only, not the login start or the 60-second limit. |
| SC-004 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. |
| SC-004 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. |

## SC-005: 0 files changed in a CLI's configuration

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-005 | Linux | Create, start, continue, edit additions, stop, and delete an agent for each of the four CLI names, with fake CLIs that write nothing. Compare size, modification time, and content hash of the home folder and the XDG folders before and after. | 0 files changed (per the test assertion; confirm in CI run) | PASS (automated) | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration`. The file runs on Linux, macOS, and Windows. It uses fake CLIs, which matches the criterion's CLI that writes nothing itself. |
| SC-005 | macOS | Same as Linux, by hand or with the Unix test | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. The same test file builds for macOS in CI, but nobody has recorded a result for this build. |
| SC-005 | Windows | Same test as Linux, with fake `.cmd` CLIs and a fixed environment: `USERPROFILE`, `APPDATA`, and `LOCALAPPDATA` point into a temporary fake home. | 0 files changed (per the test assertion; confirm in CI run) | PASS (automated) pending CI | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration`. It uses fake CLIs, not the real ones. |

## SC-006: Connections without a valid token are refused, and none from another machine succeeds

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-006 | Linux | Token: wrong proof, missing proof, late proof, wrong first frame, old token after regeneration. Remote: loopback-only bind, refusal of a peer that is not loopback. | Refused in every automated case (per the test assertions; confirm in CI run) | PASS (automated) | `src/apps/office-harnessd/tests/access_gate.rs`, tests `a_wrong_proof_is_closed_with_4401_and_nothing_else_is_sent`, `a_missing_proof_is_closed_with_4401_after_the_timeout`, `a_late_proof_is_refused`, `any_other_first_frame_is_closed_with_4401_and_gets_no_answer`, `a_proof_computed_for_another_port_is_refused`, `after_regeneration_the_old_token_fails_and_its_open_sockets_close`, `an_upgrade_with_a_wrong_host_or_any_origin_is_refused`, `the_socket_is_bound_to_loopback_only`, `a_peer_that_is_not_loopback_is_refused_and_a_loopback_peer_is_served`. Limit: these tests prove the loopback bind and the peer check. They do not make a connection from a second machine. |
| SC-006 | macOS | Same attempts by hand, plus one from another machine | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. CI builds `access_gate.rs` on macOS, but nobody has recorded a result for this build. |
| SC-006 | Windows | Same attempts by hand, plus one from another machine | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. CI builds `access_gate.rs` on Windows, but nobody has recorded a result for this build. |

## SC-007: 0 outside network connections in 1 hour

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-007 | Linux | 1-hour network capture filtered to the office-harness processes | Not measured | BLOCKED | Needs a 1-hour capture and a person to read it. No automated test covers this. |
| SC-007 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. |
| SC-007 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. |

## SC-008: First use within 2 minutes without editing a file

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-008 | Linux | Timed first use by a participant who has never used office-harness | Not measured | BLOCKED | Needs a human participant. The participant's name goes here when the run happens: (not recorded). |
| SC-008 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. Needs a human participant. |
| SC-008 | Windows | Same as Linux | Not measured | BLOCKED | Needs a person on Windows with the installer from the `release` workflow. Needs a human participant. |

## SC-009: An additions edit applies to the next session only

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-009 | Linux | Edit flags, environment variables, and the startup prompt of a running agent. Check that the running session keeps the old values and the next session gets the new ones. | Next session has the new values; running session unchanged (per the test assertions; confirm in CI run) | PASS (automated) | Server: `src/apps/office-harnessd/tests/sessions_portable.rs`, test `flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one`. Webview: `src/apps/desktop/e2e/office.spec.ts`, test `a running agent's additions edit applies to the next session, and another client's save refreshes the editor`. The webview test runs against a mock core, so it covers the screen only. |
| SC-009 | macOS | Same as Linux | Not measured | BLOCKED | Needs a person on macOS with the installer from the `release` workflow. |
| SC-009 | Windows | Same test as Linux, with fake `.cmd` CLIs. A batch file cannot record typed input, so the test proves the startup prompt by its echo in the terminal output. | Next session has the new values; running session unchanged (per the test assertions; confirm in CI run) | PASS (automated) pending CI | `src/apps/office-harnessd/tests/sessions_portable.rs`, test `flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one`. It uses fake CLIs, not the real ones. |

