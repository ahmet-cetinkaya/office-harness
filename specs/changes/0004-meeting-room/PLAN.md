---
status: approved (post-MVP, built after the MVP)
spec: specs/changes/0004-meeting-room/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-17, approved by the owner 2026-11-17. Status: approved (post-MVP, built after the MVP). The build starts after change 0003 (panel practicality) merges to `main`. -->

# Plan: meeting room (0004)

## Summary

This change adds the meeting room to the panel: the user picks a topic, 2 to 8 agents, and one of four fixed presets, and the system briefs every participant in its own terminal, collects each answer, asks one participant (the facilitator) to write the result, and saves the whole meeting as a record. While a meeting runs, the user sees each participant's state as text, opens any participant's terminal in one step, and can stop the meeting. Past meetings are listed newest first, open in the panel, and copy as Markdown. A participant of a running meeting no longer notifies on `waiting for input` (the modified 0003/FR-006).

**What you get:**
- A `meeting` server module: meeting state, the four presets, brief send as one unbroken input, answer capture by the 0001 quiet period, a 10-minute time limit per reply, records in SQLite, and the outcome rule (`completed`, `partial`, `stopped`), including `partial` after a server restart.
- New requests `meeting.start`, `meeting.stop`, `meeting.list`, `meeting.get` and events `meeting.changed`, `meeting.saved` behind the office token.
- A meeting room view in the desktop app: the start form with refusals that name each blocked agent, the running view with states and the stop action, the record view, the newest-first list, and copy as Markdown.
- The notification suppression of the modified 0003/FR-006, done in the host bridge.

**Why this approach:**
- `docs/ARCHITECTURE.md` already places R8: a `meeting` module "that reads terminal output through ports". A new module with its own tables and ledger keeps the boundary rule: the meeting module never names an `agent` or `terminal` type; the host app adapts the ports, as it already does for `TerminalHost` in `bridges/terminal_host.rs`.
- Everything the meeting needs exists on `main`: the terminal runtime can paste text and write keys (`TerminalRuntime::paste`, `write_input`), the status bridge already turns a quiet period into `waiting for input` (`bridges/agent_status_bridge.rs`), output flows to subscribers (`OutputSubscriber`, `LiveOutput`), and per-module migrations are a solved pattern (`bootstrap/migrations.rs`). The meeting adds no new terminal mechanism.
- The webview already has the shapes to reuse: one store and controller per concern (`modules/agent/state/`), an embedded terminal view, Playwright with the mock core, and `ts-rs` type generation. The meeting room is one new webview module folder.

**What it will not do:** agent-to-agent messages, debate rounds, and voting; user-defined presets and a changeable time limit; search across records and export to a file; a built-in facilitator; cleaning up the redraws of a full-screen CLI; scheduling or repeating meetings; a notification when a meeting ends; the R5 machines and the R7 scene; invites of whole teams (0002); meeting actions in the audit action record (the meeting record itself is the trail).

**Shape of the work:** 9 tasks in 2 phase PRs into one umbrella branch `feat/0004-meeting-room`, one local commit per task. **Effort:** L.

**Main risks:**
- The brief must not only reach the terminal but also be submitted, and CLIs differ at their prompt. Mitigated by T1 R2, which checks paste plus Enter against each 0001 CLI's behavior, and by the SC-007 hands-on rows. A CLI that does not answer ends in the time-limit path, so a meeting never hangs.
- The answer of a full-screen CLI can be hard to read (A1); this change does not clean it up. Mitigated by SC-007, which measures and records each CLI, and by capture from the produced text rather than a screen reconstruction.
- 0005 (office scene) and this change both edit the `app.svelte` view switch (0005 adds the scene to the same switch). Merge order: the 0005 scene lands before this change's phase 2 starts; if it has not merged by then, phase 2 rebases on `main` before T6. Whichever switch edit lands second reconciles `app.svelte` so the switch holds both view values. T1 R1 reads the merged switch before any view code. The room adds no `last_view` value, so the restore path stays panel or scene (D11), which keeps both orders safe.
- Eight participants briefed at once must land within 5 seconds (SC-002) and must not flood the 256-event backlog. Briefing is one short write per terminal, and meeting events fire on state changes only (about 10 per meeting), never per output chunk.
- A restart during the capture loses the answers that had not frozen yet. Mitigated by persisting each answer when it freezes, so the restarted server finalizes `partial` with the answers saved so far, exactly as the spec's outcome rule says.

**Decisions made for you:** the `meeting` module with tables `meeting_meetings` and `meeting_participants`, the brief as one paste plus one submit key, capture from the raw output stream stripped of control codes, each answer persisted when it freezes, the time limit injected at construction (10 minutes in production, short in tests), the meeting room as a shell view value that is not restorable (the `last_view` view field stays panel or scene), the Markdown template as a tested pure function in the webview, and no new npm dependency.

**Blockers:** the owner's approval of the spec, then 0003 merged to `main`. Nothing else.

## Goal

Deliver spec 0004: from the panel the user starts a meeting on a topic with 2 to 8 agents and a preset, watches each participant's state as text, opens any participant's terminal in one step, stops a meeting, and reads past records as a list with copy as Markdown. The record holds every answer and the facilitator's result, survives restarts, and keeps names after an agent is deleted.

