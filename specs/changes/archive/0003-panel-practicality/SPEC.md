---
status: shipped
capability: agent-panel
plan: none (delivered before plan files were kept)
supersedes: 0001/FR-017
---

<!-- Approved under the overnight directive 2026-11-17; owner may amend. -->

# 0003: Panel practicality

## Intent

The owner named switching and searching between agents as his main problem: other tools make him walk through a scene or open many windows to find an agent and its result. This change makes the desktop app one panel in which he sees every agent of every project, opens any agent's terminal in one step, switches projects in one step, and gets a notification when an agent finishes or needs him. Opening the app shows the office where he left it. This change covers R1 and R9 of `docs/PRD.md`.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4.
- The MVP is three change specs: `0001-office-server-and-terminal`, `0002-agent-hierarchy`, and `0003-panel-practicality` (this one). This change depends on 0001: it uses 0001's agents, projects, statuses, terminals, and desktop app. It does not depend on 0002; where 0002 is present, the panel also shows roles and teams (FR-003). References such as `0001/FR-014` point to requirements in 0001.
- The four R9 criteria in `docs/PRD.md` section 4.2 are approved by the owner as written (see Clarifications). FR-004 to FR-007 state them.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it. The desktop app runs on Linux, macOS, and Windows.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, status, quiet period, desktop app | As defined in 0001. |
| Panel | The main view of the desktop app. Any later view (R7, R8) opens from it and does not replace it. |
| Agent list | The list in the panel that shows every agent of the current project choice. |
| Project switcher | The control that picks one project or "All projects" for the agent list. |
| Last output line | The last non-empty line of an agent's terminal text, without color and cursor codes. |
| Notification | A message that tells the user that an agent finished, failed, or needs him. |
| Last view | The project choice, the selected agent, and the open terminal at the time the desktop app closed. |

## User stories

### US1 — See and reach every agent from one panel (Priority: P1)

The user sees every agent of every project in one list, opens any agent's terminal in one step, and switches projects in one step.

**Why this priority:** this removes the switching and searching that the owner named as the main problem (R1, R9). It is the smallest slice of this change with value on its own.

**Independent test:** create 12 agents in 3 projects. From the agent list, open each agent's terminal with one click, open one with its keyboard shortcut, and switch between the 3 projects and "All projects" with one action each. No second window opens.

**Scenarios:**

1. Given 12 agents run in 3 projects, When the user opens the panel with "All projects" selected, Then one list shows all 12 agents, each with name, project, status, and last output line.
2. Given the agent list shows agents, When the user clicks an agent's row, Then that agent's live terminal opens in the same window and gets keyboard focus.
3. Given the terminal of agent A has keyboard focus, When the user presses agent B's shortcut, Then agent B's live terminal opens and gets keyboard focus.
4. Given the panel shows "All projects", When the user picks one project in the project switcher, Then the list shows only that project's agents.
5. Given the office has a hierarchy from 0002, When the user looks at the agent list, Then each row also shows the agent's role and team.

### US2 — Get told when an agent finishes or needs me (Priority: P2)

The user gets a notification when an agent finishes, fails, or waits for him, also for events while the app was closed.

**Why this priority:** without it, the user must watch the list. It needs the list from US1 to show where the event came from.

**Independent test:** with the app open, let one agent exit and one agent stop producing output. The user gets a notification for each. Close the app, let a third agent exit, open the app, and see that notification as unread.

**Scenarios:**

1. Given an agent is `working`, When it produces no output for the quiet period, Then its status changes to `waiting for input` and the user gets a notification with the agent's name.
2. Given an agent runs, When its CLI exits, Then its status changes to `exited` or `failed` and the user gets a notification.
3. Given an agent runs, When its CLI rings the terminal bell, Then the user gets a notification that the agent needs him.
4. Given the desktop app was closed while agents changed status, When the user opens the app, Then each notification from that time shows as unread.
5. Given a notification is unread, When the user opens it, Then the agent's terminal opens and the notification shows as read.
6. Given an agent has unread notifications, When the user opens that agent's terminal by any route, Then those notifications show as read.
7. Given the user stopped an agent, When its status changes to `stopped`, Then no notification occurs.

### US3 — Open the app where I left it (Priority: P3)

When the user opens the desktop app, the panel shows the project, agent, and terminal he had open last, with the current state of every agent.

