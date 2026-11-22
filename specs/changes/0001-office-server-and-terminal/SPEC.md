---
status: in_progress
capability: office
plan: .agents/work/office-server-and-terminal/plan.md
supersedes:
---

# 0001: Office server and terminal

## Intent

The owner runs many AI coding agent CLIs and opens a separate terminal for each one to follow and steer it. This change gives him the base of office-harness: a background server that keeps his agents running while the desktop app is closed, and a desktop app in which he attaches to any agent's live terminal and types into it. The server runs only on his own machine, accepts only clients with a token, and never writes the CLIs' own configuration. This change covers R3, R4, and R6 of `docs/PRD.md` and the rules in its section 4.1. Changes 0002 (hierarchy) and 0003 (panel practicality) build on it.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4.
- The MVP is three change specs: `0001-office-server-and-terminal` (this one), `0002-agent-hierarchy`, and `0003-panel-practicality`. 0002 and 0003 depend on this change. This change depends on none.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it. The desktop app runs on Linux, macOS, and Windows.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Office | All agents and projects that one background server manages for one OS user. |
| Background server | The always-on part of office-harness. It runs the agents and keeps the office state. |
| Desktop app | The window the user opens to watch and control the office. It keeps no office state of its own. |
| Agent | One agent CLI (for example `claude` or `codex`) that office-harness runs, with a name, a CLI command, and a working folder. |
| Project | A named folder on the user's disk that groups agents. |
| Session | One run of an agent CLI, from start to exit. |
| Task | A unit of work given to an agent. Each new task starts a new session unless the user explicitly continues an old one. |
| Office additions | Extra command-line flags, environment variables, and a startup prompt that office-harness adds when it starts one agent's CLI. |
| Office token | The secret that a client must present before the background server accepts it. |
| Quiet period | The time without output after which a running agent counts as `waiting for input` (A1). |

## User stories

### US1 — Run an agent that stays alive and type into its live terminal (Priority: P1)

The user creates one agent, attaches to its live terminal, and works in it as in a native terminal. The agent keeps running when the desktop app is closed.

**Why this priority:** this is the smallest slice with value on its own. It replaces one terminal window per agent (R6) and keeps the agent alive without the app (R3, R4). Every other story and change builds on it.

**Independent test:** create one `claude` agent, attach, give it a prompt, approve a tool use with the keyboard, close the desktop app for 10 minutes, open it again, and see the output that the agent produced while the app was closed.

**Scenarios:**

1. Given the background server runs and no agent exists, When the user creates an agent with a name, a CLI command, and a working folder and starts it, Then the agent shows status `starting` and then `working`, and its terminal shows the CLI's own start screen.
2. Given an agent runs, When the user attaches to it and types text and control keys (Enter, arrow keys, Ctrl+C), Then the CLI receives every key, and the terminal shows the CLI's response, colors, and cursor movement as a native terminal does.
3. Given an agent runs and the user is attached, When the user closes the desktop app and opens it again after 10 minutes, Then the agent still runs and its terminal shows the output it produced during those 10 minutes.
4. Given an agent runs, When the user restarts the machine, logs in, and does not open the desktop app, Then the background server runs again, the agent's definition is kept, the agent shows status `interrupted`, and no agent starts without a user action.
5. Given a client has no valid office token, When it tries to read a terminal or control an agent, Then the background server refuses it and no terminal data reaches the client.
6. Given an agent has a running session, When the user chooses "start new task" on that agent and confirms, Then the running session ends and a new CLI session starts.
7. Given no agent exists, When the user opens the desktop app, Then the app says that the office is empty and offers the action that creates the first agent.

### US2 — Adjust how an agent starts without touching CLI configuration (Priority: P2)

The user gives each agent its own flags, environment variables, and startup prompt. office-harness keeps these in its own configuration and never writes the CLI's configuration files. The user chooses when to continue an old session.

**Why this priority:** role prompts and flags make agents useful for real work, and the configuration rule protects the user's existing setup. US1 already works with the CLI's defaults, so this story comes second.

**Independent test:** set a flag, an environment variable, and a startup prompt on one agent, start it, and see all three in effect. Compare the CLI's configuration folder before and after: office-harness changed no file in it. Then continue one previous session of that agent.

