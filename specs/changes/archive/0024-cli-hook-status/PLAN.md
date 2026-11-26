---
status: completed
spec: specs/changes/archive/0024-cli-hook-status/SPEC.md
commits: per-task
---

# Plan: Exact agent status from CLI hooks, `claude` first (0024)

## Summary

The server learns to take reports from `claude` and to let them drive the status of that one session. A report moves the existing status machine; the quiet monitor of the terminal keeps running underneath as the fallback.

**What you get:**
- A report endpoint on the server's loopback listener, with a per-session secret, a size cap, and a rate cap. It keeps one event kind and one flag per report.
- A `HookSessions` record in the server. It holds, per running session, the status source (`estimated`, `exact`, `late`), whether a turn is open, whether a permission ask is open, and whether a compaction runs.
- The status bridge asks `HookSessions` before it applies a quiet-period or output event. In an exact session those two events are dropped, and the reports of the CLI apply the same two status events instead. The status machine does not change.
- A launch step for `claude` that writes the office settings file, adds `--settings <file>`, and puts the secret in the session environment. The file and the secret go at every session end, and the server sweeps old files at start.
- A new Needs you event, `permission_asked`, and two detail words on the waiting event, so items and notifications say what happened.
- The directive gate and the idle sleep gate read the new state. A new setting `hook_status` (on by default) and a few words in the desktop app complete it.

**Why this approach:**
- One status machine stays the only writer of status (0022 PB-5 style). Reports are one more source of the two events it already knows: `Output` (to `working`) and `QuietPeriodElapsed` (to `waiting for input`). Every consumer (notification bridge, Needs you, sleep, the 0014 subtree read, the office scene) stays unchanged.
- The secret is a new, narrow credential in the access module, not the office token and not the agent call token. It can do one thing: deliver a report for its own session.
- The hook is an `http` hook. It needs no helper program, runs the same on the three systems, and sends no command line through a shell. The probe on the installed `claude` 2.1.322 (below) shows that it fires through `--settings`, that `$VAR` in a header resolves from the session environment, and that the payload carries the prompt text, which FR-009 drops.
- The fallback is the quiet monitor that already exists: it is never switched off, only ignored while the session is exact.