## Context

- **Repository:** 0001 is on `main` (merge 7750602). 0002 and 0003 are approved and wait for their builds. 0004's spec is approved (2026-11-17, post-MVP); its Clarifications settle D-11 of `docs/PRD.md`. This plan branches after 0003 merges, because the meeting room opens from the panel and the suppression needs 0003's notification bridge. Until then only the spec and this plan exist.
- **Existing seams this change touches** (verified in the repository at `origin/main` 7750602 on 2026-11-17):
  - Request routing: `src/apps/office-harnessd/src/server/dispatcher.rs` sends each request to the module that owns its prefix (`access` today, with `agent.*`, `project.*`, `session.*`, `settings.*` through `server/agent_requests.rs`). A `meeting.` prefix registers the same way. Requests run behind the token gate in `server/connection.rs`; nothing new guards them.
  - Terminal writes: `TerminalRuntime::paste` wraps text in the bracketed-paste markers when the CLI turned that mode on; `write_input` passes bytes unchanged; `bridges/typed_input.rs` already pastes after a run's first output and deliberately never submits (the 0001 startup prompt). The meeting brief submits, which is the one behavioral difference.
  - Status and quiet period: `TerminalEvent::QuietPeriod` reaches `bridges/agent_status_bridge.rs`, which sets `waiting for input` (FR-014 of 0001) and publishes `agent.status` through the `EventHub` (broadcast, backlog 256). A second subscriber on the same events drives the meeting states.
  - Output capture: `OutputSubscriber::deliver` receives `LiveOutput::Bytes` while the runtime holds the terminal lock; `terminal-domain` `OutputLimits` caps lines and bytes (the 0001 recent-output limit, A2); `output_store.rs` keeps the capped file per session.
  - Persistence: `bootstrap/services.rs` opens one SQLite file and runs each module's migrations through `run_migrations(conn, ledger_table, scripts)`; the `agent` module's tables show the shape (prefix, ledger, `migrations/0001_...sql`). A new module adds crates under the existing workspace globs, so the root `Cargo.toml` needs no edit.
  - Desktop shell: `src/apps/desktop/src/app.svelte` renders the 0001 connection states and then one view; `modules/agent/state/office-controller.ts` applies events to one snapshot; `modules/terminal/` exports the embedded `TerminalView`; `core/api/generated/` holds the `ts-rs` types. `boundaries.spec.ts` enforces imports through each module's `index.ts`.
  - Tests today: Rust unit tests in `<name>.tests.rs`, integration tests in `tests/` with the fake CLI harness (`tests/support/fake_cli.rs`, `serve_office_with` injects a short quiet period); `bun test src`; Playwright with `@axe-core/playwright` and `e2e/support/mock-core.ts`, which seeds the office, pushes events, records every invoke call.
- **Floor rules:** one build or test at a time on the floor; ask god for the build slot before each local build or test run (`CARGO_BUILD_JOBS=4`, `--test-threads=4`, at most 30 minutes). The desktop crate cannot build locally without GTK; CI covers it. Efe reviews each PR at its exact head sha; god merges. Nothing is pushed to `main`. No other project's name appears in any repository file.
- **Toolchain:** as 0001 to 0003 (cargo 1.100, bun 1.4.4, Node 24, Tauri v2, Svelte 5 with runes, Tailwind CSS 4, TypeScript).

## Decisions

- **D1 Module layout:** a new server module `src/modules/meeting/{domain,application,infrastructure,presentation}` with crates `meeting-<layer>`, table prefix `meeting_`, and its own ledger `meeting_schema_version`, exactly as `docs/ARCHITECTURE.md` places R8. The host app gets `bridges/meeting_bridge.rs` (the terminal port adapter) and the meeting runner that owns timers and tasks. No module depends on another; `scripts/check-boundaries.sh` needs no edit, because a crate's role comes from its path.
  - Simpler alternative rejected: put meetings in the `agent` module. A meeting is not agent state, and the module boundary is the reason `docs/ARCHITECTURE.md` exists.