**Why this priority:** it saves the steps of finding the place again after each app start, but US1 already makes that a one-step task.

**Independent test:** open project P and agent A's terminal, close the app, let agent A produce output, and open the app again. The panel shows project P and agent A's terminal with the new output.

**Scenarios:**

1. Given the user had project P and agent A open when he closed the desktop app, When he opens the app again, Then the panel shows project P with agent A's terminal open, and each agent shows its current status.
2. Given the agent of the last view was deleted, When the user opens the app, Then the panel shows the last project choice, opens no terminal, and says that the agent no longer exists.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL show the agent list, the agent terminals, and the agent details in one window, without a separate window per agent or per project.
- **FR-002** (US1): THE SYSTEM SHALL show, in one list, the name, project, status, and last output line of every agent in every project.
- **FR-003** (US1): WHERE the office has a hierarchy from 0002, THE SYSTEM SHALL also show each agent's role and team in the agent list.
- **FR-004** (US1, R9 criterion 1): WHEN the user clicks an agent in the panel or presses that agent's keyboard shortcut, THE SYSTEM SHALL open that agent's live terminal.
- **FR-005** (US1, R9 criterion 3): WHEN the user picks one project or "All projects" in the project switcher, THE SYSTEM SHALL show only the agents of that choice.
- **FR-006** (US2, R9 criterion 4): WHEN an agent exits, fails, changes from `working` to `waiting for input`, or rings the terminal bell, THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and (b) for the agent whose terminal is open in the focused desktop app window, the event shows in the notification list as read and sends no OS notification.
- **FR-007** (US2): IF a notification occurs while the desktop app is closed, THEN THE SYSTEM SHALL show it as unread when the app opens.
- **FR-008** (US3): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view.
- **FR-009** (US2): WHEN the user opens a notification, or opens the terminal of an agent that has unread notifications, THE SYSTEM SHALL show those notifications of that agent as read.

FR-002 states R9 criterion 2. FR-006 notifies only for the four events it lists: a status change that the user caused (`stopped`) and `interrupted` after a server restart notify nobody.