**Scenarios:**

1. Given an agent has office additions, When the user opens the agent's details, Then the details show each flag, each environment variable, and the startup prompt, and the user can edit them.
2. Given an agent runs, When the user edits its office additions, Then the details say that the change applies at the next session start, and the running session keeps the old values.
3. Given an agent has previous sessions and its CLI supports continuing a session, When the user picks one previous session and chooses "continue", Then the CLI continues that session.
4. Given an agent has previous sessions, When the user starts a new task without picking a session, Then the CLI starts a new session.

## Requirements

### Background server and self-hosting (R3, R4)

- **FR-001** (US1): THE SYSTEM SHALL run the office in a background server that keeps every agent running while no desktop app is open.
- **FR-002** (US1): WHEN the user logs in to the OS, THE SYSTEM SHALL start the background server without a user action and without opening the desktop app.
- **FR-003** (US1, US2): THE SYSTEM SHALL keep the office (agent definitions, projects, office additions, session lists, action record, and recent output of each agent) across restarts of the background server and of the machine.
- **FR-004** (US1): IF the background server stops while agents run, THEN THE SYSTEM SHALL show each of those agents as `interrupted` at the next server start and SHALL NOT start any of them without an explicit action.
- **FR-005** (US1): THE SYSTEM SHALL NOT send office data (agent definitions, terminal output, configuration, or usage data) to any service outside the user's machine.
- **FR-006** (US1): THE SYSTEM SHALL provide the background server and the desktop app on Linux, macOS, and Windows.

### Access control

- **FR-007** (US1): THE SYSTEM SHALL let a client read terminals, type into terminals, or control agents only after the client presents a valid office token.
- **FR-008** (US1): THE SYSTEM SHALL accept client connections only from the same machine.
- **FR-009** (US1): THE SYSTEM SHALL make the office token and the stored terminal output readable only by the OS user who owns the office.
- **FR-010** (US1): WHEN the user regenerates the office token, THE SYSTEM SHALL reject the old token and close every open connection that used it.
- **FR-011** (US1): THE SYSTEM SHALL record each create, start, stop, and delete action with its actor, its target agent, and its time, and SHALL show these entries in the target agent's details.

### Agents and projects

- **FR-012** (US1): THE SYSTEM SHALL let the user create an agent with a name, a CLI command, a working folder, and an optional project, and start, stop, and delete it from the desktop app.
- **FR-013** (US1): THE SYSTEM SHALL let the user create a project with a name and a folder, and assign each agent to at most one project.
- **FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, or `interrupted`, and `waiting for input` means that the CLI runs and produced no output during the quiet period.

### Terminal (R6)

- **FR-015** (US1): THE SYSTEM SHALL run each agent CLI in its own pseudo-terminal, so that full-screen display, colors, cursor movement, and control keys behave as in the user's native terminal.
- **FR-016** (US1): WHEN the user attaches to an agent, THE SYSTEM SHALL show the agent's recent output followed by its live output.
- **FR-017** (US1): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except one documented shortcut that moves focus back to the rest of the desktop app.
- **FR-018** (US1): WHEN the user resizes the terminal view, THE SYSTEM SHALL resize the agent's terminal to the new size.
- **FR-019** (US1): WHEN the user detaches from a terminal or the desktop app closes, THE SYSTEM SHALL keep the agent running and keep recording its output.

### Agent configuration and sessions

- **FR-020** (US1, US2): THE SYSTEM SHALL start each agent CLI with the user's own home folder, login environment, and CLI configuration, changed only by that agent's office additions.
- **FR-021** (US2): THE SYSTEM SHALL NOT create, change, or delete any file in an agent CLI's configuration files or folders.
- **FR-022** (US2): THE SYSTEM SHALL keep each agent's office additions in office-harness's own configuration and show them, editable, in the agent's details.
- **FR-023** (US2): WHEN the user edits the office additions of a running agent, THE SYSTEM SHALL keep the running session on the previous values, apply the change at the next session start, and say so in the agent's details.
- **FR-024** (US1, US2): WHEN a new task starts on an agent, THE SYSTEM SHALL start a new CLI session.
- **FR-025** (US2): WHERE the agent's CLI supports continuing a session, WHEN the user explicitly picks one of the agent's previous sessions, THE SYSTEM SHALL start the CLI so that it continues that session.

