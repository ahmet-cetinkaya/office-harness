# Server and terminal

## Intent

A background server keeps the agent CLIs running on one machine, and the desktop app attaches to their live terminals. This capability holds the base behavior: the server, the protocol clients use, the agent list, the terminal, stop and restart, and the storage. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0001-office-server-and-terminal](../changes/archive/0001-office-server-and-terminal/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0001/FR-014 | 0018, 0024 | Change 0001, Requirements |
| 0001/FR-017 | 0003, 0008 | [Change 0008](../needs-you-queue/SPEC.md), Requirements (the entry under Change 0001 is read with it) |
| 0001/FR-020 | 0024 | Change 0001, Requirements |

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003, 0011: [Agent panel](../agent-panel/SPEC.md)
- 0024: [Exact agent status from CLI hooks](../agent-status/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0018: [Idle sleep](../idle-sleep/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)

## Change 0001: Office server and terminal

### Intent

The owner runs many AI coding agent CLIs and opens a separate terminal for each one to follow and steer it. This change gives him the base of office-harness: a background server that keeps his agents running while the desktop app is closed, and a desktop app in which he attaches to any agent's live terminal and types into it. The server runs only on his own machine, accepts only clients with a token, and never writes the CLIs' own configuration. This change covers R3, R4, and R6 of `docs/PRD.md` and the rules in its section 4.1. Changes 0002 (hierarchy) and 0003 (panel practicality) build on it.

### Requirements

- **0001/FR-001** (US1): THE SYSTEM SHALL run the office in a background server that keeps every agent running while no desktop app is open.
- **0001/FR-002** (US1): WHEN the user logs in to the OS, THE SYSTEM SHALL start the background server without a user action and without opening the desktop app.
- **0001/FR-003** (US1, US2): THE SYSTEM SHALL keep the office (agent definitions, projects, office additions, session lists, action record, and recent output of each agent) across restarts of the background server and of the machine.
- **0001/FR-004** (US1): IF the background server stops while agents run, THEN THE SYSTEM SHALL show each of those agents as `interrupted` at the next server start and SHALL NOT start any of them without an explicit action.
- **0001/FR-005** (US1): THE SYSTEM SHALL NOT send office data (agent definitions, terminal output, configuration, or usage data) to any service outside the user's machine.
- **0001/FR-006** (US1): THE SYSTEM SHALL provide the background server and the desktop app on Linux, macOS, and Windows.
- **0001/FR-007** (US1): THE SYSTEM SHALL let a client read terminals, type into terminals, or control agents only after the client presents a valid office token.
- **0001/FR-008** (US1): THE SYSTEM SHALL accept client connections only from the same machine.
- **0001/FR-009** (US1): THE SYSTEM SHALL make the office token and the stored terminal output readable only by the OS user who owns the office.
- **0001/FR-010** (US1): WHEN the user regenerates the office token, THE SYSTEM SHALL reject the old token and close every open connection that used it.
- **0001/FR-011** (US1): THE SYSTEM SHALL record each create, start, stop, and delete action with its actor, its target agent, and its time, and SHALL show these entries in the target agent's details.
- **0001/FR-012** (US1): THE SYSTEM SHALL let the user create an agent with a name, a CLI command, a working folder, and an optional project, and start, stop, and delete it from the desktop app.
- **0001/FR-013** (US1): THE SYSTEM SHALL let the user create a project with a name and a folder, and assign each agent to at most one project.
- **0001/FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`, or `asleep`; for an estimated or late session, `waiting for input` means that the CLI runs and produced no output during the quiet period; for an exact session it means that no turn runs and no compaction runs, or that a permission ask is open (0024/FR-011, 0024/FR-013); and `asleep` means that the office ended the CLI after an idle time and keeps its last session for a wake (0018/FR-003). (modified by 0018, 0024)
- **0001/FR-015** (US1): THE SYSTEM SHALL run each agent CLI in its own pseudo-terminal, so that full-screen display, colors, cursor movement, and control keys behave as in the user's native terminal.
- **0001/FR-016** (US1): WHEN the user attaches to an agent, THE SYSTEM SHALL show the agent's recent output followed by its live output.
- **0001/FR-017** (US1): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts: the shortcut that moves focus back to the panel, the agent shortcuts, and the switcher shortcut (A1). (modified by 0003 and 0008; the final text is under [Change 0008](../needs-you-queue/SPEC.md)) [Change 0007](../hold-and-limits/SPEC.md) widened this list by one shortcut. See the note after the success criteria of change 0007. Active change 0008 (needs you queue, not shipped yet) modifies 0001/FR-017 again: it adds the jump shortcut to the list.
- **0001/FR-018** (US1): WHEN the user resizes the terminal view, THE SYSTEM SHALL resize the agent's terminal to the new size.
- **0001/FR-019** (US1): WHEN the user detaches from a terminal or the desktop app closes, THE SYSTEM SHALL keep the agent running and keep recording its output.
- **0001/FR-020** (US1, US2): THE SYSTEM SHALL start each agent CLI with the user's own home folder, login environment, and CLI configuration, changed only by that agent's office additions and, for `claude`, by the office settings file of 0024/FR-001, which only adds the status hooks. (modified by 0024)
- **0001/FR-021** (US2): THE SYSTEM SHALL NOT create, change, or delete any file in an agent CLI's configuration files or folders.
- **0001/FR-022** (US2): THE SYSTEM SHALL keep each agent's office additions in office-harness's own configuration and show them, editable, in the agent's details.
- **0001/FR-023** (US2): WHEN the user edits the office additions of a running agent, THE SYSTEM SHALL keep the running session on the previous values, apply the change at the next session start, and say so in the agent's details.
- **0001/FR-024** (US1, US2): WHEN a new task starts on an agent, THE SYSTEM SHALL start a new CLI session.
- **0001/FR-025** (US2): WHERE the agent's CLI supports continuing a session, WHEN the user explicitly picks one of the agent's previous sessions, THE SYSTEM SHALL start the CLI so that it continues that session.
- **0001/FR-026** (US1): IF the desktop app cannot reach the background server, THEN THE SYSTEM SHALL show that the server is not reachable and offer an action that starts it.
- **0001/FR-027** (US1): IF the background server rejects the desktop app's office token, THEN THE SYSTEM SHALL show a not-authorized state that tells the user how to give the app a valid token.
- **0001/FR-028** (US1, US2): THE SYSTEM SHALL make every desktop app action available from the keyboard.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Office | Everything one background server manages for one OS user. | owner (OS user), office token | One per OS user per machine. Has projects and agents. |
| Machine | The computer that an agent runs on. | name | The MVP office has exactly one machine. Each agent belongs to one machine. It exists so that R5 can add more. |
| Project | A named folder that groups agents. | name, folder | Name is unique in the office. Has zero or more agents. |
| Agent | One agent CLI that office-harness runs. | name, CLI command, working folder, project (optional), status | Name is unique in the office. Belongs to one machine and to at most one project. |
| Office additions | What office-harness adds when it starts one agent. | flags, environment variables, startup prompt | Belongs to exactly one agent. |
| Session | One run of an agent CLI. | start time, end time, exit result, recent output, continued-from session (optional) | Belongs to one agent. An agent has at most one running session. |
| Action record entry | One create, start, stop, or delete action. | actor, target agent, action, time, result | Belongs to the office and shows in the target agent's details. |

### Success criteria

- **0001/SC-001:** On the same machine, 95% of typed characters appear in the attached terminal within 100 ms.
- **0001/SC-002:** On each of Linux, macOS, and Windows, a user completes a full task in each of `claude`, `codex`, `gemini`, and `opencode` through the attached terminal (start, prompt, approve a tool use with the keyboard, interrupt with Ctrl+C, continue a previous session) without opening any other terminal.
- **0001/SC-003:** After the desktop app stays closed for 60 minutes, 100% of the agents that were running still run, and the app shows their output from that time on reopen, within the recent-output limit.
- **0001/SC-004:** After a machine restart and login, the background server runs within 60 seconds without the desktop app, and 100% of agent definitions, projects, office additions, and session lists are present.
- **0001/SC-005:** During a test office session with a CLI that writes nothing itself, office-harness changes 0 files in that CLI's configuration files and folders.
- **0001/SC-006:** 100% of connection attempts without a valid office token are refused, and 0 connection attempts from another machine succeed.
- **0001/SC-007:** During 1 hour of operation, office-harness's own components open 0 network connections to addresses outside the machine. Connections that the agent CLIs open are not counted.
- **0001/SC-008:** A first-time user creates an agent and types into its terminal within 2 minutes of the first app start, without editing any file.
- **0001/SC-009:** After the user edits an agent's office additions, 100% of the edited flags, environment variables, and the startup prompt are in effect in the next session, and 0 of them change the running session.

### Notes from Deviations

- **FR-011 lists four actions; the record holds more.** Requirement: record each create, start, stop, and delete action. Shipped: the record also holds `new_task` and `continue` (0001, T7), `team_create`, `team_delete`, `directive`, and `refused` (0002), `hold` and `release` (0007), and `task_text` (0011), 13 kinds in all (`src/modules/audit/domain/src/action_kind.rs:9`). Reason: each later change records its own actions in the same record, and the user sees them in the same details. The data model lists them. Recorded 2026-11-23. Changed by 0017: two more kinds, `end_leftover` and `helper_limit`, are in `specs/changes/archive/0017-orphan-sweep/contracts/sweep-protocol.md`.
- **Status wire value is `waiting_for_input` (FR-014).** The display text is `waiting for input`. The server stores and sends `waiting_for_input` (`src/modules/agent/domain/src/enums/agent_status.rs:48`). Reason: the wire value is one token. Recorded 2026-11-23.
- **A clean server stop sets running agents to `interrupted` at once (FR-004).** A signal and `server.restart` stop each child, set the agent and its session to `interrupted`, and tell the connected clients before they close. FR-004 still covers an unclean stop, where the status changes at the next server start.
- **Windows stop uses `taskkill`, not a Job Object (FR-006, SC-002).** The server ends the process tree with `taskkill /PID <pid> /T /F`, because a Job Object needs `unsafe` code and the workspace forbids it outside `private-fs`. `/T` follows parent links, so it misses a descendant whose parent already exited. The Windows checks stay open until someone runs them on a Windows machine. Accepted on 2026-11-15; the project owner may revisit it.
- **The `gemini` continue check is open (FR-025, SC-002).** The adapter continues with `--resume <uuid>`, the id that office-harness chose. The hands-on check with the real `gemini` is still open; if only an index works, the mapping is new work.
- **`opencode` starts with a session id in its own shape (FR-024, FR-025, SC-002).** The adapter passes `--session ses_<12 hex digits><14 hex digits>`: the first 12 digits are the inverted millisecond counter that the CLI itself uses, taken at the start of the session, and the 14 after them come from the office session id. Reason: `opencode` 2.2.5 accepts any id that starts with `ses_`, but its free tier answers "Free usage limit reached" to an id whose counter is not near the present time (6 of 6 random ids refused, ids with a current counter answered), so a plain `ses_` and 32 hex digits made every office start of `opencode` fail at the first prompt on the free tier. The id is recorded when the session starts and a continue passes the recorded id unchanged, so a session recorded with the older shape keeps its id. On the free tier, such an older session is still refused at its next prompt (2 of 2 tried), because the refusal depends on the id; a session on another provider is not affected by the id shape. Recorded 2026-11-27.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0001-office-server-and-terminal - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