**Delivery:** one PR (#142, with the spec), 7 tasks, committed per task.

## Goal

Ship every requirement of spec 0024 with automated evidence. Then run the hands-on checks with a real signed-in `claude` for SC-001, SC-002, SC-003, SC-006, SC-007, and SC-011.

## Context

- Status path: `terminal_session.rs:425-470` (quiet monitor), `agent_status_bridge.rs:62-176` (`apply`, `status_event`), `status_machine.rs:18-22`. The bridge holds `StatusOrder` while it applies an event.
- Consumers of the published `agent.status` frame: `notification_bridge.rs:147-195` (`apply_frame`, `judge`, `status_change`, `attention_detail`), `idle_sleep_gate.rs`, `directive_bridges.rs:85-89` (`screen_is_clear`).
- Waits: `wait_tracker.rs` (`classify`, `session_started`, `session_ended`), `person_input.rs` (`stamp`, called from `terminal_requests.rs`).
- Launch: `session_launcher.rs:281-330` (`build_launch`), `terminal_host.rs:110-203` (`prepare`, `start`), `claude.rs` (adapter), `command_builder.rs`.
- Server: `router.rs` (axum, `require_loopback_peer`), `running_server.rs:223-244` (loopback bind, port), `handshake.rs`. Credentials: `memory_agent_call_store.rs`, `agent_authentication_handler.rs`. Private files: `src/shared/private-fs`.
- Settings precedent: the on or off setting `worktree_dependency_links` (0021 D10): `settings.rs`, `worktree-dependencies` module of the desktop, `e2e/worktree-dependencies.spec.ts`.
- Probe (2026-11-23, `claude` 2.1.322, `-p`, a throwaway listener on loopback): `--settings <file>` with `http` hooks fired `UserPromptSubmit` and `Stop` with the header `Authorization: Bearer <value of $PROBE_SECRET>`; the payload keys include `hook_event_name`, `session_id`, `prompt`, `last_assistant_message`, and `agent_type`. The listener answered 200 with an empty body and the CLI went on. This does not prove interactive mode, the trust prompt, `PermissionRequest`, `PreCompact`, or Esc; T7 checks those hands-on (A1).
- Tests: `tests/needs_you.rs`, `tests/idle_sleep.rs`, `tests/support/fake_cli.rs` and `running_office.rs`, the unit tests beside each source (`*.tests.rs`), and `e2e/*.spec.ts` with `support/mock-core.ts`.

## Decisions

- **D1: The report endpoint.** `POST /v1/hook/{session_id}` on the existing listener, behind `require_loopback_peer`.
  - It answers only an empty `200` for an accepted report and one empty `404` for every refusal (unknown session, wrong secret, ended session). The answer has no body, so the CLI reads no decision (FR-005).
  - Extra refusals, all `404` and logged without the secret: a request with an `Origin` header (a browser), a content type that is not JSON, a body over 64 KiB (`DefaultBodyLimit`), and a session over 50 reports per second (a per-session token bucket in memory).
  - Rate-limit state exists only for sessions that presented a valid secret, and it is dropped with the secret at the session end, so a flood of bad reports grows no state. Refusals are counted, not listed: the log gets one line per minute with the count. No log line holds a body, a parse error text, or a secret (Efe r1 L3).
  - Secret check: header `Authorization: Bearer <secret>`, compared in constant time. The URL carries the session id (the office session id), never the secret.
  - Body: parsed as JSON into a struct with only `hook_event_name` and an optional `agent_id`. Every other key is dropped by the parser and never stored or logged (FR-009). A body that is not JSON, or an unknown `hook_event_name`, is a `200` with no effect (FR-022).
- **D2: The secret.** A port `HookSecretStore` in the access module (`issue(agent_id, session_id) -> HookSecret`, `authorize(session_id, presented) -> Option<AgentId>`, `revoke_session`, `revoke_agent`) with an in-memory implementation next to `memory_agent_call_store.rs`. 32 random bytes from the OS source, hex text. It is issued in `TerminalHostBridge::prepare`, next to the agent call credential, and revoked at the same three places (exit in `agent_status_bridge.rs`, prepare failure, spawn failure). It is held in memory only and is never logged: the type has no `Debug` and no `Display`.
- **D3: What the CLI gets.** `TerminalHostBridge::prepare` gains the work when the request says `status_hooks`:
  - It writes `<data folder>/hooks/<session_id>.json` with `private-fs` (owner only; the folder is created with `create_private_dir_all`). The file holds only `hooks` for `UserPromptSubmit`, `Stop`, `StopFailure`, `PermissionRequest`, `PreCompact`, and `PostCompact`. Each entry is one `http` hook: the URL of D1 with the server's port, the header `Authorization: Bearer $OFFICE_HARNESS_HOOK_SECRET`, `allowedEnvVars` with that one name, and `timeout: 2` (FR-006).
  - It adds the environment variable `OFFICE_HARNESS_HOOK_SECRET` to the local list of variables that `prepare` passes to the launcher, the list that already carries the agent call credential. That list becomes the spawn environment only. The secret never enters `OfficeAdditions`, so it is not in the `additions_snapshot` of the session row or in any other stored row (Efe r1 M3). The file holds no secret.
  - It returns `extra_arguments: ["--settings", "<path>"]` in `PreparedLaunch`; `build_launch` appends them to the adapter's arguments before `validate_arguments`.
  - When it cannot write the file it returns no arguments and a reason, and the session starts estimated (edge case "settings file cannot be written"). The reason travels to the hook state (D6).
- **D4: When the file is made.** `build_launch` sets `status_hooks` only when all hold: the adapter says `status_hooks()` (a new default-false method of `CliAdapter`, true for `ClaudeAdapter`), the setting `hook_status` is on, no argument of the command line or of the office flags is `--settings` or starts with `--settings=` (FR-004), and the version check passes. The reason of each refusal is one text for the details (D6).
- **D5: Version check.** The host runs `<program> --version` once per program path, with a 5 second limit, caches the answer for the server's life, and parses the first `major.minor.patch`. A constant `MIN_HOOK_VERSION` holds the first version with `http` hooks, fixed by T7 from the CLI's changelog and the hands-on run; 2.1.322 is known to work. An unreadable or lower version means no file and the reason `the CLI version does not support status hooks`. The check runs in `prepare`, which already runs on a blocking thread.
- **D6: HookSessions.** A new `bridges/hook_sessions.rs`, shared by `Arc` like `WaitTracker`. Per agent: `source`, `reason` (for `estimated`), `run`, `turn_open`, `ask_open`, `compacting`, `last_report`.
  - `session_started(agent, run, reason: Option<String>)` makes an `estimated` record (called beside `wait_tracker.session_started`, from `terminal_host.rs`); `session_ended(agent)` removes it (called at exit, beside `waits.session_ended`).
  - `apply_report(agent, run, kind) -> HookDecision` updates the state and returns the `StatusEvent` to apply and the frame detail. The first accepted report makes the session `exact`; a report of a `late` session makes it `exact` again.
  - `filter_terminal_event(agent, TerminalEvent) -> Pass | Drop`: for `exact`, `OutputResumed` and `QuietPeriod` are dropped, except a `QuietPeriod` with `(turn_open || compacting) && !ask_open`, which sets `late` and passes (FR-014). For `late` and `estimated`, all pass. `FirstOutput`, `Exited`, `Bell`, and `OutputStorageFailed` always pass.
  - It publishes `hook.changed` (D9) when `source`, `ask_open`, or `compacting` changes.
- **D7: Applying a report.** The report handler (server layer) calls `HookStatusBridge::apply_report`, which takes `StatusOrder`, checks that the run is still current (`runtime.current_run`), asks `HookSessions`, and runs the same `change_status.handle` and `hub.publish` as `AgentStatusBridge::apply`. Event mapping:

  | Report | Status event | Frame detail |
  |---|---|---|
  | `UserPromptSubmit` (main) | `Output` (waiting to working) | none |
  | `Stop` (main) | `QuietPeriodElapsed` | `turn_stopped` |
  | `StopFailure` (main) | `QuietPeriodElapsed` | `turn_failed` |
  | `PermissionRequest` (main) | `QuietPeriodElapsed` | `permission_asked` |
  | `PreCompact` | `Output` | none |
  | `PostCompact` | `QuietPeriodElapsed` if no turn is open, else none | `turn_stopped` |

  A report with `agent_id` set is from a helper sub-agent and changes nothing (FR-023). The ask ends (`ask_open = false`, `Output` if a turn is open) at a person input (D8), `Stop`, `StopFailure`, `UserPromptSubmit`, `PreCompact`, `PostCompact`, and the end of the session. When the status machine ignores an event (the status already is the target) no frame goes out; for `PermissionRequest` the bridge then calls `NotificationBridge::record_permission_ask` itself, so the item and the notification still appear (D10).
- **D8: Person input and waits.** Every change of an exact session is prompted: `WaitTracker::set_exact(agent, bool)` makes `classify` return `Prompted` and keeps `ended_since_wait` cleared, and the work bound timer is not armed (`classify` never returns an unprompted change, so `arm_work_bound` does not run). `HookSessions::person_input(agent)` is called from the same call site as `PersonInput::stamp`; it ends an open ask. The `late` state calls `set_exact(false)`, so the 0022 rules apply again.
- **D9: Wire and storage.** No migration for the hook state (A9). One wire event, `hook.changed` `{agent_id, source, reason?, ask_open, compacting}`, and one query, `hook.states`, which returns the same for every running agent. The `agent` JSON does not change. The new Needs you kind `permission_asked` is added to `NotificationEvent` (`ALL` goes from 9 to 10), the stored and wire name `permission_asked`, and to the migrations only if a column constraint lists the kinds (T4 checks `0001-0004`; if so one new migration widens it). The waiting kind keeps its detail column, now `turn_stopped`, `turn_failed`, or the quiet period in ms.
- **D10: Notification and Needs you.** In `apply_frame`, a `waiting_for_input` frame with detail `permission_asked` records `NotificationEvent::PermissionAsked`; with detail `turn_stopped` or `turn_failed` it records `WaitingForInput` with that detail in place of the quiet period (`attention_detail`). `permission_asked` is not a waits-only kind, so an item with it is not plain for 0018 (FR-017) with no change to the sleep rule of 0022. It sorts in the `waiting for input` group (0008/FR-003).
- **D11: Gates.** `DirectiveTargetBridge` readiness adds `!hooks.blocks_typing(agent)` (`ask_open || compacting`) to `screen_is_clear`'s caller; the screen check stays (PB-4). `IdleSleepGate::check` and the check right before the stop add `!hooks.ask_open(agent)` as a blocking condition (FR-017), also with no item. `keeps_from_idle_end` (the 0020 temp idle end, `temp_run.rs`) adds the same condition (FR-025, Efe r1 M2).
- **D12: The setting.** `hook_status` joins `settings.rs` as an on or off text with default `on`, validated like `worktree_dependency_links`. `build_launch` reads it for each session start (FR-024); a running session is never changed.
- **D13: Cleanup (FR-010).** Exit of the child deletes the file and revokes the secret in `agent_status_bridge.rs` (stop, sleep, and the server's stop end in an exit); a failed prepare or spawn deletes it in `terminal_host.rs`. At start, before any agent starts, the bootstrap deletes every file in `<data folder>/hooks/` (no session runs yet). A failed delete is logged with the path and no secret and does not stop the server.
- **D14: Desktop.** A `hook-status` module like `worktree-dependencies`: the setting row, its help text, and its controller. The agent list and details read `hook.states`: status text gets `asks permission` or `compacting` next to it, and the details get `Status source: exact | estimated | late` with the reason text. `eventText` gets `permission_asked` ("asks permission") and the two waiting details ("finished its turn", "its turn ended with an error"); the desktop still shows the server's detail as sent for the other kinds and writes the words from the kind or the three known details.
- **D15: Tests with no real CLI.** A test helper in `tests/support` posts reports to a running test server with the right header, so each state change is driven the way the CLI does it. A fake CLI that prints a hint line on its own (`fake_cli.rs`) covers the redraw case. Real `claude` runs are hands-on (T7).

## Scope

In: the server (access, bridges, router, launch, settings), the wire additions, the desktop setting and texts, tests, docs, and the QA report. Out: tool events, other CLIs, spend limits, sub-agent detail, and a history of reports (spec Out of scope).

## Tasks

Each task commits with its own tests. Before a heavy build: `free -g` at least 6, at most 3 cargo, rustc, or playwright jobs, `RUSTFLAGS='-Clinker-features=-lld'`, `CARGO_BUILD_JOBS=4`.

### T1: Secret store and report endpoint (D1, D2)
- `HookSecretStore` and its memory implementation with unit tests: issue, authorize (right, wrong, other session, revoked), constant-time path.
- The route, the refusal rules, the size and rate caps, and the empty answers. Tests in `tests/hook_reports.rs`: 100 bad reports change nothing and give one answer (SC-004), a non-loopback peer is refused, an `Origin` header is refused, a body of 1 MB is refused, 60 reports in one second drop the excess.
- Refusals are counted, one log line per minute, no body in any log line, and no rate-limit state for a session without a valid secret (D1).
- Traceability: FR-007, FR-008, FR-009, FR-022, SC-004.

### T2: HookSessions, status application, waits (D6, D7, D8)
- `HookSessions` with unit tests for every row of the D7 table, the late rule, the ask end rules, and helper sub-agent reports.
- `AgentStatusBridge` filters terminal events; `HookStatusBridge::apply_report` applies reports under `StatusOrder` with the run check; `WaitTracker::set_exact`.
- Tests in `tests/hook_status.rs`: US1 scenarios 1 to 6, the redraw case (a fake CLI that prints a line every 2 s with no turn: 0 status changes, 1 notification, SC-002 in miniature), a lost `Stop` (late after the quiet period, SC-011 in miniature), and an exact session with unchanged 0022 tests for estimated sessions.
- Traceability: FR-011 to FR-015, FR-023, SC-001, SC-002, SC-011.

### T3: Settings file, launch, cleanup (D3, D4, D5, D12, D13)
- `CliAdapter::status_hooks`, `PrepareLaunch.status_hooks`, `PreparedLaunch.extra_arguments`, the file writer, the version check with a cache, the `hook_status` setting, and the cleanup at the five session ends and at start.
- Tests: file content (only `hooks`, six events, no secret, owner-only mode on Unix), no file for `codex`, `gemini`, `opencode`, and a custom command (SC-008), no file with the setting off (SC-012), no file when the flags hold `--settings` or `--settings=x` (FR-004), no file for an old or unreadable version, a failed file write, file removed after exit, failure, stop, sleep, start failure, and server stop (SC-005), stale files removed at start, a woken session gets a new file and secret (PB-8), no change in any CLI folder (SC-009), and a search of `office.db` and the log for the secret finds 0 hits (SC-014).
- Traceability: FR-001 to FR-006, FR-010, FR-024, SC-005, SC-008, SC-009, SC-012.

### T4: Needs you, notification, and the two gates (D10, D11)
- The `permission_asked` kind through domain, store, wire, and the notification bridge; the two waiting details; `record_permission_ask`.
- `hooks.blocks_typing` in the directive gate and `hooks.ask_open` in the sleep gate.
- Tests: US2 scenarios 1 to 7 (SC-003 and SC-013 in miniature: 50 directives during an ask type 0 characters; a sleep with the item dismissed does not happen), the item sorts in the waiting group, an item with the ask is not plain.
- Traceability: FR-013, FR-016, FR-017, FR-018, 0008/FR-002, SC-003.

### T5: Desktop (D9, D14)
- Wire types and the `hook.changed` handler, the `hook-status` module, the details and list text, and `eventText`.
- Unit tests for the models and controller, and the e2e spec `hook-status.spec.ts` with `mock-core.ts`: the setting row by keyboard, the texts for exact, estimated, late, asks permission, and compacting, with no color-only state.
- Traceability: FR-018, FR-019, FR-020, FR-024, US5, the Visible states table.

### T6: Docs and spec upkeep
- Living-spec text for the five modified requirements stays in this spec until the archive step; `PLAN.md` status, the QA report skeleton, `docs/` text for the new setting, and the Turkish spec report under `.agents/work/spec-reports/` with its index row (local, git-ignored).

### T7: Hands-on run and QA report
- With a real signed-in `claude` (interactive, through the office): check A1 (interactive hooks, the trust prompt, `PermissionRequest`, `PreCompact` and `PostCompact`, the Esc case, `agent_id` of helper sub-agents, the user-hook marker of SC-006, the failing endpoint of SC-007), fix `MIN_HOOK_VERSION` (D5), and measure SC-001, SC-002, SC-003, and SC-011. Record each result, with the version, in `qa-report.md`. A fact that fails changes the plan, the code, or A1 before the PR leaves draft.

## Traceability

| Requirement | Task |
|---|---|
| FR-001 to FR-006, FR-024 | T3 |
| FR-007 to FR-009, FR-022 | T1 |
| FR-010 | T3 |
| FR-011 to FR-015, FR-023 | T2 |
| FR-016, FR-017, FR-018, FR-025 | T4 |
| FR-019, FR-020 | T5 |
| FR-021 | T3 (SC-008 test) |
| SC-001, SC-002, SC-011 | T2, T7 |
| SC-003, SC-013 | T4, T7 |
| SC-004 | T1 |
| SC-005, SC-008, SC-009, SC-012, SC-014 | T3 |
| SC-006, SC-007, SC-010 | T7, with T3 and T1 tests for the parts that need no real CLI |

## QA and evidence

- `cargo test` for the touched crates and the daemon tests, `cargo clippy -D warnings`, `cargo fmt --check`, and the desktop's `bun test`, lint, and type check, as the CI runs them. The desktop crate needs system libraries that this machine may lack; CI covers it.
- e2e with Playwright for the desktop (T5), at most one run at a time.
- Hands-on for the criteria that need a real `claude` (T7). Windows and macOS: 3-OS CI evidence, per the owner's rule (2026-11-16).
- A secure-engineering pass by Efe on D1, D2, D3, and D13 before the PR leaves draft.

## Final verification

Merge `origin/main`, run the CI checks locally where the machine allows, regenerate the Turkish report, then ask Efe for the final gate.

## Result