### Desktop app states

- **FR-026** (US1): IF the desktop app cannot reach the background server, THEN THE SYSTEM SHALL show that the server is not reachable and offer an action that starts it.
- **FR-027** (US1): IF the background server rejects the desktop app's office token, THEN THE SYSTEM SHALL show a not-authorized state that tells the user how to give the app a valid token.
- **FR-028** (US1, US2): THE SYSTEM SHALL make every desktop app action available from the keyboard.

## User experience

**Flow:**

1. The user opens the desktop app. The app connects to the background server.
2. The app shows every agent with its name and status, or the empty state with the action that creates the first agent.
3. The user creates an agent: name, CLI command, working folder, optional project, and optional office additions.
4. The user selects an agent and attaches. The agent's live terminal opens and gets keyboard focus.
5. The user types into the terminal. One documented shortcut moves focus back to the rest of the app.
6. In the agent's details, the user edits office additions, reads the action record, starts a new task, or continues a previous session.
7. The user closes the app. The agents keep running.

The fast-access layout for many agents (one list with the last output, the project switcher, notifications, and resume of the last view) is change 0003.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading | The app shows that it connects to the background server. | FR-026 |
| Success | The agents with their statuses, and the attached terminal. | FR-014, FR-016 |
| Empty | The office is empty, with the action that creates the first agent. | US1 scenario 7 |
| Error: server not reachable | The server is not reachable, with an action that starts it. | FR-026 |
| Error: agent failed | The agent shows `failed` with the CLI's last output, for example "command not found". | FR-014, Edge cases |
| Unauthorized | The token was rejected, with the steps to give the app a valid token. | FR-027, US1 scenario 5 |
| Server busy | The server refused the connection because all handshake slots were busy (close code `1013`). The app retries and never shows the unauthorized state for it. | FR-027, FR-026 |
| Agent statuses | `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`. | FR-014 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- Every desktop app action works from the keyboard (FR-028), and keyboard focus is always visible.
- The terminal does not trap focus: one documented shortcut leaves it (FR-017).
- Status is text, optionally with an icon, never color alone (FR-014).
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text) in every theme that the app ships.
- Each agent entry has an accessible name that includes the agent's name and status.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Office | Everything one background server manages for one OS user. | owner (OS user), office token | One per OS user per machine. Has projects and agents. |
| Machine | The computer that an agent runs on. | name | The MVP office has exactly one machine. Each agent belongs to one machine. It exists so that R5 can add more. |
| Project | A named folder that groups agents. | name, folder | Name is unique in the office. Has zero or more agents. |
| Agent | One agent CLI that office-harness runs. | name, CLI command, working folder, project (optional), status | Name is unique in the office. Belongs to one machine and to at most one project. |
| Office additions | What office-harness adds when it starts one agent. | flags, environment variables, startup prompt | Belongs to exactly one agent. |
| Session | One run of an agent CLI. | start time, end time, exit result, recent output, continued-from session (optional) | Belongs to one agent. An agent has at most one running session. |
| Action record entry | One create, start, stop, or delete action. | actor, target agent, action, time, result | Belongs to the office and shows in the target agent's details. |

## Success criteria

