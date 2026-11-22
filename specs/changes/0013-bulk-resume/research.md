# Research: Resume all interrupted agents (0013)

Date: 2026-11-23. Repository: `main` at 6e3d197.

## R1: Which CLI processes outlive a SIGKILL of the server, and does a continue work afterwards?

**Question.** 0001/FR-004 marks running agents `interrupted` after an unclean stop, but nothing showed what happens to the CLI processes themselves. A CLI that keeps running after the server is gone spends quota unseen and can keep its session busy, which can make a later continue fail.

**Method.** The test `src/apps/office-harnessd/tests/server_crash.rs` (Linux only, because it reads process start times from `/proc`):

1. Starts the real server binary with a stand-in login shell and three fake CLIs:
   - `ignores-hangup`: ignores SIGHUP (`trap '' HUP`) and loops.
   - `ends-on-hangup`: runs `sleep`, which ends on SIGHUP.
   - `claude`: a stand-in that records its arguments and then runs `sleep`.
2. Starts one agent on each and records each CLI's process id and start time.
3. Kills the server with SIGKILL and waits 3 seconds.
4. Checks each process by process id and start time, so a reused process id does not count.
5. Starts the server again, checks the statuses, and continues the claude stand-in's session with `session.continue`.

**Result** (run on 2026-11-23, Linux, test passes in 3.5 s):

| Check | Result |
|---|---|
| CLI that ignores SIGHUP, after the SIGKILL | Still runs: an orphan. |
| CLI that ends on SIGHUP, after the SIGKILL | Gone. |
| claude stand-in, after the SIGKILL | Gone. |
| Statuses after the next server start | All three agents are `interrupted` (0001/FR-004). |
| The orphan after the next server start | Still runs; the server does not see it. |
| `session.continue` of the claude stand-in | Works; the CLI starts with `--resume <id>`. |

**Why.** The terminal library starts each CLI as the leader of its own session, with the terminal as its controlling terminal. When the server process dies, the kernel closes the terminal's server side and sends SIGHUP to that session leader. A CLI that keeps the default action of SIGHUP ends; a CLI that ignores or handles SIGHUP keeps running, and so do the processes it started outside its foreground group.

**Not measured.** The real `claude`, `codex`, `gemini`, and `opencode` CLIs were not run in this test (no sign-in or quota use in an automated test). Whether a given real CLI handles SIGHUP is therefore unknown. The test proves the mechanism with stand-ins.

**Decision for this change.** Orphans can survive, so a sweep is worth a later change. It is not part of this change: a safe sweep stores each session's process id, start time, and command, reads them back on Linux, macOS, and Windows in three different ways, and ends only the processes whose three values still match. That is more than this small change should carry. This change does not depend on it: an orphan that blocks a continue makes the continue fail, and the new-session fallback (FR-004) applies (spec A5).

## R2: How does a CLI refuse to continue a session?

**Known case.** During hands-on QA of 0001, `opencode` rejected a continue with a session id in UUID form; it accepts only ids that start with `ses`. Fixed on `main` by starting `opencode` with an id in that shape (commit 4068b4d).

**Not verified.** How each CLI behaves when it cannot continue a session (ends with an error, or keeps running and shows the error) was not checked for every CLI. The spec therefore treats only the end of the process as a refusal (spec A3, A4), which needs no knowledge of a CLI's messages.