- **D2 The unbroken brief (FR-001):** the brief is the topic, the user's optional extra instruction, and the preset's instruction, composed by `meeting-application` and delivered by the bridge as one `paste` into the participant's terminal with the submit key appended in the same locked step, so no key of the user can split it (PB-5: the terminal never locks; text the user types meanwhile lands after the brief, as the spec's edge case says). This differs from the 0001 startup prompt, which never submits; `waiting for input` means the CLI sits at its prompt, so the submit is expected. T1 R2 confirms per CLI.
- **D3 Answer capture (A1, A2):** the bridge subscribes to the participant's live output when it sends the brief and stops when the participant is done. The brief itself echoes in most terminals (the paste redraw or the CLI's input echo); that echo is not answer. The application discards the captured output up to and including the echo of the brief text (or, when a CLI echoes nothing, discards nothing); the answer window and the `answering` state start after the echo, not at the first output byte. The application freezes an answer when the quiet period returns the participant to `waiting for input`, stores the remaining text stripped of color and cursor codes, cut at the 0001 output limit with a note that says where it was cut. Capture reads the produced stream, not a screen rebuild, so an in-place redraw stays as produced (A1); SC-007 records how readable each CLI's answer is. This change has exactly two stages that paste input and capture a reply, the brief (D2) and the result request (D8); the echo rules above govern both, so no stage saves its own echo.
- **D4 States and timing (FR-006, A5):** a participant is `briefed`, `answering`, `answered`, `no answer`, or `left`. The answer window opens after the terminal echoes the submitted brief (D3) and closes at the time limit, 10 minutes from the brief. The quiet period closes it earlier only once reply output beyond the echo has arrived. Echo-only output never closes the window at the quiet period: that participant stays in the window until the limit and then freezes as `no answer`, never an empty `answered`. Output beyond the echo with no text freezes as an empty `answered` at the quiet period. The facilitator's result window uses the same rules, with the limit measured from the result request (D8). The limit is a constructor argument (10 minutes in production, short values in tests), not a setting, because A5 fixes it for this change.
- **D5 Records and restart (FR-005, FR-008):** one `meeting_meetings` row per meeting and one `meeting_participants` row per seat, with the agent name copied into the seat so it survives a deleted agent (US3 scenario 4). Each answer is written to its row when it freezes. A meeting found still running at server start is finalized `partial` with the answers saved so far, the same way `mark_running_as_interrupted` handles agents. The outcome follows the spec's rule verbatim; the state machine that computes it is domain code with table-driven tests.
- **D6 Protocol:** requests `meeting.start`, `meeting.stop`, `meeting.list`, `meeting.get`; events `meeting.changed` (a state change, carrying the participant states and the deadline timestamps) and `meeting.saved` (the record is written). Refusals use the 0001 error contract and name each blocked participant and the reason (FR-003). `meeting.list` returns newest first, at most 500 rows per call, the same bound the 0003 plan chose for notifications. No paging in this change, so a 501st and older record is stored but not listed; the record view still opens any record by id through `meeting.get`. Known limit, stated in the README (T8); paging is a later change (A6 has no search).
- **D7 Presets (FR-002, A4):** four constants in `meeting-domain`: Decision, Brainstorm, Review, Status round, each with its brief instruction and its result format text. T1 fixes the exact texts in `contracts/meeting-protocol.md`; the spec's Terms table is the source. Presets are not user-editable.
- **D8 Facilitator stage (FR-004, A3):** when every participant is `answered`, `no answer`, or `left`, the bridge sends the facilitator one paste: all answers, labeled by name, plus the preset's result format. The result request runs through the same capture seam and the same echo and window rules as the brief (D3, D4): the captured output is discarded up to and including the echo of the pasted text, the result window opens after that echo, and the quiet period closes it only after reply output beyond the echo; output that holds only the echo waits for the limit, measured from the result request, and then ends the meeting `partial` with the answers shown and "no result was written", so an echo-only facilitator never yields an echo as the result. When reply output beyond the echo arrives, the facilitator's captured reply is the result, stored as written, never parsed. A facilitator that fails or exits also ends the meeting `partial` with the answers shown and "no result was written".
- **D9 Stop (FR-007, SC-005):** `meeting.stop` detaches every capture, sends no further input for the meeting, leaves every agent running, and saves the record with outcome `stopped` at once, well within the 2 seconds of SC-005. The desktop app asks for confirmation first (an existing confirm dialog pattern).
- **D10 Notification suppression (MODIFIED 0003/FR-006):** the merged 0003 notification bridge asks the meeting registry, through a host-side lookup, before it creates a `waiting for input` notification; a participant of a running meeting notifies nobody. Exits, failures, and bells still notify. No module dependency is created; the lookup is one host function, the same pattern as the other cross-module needs in `docs/ARCHITECTURE.md`.
- **D11 Webview layout (US1 to US3):** a new folder `src/apps/desktop/src/modules/meeting/` with `index.ts`, a service, a store and controller in the office-controller pattern, the room view, the start form, the record view, and the list. The room opens from the panel as a view value in the shell; `app.svelte` owns the switch, as the 0005 plan does for the scene. The room is not a `last_view` value: the `view` field of 0005 stays panel or scene, the app opens the panel on start, and the panel shows the running meeting or the last record there, per the spec's edge case "the desktop app is closed during the meeting". Whichever plan edits the switch second reconciles `app.svelte` to hold both view values (merge order in Main risks). `meeting.get` and the events restore a running meeting after a reconnect or an app restart (PB-4: the meeting never stops because the app closed).
- **D12 Record copy (FR-009):** a pure function `record-markdown.ts` renders a record as Markdown (template in `contracts/meeting-protocol.md`), tested with `bun test`; the copy action writes through `navigator.clipboard` and falls back to a focused read-only view of the text when the clipboard refuses, with a "copied" announcement either way. T1 R5 confirms the clipboard in all three webviews.
- **D13 Accessibility (PB-3):** every control works from the keyboard with a visible focus mark; participant states and the outcome are text, never color alone; one polite live region announces participant state changes and the outcome change; pickers are dialogs that take and return focus; text contrast comes from the shared theme. Playwright with axe covers the room at 1280 and 360 widths, as 0001 did.
- **D14 Tests:** Rust unit tests for the domain and the application with port fakes; integration tests through the real server and socket with the extended fake CLI harness; `bun test` for the webview logic; Playwright with the extended mock core for the views; hands-on rows for SC-001 and SC-007. `tests/support/fake_cli.rs` gains meeting CLIs: one that answers and returns to waiting, one that echoes the brief and then answers, one that stays silent (echoing the brief, replying nothing), one that echoes the brief, waits longer than a quiet period, and then answers, one that exits after the brief, one that redraws a full screen, and two that play the facilitator: one echoes whatever it receives (its result request with the answers) and then writes the result, and one echoes its result request and then stays silent (so an echo-only facilitator is covered); each fake echoes exactly what it receives, never less. A serve helper injects a short time limit, as it injects a short quiet period today.
- **D15 Delivery and commits:** phased delivery into the umbrella `feat/0004-meeting-room`, per-task local commits, Efe gates each phase PR at its head sha, god merges (the 0003 procedure).
- **Constitution check:** the repository has no instruction file; `docs/ARCHITECTURE.md` is binding, and this plan adds the module its Modules table already names for R8. The owner's global rules apply: clean code, no secrets, explicit approval for remote actions.