- **SC-001:** On the same machine, 95% of typed characters appear in the attached terminal within 100 ms.
- **SC-002:** On each of Linux, macOS, and Windows, a user completes a full task in each of `claude`, `codex`, `gemini`, and `opencode` through the attached terminal (start, prompt, approve a tool use with the keyboard, interrupt with Ctrl+C, continue a previous session) without opening any other terminal.
- **SC-003:** After the desktop app stays closed for 60 minutes, 100% of the agents that were running still run, and the app shows their output from that time on reopen, within the recent-output limit.
- **SC-004:** After a machine restart and login, the background server runs within 60 seconds without the desktop app, and 100% of agent definitions, projects, office additions, and session lists are present.
- **SC-005:** During a test office session with a CLI that writes nothing itself, office-harness changes 0 files in that CLI's configuration files and folders.
- **SC-006:** 100% of connection attempts without a valid office token are refused, and 0 connection attempts from another machine succeed.
- **SC-007:** During 1 hour of operation, office-harness's own components open 0 network connections to addresses outside the machine. Connections that the agent CLIs open are not counted.
- **SC-008:** A first-time user creates an agent and types into its terminal within 2 minutes of the first app start, without editing any file.
- **SC-009:** After the user edits an agent's office additions, 100% of the edited flags, environment variables, and the startup prompt are in effect in the next session, and 0 of them change the running session.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The office has no agents. | Empty | The app shows the empty state with the create action (US1 scenario 7). |
| An agent produced no output yet. | Empty | The terminal is blank, and the status shows `starting` or `working`. |
| The CLI command does not exist or cannot start. | Invalid | The agent shows `failed` with the error text. Other agents are not affected. |
| The working folder or the project folder does not exist. | Invalid | The system refuses to start the agent and names the missing folder. |
| Two agents or two projects get the same name. | Invalid | The system refuses the second one and names the conflict. |
| An office addition is malformed, for example an environment variable without a name. | Invalid | The system refuses to save it and names the entry. |
| The user deletes a project that still has agents. | Invalid | The system refuses and names the agents that are still assigned to it. |
| The agent's CLI does not support continuing a session. | Boundary | The system does not offer "continue" for that agent (FR-025). |
| The output is longer than the recent-output limit. | Boundary | The system keeps the newest output up to the limit and drops the oldest. The terminal says that older output was dropped. |
| Two clients are attached to the same terminal. | Concurrent | Both see the same output, and keys from both reach the CLI. The terminal size follows the client that resized last. |
| The user starts a new task while a session runs. | Concurrent | The system asks for confirmation, then ends the running session and starts a new one. |
| Two clients edit the same agent's office additions. | Concurrent | The last save wins, and the other client shows the saved values. |
| The user deletes an agent while another client is attached to it. | Concurrent | The other client shows that the agent was deleted and closes the terminal. |
| A client has no token, a wrong token, or an old token. | Unauthorized | The server refuses it (FR-007, FR-010), and the desktop app shows the unauthorized state (FR-027). |
| A connection comes from another machine. | Unauthorized | The server refuses it (FR-008). |
| The desktop app crashes. | Failure | The agents keep running (FR-019). |
| The background server stops, the user logs out, or the machine restarts. | Failure | At the next server start, the agents that ran show `interrupted`, and none restarts on its own (FR-004). |
| An agent CLI crashes or exits with an error. | Failure | The agent shows `failed` with its last output. |
| The disk is full when the system saves state or output. | Failure | The system tells the user that it cannot save, keeps the agents running, and keeps the last saved office state readable. |

## Assumptions

- **A1:** The quiet period is 5 seconds by default, and the user can change it. The MVP does not tell "finished" apart from "asks a question": both show as `waiting for input`.
- **A2:** The recent-output limit is 10,000 lines per agent by default, and the user can change it.
- **A3:** Stopping an agent first asks its CLI to end, then ends it by force after 10 seconds.
- **A4:** In this change, the actor in the action record is always the user. Change 0002 adds agents as actors.
- **A5:** Office additions are set per agent. The MVP has no shared presets and no inheritance.
- **A6:** An agent's working folder is its project's folder by default. An agent without a project needs an explicit folder.
- **A7:** The office token protects against other OS users and against programs that do not run as the office owner. It is not a sandbox against an agent CLI that runs as the same OS user, because such an agent can read the user's files.
- **A8:** One background server runs one office for one OS user. Two OS users on one machine have two separate offices.
- **A9:** The desktop app on the same machine gets the office token without a user action, because it runs as the same OS user.
- **A10:** The office runs only while the user is logged in. Logout or shutdown ends the agents, and the office state stays (FR-003, FR-004). This is an owner decision (see Clarifications).
- **A11:** The MVP has no automatic update check, because FR-005 forbids outside connections from office-harness.
- **A12:** The focus shortcut uses a key combination that common agent CLIs do not use, and the user can change it. The default is `Ctrl+Shift+F12`. KDE, GNOME, Windows, and macOS do not take it globally, so it reaches the app. The earlier choice, `Ctrl+Alt+Escape`, is the global "Kill a window" shortcut on KDE and never reaches the app there.
- **A13:** Later releases stay open. R5: each agent belongs to a machine, and every client, the desktop app included, uses the office token, so a remote client later needs no second access model. R7 and R8 can use the agents, statuses, and terminals that this change defines.

