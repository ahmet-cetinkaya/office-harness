---
status: completed
spec: specs/changes/archive/0019-cli-sign-in-check/SPEC.md
commits: per-task
---

# Plan: Check that the CLI is signed in before a start (0019)

## Summary

Each CLI adapter names its read-only status command and its sign-in words. Before a start request takes the status order, the agent gateway runs the status command through a new port, with the agent's program, folder, login environment, and additions, without the agent-call credential, for at most 5 seconds; `SessionLauncher::prepare` then uses that answer when the agent did not change meanwhile. A not-signed-in answer makes the start a "never started" start with one reason; every other answer lets the start go on. After the CLI starts, a watcher reads the bottom of the screen for 20 seconds; when it finds the CLI's sign-in words and nobody typed, the office ends the CLI and the session ends as not signed in.

**What you get:**
- `CliAdapter::sign_in` (status command and words) for `claude` and `codex`, sign-in screen words for `claude`, `codex`, and `gemini`.
- A port `StatusCommandRunner` and its process runner: no input, no terminal, a process group, 64 KiB cap, 5 s deadline, the whole group ended at the deadline.
- The answer and reason rules as pure functions in the agent application, with the visible form moved to the kernel so both modules share it.
- A sign-in watch next to the startup-prompt typing in the server, and one office step that ends the CLI as not signed in.
- A session end result `not-signed-in: <reason>` and its label in the desktop session list.

**Why this approach:**
- Every start (start, new task, continue, agent-made start, bulk resume) goes through `SessionLauncher::prepare`, so one call covers FR-001 for all of them, and a refusal there changes nothing yet (prepare runs before a running session ends).
- The check runs before `TerminalHost::prepare`, which issues the agent-call credential, so a refused start issues no credential and the status command never sees one.
- A not-signed-in start reuses the "never started" path (`Readiness::NotStartable`), so the session list, the action record failure, the `failed` status, and the 0013 "did not start" rule (PB-4) work without new code.

**What it will not do:** sign in, read CLI files, check a key, reuse answers, re-check during a session, or offer settings (spec Out of scope).

**Shape of the work:** 5 tasks in one PR (`feat/0019-cli-sign-in-check`), one commit per task. **Effort:** M.

**Main risks:**
- **A hanging status command delays other starts and stops.** The check runs before the status order is taken (D13), so nothing waits for it; SC-008 tests a stop during a hanging check.
- **A working CLI refused by a check that cannot see its sign-in.** No check runs with own arguments, flags, or a sign-in key variable (D14).
- **A false match ends a working agent.** The watch reads only the bottom 15 rows, only for 20 s, only for the four CLIs, and stops at the person's first key (A5); the reason quotes the row.
- **A process tree outlives the deadline.** Unix: own process group and `killpg`. Windows: `taskkill /T /F /PID`, as the terminal module ends a tree. SC-002 checks that nothing remains.

**Blockers:** none.

## Goal

Deliver spec 0019: a start of a CLI that is not signed in ends at once with one clear reason; a slow, missing, or unknown status command never blocks or refuses a start; a sign-in screen in the first 20 seconds ends the start with the same kind of reason; no CLI output shows except one capped line in visible form.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 6e2a8bd on 2026-11-23.

- **Starts:** `SessionLauncher::prepare` (`M/agent/application/src/common/session_launcher.rs:122`) checks the folder and the additions, then `build_launch` calls `TerminalHost::prepare` (which issues the credential, `H/bridges/terminal_host.rs:98`) and the adapter. `Readiness::NotStartable(text)` stores a session that never ran (`ExitResult::SpawnError`), sets `failed`, and returns `StartOutcome.failure`. Callers: `StartAgentHandler`, `StartNewTaskHandler`, `ContinueSessionHandler`, `ResumeAgentHandler`; each records the action with `ActionJournal::record` or `record_detailed` (`M/agent/application/src/common/action_journal.rs`).
- **Adapters:** `CliAdapter` (`M/agent/application/src/common/interfaces/cli_adapter.rs`), registry by program stem (`M/agent/infrastructure/src/services/cli_adapters/registry.rs`).
- **Process runner pattern:** `run_login_shell` (`M/agent/infrastructure/src/services/login_shell.rs`, Unix only): null input, piped output read on a thread, `process_group(0)`, `killpg` at the deadline. Windows tree end: `M/terminal/infrastructure/src/services/process_tree.rs:90` (`taskkill /T`).
- **Login environment and program:** `TerminalLauncher::prepare` (`H/bridges/terminal_launcher.rs`) resolves the program and builds the login environment with the additions, without the credential.
- **Typed startup prompt:** `type_after_first_output` (`H/bridges/typed_input.rs`) waits for `FirstOutput` of the run on `TerminalEvents`; `TerminalRuntime::screen` gives a `ScreenView` (`M/terminal/domain/src/value_objects/screen_view.rs`, `bottom_rows`). `PersonInput::stamp` (`H/bridges/person_input.rs`) is called on every key of a person.
- **Visible form:** `visible_text` (`M/hierarchy/domain/src/services/visible_text.rs`), used by 0014 and 0015.
- **Status order:** `AgentGateway::run` (`H/server/agent_requests.rs:153`) holds `status_order` for start and stop requests; `AgentStatusBridge` (`H/bridges/agent_status_bridge.rs`) ends a session on `Exited` and publishes `agent.status`, which the notification bridge turns into a `failed` notification.
- **Desktop:** `D/modules/agent/components/session-list.svelte` (`endText`); the action record shows `failure` and `detail` (`D/modules/audit/components/action-record.svelte`); e2e mock core `src/apps/desktop/e2e/support/mock-core.ts`.