## Scope

**In:**
- FR-001 to FR-009, the modified 0003/FR-006, and SC-001 to SC-007 of spec 0004.
- Server: the `meeting` module, the host bridge and runner, the protocol surface, the restart sweep, the notification suppression, the extended fake CLI harness.
- Desktop: the meeting room module (start form, running view, stop, record view, list, copy as Markdown), the shell view value, the `app.svelte` switch edit (the room stays out of the `last_view` restore).
- `e2e/support/mock-core.ts` extensions for meeting fixtures; regenerated wire types; README and `docs/ARCHITECTURE.md` additions.

**Out (guards against unrequested additions):**
- Debate rounds, agent-to-agent messages, voting, and anything beyond one brief and one result per participant (A2).
- User-defined presets, a changeable time limit, a built-in facilitator (A4, A5, Clarifications).
- Search across records, export to a file, a notification when a meeting ends, cleanup of full-screen redraws (Out of scope of the spec).
- Meeting actions in the audit action record; the record itself is the trail (the spec names no audit requirement).
- Directives through the 0002 hierarchy; the spec depends on 0001 and 0003 only, so the brief goes through the terminal port directly, the same shape a 0002 directive would use later.
- R5 machines, the R7 scene (separate plans), scheduling, repeating.
- Any new visual design beyond the spec's flow (the spec says none exists).

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Server meetings | Meeting module, brief and capture, protocol, restart sweep, suppression | T1 to T5 | `feat/0004-p1-server` (umbrella) | p1 → umbrella |
| 2 Desktop meeting room | Room view, records, copy, docs, QA | T6 to T9 | `feat/0004-p2-desktop` (umbrella after p1) | p2 → umbrella, then umbrella → `main` |

The umbrella `feat/0004-meeting-room` is created from `main` after the spec is approved and 0003 merges, and holds the approved spec and this plan (committed by the docs PR). Each phase PR runs CI on three OSes and gets Efe's review at its head sha.

## Tasks

Paths are relative to the repository root. `M/` stands for `src/modules/`. `D/` stands for `src/apps/desktop/src/`.

### T1: Research and design artifacts (no product code)

- **Files:** `specs/changes/0004-meeting-room/research.md`, `data-model.md`, `contracts/meeting-protocol.md`.
- **Work:**
  - R1: the merged 0003 panel and shell: how the panel opens a view, how it embeds a terminal in one step (0003/FR-004), the exact exports of `modules/panel/index.ts`, the merged `app.svelte` view switch, and the `last_view` shape at that commit (with the 0005 `view` field if the scene landed). Record what D11 adds to the switch, that the room stays out of the restore path, and what it reuses.
  - R2: the brief delivery per CLI: what `paste` plus the submit key does at each 0001 CLI's waiting prompt, with the bracketed-paste state read from the runtime; cite the 0001 deviation that the startup prompt never submits and why the meeting differs. Record the exact bytes of the submit for each platform. Record the echo of the submitted brief per CLI (the bracketed-paste redraw, the input echo, or no echo at all) and how the capture can tell the echo from the reply (D3); a CLI that prints a placeholder such as "[Pasted text +N lines]" instead of the full paste counts as echo: the answer window opens after that placeholder, and it is stripped with the echo. Since the result request is a larger paste through the same seam (D8), record whether a CLI's echo behavior differs by paste size.
  - R3: the capture seam: a second subscriber on the terminal events (`QuietPeriod`, `Exited`), the live-output subscription for an agent that already has an attached client, the `OutputLimits` cut, and the stripping of color and cursor codes over raw bytes; how the echo of the submitted text is located in the raw stream and stripped, for the brief and for the result request alike (D3, D8); reuse the recorded-output fixtures of the earlier research for redraw cases.
  - R4: the merged 0003 notification bridge: where it creates the `waiting for input` notification and where the meeting registry lookup goes (D10); cite file and function.
  - R5: `navigator.clipboard` in WebKitGTK, WebView2, and WKWebView inside Tauri v2 (secure context, permission prompts, focus rules) and the fallback; the Markdown template draft; cite sources.
  - R6: WAI-ARIA Authoring Practices for the multi-select picker dialog, the preset select, and the polite live region; cite each.
  - `data-model.md`: the DDL of `meeting_meetings` and `meeting_participants` (ids, topic, extra instruction, preset, facilitator, times, outcome, state, per-seat agent id, copied agent name, state, answer, cut note, answer times), the indexes, and the ledger name.
  - `contracts/meeting-protocol.md`: the four preset texts (instruction and result format), every request and event with field tables, the refusal body that names participants and reasons, the deadline timestamp fields, and the Markdown template of D12, and nothing else.