## Clarifications

### Session 2026-11-15

- Q: Which agent CLIs must work on day one? → A: `claude`, `codex`, `gemini`, and `opencode`, with full support including session continuation (FR-025, SC-002).
- Q: Must agents keep running after the user logs out? → A: No. Agents run only while the user is logged in (A10).
- Q: Should the MVP spec be split? → A: Yes, into `0001-office-server-and-terminal`, `0002-agent-hierarchy`, and `0003-panel-practicality`. 0002 and 0003 depend on 0001.

## Out of scope

- The agent hierarchy (CEO, team leads, members), directives, and agents that create agents: change 0002.
- The one-list overview with last output, one-step terminal access, the project switcher, notifications, and resume of the last view: change 0003.
- R5 distributed machines: agents on other machines, a shared control plane, connections from other machines, and TLS.
- R7 office scene and R8 meeting room.
- Server installation with Docker Compose.
- A web client or a mobile client.
- Automatic restart of interrupted agents.
- Agents that keep running after the user logs out (A10).
- Shared presets for office additions (A5).
- Session continuation for CLIs other than `claude`, `codex`, `gemini`, and `opencode`.
- Voice input, screen sharing, and project boards, as `docs/PRD.md` section 6 states.
- Any office-harness change to a CLI's own configuration files (FR-021).
- **Split:** done by owner decision on 2026-11-15 (see Clarifications). This change keeps 28 FRs, above the limit of ten, because the owner set its scope.

## Deviations

Recorded by task T5 (terminal module) of the plan, where the build differs from the design text.

- **The `plan:` front matter names a local file.** `.agents/work/office-server-and-terminal/plan.md` exists in the owner's checkout. The folder `.agents/` is git-ignored, so the file is not in the repository. Recorded 2026-11-23 (ACF-106 archive).
- **FR-011 lists four actions; the record holds more.** Requirement: record each create, start, stop, and delete action. Shipped: the record also holds `new_task` and `continue` (0001, T7), `team_create`, `team_delete`, `directive`, and `refused` (0002), `hold` and `release` (0007), and `task_text` (0011), 13 kinds in all (`src/modules/audit/domain/src/action_kind.rs:9`). Reason: each later change records its own actions in the same record, and the user sees them in the same details. The data model lists them. Recorded 2026-11-23 (ACF-106 archive).
- **Status wire value is `waiting_for_input`.** FR-014 and the User experience table write the display text `waiting for input`. The server stores and sends `waiting_for_input` (`src/modules/agent/domain/src/enums/agent_status.rs:48`). Reason: the wire value is one token. The contract and the data model say so. Recorded 2026-11-23 (ACF-106 archive).
- **Windows stop uses `taskkill`, not a Job Object.** Research R2 item 4 asks for a Job Object. The server ends the tree with `taskkill /PID <pid> /T /F` instead, because a Job Object needs `unsafe` code and the workspace forbids it outside `private-fs`. The gap: `/T` follows parent links, so it misses a descendant whose parent already exited. The end runs only while the root child is alive, because a freed process id can belong to an unrelated process. The Windows checks for T5 (f) and (h) stay BLOCKED under PB-3 (no Windows machine) until someone runs them. Accepted for now on 2026-11-15; the project owner may revisit it. The Job Object decision waits for Windows QA (PB-3).
- **One scoped `unsafe` call turns Ctrl+C back on for the agents on Windows.** A process that is started in a new process group ignores Ctrl+C, and its children inherit that, so a stop could end an agent only after the grace period. The crate `console-control` calls `SetConsoleCtrlHandler(NULL, FALSE)` once when the server starts, before it starts any agent. The workspace still forbids `unsafe` code. Like `private-fs`, this crate denies it, and one Windows-only module allows it. If the call fails, the server logs a warning and keeps working; the forced end after the grace period is then the only way a stop ends. On Windows, the desktop app does not start the server in a new process group (`src/apps/desktop/src-tauri/src/server_process.rs:140`, `DETACHED_PROCESS` only). On Linux and macOS it does: `command.process_group(0)` (`server_process.rs:134`) keeps a Ctrl+C of the app's terminal away from the server. Corrected 2026-11-23 (ACF-106 archive). Accepted by the owner on 2026-11-16.
- **The PTY host sets the terminal type.** Every agent starts with `TERM=xterm-256color` and `COLORTERM=truecolor`, set after the login environment and before the office additions, so an addition can replace them. A server started by a service has no terminal to inherit them from, and every terminal emulator sets them for its children. Accepted on 2026-11-15; the project owner may revisit it.
- **The login shell starts in its own process group, not its own session.** A new session needs `pre_exec`, which is `unsafe`, and `Command::setsid` is not stable. Forced-interactive bash opens the controlling terminal itself, so a server started from a terminal can see the shell stop on that terminal; the spawn then falls back after the 5-second limit. A server started at login has no controlling terminal and is not affected. Accepted on 2026-11-15; the project owner may revisit it.
- **One output file for each session.** Task T7 replaced the temporary "one output file for each agent": the output file is named by the session id, and `terminal.attach` shows the file of the agent's last session when the agent does not run. See the T7 entries below.
- **No console window on Windows** comes from `windows_subsystem = "windows"` in the server's `main.rs`. It is untested on Windows (PB-3).