Later additions to the shortcuts and the events (recorded 2026-11-23; no requirement of this spec changed). The fixed shortcuts are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` here. Change 0008 added `Ctrl+Shift+F11` (jump to the agent that needs you), and change 0007 added `Ctrl+Shift+H` (the office hold). The server holds 12 reserved shortcuts in all (`src/modules/agent/domain/src/value_objects/settings.rs:33`). Change 0007 (FR-008) added the notification event `auto_hold` to the four events of FR-006, so the server knows five (`src/modules/notification/domain/src/enums/notification_event.rs:21`). Change 0008 added the Needs you queue. It is a separate queue with its own requests and event, not a notification event.

### MODIFIED

- **0001/FR-017** (US1): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts: the shortcut that moves focus back to the panel, the agent shortcuts, and the switcher shortcut (A1).

### PRESERVED

- **PB-1:** Closing the desktop app keeps every agent running and recording its output (0001/FR-019).
- **PB-2:** office-harness sends no office data outside the user's machine; notifications are local to the machine (0001/FR-005).
- **PB-3:** Every desktop app action is available from the keyboard, including the actions that this change adds (0001/FR-028).
- **PB-4:** Every status shows as text, never color alone, in the agent list too (0001/FR-014).
- **PB-5:** The loading, server-not-reachable, and not-authorized states of 0001 still show before the panel (0001/FR-026, 0001/FR-027).

## User experience

**Flow:**

1. The user opens the desktop app. After the 0001 connection states, the panel shows the last view, or "All projects" on first use.
2. The agent list shows every agent of the current project choice with its name, project, status, and last output line, plus role and team when 0002 is present.
3. The user clicks a row or presses an agent's shortcut. The agent's live terminal opens in the same window and gets keyboard focus.
4. The user picks a project or "All projects" in the project switcher. The list changes in one step.
5. A notification arrives when an agent finishes, fails, or needs the user. Opening it opens that agent's terminal.
6. The user closes the app. On the next start, the panel shows the last view and the unread notifications.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading, server not reachable, unauthorized | As in 0001. | PB-5 |
| Success | The agent list for the current project choice, and the open terminal. | FR-002, FR-004, US1 scenarios 1 to 4 |
| Empty office | The office is empty, with the action that creates the first agent (0001). | 0001 US1 scenario 7 |
| Empty project | The project has no agents, with the action that creates one. | Edge cases |
| Unread notifications | The number of unread notifications as text, and the list of them. | FR-007, FR-009, US2 scenarios 4 to 6 |
| Last view gone | The last agent or project no longer exists; the panel says so. | US3 scenario 2, Edge cases |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The agent list, the project switcher, and the notification list work from the keyboard (PB-3). Keyboard focus is always visible.
- Each agent row has an accessible name that includes the agent's name, project, and status, plus role when 0002 is present.
- The unread count is text, and screen readers announce a new notification.
- Status is text, optionally with an icon, never color alone (PB-4).
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | last output line | As in 0001. |
| Notification | A status event that the user must see. | agent, event (`exited`, `failed`, `waiting for input`, bell), time, read or unread | Belongs to the office. Refers to one agent. |
| Last view | Where the user was when the desktop app closed. | project choice, selected agent, open terminal | One per desktop app on the machine. |

## Success criteria

- **SC-001:** With 30 agents across 3 projects that run at the same time, the agent list in one window shows all 30, and each status change and each change of the last output line shows in the list within 2 seconds.
- **SC-002:** From the panel, the user opens any agent's live terminal with 1 click or 1 shortcut, and the terminal shows live output within 1 second.
- **SC-003:** The user switches from one project to another, or to "All projects", with 1 action.
- **SC-004:** While the desktop app runs, the user gets a notification within 10 seconds after an agent exits, fails, rings the bell, or changes to `waiting for input` (that is, within 10 seconds after the quiet period ends).
- **SC-005:** After the desktop app was closed while 3 agents changed status, 100% of those notifications show as unread on reopen.
- **SC-006:** When the user opens the desktop app, the panel shows the last view within 3 seconds.
- **SC-007:** With 30 agents running, a user finds the agent that waits for him and types into its terminal within 10 seconds of the notification.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| A project has no agents. | Empty | The list says that the project has no agents and offers the create action. |
| An agent produced no output yet. | Empty | The last output line is blank, and the status shows `starting` or `working`. |
| An agent has no project. | Empty | It shows only under "All projects". |
| The last output line is longer than the row or contains color and cursor codes. | Boundary | The row shows the text without codes, shortened to fit, and the full text stays in the terminal. |
| More than nine agents are in the list. | Boundary | The first nine have direct shortcuts; the switcher shortcut reaches every other agent (A1). |
| Many agents finish at the same moment. | Boundary | Each event becomes its own notification, and the panel lists all of them. |
| The user changes the focus shortcut (0001 A12) to a key combination that an agent shortcut or the switcher shortcut uses. | Invalid | The system refuses it and names the conflict. |
| The agent or the project of the last view was deleted. | Invalid | The panel shows "All projects" or the last project, opens no terminal, and says what no longer exists (US3 scenario 2). |
| A notification refers to an agent that was deleted. | Invalid | The notification stays and says that the agent was deleted; opening it opens no terminal. |
| One agent rings the bell again while its bell notification is still unread. | Boundary | No second notification occurs; the unread one stays. |
| An agent changes status while the user reads the list. | Concurrent | The row updates in place, and the selection and keyboard focus stay where they are. |
| The OS blocks notifications for the desktop app. | Failure | The panel still shows each notification as unread. |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (PB-5). |

## Assumptions

- **A1:** The first nine agents in the current list have direct keyboard shortcuts, and one shortcut opens a switcher that finds any agent by name. For FR-004 and SC-002, the switcher shortcut counts as the one shortcut for agents without a direct shortcut. App shortcuts use key combinations that common agent CLIs do not use. The MVP ships fixed agent and switcher shortcuts (see Clarifications); only the focus shortcut of 0001 A12 is changeable.
- **A2:** While the desktop app runs, notifications show as OS notifications and as unread items in the panel. While it is closed, the background server keeps them (FR-007).
- **A3:** The MVP does not tell "finished" apart from "asks a question": both show as `waiting for input` after the quiet period (0001 A1). A terminal bell counts as "needs the user".
- **A4:** The last view is kept per desktop app on the machine, not per agent or per project.
- **A5:** Notifications stay until the user reads them. The MVP has no automatic cleanup of old notifications.
- **A6:** Later releases stay open: the R7 office scene and the R8 meeting room open from the panel and do not replace it (`docs/PRD.md` section 4.1).
- **A7:** The exception in FR-006(b) applies only to events that occur while the window has focus; the user sees the agent's terminal then, so an OS notification adds nothing.

## Clarifications

### Session 2026-11-15

- Q: Do the four proposed R9 criteria in `docs/PRD.md` section 4.2 hold? → A: Yes, approved as written (FR-002, FR-004, FR-005, FR-006).
- Q: Should the MVP spec be split? → A: Yes, into `0001-office-server-and-terminal`, `0002-agent-hierarchy`, and `0003-panel-practicality`. This change depends on 0001.

### Session 2026-11-17 (overnight directive)

Decisions with reversible defaults, so that the plan can start. The owner may amend any of them.

- Q: Which keys are the agent and switcher shortcuts? → A: Agent 1 to 9 use `Ctrl+Shift+F1` to `Ctrl+Shift+F9`, and the switcher uses `Ctrl+Shift+F10`, the same family as the focus shortcut `Ctrl+Shift+F12` (0001 A12). On most macOS keyboards the F-keys need the Fn key (or the system setting that makes F1, F2 act as standard function keys), so the plan checks the defaults against each OS and the webview and may change them. Reversible: the keys are one table.
- Q: Can the user change the agent and switcher shortcuts? → A: Not in the MVP. The shortcuts are fixed and documented; only the focus shortcut stays changeable (0001 A12). Reversible: a later change can add settings without changing FR-004.
- Q: What clears an unread notification? → A: Opening it, or opening that agent's terminal (FR-009). The MVP has no "mark all as read" action (A5).
- Q: Does a bell that repeats flood the list? → A: No. One unread bell notification per agent; more bells add nothing while it stays unread (edge cases).
- Q: Do `stopped` and `interrupted` notify? → A: No. The user caused `stopped`, and `interrupted` comes from a restart (FR-006).
- Q: Does the user get an OS notification for the agent that he looks at? → A: No (A7).
- Q: How does the last output line stay current for 30 agents? → A: The plan decides. The requirement is SC-001: each status or last-line change shows within 2 seconds.

The two entries below sharpen the shortcut decisions of 2026-11-17 with the `research.md` R1 findings. They add no requirement and change none. Approved by the owner on 2026-11-19.

- Q: What does the office document for macOS in its default media-key mode? → A: In the default mode the top-row keys send media and brightness commands, and macOS hands F1 to F12 to the app only while Fn (or Globe) is held or "Use F1, F2, etc. keys as standard function keys" is turned on (`research.md` R1). `Ctrl+Shift+F1` to `Ctrl+Shift+F10` can therefore be unusable out of the box on macOS. The README keeps the Fn note and records the fallback table below, so the user who cannot change the setting switches the macOS build to the `Cmd+Alt` family instead.
- Q: Are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` verified on KDE Plasma? → A: Not yet. The primary Plasma documentation binds no `Ctrl+Shift+F1` to `Ctrl+Shift+F12`, but a secondary source names the family among Plasma "standard" bindings, so the evidence that the keys are free is one-sided (`research.md` R1). Until the hands-on QA pass checks Plasma live, the office treats the defaults as unverified on KDE; when a distribution binds the family globally, the fallback is the table below's last row.

The fallback table from `research.md` R1 (recorded, approved with the entries above):

| Default | Where it fails | Fallback |
|---|---|---|
| `Ctrl+Shift+F1` to `Ctrl+Shift+F9` | macOS in default media-key mode, unless Fn is held | Keep the key and document Fn (the Clarifications already expect this), or switch the macOS build to `Cmd+Alt+1` to `Cmd+Alt+9`, which no webview or macOS default takes |
| `Ctrl+Shift+F10` | same as above | `Cmd+Alt+0` on macOS |
| `Ctrl+Shift+F1` to `Ctrl+Shift+F10` | KDE, if a distribution binds the Plasma "standard" family globally | `Cmd`/`Ctrl+Alt+1..9,0` family; one table entry per key, so a per-OS build change is small |

## Out of scope

- Search across terminal output of all agents.
- Grouping or sorting options for the agent list beyond the project switcher.
- Notifications on other devices, by mail, or by chat, and any notification that leaves the machine (PB-2).
- Telling "finished" apart from "asks a question" (A3).
- The agent hierarchy: change 0002.
- R5 distributed machines, R7 office scene, and R8 meeting room.
- **Split:** done by owner decision on 2026-11-15 (see Clarifications).