- **Depends on:** the gate (the spec approved, 0003 merged), because R1 and R4 read merged code.
- **Acceptance:** every research item has a "verified <date>, source <file or link>" line; the contract lists every type that T2 to T7 use, and nothing else.

### T2: Meeting module domain and persistence

- **Files:** `M/meeting/domain/`, `M/meeting/infrastructure/`, `Cargo.lock`.
- **Work:**
  - (a) Domain: `Meeting`, `Participant`, the participant states and the meeting states, the outcome values, and `Preset` with the four fixed texts (D7). The start validation lives here: topic not empty after trimming, 2 to 8 participants, each agent once, the facilitator among them; a refusal names every violated rule (FR-003 inputs, edge cases).
  - (b) The outcome function: `completed`, `partial`, or `stopped` from the participant states, the facilitator result, and the restart flag, as the spec's rule states.
  - (c) Infrastructure: the repositories and the migration from `data-model.md`; the seat row keeps the copied agent name; the answer cut note is stored (FR-005, US3 scenario 4).
- **Depends on:** T1.
- **Acceptance:**
  - (a) Domain tests: each invalid start refused with a message that names the field or agent (empty topic, 1 participant, 9 participants, a repeated agent, a facilitator outside); exactly four presets with their texts; every outcome rule branch, including the restart flag and the failed facilitator.
  - (b) Persistence tests on a temporary SQLite file: the migration runs and is idempotent; a meeting round trips with 8 seats; the list returns newest first; a seat keeps its name and answer after the agent row is gone.
  - fmt, clippy, `cargo test` for the new crates, `scripts/check-boundaries.sh`.

### T3: Brief send, answer capture, and the facilitator stage

- **Files:** `M/meeting/application/`, `src/apps/office-harnessd/src/bridges/meeting_bridge.rs`, `src/apps/office-harnessd/src/bootstrap/` (wiring), `src/apps/office-harnessd/src/bridges/mod.rs`.
- **Work:**
  - (a) Application ports: `MeetingAgents` (status and existence lookups), `MeetingTerminals` (brief write, live-output and event subscription per participant), `MeetingRecords` (the repository port), `Clock`. No `tokio`, no terminal types; the host adapts.
  - (b) `start_meeting`: validate, refuse with names and reasons (a participant not `waiting for input`, a participant of another running meeting, count bounds, empty topic), then brief each participant as one unbroken paste plus submit (D2) and record each brief time (FR-001, FR-003).
  - (c) The capture and the states: `briefed` at the brief, `answering` after the echo of the brief (D3), freeze at the quiet period when reply output beyond the echo has arrived (`answered`, text stripped of the echo, color, and cursor codes, cut at the limit) or at the time limit (`no answer` when only the echo arrived, however long the silence lasts); an exit or a failure marks `left` and the meeting goes on (FR-008); each frozen answer persists at once (D5).
  - (d) The facilitator stage: when no participant is pending, send the facilitator the answers and the result format (D8); its reply freezes as the result through the same echo and window rules as the brief (the echo of the result request is stripped, the window opens after the echo, the quiet period closes only after reply output, echo-only output waits for the limit and ends `partial` with "no result was written"); a facilitator failure, exit, or silence at the limit ends `partial` (FR-004).
  - (e) `stop_meeting`: detach every capture, mark `stopped`, save (D9).
  - (f) The runner lives in the host bridge: one task per meeting for the deadline ticks, the terminal events and output fed through the ports. The application stays tokio-free.
- **Depends on:** T2.
- **Acceptance:**
  - (a) Application tests with port fakes: one paste-plus-submit write per participant and nothing else; the brief text for each preset carries topic, extra instruction, and preset instruction; every state transition in D4, including empty output as `answered`, an echoed brief stripped from the answer, a reply after more than a quiet period of silence since the echo still freezing as `answered` (the quiet period on echo-only output does not close the window), and echo-only output as `no answer` at the limit, not before; `left` on exit; the facilitator stage composition, with the same echo cases run against the result request: an echoed request stripped from the result, echo-only output ending `partial` with "no result was written" at the limit, not before, and a reply after more than a quiet period of silence still freezing as the result; the second-meeting refusal while the agent sits in a running meeting, also after it answered (edge case).
  - (b) Bridge tests with fake terminal events and output: the states move as the events arrive; two meetings with disjoint agents run at once; a stop detaches and saves.
  - fmt, clippy, `cargo test`, `scripts/check-boundaries.sh`.

### T4: Meeting protocol, restart sweep, and the integration harness