## Decisions

- **D1 Adapter knowledge.** `CliAdapter` gains `fn sign_in(&self) -> SignIn` with default `SignIn::default()`. `SignIn { status: Option<StatusCommand>, screen_words: &'static [&'static str] }`, `StatusCommand { arguments: &'static [&'static str], not_signed_in_words: &'static [&'static str] }`. Values: `claude` status `["auth", "status", "--text"]`, words `["not logged in"]`, screen words `["claude account with subscription", "anthropic console account", "please run /login", "run /login"]`; `codex` status `["login", "status"]`, words `["not logged in"]`, screen words `["sign in with chatgpt", "provide your own api key"]`; `gemini` screen words `["how would you like to authenticate", "waiting for authentication"]`; `opencode` and generic: none (research R1, R4). The CLI's display name is the program stem, lower case (already how the registry picks).
- **D2 Port and answer.** New port `StatusCommandRunner::run(&StatusRun) -> StatusRunResult` in `M/agent/application/src/common/interfaces/`. `StatusRun { program: String, arguments: Vec<String>, working_folder: PathBuf, environment_additions: Vec<(String, String)>, timeout: Duration }` (the unresolved program and the additions, as `PrepareLaunch` takes them). `StatusRunResult::{Ended { exit_code: i32, output: String }, TimedOut, CouldNotRun(String)}`; `output` is stdout then stderr, lossy UTF-8, at most 64 KiB. Pure rules in `M/agent/application/src/common/sign_in_check.rs`: `answer(&StatusRunResult, &StatusCommand) -> SignInAnswer::{SignedIn, NotSignedIn { line }, NoAnswer(NoAnswerReason::{TimedOut, CouldNotRun, Unclear})}` (exit 0 = signed in; non-zero AND a word, lower case = not signed in; else unclear). `line` = the first non-empty line that holds a word, in visible form, cut to 200 characters (`char` count).
- **D3 Where the answer is used.** In `SessionLauncher::prepare`, after `existing_working_folder` and `used_additions`, before `build_launch`: the launcher builds the planned `StatusRun` (D14 may skip it) and takes the answer that D13 left for this agent. A planned run equal to the checked one uses its decision; a different one gives no answer with the note "the agent changed during the check"; no left answer (a start that did not pass the gateway) runs the check in place. `NotSignedIn` gives `Readiness::NotSignedIn(reason)` and skips `build_launch` (no credential issued); `start` treats it as `NotStartable`. `NoAnswer` sets `PreparedStart.sign_in_note`, which `start` copies into a new `StartOutcome.sign_in_note: Option<String>`. Timeout constant `SIGN_IN_TIMEOUT = 5 s`, passed through `LauncherServices`.
- **D4 Reasons (FR-007).** `not_signed_in_reason(cli, Found::StatusCommand(args) | Found::Screen, line)`: `"<cli> is not signed in: <how>. Sign in to <cli> in your own terminal, then start the agent again. It showed: <line>"`, where `<how>` is `` `<cli> <args>` said so `` or `its first screen asks to sign in`. Notes: `"the sign-in check gave no answer: <timed out after 5 s | could not run: <error, visible form, 200 chars> | unclear answer>"`; the note never holds output (FR-008).
- **D5 Recording the note.** The four start handlers pass `outcome.sign_in_note` as the entry's `detail` through `record_detailed`; `ResumeAgentHandler`, which already has a `detail` (its reason), joins both with `"; "`. The answer of a start does not carry the note.
- **D6 Process runner.** `M/agent/infrastructure/src/services/status_command.rs`, cross-platform, implementing the run part: `Command::new(resolved program).args(...).current_dir(folder).env_clear().envs(environment)`, null stdin, piped stdout and stderr read on two threads into one shared buffer capped at 64 KiB (reading goes on, so a full pipe never blocks the child, but bytes past the cap are dropped). Unix: `process_group(0)`, `killpg(SIGKILL)` at the deadline. Windows: `CREATE_NO_WINDOW`, `taskkill /T /F /PID` at the deadline, then `kill`. The wait polls `try_wait` every 10 ms until the deadline. The output is never logged.
- **D7 Program and environment.** `H/bridges/status_command_bridge.rs` implements `StatusCommandRunner`: `TerminalLauncher::prepare(program, folder, additions)` gives the resolved program and the login environment (without the credential, which only `TerminalHostBridge::prepare` adds); it then drops every variable with the reserved prefix `OFFICE_HARNESS_` and calls the runner of D6. A `prepare` failure gives `CouldNotRun`. A Windows batch program runs through `std::process::Command`, which applies its own batch quoting to the fixed arguments.
- **D8 Visible form shared.** Move `visible_text` to `src/kernel/src/visible_text.rs` (`kernel::visible_text`); `hierarchy_domain` keeps `pub use kernel::visible_text` so its callers do not change. Tests move with it.
- **D9 Sign-in watch.** `StartTerminal` gains `sign_in: SignIn` (from the adapter; empty words for the others). After a spawn, `TerminalHostBridge::start` calls `SignInWatch::start` (`H/bridges/sign_in_watch.rs`) unless the words are empty or the startup prompt holds a word (FR-011). A thread polls every 250 ms until 20 s after the start: it stops when the run changed or ended, when `PersonInput::typed_since(agent, start)` (a person's key) or `PersonInput::office_typed_since(agent, start)` (a directive: `DirectiveWriterBridge::deliver` stamps it before its paste) is true; else it reads `runtime.screen`, and when one of `bottom_rows(15)` holds a word (lower case), calls the office step of D10 with the quoted row. Period and poll are a `LoginSetup` knob (`SignInWatchTiming`) for tests.
- **D10 Office step.** No new request, so no client or agent can reach it. The watch marks the run in a shared `SignInEnds` record (agent, run, reason), then stops the child through the terminal runtime (grace 2 s; a sign-in screen holds no work). When the run's `Exited` event reaches `AgentStatusBridge::apply` (under the status order), the bridge takes the mark for exactly that run: it ends the session with the new `ExitResult::NotSignedIn(reason)` (`not-signed-in: <reason>`), applies `ExitedWithFailure` with the reason as the event detail, and records `stop` with the reason as failure under `ActorScope::enter(Actor::Office)`. The published `agent.status` `failed` makes the one `failed` notification (FR-005). A run without a mark ends as before.
- **D11 Desktop.** `endText` maps `not-signed-in: <reason>` to `not signed in. <reason>` and `spawn-error: <text>` to `never started: <text>`. No other UI change; the start answer's failure already shows as the notice.
- **D12 Contracts and docs.** `contracts/sign-in-protocol.md` in this change folder: the new `exit_result` form, the action record `detail` note, the refusal of a new task or continue on a running agent (0001 is archived, so its contract is not edited). README: a section on the check. Research and qa-report as in T5.
- **D13 Check before the order (FR-009, M1).** New `SignInPrecheck` in the agent application (`M/agent/application/src/common/sign_in_precheck.rs`) with the stores, the adapters, the runner, and the timeout. `AgentGateway::run` calls `precheck.before(request)` before it takes `status_order`, for `agent.start`, `agent.new_task`, `agent.resume` (field `agent_id`) and `session.continue` (field `session_id`, read to its agent). It runs the check and keeps `(agent_id, planned StatusRun, decision)` in a thread-local scope (`SignInScope`, the pattern of `ActorScope`), cleared when the guard drops at the end of the request. The launcher reads it on the same blocking thread (D3). Agent-made starts and bulk resume starts pass `AgentGateway::run`, so they are prechecked too. A failure to read the agent leaves no answer, and the launcher checks in place.
- **D14 Skip rules (FR-010, M2).** The planned run is `None`, so no check runs and no note is written, when the agent's command has fixed arguments, the additions hold flags, or an addition names one of the adapter's `StatusCommand::key_variables` (A11). The bridge of D7 also returns `StatusRunResult::Skipped` when the login environment sets one of them (non-empty); the launcher treats it as no check. Names only, never values.
- **D15 Running agents (FR-002, M4).** `SessionLauncher::prepare_replacing(agent, kind)`: `prepare`, then, when the agent runs and the readiness is `NotSignedIn(reason)`, `Err(ApplicationError::Refused(reason))`. `StartNewTaskHandler` and `ContinueSessionHandler` call it instead of `prepare`, so the running session is never ended for a start that cannot run; the journal records the refusal as the failure.

## Scope

- In: D1 to D12, unit and integration tests, one e2e, docs, the QA report.
- Out: everything under the spec's Out of scope.

## Tasks

### T1 Kernel visible form and agent rules (D1, D2, D4, D8)
- Move `visible_text` to the kernel with a re-export; add `SignIn`, `StatusCommand`, the port, `sign_in_check.rs` with `answer`, the reason and note texts; `CliAdapter::sign_in` default.
- Tests: answer table (exit 0 with words = signed in; exit 1 with words = not signed in; exit 2 usage text = unclear; empty output; timed out; could not run), the line rule (first matching line, visible form of C0, DEL, C1, and bidi, 200-char cut on a multibyte line), the reason and note texts hold no other output.

### T2 Launcher, handlers, adapters, runner (D1, D3, D5, D6) - depends on T1
- `SessionLauncher` gains the runner and the timeout; `StartOutcome.sign_in_note`; four handlers record the note; adapter values; `status_command.rs` with Unix and Windows tree ends.
- Tests (application, with a fake runner in `test_support`): not signed in on start, new task, continue, and resume of a stopped agent gives `failed`, a never-started session with the reason, a failure entry, and 0 terminal starts; on a running agent a new task and a continue are refused with the reason and nothing is stopped (D15); the credential port is not called (`TerminalHost::prepare` count 0); no answer gives a start and a `detail` note; no status command, fixed arguments, flags, or a key-variable addition give 0 runner calls (D14); a prechecked answer is used without a second run, and a changed command gives the "changed" note (D13). Runner tests also cover a Windows `.cmd` stand-in that starts a child, ended at the deadline (L1). Runner tests (`#[cfg(unix)]` stand-in scripts, and a portable test that runs the test binary itself as a child that sleeps): exit and output capture, 64 KiB cap with 1 MiB output, deadline ends a child and its grandchild (check by PID after 1 s), stdin is null.

### T3 Server: bridge, watch, office step (D7, D9, D10) - depends on T2
- `status_command_bridge.rs`, `sign_in_watch.rs`, `PersonInput::typed_since`, `EndAtSignInScreenHandler`, `ExitResult::NotSignedIn`, wiring in bootstrap, `ServerOptions` knobs.
- Tests (`tests/sign_in_check.rs`, fake CLIs named `claude`, `codex`, `gemini` on `PATH`, Unix shell scripts): SC-001 (12 starts, including an agent-made start whose answer holds the reason), SC-002 (hanging status command, start within 6 s, no process left), SC-003 (screen words, end within 3 s, one record with actor office, one notification), SC-004 (person typed, or words after the period: not ended; a key typed into another agent does not stop this watch, L1), SC-005 (10 KiB with controls, bidi, and a key-shaped line: nothing in answers, record, sessions, server log), SC-006 (temp home unchanged after a check), SC-008 (a stop of another agent during a hanging check ends within 1 s), SC-009 (own arguments, flags, key variable in additions and in the login environment: started, 0 checks), SC-010 (words only in the startup prompt or in a directive: not ended), the credential is absent from the status command's environment, and a bulk resume of a not-signed-in agent ends on hold with the reason (PB-4).

### T4 Desktop (D11) - depends on T3 for the contract only
- `endText` labels; unit test; e2e: mock core returns a failed start whose session ended `not-signed-in: ...`, the details show "not signed in." and the reason, and the action record entry shows the failure; axe scan of the details as in the existing agent e2e.

### T5 Docs and evidence - depends on T1 to T4
- Panel protocol notes, README line, SPEC status, `qa-report.md` (every SC with its test name; SC-007 by hand with real `claude` and `codex`, signed in and not signed in through a temporary configuration folder in the office additions, never by signing the user's own account out).

## Verification

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: `bun run check`, `bun run lint`, `bun run format:check`, `bun run test`, Playwright.
- The repository's boundary check script.
- CI green on Linux, macOS, and Windows.
- Hands-on SC-007 on Linux. A row not run stays NOT RUN, never PASS.