Recorded by task T6 (agent and project API) of the plan. Each is a decision of T6 that waits for acceptance.

- **`settings.set` for the quiet period and the output limit applies after a server restart.** A1 and A2 say the user can change them, and they are stored at once, but the terminal runtime reads them when the server starts. The stop grace period (A3) and the focus shortcut apply at once. A live change for new sessions can be added later without a protocol change.
- **`agent.start` returned the agent's status, not a session.** Fixed by task T7: the result of `agent.start`, `agent.new_task`, and `session.continue` is now a session. The status and the failure text still reach clients in the `agent.status` event.
- **Agent requests that change a status or a terminal run one at a time.** `agent.start`, `agent.stop`, and `agent.delete` take one lock that the status bridge also takes. A stop that waits for its grace period (A3, 10 seconds by default) therefore delays the status events of other agents for that time.
- **A status bridge that falls behind logs the gap but does not resync yet.** If the terminal module produces more than 256 events before the status bridge reads them, the bridge writes an error line to `server.log`, and an agent status can stay old until its next event. A resync of the statuses from the running terminals is left for a later task. Clients that fall behind are closed with `1011` and reload, so they do not keep an old status.

Recorded by task T7 (office additions, sessions, and CLI adapters) of the plan. Each is a decision of T7 that waits for acceptance.