- **Files:** `M/meeting/presentation/`, `src/apps/office-harnessd/src/server/dispatcher.rs`, `src/apps/office-harnessd/src/bootstrap/{services.rs,mod.rs}`, `src/apps/office-harnessd/tests/` (harness and new tests), `src/apps/desktop/src-tauri/` (regenerated wire types), `src/apps/desktop/src/core/api/generated/`.
- **Work:**
  - (a) Handlers for `meeting.start`, `meeting.stop`, `meeting.list`, `meeting.get` with the contracts of T1; the refusal body names each participant and reason; events `meeting.changed` and `meeting.saved` published through the hub (D6).
  - (b) The startup sweep: a meeting left running is finalized `partial` with the saved answers (D5, edge case), before clients can connect.
  - (c) Register the `meeting.` prefix in the dispatcher, the gateway in the composition root, and the crates in the workspace (the globs already cover them).
  - (d) Extend `tests/support/fake_cli.rs` with the meeting CLIs of D14 and a serve helper with an injected time limit; regenerate the desktop wire types with `ts-rs`.
- **Depends on:** T3.
- **Acceptance:** integration tests through the real server, the socket, and the fake CLIs (the pattern of `tests/agent_protocol.rs` and `tests/server_restart.rs`):
  - (a) US1 scenario 1: three waiting agents are briefed; each terminal receives the brief text.
  - (b) SC-002: eight participants all have the brief within 5 seconds of the start.
  - (c) SC-003: with fake agents that echo the brief and then print known answers, 100% of the saved answers equal the produced text after the echo and hold no brief text, and the result equals the facilitator's reply, which comes from an echoing facilitator fake: the saved result holds no text of the pasted answers or the result format. One of the participants waits longer than a quiet period after the echo before it answers: the record still marks it `answered` with that reply.
  - (d) US1 scenario 5 and SC-004: one silent participant that echoes the brief and answers nothing, the meeting ends `partial` within the injected limit plus 10 seconds; the record marks "no answer" for it no earlier than the limit, and its seat holds no brief text. The facilitator fake echoes its result request and stays silent: the meeting still ends `partial` no earlier than the result-request limit, and the record says that no result was written, with no answer text in it.
  - (e) FR-007 and SC-005: a stop sends no further input, leaves the agents running, and saves the record within 2 seconds.
  - (f) FR-008: an exit during the meeting marks `left`; the record completes with the rest.
  - (g) Concurrent meetings with disjoint agents both run and both save; a refusal names every blocked participant.
  - (h) A restart during a meeting reopens with outcome `partial` and the answers saved so far; the sweep runs before any client connects.
  - (i) The facilitator gives no reply within the limit: the meeting ends `partial`, the answers show, and the record says that no result was written (edge case).
  - (j) Text typed into a participant's terminal during the meeting lands after the brief and the saved answer holds it (PB-5, edge case).
  - (k) A participant's agent is deleted mid-meeting: the participant is `left`, the meeting ends with the rest, and the seat keeps the agent name (FR-008, edge case).
  - fmt, clippy, `cargo test --workspace`, `scripts/check-boundaries.sh`.

### T5: Notification suppression (MODIFIED 0003/FR-006)