- **`gemini` continues with `--resume <uuid>`, not an index.** Research R3 says `--resume` takes an index from `--list-sessions`, and that a UUID is unverified. The adapter passes the id that office-harness chose with `--session-id` and continues with `--resume <that id>` (`src/modules/agent/infrastructure/src/services/cli_adapters/gemini.rs:22,32`). It has no mapping to an index and does not call `--list-sessions`. The hands-on check T7(g) with the real `gemini` is still open. If it shows that only an index works, the mapping is new work. The same check settles the id format of `opencode` (`--session <id>`).
- **The CLI session id of `claude`, `gemini`, and `opencode` comes from the session's own id.** The adapter gets the office session id as the id to propose, so one UUID names the session in the database, in its output file, and in the CLI. `opencode` accepts only ids that start with `ses`, so its adapter passes `ses_` followed by the UUID without hyphens (research R3, checked by hand with `opencode` 2.2.5).
- **The startup prompt is pasted after the first output and is never submitted.** For every CLI (`claude`, `codex`, `gemini`, `opencode`) and for a generic command, the server pastes the startup prompt into the terminal after the run's first output. It never sends Enter, so a CLI dialog that is shown first (a trust or permission question) is never answered by office-harness; the user presses Enter to submit the prompt. The paste is wrapped in the bracketed-paste markers when the CLI turned that mode on. A CLI that sets the mode later than its first output gets a plain paste. A submit per adapter may return after the check with the real CLIs, which is part of T7(g). Without the bracketed-paste markers, a line break inside the prompt reaches the CLI as a key press, so T7(g) also checks a multi-line prompt against each real CLI's first screen.
- **`codex` is attributed by its session files.** The adapter lists `<CODEX_HOME or home>/.codex/sessions` before the start and reads the first line of each new `rollout-*.jsonl` file (`payload.id` and `payload.cwd`). That format is unverified until T7(g). It looks for 15 seconds. Two starts that overlap in one folder both give up, because neither can know which file is its own (R3). A start keeps its claim for one more look period after it found nothing, so a late file is not given to a later start in that window; a file that appears more than two look periods after its start can still be matched to a later start in the same folder.
- **No reason is stored for a session without an id.** `session.list` computes `can_continue` and `reason` from the adapter, the session, and its exit result. A session whose id was not matched says that office-harness "could not tell which of the CLI's sessions belongs to this one"; the reasons "not matched" and "ambiguous" are not told apart.
- **Office additions are one JSON document in one column** (`agent_office_additions.additions`), and a session's snapshot is the same document, not separate columns for flags, environment, and prompt.
- **A session ends as `signal` only when the user stops it.** A child that a signal ended from outside shows its exit code, because the terminal reports only a code.
- **The server ends the running agents when it stops cleanly.** A signal and `server.restart` stop each child, set the agent and its session to `interrupted`, and tell the connected clients before they close with 1001. Before T7 the statuses changed only at the next start (FR-004), which still covers an unclean stop. `server.restart` writes no action record, because the action record has no entry for the server.
- **The output file of a deleted agent stays.** Deleting an agent deletes its additions and sessions, but the output files of those sessions remain in `output/`. A later change can remove them.
- **The login environment resolves before the status `starting`.** A start now finds the program and builds the command line first, because the refusal of an unsafe argument must leave the status unchanged. A slow login shell therefore delays `starting` by up to its 5 seconds.
- **The Windows rules of T7(e) run on Linux with a fake `.cmd` file.** The checks use the same code path (`.cmd` extension, typed prompt, refused `%`), but the real `cmd.exe` is BLOCKED under PB-3 until someone runs it on Windows.

Recorded by task T11 (login start and installers) of the plan. Each is a decision of T11 that waits for acceptance.

- **`unregister` turns login start off for the app too.** Following plan D13, the app registers only on its first start and on the first start of a new version (this version has no copy under `bin/` yet). `office-harnessd unregister` also writes the file `login-start-off` in the data folder, and the app then does not register again; the "Start the server at login" button and `office-harnessd register` remove the file.
- **The subcommands use only the default data folder.** The login item starts the server without arguments, so a login item for another folder would start a server on the default one. `register`, `unregister`, and `status` refuse every argument with exit code 2; the app's start passes the one internal flag `--at-app-start`.
- **`register` does not start the server, and it does not log the chosen mode.** Research R4 says that `register` runs `systemctl --user start` and records the mode in `server.log`. The app starts the server itself when it is not running, and the server starts at the next login, so no extra start is needed. The mode is visible in the login item itself.
- **Old copies are not removed.** Each version keeps its folder under `bin/`, so a running older server keeps its program file. `unregister` removes all of them.
- **On Linux, `unregister` and `status` see the systemd user unit only when `systemctl --user` answers.** Without a user bus they see only the XDG autostart entry. The README tells the user to run them in the desktop session.
- **A systemd drop-in stops restarts of a second server.** The crate's unit has `Restart=on-failure`, and a second server exits with code 75 because one already runs. `register` writes `office-harnessd.service.d/office-harness.conf` with `RestartPreventExitStatus=75`, and `unregister` removes it.
- **The login item path is quoted, and some characters are refused.** The crate writes the Linux `ExecStart` and `Exec` lines and the Windows `Run` value without quotes. office-harness quotes the path and refuses `"` (all three) and `\`, `$`, `` ` ``, `%`, and a line break (Linux), instead of escaping them for each format.