- **Files:** `src/apps/office-harnessd/src/bridges/notification_bridge.rs` (the merged 0003 file, name from T1 R4), `src/apps/office-harnessd/src/bootstrap/`.
- **Work:** before the bridge creates a `waiting for input` notification, it asks the meeting registry; a participant of a running meeting notifies nobody. Exits, failures, and bells notify as before (D10). No `meeting` crate appears in the `notification` module.
- **Depends on:** T4 (the registry exists), and the gate (0003's bridge is merged).
- **Acceptance:** bridge tests with fake events: a participant of a running meeting creates no `waiting for input` notification; the same agent after its meeting ends notifies again; an exit, a failure, and a bell during a meeting still notify. fmt, clippy, `cargo test`.

### T6: Meeting room view: start, follow, stop

- **Files:** `D/modules/meeting/` (service, store, controller, room view, start form, live view), `D/app.svelte`, `D/e2e/support/mock-core.ts` (meeting seeds, request answers, event pushes), `D/core/api/generated/` (types from T4).
- **Work:**
  - (a) The meeting service and controller in the office-controller pattern: `meeting.list` and `meeting.get` on load and on every reconnect, `meeting.changed` and `meeting.saved` applied in place; a running meeting survives a reconnect (PB-4).
  - (b) The start form: the topic field (empty and spaces-only refuse with the message), the optional extra instruction, the participant picker (each agent once, 2 to 8, statuses visible, order kept), the preset picker (the four), the facilitator confirmed among the participants with the first chosen as default; a refused start names every agent and reason (FR-003, edge cases).
  - (c) The running view: each participant's state as text (`briefed`, `answering`, `answered`, `no answer`, `left`), the time left from the deadline timestamps, the stop action always visible with confirmation (D9), and each participant's live terminal in one step through the merged 0003 mechanism (FR-006, US2 scenarios 1 and 2).
  - (d) Fewer than two agents in the office: the room says that a meeting needs 2 agents and offers the create action (0001); the 0001 connection states stay in front of the room (PB-3 of the spec's table).
  - (e) One polite live region announces participant state changes and the outcome (D13).
- **Depends on:** T5 (so the phase 2 branch holds the whole server), T1 R1.
- **Acceptance:**
  - (a) `bun test`: events apply in place and keep the focus; a reconnect reloads and restores a running meeting; the refusal text of each invalid form state.
  - (b) Playwright with axe at 1280 and 360 widths: the form end to end against the mock core (US1 independent test shape), a refused start that names agents (US1 scenario 4), the running view states from pushed events, the stop that saves and shows the record (US2 independent test), two meetings with disjoint agents both visible (edge case), fewer than two agents shows the message and the create action.
  - svelte-check, eslint, prettier --check.

### T7: Records: list, record view, copy as Markdown

- **Files:** `D/modules/meeting/` (record view, list, `services/record-markdown.ts` and its spec), `D/e2e/support/mock-core.ts`.
- **Work:**
  - (a) The list: topic, preset, time, and outcome, newest first, with the empty-list state that offers the start action (FR-009, US3 scenario 1).
  - (b) The record view: every field, answers marked `no answer`, cut answers with the note, the outcome as text, and a deleted agent's seat with its name and answer (FR-005, US3 scenarios 2 and 4).
  - (c) Copy as Markdown: the pure function of D12 with the template of T1, the clipboard write with the focused fallback, and the "copied" announcement (FR-009, US3 scenario 3).
- **Depends on:** T6.
- **Acceptance:**
  - (a) `bun test`: the Markdown of a complete, a partial, and a stopped record each hold topic, preset, the answers, and the result; a record with 8 participants and one cut answer renders the note.
  - (b) Playwright with axe: 50 seeded meetings, find one by topic, open it, and copy it within 30 seconds (SC-006, automated measurement); the copied text holds the topic, the preset, the answers, and the result (US3 independent test); a deleted agent's record keeps its name and answer.
  - svelte-check, eslint, prettier --check.

### T8: Documentation

- **Files:** `README.md`, `docs/ARCHITECTURE.md`.
- **Work:** README: the meeting room, the four presets, how to start, follow, stop, read, and copy a record, the 10-minute limit, the list cap of 500 rows with no paging (D6), and the notification change (a participant of a running meeting does not notify on `waiting for input`). `docs/ARCHITECTURE.md`: the `meeting` module row (change 0004, its own FRs) and the phases status line for 0004.
- **Depends on:** T7.
- **Acceptance:** every behavior in the README matches the code and the spec; `rg` finds no other project's name or outside path in the changed docs.

### T9: QA report and final verification

- **Files:** `specs/changes/0004-meeting-room/qa-report.md`.
- **Work:** run the checks in "Final verification", write one verdict per SC (SC-001 to SC-007) with the evidence, and list the hands-on rows.
- **Depends on:** T8.
- **Acceptance:** SC-002 (eight briefs in 5 seconds), SC-003 (100% equality), SC-004 (limit plus 10 seconds), and SC-005 (2 seconds) come from the integration tests; SC-006 (30 seconds for 50 records) from the Playwright run; SC-001 (start in under 60 seconds) from the e2e run plus one hands-on row against a real server with 5 waiting agents. SC-007 is one hands-on row per 0001 CLI (four rows): a person holds a one-line-question meeting with that CLI and reads the saved answer, and the report records for each whether the reply is readable text. Every hands-on row is BLOCKED until a person runs it, never PASS.

## Dependencies

T1 → T2 → T3 → T4 → T5, then T6 → T7 → T8 → T9.

- **Sequential:** T2 to T4 build one module bottom up and share its files and `Cargo.lock`; T5 edits the bridge that T4's registry feeds; T6 and T7 change the same webview folder.
- **Parallel:** none; the shared files make parallel tasks collide.
- **Exclusive:** `Cargo.lock`, the meeting module, the host bridges, and `D/modules/meeting/` are owned by one running task at a time.
- **Gate before everything:** the owner approves the spec, and 0003 is merged to `main`; the umbrella branch is created from that `main`; T1 starts only then.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; the sandbox dotfiles are never staged). After each commit, `git status --porcelain` lists only the sandbox dotfiles and the other draft specs.

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 0 | spec | the docs PR | `docs(spec-0004): approve meeting room` |
| 1 | T1 | `specs/changes/0004-meeting-room/{research.md,data-model.md,contracts/}` | `docs(spec-0004): verify meeting research, data model, and protocol` |
| 2 | T2 | `src/modules/meeting/`, `Cargo.lock` | `feat(meeting): model meetings and store records` |
| 3 | T3 | `src/modules/meeting/`, `src/apps/office-harnessd/src/bridges/`, `src/apps/office-harnessd/src/bootstrap/` | `feat(meeting): brief participants and capture answers` |
| 4 | T4 | `src/modules/meeting/`, `src/apps/office-harnessd/src/`, `src/apps/office-harnessd/tests/`, `src/apps/desktop/src-tauri/`, `src/apps/desktop/src/core/api/generated/` | `feat(meeting): serve the meeting protocol and finalize records` |
| 5 | T5 | `src/apps/office-harnessd/src/bridges/`, `src/apps/office-harnessd/src/bootstrap/` | `feat(notification): quiet the waiting notice during meetings` |
| 6 | T6 | `src/apps/desktop/src/modules/meeting/`, `src/apps/desktop/src/app.svelte`, `src/apps/desktop/e2e/support/mock-core.ts` | `feat(desktop): add the meeting room view` |
| 7 | T7 | `src/apps/desktop/src/modules/meeting/`, `src/apps/desktop/e2e/support/mock-core.ts` | `feat(desktop): list meeting records and copy them as Markdown` |
| 8 | T8 | `README.md`, `docs/ARCHITECTURE.md` | `docs: describe the meeting room and the meeting module` |
| 9 | T9 | `specs/changes/0004-meeting-room/qa-report.md` | `test(spec-0004): record QA results` |

## QA and evidence

- **Per task:** fmt, clippy, `cargo test --workspace` for the crates the task touches, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, eslint, `prettier --check`, Playwright with axe, each as far as the task touches it. Ask god for the build slot before each local build or test run.
- **Per phase PR:** CI green on all three OSes, Efe's review at the head sha, evidence in the PR body.
- **Security pass:** `secure-engineering` on T3 and T4: the meeting requests sit behind the office-token gate; the brief goes only to the participants' terminals (PB-1); no office data leaves the machine and records stay local (PB-2); answer text is bounded by the output limits; the refusal and event bodies carry no terminal output beyond the answers the spec saves.
- **Real surface:** SC-001 against a real server and the four SC-007 readability rows (one per 0001 CLI) are hands-on rows in the QA report; under the overnight directive they are recorded as BLOCKED (need a person), never PASS.
- **Rollback:** each phase is a PR into the umbrella; a failed phase is reverted there. On a user's machine the change adds meeting tables with their own ledger; a server built before this change ignores them, and an app built before it never sends a `meeting.` request. No existing table, setting, or CLI file changes.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-007.
2. **Architecture compliance:** `scripts/check-boundaries.sh` and `boundaries.spec.ts` pass; the meeting module imports no other module; the webview meeting module imports only `core/`, `shared/`, its own files, and other modules' `index.ts`.
3. **Code quality:** all lints clean; a `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office created by 0001 and 0003 opens and works after the upgrade; the meeting tables add on; a record written after the change still reads on the older server (its tables are ignored, its file untouched) and on the newer one (integration tests, all three OSes in CI).
5. **Scope fidelity:** nothing from the Out list is in the diff; the room has no control the spec does not name; `rg` finds no other project's name, outside path, or person in docs or code.
6. **Readiness:** `/ac:verify` passes on the umbrella head before the umbrella PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T3, T4, T6 | T3(b) one paste plus submit per participant; T4(a) brief texts in the integration run; T6(b) the form that starts it |
| FR-002 | T2, T6 | T2(a) the four presets with their texts; T6(b) the preset picker offers exactly those |
| FR-003 | T2, T3, T6 | T2(a) validation refusals; T3(b) the registry refusal for a second meeting; T6(b) the refusal that names each agent and reason |
| FR-004 | T3, T4 | T3(d) the facilitator stage and the result as written; T4(c) result equality in SC-003 |
| FR-005 | T2, T3, T7 | T2(c) the tables and the cut note; T3(c) answers persist when they freeze; T7(b) the record view with every field |
| FR-006 | T3, T6 | T3(c) the states; T6(c) states as text, the time left, and the terminal in one step |
| FR-007 | T3, T4, T6 | T3(e) the stop; T4(e) no further input and the save within 2 seconds; T6(c) the confirmed stop action |
| FR-008 | T3, T4 | T3(c) `left` and the meeting goes on; T4(f) the integration exit run; T4(k) the deleted agent mid-meeting |
| FR-009 | T4, T7 | T4(a) `meeting.list` newest first; T7(a) the list, T7(b) the record view, T7(c) copy as Markdown |
| 0003/FR-006 (modified) | T5 | T5 the suppression with exits, failures, and bells still notifying |
| US1 | T3, T4, T6 | T4(a) to T4(d) the start, the answers, the result, the refusal, and the time limit; T4(i) the silent facilitator; T6(b) the form |
| US2 | T4, T6 | T6(c) the states, the terminal in one step, the stop; T4(e) the stopped record |
| US3 | T4, T7 | T7(a) to T7(c) the list, the record, the copy; T2(c) and T7(b) the deleted agent's name |
| SC-001 | T6, T9 | T6(b) the e2e start measurement; T9 the hands-on row |
| SC-002 | T4 | T4(b) eight briefs within 5 seconds |
| SC-003 | T4, T9 | T4(c) 100% equality of answers and result |
| SC-004 | T3, T4 | T4(d) partial within the injected limit plus 10 seconds |
| SC-005 | T4 | T4(e) the save within 2 seconds with no further input |
| SC-006 | T7 | T7(b) 50 records, find and copy within 30 seconds |
| SC-007 | T9 | T9 four hands-on rows, one per 0001 CLI, recorded BLOCKED until run |
| PB-1 to PB-5 | T3, T4, T6 | PB-1: input only to participant terminals in T3(b) and the security pass; PB-2: records local, no outbound call; PB-3: keyboard and text states in T6 and D13; PB-4: the meeting runs server-side, the reconnect restores it in T6(a); PB-5: the terminal never locks, user keys land after the brief in D2, and T4(j) types during a meeting |
