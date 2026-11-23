---
status: shipped
capability: office
plan:
supersedes:
---

# Office

## Intent

The office is the capability that runs and shows many AI coding agent CLIs for one owner on one machine. A background server keeps the agents running. A desktop app attaches to their live terminals, shows them in one panel, and shows an optional office scene. Agents sit in a hierarchy, create and direct other agents, and stay under a hold switch and rate limits. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0001-office-server-and-terminal](../changes/archive/0001-office-server-and-terminal/SPEC.md)
- [0002-agent-hierarchy](../changes/archive/0002-agent-hierarchy/SPEC.md)
- [0003-panel-practicality](../changes/archive/0003-panel-practicality/SPEC.md)
- [0005-office-scene](../changes/archive/0005-office-scene/SPEC.md)
- [0007-hold-and-limits](../changes/archive/0007-hold-and-limits/SPEC.md)
- [0012-safe-directive-delivery](../changes/archive/0012-safe-directive-delivery/SPEC.md)
- [0013-bulk-resume](../changes/archive/0013-bulk-resume/SPEC.md)
- [0014-subtree-status](../changes/archive/0014-subtree-status/SPEC.md)
- [0015-upward-reports](../changes/archive/0015-upward-reports/SPEC.md)
- [0016-agent-file-browser](../changes/archive/0016-agent-file-browser/SPEC.md)

Changes 0008 to 0011 merge in when they are archived. 0004, 0006, and changes after 0016 are not merged.

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.
- Ids of 0001 keep the form `0001/FR-nnn`. The one requirement of 0001 that a merged change modified is 0001/FR-017.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0001/FR-017 | 0003 | Change 0001, Requirements |
| 0002/FR-008 | 0014, 0015 | Change 0002, Requirements |
| 0002/FR-009 | 0007 | Change 0002, Requirements |
| 0002/FR-010 | 0012 | Change 0002, Requirements |
| 0003/FR-006 (in part) | 0007, 0015 | Change 0003, Requirements |
| 0003/FR-008 (in part) | 0005 | Change 0003, Requirements |

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
- **0001/FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, or `interrupted`, and `waiting for input` means that the CLI runs and produced no output during the quiet period.
- **0001/FR-015** (US1): THE SYSTEM SHALL run each agent CLI in its own pseudo-terminal, so that full-screen display, colors, cursor movement, and control keys behave as in the user's native terminal.
- **0001/FR-016** (US1): WHEN the user attaches to an agent, THE SYSTEM SHALL show the agent's recent output followed by its live output.
- **0001/FR-017** (US1): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts: the shortcut that moves focus back to the panel, the agent shortcuts, and the switcher shortcut (A1). (modified by 0003) Change 0007 widened this list by one shortcut. See the note after the success criteria of change 0007. Active change 0008 (needs you queue, not shipped yet) modifies 0001/FR-017 again: it adds the jump shortcut to the list.
- **0001/FR-018** (US1): WHEN the user resizes the terminal view, THE SYSTEM SHALL resize the agent's terminal to the new size.
- **0001/FR-019** (US1): WHEN the user detaches from a terminal or the desktop app closes, THE SYSTEM SHALL keep the agent running and keep recording its output.
- **0001/FR-020** (US1, US2): THE SYSTEM SHALL start each agent CLI with the user's own home folder, login environment, and CLI configuration, changed only by that agent's office additions.
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

- **FR-011 lists four actions; the record holds more.** Requirement: record each create, start, stop, and delete action. Shipped: the record also holds `new_task` and `continue` (0001, T7), `team_create`, `team_delete`, `directive`, and `refused` (0002), `hold` and `release` (0007), and `task_text` (0011), 13 kinds in all (`src/modules/audit/domain/src/action_kind.rs:9`). Reason: each later change records its own actions in the same record, and the user sees them in the same details. The data model lists them. Recorded 2026-11-23 (ACF-106 archive).
- **Status wire value is `waiting_for_input` (FR-014).** The display text is `waiting for input`. The server stores and sends `waiting_for_input` (`src/modules/agent/domain/src/enums/agent_status.rs:48`). Reason: the wire value is one token. Recorded 2026-11-23 (ACF-106 archive).
- **A clean server stop sets running agents to `interrupted` at once (FR-004).** A signal and `server.restart` stop each child, set the agent and its session to `interrupted`, and tell the connected clients before they close. FR-004 still covers an unclean stop, where the status changes at the next server start.
- **Windows stop uses `taskkill`, not a Job Object (FR-006, SC-002).** The server ends the process tree with `taskkill /PID <pid> /T /F`, because a Job Object needs `unsafe` code and the workspace forbids it outside `private-fs`. `/T` follows parent links, so it misses a descendant whose parent already exited. The Windows checks stay open until someone runs them on a Windows machine. Accepted on 2026-11-15; the project owner may revisit it.
- **The `gemini` continue check is open (FR-025, SC-002).** The adapter continues with `--resume <uuid>`, the id that office-harness chose. The hands-on check with the real `gemini` is still open; if only an index works, the mapping is new work.

## Change 0002: Agent hierarchy

### Intent

With 12 or more agents, a flat set of terminals is hard to follow. This change organizes the agents of the office as a company: a CEO, teams with a team lead each, and members under the lead. A parent agent can create agents below it and send them directives, so that the CEO gives directives to team leads and a lead gives work to its members. This change covers R2 of `docs/PRD.md`.

### Requirements

- **0002/FR-001** (US1, US2): THE SYSTEM SHALL organize agents in a tree with the roles CEO, team lead, and member, where a team lead's parent is the CEO when a CEO exists and a member's parent is its team lead. A member has no child agents in the office.
- **0002/FR-002** (US1): THE SYSTEM SHALL let the user create a team with a name, an optional project, and one team lead.
- **0002/FR-003** (US1): THE SYSTEM SHALL let the user create an agent with a role only at a place in the tree that FR-001 allows, and SHALL name the allowed roles when it refuses another place.
- **0002/FR-004** (US1): THE SYSTEM SHALL show each agent under its parent in the desktop app's hierarchy view.
- **0002/FR-005** (US1): THE SYSTEM SHALL NOT limit the number of agents in an office to a fixed maximum.
- **0002/FR-006** (US1): WHEN the user deletes an agent that has descendants, THE SYSTEM SHALL show how many agents will stop and be deleted, and SHALL delete them only after the user confirms.
- **0002/FR-007** (US1): WHEN the user stops an agent that has descendants, THE SYSTEM SHALL offer to stop all of its descendants in the same action.
- **0002/FR-008** (US1, US2): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that FR-001 allows, and send reports only to its own parent or to the person. (modified by 0014, 0015)
- **0002/FR-009** (US1, US2, US3): IF an agent asks for an action on an agent that is not its descendant, for a role or action that 0002/FR-008 does not allow, for an action while it, its target, or the office is on hold, or for an action beyond a rate limit, THEN THE SYSTEM SHALL refuse the action, tell the agent the reason in the answer to its request, and record the refusal. (modified by 0007)
- **0002/FR-010** (US2): WHEN a parent agent sends a directive, THE SYSTEM SHALL enter the directive text into the target agent's terminal as one unbroken input, at the time that 0012/FR-001 and 0012/FR-002 allow, followed by Enter when 0012/FR-003 allows it. (modified by 0012)
- **0002/FR-011** (US1, US2): THE SYSTEM SHALL add directives and refused actions to the action record and SHALL record a named agent as the actor of each action that an agent takes.

### Preserved behaviors

- **0002/PB-1:** An agent without a role keeps the behavior of 0001: the user creates, starts, attaches to, stops, and deletes it, and it shows outside the hierarchy.
- **0002/PB-2:** Each new task starts a new CLI session, also when a parent agent starts it (0001/FR-024).
- **0002/PB-3:** office-harness creates, changes, or deletes no file in an agent CLI's configuration, also when it lets agents call the office (0001/FR-021).
- **0002/PB-4:** Agents in the hierarchy keep running while no desktop app is open, and the teams and the tree stay across server and machine restarts (0001/FR-001, 0001/FR-004).
- **0002/PB-5:** Every desktop app action is available from the keyboard, including the actions that this change adds (0001/FR-028).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | role (optional), parent agent (optional), team (optional) | Has at most one parent and belongs to at most one team. A member's team is its lead's team. |
| Team | A group of agents for one area or project. | name, project (optional) | Name is unique in the office. Has one team lead and zero or more members. |
| Directive | Text that a parent agent sends to a descendant. | sender agent, target agent, text, time | Recorded as an action record entry. |
| Action record entry (extends 0001) | One recorded action. | action now also `directive` and `refused`; actor can be a named agent | Belongs to the office and shows in the target agent's details; a refused directive also shows in the sender's details. |

### Success criteria

- **0002/SC-001:** A user builds a 15-agent, 3-level hierarchy (1 CEO, 2 team leads, 12 members) from the desktop app alone, without editing any file, in under 10 minutes.
- **0002/SC-002:** With 30 agents in 3 levels that run at the same time, the hierarchy view shows 100% of them under their correct parents.
- **0002/SC-003:** In a test of 20 agent requests that FR-008 does not allow, 100% are refused, and 100% show in the action record with the actor and the reason.
- **0002/SC-004:** In a test of 100 directives sent while the user types in the target terminal, 100% arrive as one unbroken input followed by Enter.
  - Amended by 0012: while the user types, or leaves typed text unsent, the directives wait in the target's queue and are typed after the user's input, each as one unbroken input followed by Enter (0012/FR-002, 0012/SC-003, 0012/SC-008).
- **0002/SC-005:** Deleting a team lead with N members removes exactly N + 1 agents after the user confirms, and 0 agents when the user cancels.
- **0002/SC-006:** A team lead agent creates a member and gives it a directive with 0 user actions between the two steps.

### Notes from Deviations

- **A5 was not shipped until ACF-197 (closed): leads did not move under a CEO that the user creates later.** Requirement (A5): when the user creates a CEO later, the existing team leads move under it. Shipped: a lead takes the CEO as its parent only when it is placed, and only if a CEO exists then (`resolved_parent` in `src/modules/hierarchy/domain/src/services/tree_rules.rs:128`). No code rewrites `parent_agent_id` of the leads when a CEO is placed; the only write of the place is for the one agent that is placed (`src/modules/hierarchy/infrastructure/src/services/agent_places.rs:55`). A lead that was placed before the CEO keeps no parent, so the descendant rule (`is_descendant_of`, `tree_rules.rs:171`) does not make it a descendant of the CEO. The tree view still shows the team. Reason: not recorded. This is a gap in the code, tracked as a code bug (see the QA report), not a decision to drop A5. Recorded 2026-11-23 (ACF-106 archive). Fixed on 2026-11-23 (ACF-197): placing a CEO now moves every team lead that has no parent under the CEO, in the same transaction as the CEO's own place write (`AgentPlaces::adopt_parentless_leads`, called from `HierarchyRepository::place_agent`). Tests: `placing_a_ceo_moves_the_parentless_leads_under_it` and `a_failed_ceo_placement_moves_no_lead` (`hierarchy_repository_tests.rs`), and the wire test `a_ceo_created_after_a_team_takes_the_team_lead_under_it` (`tests/agent_subtree.rs`). An office saved before the fix is repaired once at each start: when exactly one CEO exists, every lead without a parent moves under it (`HierarchyRepository::repair_parentless_leads`; tests in `H/tests/startup.rs`). Shipped behavior now matches A5; this Deviation is closed.


## Change 0003: Panel practicality

### Intent

The owner named switching and searching between agents as his main problem: other tools make him walk through a scene or open many windows to find an agent and its result. This change makes the desktop app one panel in which he sees every agent of every project, opens any agent's terminal in one step, switches projects in one step, and gets a notification when an agent finishes or needs him. Opening the app shows the office where he left it. This change covers R1 and R9 of `docs/PRD.md`.

### Requirements

- **0003/FR-001** (US1): THE SYSTEM SHALL show the agent list, the agent terminals, and the agent details in one window, without a separate window per agent or per project.
- **0003/FR-002** (US1): THE SYSTEM SHALL show, in one list, the name, project, status, and last output line of every agent in every project.
- **0003/FR-003** (US1): WHERE the office has a hierarchy from 0002, THE SYSTEM SHALL also show each agent's role and team in the agent list.
- **0003/FR-004** (US1, R9 criterion 1): WHEN the user clicks an agent in the panel or presses that agent's keyboard shortcut, THE SYSTEM SHALL open that agent's live terminal.
- **0003/FR-005** (US1, R9 criterion 3): WHEN the user picks one project or "All projects" in the project switcher, THE SYSTEM SHALL show only the agents of that choice.
- **0003/FR-006** (in part) (US3): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, is put on hold automatically (0007/FR-008), or sends a report for the person (0015/FR-007), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and while an unread report notification of that agent exists, a further report adds none, and (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification. Exception (b) does not apply to the automatic hold or to a report, because the terminal does not show them. (modified by 0007, 0015)
- **0003/FR-007** (US2): IF a notification occurs while the desktop app is closed, THEN THE SYSTEM SHALL show it as unread when the app opens.
- **0003/FR-008** (US2): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view, where the last view now also names the open view (panel or scene), and SHALL open that view. This changes steps 1 and 6 of the 0003 Flow, which now read "the open view" where they read "the panel", and 0003/SC-006, which now measures the open view. (modified by 0005)
- **0003/FR-009** (US2): WHEN the user opens a notification, or opens the terminal of an agent that has unread notifications, THE SYSTEM SHALL show those notifications of that agent as read.

FR-002 states R9 criterion 2. FR-006 notifies only for the four events it lists: a status change that the user caused (`stopped`) and `interrupted` after a server restart notify nobody.
Later additions to the shortcuts and the events (recorded 2026-11-23, ACF-106 archive; no requirement of this spec changed). The fixed shortcuts are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` here. Change 0008 added `Ctrl+Shift+F11` (jump to the agent that needs you), and change 0007 added `Ctrl+Shift+H` (the office hold). The server holds 12 reserved shortcuts in all (`src/modules/agent/domain/src/value_objects/settings.rs:33`). Change 0007 (FR-008) added the notification event `auto_hold` to the four events of FR-006, so the server knows five (`src/modules/notification/domain/src/enums/notification_event.rs:21`). Change 0008 added the Needs you queue. It is a separate queue with its own requests and event, not a notification event.

This change modified: 0001/FR-017. The final text is under Change 0001, Requirements.

### Preserved behaviors

- **0003/PB-1:** Closing the desktop app keeps every agent running and recording its output (0001/FR-019).
- **0003/PB-2:** office-harness sends no office data outside the user's machine; notifications are local to the machine (0001/FR-005).
- **0003/PB-3:** Every desktop app action is available from the keyboard, including the actions that this change adds (0001/FR-028).
- **0003/PB-4:** Every status shows as text, never color alone, in the agent list too (0001/FR-014).
- **0003/PB-5:** The loading, server-not-reachable, and not-authorized states of 0001 still show before the panel (0001/FR-026, 0001/FR-027).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | last output line | As in 0001. |
| Notification | A status event that the user must see. | agent, event (`exited`, `failed`, `waiting for input`, bell), time, read or unread | Belongs to the office. Refers to one agent. |
| Last view | Where the user was when the desktop app closed. | project choice, selected agent, open terminal | One per desktop app on the machine. |

### Success criteria

- **0003/SC-001:** With 30 agents across 3 projects that run at the same time, the agent list in one window shows all 30, and each status change and each change of the last output line shows in the list within 2 seconds.
- **0003/SC-002:** From the panel, the user opens any agent's live terminal with 1 click or 1 shortcut, and the terminal shows live output within 1 second.
- **0003/SC-003:** The user switches from one project to another, or to "All projects", with 1 action.
- **0003/SC-004:** While the desktop app runs, the user gets a notification within 10 seconds after an agent exits, fails, rings the bell, or changes to `waiting for input` (that is, within 10 seconds after the quiet period ends).
- **0003/SC-005:** After the desktop app was closed while 3 agents changed status, 100% of those notifications show as unread on reopen.
- **0003/SC-006:** When the user opens the desktop app, the panel shows the last view within 3 seconds.
- **0003/SC-007:** With 30 agents running, a user finds the agent that waits for him and types into its terminal within 10 seconds of the notification.


## Change 0005: Office scene

### Intent

The owner liked the idea of an office that he can look at, but he disliked walking to an agent to see its result and the slow moves between projects. This change adds an optional second view to the panel: a flat picture of the office in which each agent sits at a desk, each project is a floor, and each team is a department. The picture shows what the panel list shows, and the user opens any agent's terminal from a desk in one step. It adds no way to do work that the panel cannot do. This change covers R7 of `docs/PRD.md`.

### Requirements

- **0005/FR-001** (US1): THE SYSTEM SHALL offer an office scene as a second view that the user opens from the panel and leaves for the panel in one step each.
- **0005/FR-002** (US1): WHILE the scene is open, THE SYSTEM SHALL show one desk for every agent of the project choice, each with the agent's name, its status as text, and its last output line.
- **0005/FR-003** (US1): THE SYSTEM SHALL group the desks by floor (one per project) and, where 0002 is present, by department (one per team), with the team lead's desk first in its department.
- **0005/FR-004** (US1): WHEN the status or the last output line of an agent changes, THE SYSTEM SHALL update the desk of that agent without a user action.
- **0005/FR-005** (US1): WHEN the user opens a desk, THE SYSTEM SHALL open that agent's live terminal in the same window, with keyboard focus, as 0003/FR-004 does for the agent list.
- **0005/FR-006** (US2): WHEN the user returns from the scene to the panel, THE SYSTEM SHALL show the panel with the project choice and the selected agent that it had before the scene opened.
- **0005/FR-007** (US2): WHILE the scene is open, THE SYSTEM SHALL apply the project switcher and a search for an agent by name to the scene, with the same result as in the panel.
- **0005/FR-008** (US3): THE SYSTEM SHALL make every scene action available from the keyboard, with a visible focus mark, and SHALL announce each change of a desk's status to screen readers.
- **0005/FR-009** (US3): WHERE the user's system asks to reduce motion, THE SYSTEM SHALL show the scene with no moving or flashing element.

This change modified: 0003/FR-008. The final text is under 0003/FR-008.

### Preserved behaviors

- **0005/PB-1:** The scene shows data and starts no action of its own: it does not create, start, stop, or delete agents, and it sends no input to any terminal except through the terminal that the user opens (0001/FR-017).
- **0005/PB-2:** The panel stays the main interface; it keeps every function it has, and the scene adds none that the panel lacks (`docs/PRD.md` section 4.1; 0003 Terms).
- **0005/PB-3:** Every state shows as text, never color alone (0001/FR-014), and every scene action is available from the keyboard (0001/FR-028).
- **0005/PB-4:** Opening or closing the scene starts, stops, or changes no agent. Agents keep running while no desktop app is open (0001/FR-019).
- **0005/PB-5:** office-harness sends no office data outside the user's machine; the scene reads the same data as the panel and stores nothing else (0001/FR-005).
- **0005/PB-6:** The scene writes no CLI configuration file and no project file (0001/FR-021).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Desk | One agent in the scene. | agent name, status, last output line | One per agent. Shows an existing agent; it is not stored. |
| Floor | The desks of one project. | project name, agent count | One per project, plus "No project". Not stored. |
| Department | The desks of one team on a floor. | team name | One per team. Exists only where 0002 is present. Not stored. |
| Last view (extends 0003) | Where the user left the app. | open view (panel or scene), in addition to the 0003 attributes | One per desktop app on the machine, as in 0003 A4. |

### Success criteria

- **0005/SC-001:** With 12 agents in 3 projects, a user who sees the panel opens the scene, finds a named agent's desk, and opens its terminal in under 10 seconds.
- **0005/SC-002:** After an agent's status changes, the desk shows the new status, and after an agent's last output line changes, the desk shows the new line, within 2 seconds in 100% of 50 changes of each kind.
- **0005/SC-003:** In the scene and in the panel, the same 12 agents show the same status text and the same last output line in 100% of 50 samples.
- **0005/SC-004:** With 100 agents in 10 projects, the scene opens in under 2 seconds, and the user finds and opens a named agent's terminal in under 15 seconds.
- **0005/SC-005:** With only the keyboard, a user opens the scene, opens a desk, and returns to the panel with 0 pointer actions.
- **0005/SC-006:** After a round trip panel, scene, panel, the project choice and the selected agent equal their earlier values in 100% of 20 trials.
- **0005/SC-007:** With the system setting "reduce motion" on, a scan of the scene while 12 agents change status finds 0 moving or flashing elements.


## Change 0007: Hold switch and agent action limits

### Intent

With 0002, agents create agents, start and stop them, start new tasks, and send directives without the user. One agent that loops can flood a terminal with directives or churn agents faster than the user can read. This change gives the user one action that holds every agent-made action in the office, a hold for one agent and its descendants, and rate limits that refuse an agent's actions beyond a set pace. The terminals keep running through a hold. This change amends the approved 0002.

### Requirements

- **0007/FR-001** (US1): THE SYSTEM SHALL let the user hold and release the whole office with one action each from the desktop app.
- **0007/FR-002** (US1): WHILE the office is on hold, THE SYSTEM SHALL refuse every agent-made action with the reason that the office is on hold.
- **0007/FR-003** (US2): THE SYSTEM SHALL let the user hold and release one agent from the desktop app.
- **0007/FR-004** (US2): WHILE an agent is on hold, THE SYSTEM SHALL refuse every agent-made action taken by that agent or one of its descendants, and every agent-made action whose target is that agent or one of its descendants, with the reason that the agent is on hold.
- **0007/FR-005** (US1, US2): WHILE the office or an agent is on hold, THE SYSTEM SHALL keep every agent's terminal and CLI running, deliver the user's typing, and carry out the user's own actions from the desktop app as without a hold.
- **0007/FR-006** (US1, US2): THE SYSTEM SHALL keep the office hold and every agent hold across server and machine restarts until the user releases it.
- **0007/FR-007** (US3): IF an agent's agent-made action would exceed one of the rate limits in the table under Assumption A3, THEN THE SYSTEM SHALL refuse the action with a reason that names the limit and its value.
- **0007/FR-008** (US3): WHEN an agent is refused by a rate limit for the third time within 60 seconds, THE SYSTEM SHALL put that agent on hold and notify the user once, naming the agent.
- **0007/FR-009** (US3): THE SYSTEM SHALL let the user change each rate limit in the office settings, and SHALL apply a changed value to the next agent-made action.
- **0007/FR-010** (US1, US2, US3): THE SYSTEM SHALL add every hold, release, and refusal of this change to the action record, with the user as the actor of a hold or release that the user takes, the office as the actor of an automatic hold, and the refused agent as the actor of a refusal. A refused directive appears in the action record of the sender and in the action record of the target.
- **0007/FR-011** (US1, US2): THE SYSTEM SHALL show in the desktop app, as text, whether the office is on hold and which agents are on hold.

This change modified: 0003/FR-006, 0002/FR-009. The final text is under the change that owns each id.

### Preserved behaviors

- **0007/PB-1:** The user's own actions from the desktop app (create, start, stop, delete, new task, typing into a terminal) are never held or rate-limited (0002 A1; 0001/FR-017 as modified by 0003).
- **0007/PB-2:** A hold refuses new agent-made actions only; it stops no running CLI and closes no terminal. Stopping a CLI stays the existing stop action (0001, 0002/FR-007).
- **0007/PB-3:** office-harness creates, changes, or deletes no file in an agent CLI's configuration (0001/FR-021).
- **0007/PB-4:** Every desktop app action is available from the keyboard, including the hold and release actions (0001/FR-028).
- **0007/PB-5:** The tree rules of 0002 stay as they are: three levels, directives down only, a member creates no office agents (0002/FR-001, 0002/FR-008).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Office hold | Whether the office holds agent-made actions. | on or off, since when, who set it | One per office; stored. |
| Agent hold | Whether one agent and its subtree are held. | on or off, since when, who set it (user or office), the reason | At most one per agent; stored with the agent; removed with the agent. |
| Rate limit setting | One limit of Assumption A3. | kind, value, default | One per kind in the office settings. |
| Action record entry (extends 0002) | One recorded action. | actions now also `hold`, `release`; actor can be the office for an automatic hold | Belongs to the office. |

### Success criteria

- **0007/SC-001:** In a test where one agent asks to send 100 directives within 10 seconds, at most 10 directives reach terminals, and 100% of the refused requests show in the action record with the agent as actor and a reason: the limit for the refusals before the automatic hold, and the hold for the refusals after it.
- **0007/SC-002:** After the user presses the office hold action, 100% of agent-made actions are refused, measured from the first request after the action, while 100% of the user's typed keys still reach the terminals.
- **0007/SC-003:** After a server restart with the office on hold, 0 agent-made actions succeed until the user releases the hold.
- **0007/SC-004:** A looping agent that hits a limit 3 times within 60 seconds causes exactly 1 notification to the user and is on hold after the third refusal.
- **0007/SC-005:** The user holds the office from the keyboard in 1 key press from any panel view, a focused terminal included (A9, as changed by Deviations, D9 = b). The first approved text said "whose keyboard focus is outside a terminal".

### Notes from Deviations

- **Hold shortcut from a terminal (D9 = b, owner-decided 2026-11-19).** This changes A9 and SC-005. `Ctrl+Shift+H` is one of the app shortcuts that a focused terminal lets out (0003's MODIFIED 0001/FR-017 gets the office hold shortcut beside the focus, agent, and switcher shortcuts). The user holds or releases the office in 1 key press from any panel view, a focused terminal included. The CLI no longer gets `Ctrl+Shift+H`.

### Rate limits (assumption A3 of 0007)

The rate limits and their defaults, each counted over a rolling 60 seconds:

| Limit | Default | Reason |
|---|---|---|
| Directives sent by one agent | 10 | A lead that briefs 8 members in one burst fits; a loop does not. |
| Directives sent to one target agent | 3 | One terminal is one input stream; more is flooding. |
| Agents created by one agent | 5 | A lead that creates a 5-member team fits. |
| Starts, stops, and new tasks by one agent | 10 | One restart round of a 5-member team fits twice. |
| Reports sent by one agent (added by 0015) | 5 | An agent reports each step of a short task, and a loop stops. |

## Change 0012: Safe directive delivery

### Intent

A parent agent's directive was typed into the target's terminal at once, with Enter, whenever the target ran. When the target's CLI showed a question for a person, the Enter or a letter of the text could answer it, and nobody saw it. This change puts each directive in a queue per target and types it only when the target waits for input, no person types or has unsent text there, no text the office typed without Enter waits there, and the screen shows no question that the office recognises. Enter follows only after the text, or the CLI's placeholder for it, shows at the input line. The sender learns whether its directive was submitted, waits, or was left unsubmitted, and a directive that cannot be typed in time is withdrawn and recorded. This change modifies 0002/FR-010 and amends 0011 (the task line shows the last submitted directive). What the screen checks cannot cover is under Residual risk in the change spec.

### Requirements

- **0012/FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one left the queue and the conditions of 0012/FR-002 hold again.
- **0012/FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, no unsent person text waits at its input line, no pending office text (a startup prompt or an unsubmitted directive) waits in the target's current session, and the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **0012/FR-003** (US1): WHEN the system has typed a directive, THE SYSTEM SHALL press Enter only IF the end of the directive's text, or a new paste placeholder, appears at the input line of the target's screen within 2 seconds, and otherwise SHALL press no Enter, leave the typed text in the terminal, and record the directive as unsubmitted with the reason in the sender's and the target's action records.
- **0012/FR-004** (US2): WHEN the system accepts a directive, THE SYSTEM SHALL answer the sender that the directive was submitted, that it waits with its place in the target's queue, or that it was left unsubmitted; IF 20 directives already wait for the target, THEN THE SYSTEM SHALL refuse the directive, say that the target's queue is full, and record the refusal.
- **0012/FR-005** (US2): IF a waiting directive was not typed within 10 minutes, its target stopped running, the target is no longer the sender's descendant, a hold would now refuse the directive (0007/FR-002, 0007/FR-004), or the background server stops normally, THEN THE SYSTEM SHALL withdraw the directive without typing it and SHALL record the withdrawal with the directive text and the reason in the sender's and the target's action records.

This change modified: 0002/FR-010. The final text is under the change that owns each id.

### Preserved behaviors

- **0012/PB-1:** The checks at sending stay as they are, with their refusals and records: the target must be a running descendant, the text must follow the directive rules (0002/FR-009, 0002/FR-011), and holds and rate limits apply (0007).
- **0012/PB-2:** An accepted directive is recorded once in the target's action record, with the sender as the actor (0002/FR-011). A directive that waits is accepted.
- **0012/PB-3:** The task line shows the text of the last directive that was submitted (0011/FR-002). A directive that waits, was withdrawn, or was left unsubmitted does not change it.
- **0012/PB-4:** The office writes nothing into a CLI's configuration (0001/FR-021). Both screen checks read only the screen copy that the office keeps.
- **0012/PB-5:** Person input reaches the terminal at once and unchanged (0001); the queue never holds it back.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Waiting directive | A directive the office accepted and has not typed yet. | sender, target, text, time accepted, place in the queue | Belongs to one target's queue until it is typed or withdrawn. Lives only while the background server runs; a normal stop withdraws it (0012/FR-005), and a crash drops it without a withdrawal entry. |

### Success criteria

- **0012/SC-001:** Of 50 directives sent to an agent whose screen shows a permission question, 0 characters are typed into its terminal while the question shows.
- **0012/SC-002:** Of 20 directives sent to an agent that shows an unrecognised question and does not echo input, 0 are followed by Enter.
- **0012/SC-003:** Of 20 directives sent while a person types into the target's terminal, 0 are typed earlier than 30 seconds after the person's last input.
- **0012/SC-004:** 10 directives sent in a row to one working agent are each typed exactly once, 100% in the order they were sent.
- **0012/SC-005:** 100% of withdrawn and unsubmitted directives have an entry with the text and the reason in the sender's and the target's action records.
- **0012/SC-006:** A directive sent to an agent that waits for input and echoes input, with no open question and no person input in the last 30 seconds, is submitted within 2 seconds of sending.
- **0012/SC-007:** Of 10 directives sent to an agent with pending office text, a pasted startup prompt or an unsubmitted directive, 0 are typed before a person input reaches that terminal.
- **0012/SC-008:** Of 10 directives sent while a person's half-typed line waits unsent, 0 are typed until the person sends or clears the line.

### Notes from Deviations

- **One delivery per target, not one write lock for all.** A delivery can wait up to 2 seconds for the input line, so the handler marks one delivery per target. Directives to other targets go on meanwhile, and writes and receipts of one target keep their order.
- **Readiness has three answers.** The handler needs only ready, busy, or gone, so the reasons a target is busy are not carried.
- **Input-line check by count.** The check counts how often the text's end, and a paste placeholder, show at the input line before and after typing, so a line that already held the same text elsewhere does not count.
- **Commits.** The hierarchy change and the server change do not build apart, so their code and tests are one commit, and the documents are another.
- **Gap found after ship (2026-11-23).** 0012/FR-002 is not met for an unnumbered choice list, such as the start question of `claude` in a new folder: the office typed the directive after the quiet period, without Enter (0012/FR-003 held). A follow-up change fixes it.

## Change 0013: Resume all interrupted agents

### Intent

After a server restart, the user must resume each interrupted agent one by one: open it, pick its last session, and continue it. When a CLI cannot continue its session, the agent ends at once and the user must notice it. This change adds one confirmed action that resumes every interrupted agent that the user checks. When a CLI refuses to continue, the office starts that agent once in a new session. An agent that still fails is put on hold. The office still starts no agent by itself (0001/FR-004). This change extends 0001 and 0007.

### Requirements

- **0013/FR-001** (US1): WHILE at least one agent of the office is `interrupted`, THE SYSTEM SHALL offer a "Resume interrupted" action that shows the number of interrupted agents.
- **0013/FR-002** (US1, US3): WHEN the user opens the "Resume interrupted" action, THE SYSTEM SHALL list every interrupted agent in start order, with what will happen to it (continue its last session, shown with that session's start time, or start a new session, shown with the reason) and with whether it is on hold, SHALL check every listed agent that is not on hold, and SHALL start nothing before the user confirms.
- **0013/FR-003** (US1, US2): WHEN the user confirms, THE SYSTEM SHALL, for each checked agent that is still `interrupted`, continue its last session where the agent's CLI supports continuing it (0001/FR-025) and otherwise start a new session (0001/FR-024), SHALL start every member before its lead and every lead before the CEO, and SHALL skip each checked agent that is no longer `interrupted`, with that reason.
- **0013/FR-004** (US2): IF a session that this action continued ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL start the agent once in a new session and SHALL record that start in the agent's action record with the office as actor and the reason (which session the CLI refused, and how).
- **0013/FR-005** (US3): IF a new session that this action started ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL NOT start the agent again, SHALL put the agent on hold (0007/FR-003) with the office as actor and the reason, and SHALL notify the user once, naming the agent.
- **0013/FR-006** (US1, US2, US3): THE SYSTEM SHALL show the result of each checked agent as text as soon as it is known: checking (during the check period), continued, new session (with the reason), on hold (with the reason), or skipped (with the reason).
- **0013/FR-007** (US2): WHEN this action starts a new session instead of continuing the last one, for any reason, THE SYSTEM SHALL keep the reason with that start's entry in the agent's action record, so the user sees in the agent's details that the CLI's earlier context was not carried over.
- **0013/FR-008** (US1): WHERE the last session of a listed agent was still open when the server stopped uncleanly, THE SYSTEM SHALL mark that agent in the list, SHALL show, before the user confirms, a warning that the CLI processes of those agents from before the stop may still run and that resuming can start a second copy of a CLI next to them, and SHALL keep Confirm unavailable while a marked agent is checked until the user acknowledges the warning.

### Preserved behaviors

- **0013/PB-1:** No agent starts without an explicit action of the user; the server starts no interrupted agent by itself at its start (0001/FR-004).
- **0013/PB-2:** The user can still continue any previous session of an agent from its details, also a session that the CLI refused before (0001/FR-025).
- **0013/PB-3:** A hold refuses only agent-made actions; the user's own actions, this action included, work as before (0007 PB-1, PB-2).
- **0013/PB-4:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).
- **0013/PB-5:** Each start, continue, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).
- **0013/PB-6:** The notification of FR-005 is a notification of an automatic hold under 0003/FR-006 (as modified by 0007): it names the agent and the event, and the focused-terminal exception does not apply to it.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Resume result | What the action did to one checked agent. | agent, result (checking, continued, new session, on hold, skipped), reason | Shown while the dialog is open; not stored. What happened stays in the action record and the hold list. |
| Hold (extends 0007) | An agent hold. | reason, which can now also be a failed resume | Set by the office (FR-005). |
| Session (extends 0001) | One run of an agent CLI. | end result: also "lost", when the session was still open at an unclean stop | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | reason (optional): why a start began a new session instead of a continue | Belongs to the agent it names (0001/FR-011). |

### Success criteria

- **0013/SC-001:** With 30 interrupted agents, the user resumes all of them with 2 activations after a clean stop (open the action, confirm) and 3 after an unclean stop (open, acknowledge the warning, confirm), and 100% of them have a started session within 60 seconds of the confirm.
- **0013/SC-002:** With a CLI that ends within 1 second of every continue, 100% of the checked agents run a new session within 30 seconds of the confirm, and each has exactly 1 action record entry for that start with the office as actor and a reason.
- **0013/SC-003:** An agent whose CLI ends within 1 second of every start is started at most 2 times by one bulk action, is on hold after it, and causes exactly 1 notification.
- **0013/SC-004:** After 10 server starts that follow a SIGKILL of the server, 0 agents start without an action of the user.
- **0013/SC-005:** In a tree of 1 CEO, 2 leads, and 4 members, all interrupted, 100% of the members start before their lead and both leads start before the CEO.
- **0013/SC-006:** After a SIGKILL of the server while agents run, the warning of FR-008 shows and marks 100% of those agents, and 0 confirms are possible before the acknowledgement while a marked agent is checked; after a clean stop, the warning shows 0 times.

## Change 0014: A parent reads the state of its descendants

### Intent

A parent agent sends directives down the tree (0002), but it cannot see what its team does. This change lets a running agent read the state of its own descendants from inside its session: name, place in the tree, status, last output time, Needs you, hold, and task line. The agent reads state, never terminal content, and only below itself. This change amends the approved 0002.

### Requirements

- **0014/FR-001** (US1): WHEN a running agent reads its subtree, THE SYSTEM SHALL answer with one entry for each of its descendants, each entry with the descendant's name, parent, role, status, last output time, whether it has an item in Needs you, whether an office hold or an agent hold covers it under the rules of 0007/FR-004, and its task line with the time the task text was set.
- **0014/FR-002** (US1): THE SYSTEM SHALL include in the answer only the descendants of the reading agent, as the tree is at the moment of the read, and SHALL NOT include the reading agent itself, its ancestors, or any other agent.
- **0014/FR-003** (US1): THE SYSTEM SHALL NOT include terminal content, the full task text past its task line, the CLI command, the working folder, environment values, or office additions in the answer.
- **0014/FR-004** (US1): THE SYSTEM SHALL answer a read while the office or an agent is on hold as without the hold, and SHALL NOT count a read toward any rate limit or automatic hold of 0007.
- **0014/FR-005** (US1): THE SYSTEM SHALL NOT add a read that it answers to the action record, and SHALL record a read that it refuses as 0002/FR-009 and 0002/FR-011 require.

This change modified: 0002/FR-008. The final text is under the change that owns each id.

### Preserved behaviors

- **0014/PB-1:** The 0002 tree rules stay as they are: an agent creates, starts, stops, gives new tasks to, and sends directives to its own descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007).
- **0014/PB-2:** Holds and rate limits refuse agent-made actions as before (0007/FR-002, FR-004, FR-007, FR-008). A read is not an agent-made action, so it adds no new way past a hold.
- **0014/PB-3:** A read changes no state: no status, task text, Needs you item, hold, notification, or action record entry changes because an agent read it (0008, 0011/FR-007).
- **0014/PB-4:** The office reads no CLI prompt and creates, changes, or deletes no file in a CLI's configuration (0001/FR-021).
- **0014/PB-5:** The desktop app shows the same as before, and the agent's connection to the office still gets no events and no terminals (0002).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Subtree entry | The state of one descendant at the moment of a read. Not stored. | agent, name, parent, role, status, last output time, in Needs you (yes or no), covered by a hold (yes or no), task line and the time of its task text | One per descendant of the reading agent, in one answer. |

### Success criteria

- **0014/SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each of the 9 agents reads once. 100% of the answers list exactly the reader's descendants: 0 entries for the reader itself, a sibling, an ancestor, or an agent in another branch.
- **0014/SC-002:** Each descendant prints a known marker text in its terminal before the read. 0 answers hold any part of a marker text.
- **0014/SC-003:** After a member's status changes to `working`, then to `waiting for input`, then to `stopped`, its lead's next read after each change shows the new status (3 of 3).
- **0014/SC-004:** While the office is on hold, 100% of 10 reads are answered. With no hold, after 30 reads within 60 seconds, the reader's next directive is not refused by a rate limit, and 0 automatic holds occur.
- **0014/SC-005:** With 30 agents in the office, a read answers within 2 seconds.
- **0014/SC-006:** In 10 attempts to point a read at a sibling, an ancestor, or an agent of another branch, 100% are refused and recorded, and 0 answers hold data of that agent.
- **0014/SC-007:** A descendant whose task text holds an escape character (U+001B), a one-byte control sequence introducer (U+009B), a right-to-left override (U+202E), and a second line: the reader's answer holds only the first line, and 0 control or bidirectional format characters reach the reader as themselves.

## Change 0015: Upward reports

### Intent

Directives go down the tree (0002), and a parent can read its descendants' state (0014), but nothing comes back up as words. A lead that briefed five members learns which one finished, or which one is stuck, only by guessing from a status, and the CEO cannot ask the owner for a decision except by going quiet. This change lets a running agent send a short report to its own parent: that it is done, that it has a question, that it is blocked, or a piece of information. The office types the report into the parent's terminal through the safe delivery of 0012, so a report never presses Enter into a question meant for a person. A report from the CEO, from an agent with no parent, or one that the sender marks for the person, goes to the owner instead: it joins Needs you (0008). The parent answers with a directive (0002); the owner answers in the agent's terminal.

### Requirements

- **0015/FR-001** (US1, US2, US3): WHEN a running agent sends a report with a kind of `done`, `question`, `blocked`, or `info` and a text of 1 to 1,000 characters that is not blank, THE SYSTEM SHALL accept it; otherwise THE SYSTEM SHALL refuse it, tell the sender the reason, and record the refusal.
- **0015/FR-002** (US1, US2): THE SYSTEM SHALL send an accepted report to the sender's parent as the tree is at the moment of the report, or to the person when the sender has no parent or marks the report for the person, and SHALL NOT take any input that names another agent as the report's receiver.
- **0015/FR-003** (US3): WHEN the system accepts a report, THE SYSTEM SHALL replace each control character (C0, DEL, C1, including line breaks), escape code, and bidirectional format character in the text with its visible form, and SHALL use only the visible form in the parent's terminal, in Needs you, and in the action record.
- **0015/FR-004** (US1): WHEN a report goes to a parent, THE SYSTEM SHALL type the report heading and the report text into the parent's terminal as one line, in the parent's queue together with its directives, under the rules of 0012/FR-001 to 0012/FR-005 as they apply to a directive whose target is the parent, and SHALL answer the sender that the report was submitted, waits with its place in the queue, or was left unsubmitted.
- **0015/FR-005** (US1): IF the sender's parent is not running, THEN THE SYSTEM SHALL refuse a report to it, say that the parent is not running, and record the refusal; IF a waiting report's sender is no longer a child of the report's parent, THEN THE SYSTEM SHALL withdraw it as 0012/FR-005 withdraws a directive whose target left the sender's subtree.
- **0015/FR-006** (US3): THE SYSTEM SHALL treat sending a report as an agent-made action under 0007: the office hold and an agent hold that covers the sender refuse it (0007/FR-002, 0007/FR-004), and a rate limit "reports sent by one agent", 5 per rolling 60 seconds by default, refuses a report beyond it (0007/FR-007), counts toward the automatic hold (0007/FR-008), and is changed by the user in the office settings like the other limits (0007/FR-009).
- **0015/FR-007** (US2): WHEN the system accepts a report for the person, THE SYSTEM SHALL record it as an attention event of the sender, notify the owner with the sender's name and the report's kind as 0003/FR-006 (as modified) allows, and answer the sender that the report is in Needs you.
- **0015/FR-008** (US2): WHILE an item in Needs you holds a report, THE SYSTEM SHALL keep showing that report on the item when later attention events change the item's latest event, and SHALL replace it only with the sender's next report for the person.
- **0015/FR-009** (US1, US2, US3): THE SYSTEM SHALL add each accepted report to the action record with the sender as the actor, the kind, and the text in visible form, under the parent for a report to a parent and under the sender for a report for the person, and SHALL record each refusal in the sender's action record and, when the report had a parent, in the parent's.
- **0015/FR-010** (US1): WHILE reports wait in a parent's queue, THE SYSTEM SHALL keep at most one waiting report per sender, replacing a sender's waiting report with its newer report in the same place and recording the replaced one as withdrawn, and SHALL keep at most 10 waiting reports per queue, refusing a further report with the reason that the parent's queue has no room for reports, and SHALL NOT count such a refusal toward any rate limit or automatic hold.

This change modified: 0002/FR-008, 0003/FR-006, 0008/FR-002, 0008/FR-003, 0008/FR-005. The final text is under the change that owns each id.

### Preserved behaviors

- **0015/PB-1:** The 0002 tree rules stay: directives go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007). A report goes up one level only, or to the person.
- **0015/PB-2:** A directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals. Reports share the queue and do not jump it; they take at most 10 of its places (FR-010), so directives always keep at least 10.
- **0015/PB-3:** The task line shows the last submitted directive (0011/FR-002, 0012 PB-3). A report changes no task text, neither the sender's nor the parent's.
- **0015/PB-4:** Needs you keeps one item per agent, its order inside a group, the jump shortcut, the count, and dismiss (0008). An item without a report behaves as before.
- **0015/PB-5:** The office reads no CLI prompt and writes nothing into a CLI's configuration (0001/FR-021).
- **0015/PB-6:** The existing rate limits, their defaults, and their settings stay as they are (0007 A3); the reports limit is a new one beside them.
- **0015/PB-7:** The agent's connection to the office still gets no events and no terminals (0002); a report's answer is the only thing it returns.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Report | One report an agent sent. | sender, kind, text in visible form, time sent, receiver (a parent or the person) | A report to a parent waits in the parent's queue like a directive until it is typed or withdrawn (0012). A report for the person lives on the sender's Needs you item. Both are in the action record. |
| Item (0008), extended | One agent that waits for the owner. | as in 0008, plus the latest report for the person (kind, text, time), if any | One per agent (0008). |

### Success criteria

- **0015/SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each member and each lead sends one report. 100% of the reports reach exactly the sender's parent's terminal, and 0 reach any other terminal.
- **0015/SC-002:** In 10 attempts to point a report at another agent, by a field naming an agent, a team, or a branch, 100% are refused and recorded, and 0 reach any terminal or Needs you.
- **0015/SC-003:** A report for the person shows in Needs you, with its kind and text, and the window title count changes, within 2 seconds after the agent sends it.
- **0015/SC-004:** A report text that holds U+001B, U+009B, U+202E, and a line feed: 0 of these characters reach the parent's terminal, Needs you, or the action record as themselves, and the parent's terminal receives the report as one line.
- **0015/SC-005:** A report of 1,000 characters is accepted; one of 1,001 characters and a blank one are refused and recorded (2 of 2).
- **0015/SC-006:** Of 10 reports sent to a parent whose screen shows a permission question, 0 characters are typed into the parent's terminal while the question shows.
- **0015/SC-007:** Under an office hold, 100% of 5 reports are refused with the hold reason. With no hold, the 6th report within 60 seconds is refused with the limit, and after the third such refusal the sender is on hold automatically.
- **0015/SC-008:** A lead that received a member's report sends a directive to the id in the report heading, and the member's terminal receives it (1 of 1).
- **0015/SC-009:** A report item stays in Needs you while its agent changes to `working` and back to `waiting for input`, and leaves when the owner opens the agent's terminal (1 of 1, and 1 of 1 for dismiss).
- **0015/SC-010:** 100% of accepted, refused, withdrawn, and unsubmitted reports have an entry in the action records that FR-009 names.
- **0015/SC-011:** With 10 reports from 10 children waiting in a working lead's queue, a directive from the CEO to that lead is accepted (1 of 1), an 11th report from another child is refused, and a second report from a child whose report waits leaves exactly one waiting report of that child.
- **0015/SC-012:** 5 reports for the person from one agent, with the first notification unread, add exactly 1 notification, and the item shows the 5th report.

### Notes from Deviations

- **Visible form in the use case, not the gateway (PLAN D3).** The plan made the visible form in `agent_call.rs`. The use case (`SendReportHandler`) makes it instead, so the length check and the visible form sit in one place and every receiver gets the same form. The 0014 `visible_text` moved from the server app to the hierarchy domain for that; the subtree read imports it from there.
- **Ports bundle.** `SendReportHandler::new` takes a `ReportPorts` bundle, as `SendDirectiveHandler::new` takes `DirectivePorts`, and holds the directive handler whose queue a report to a parent shares.
- **Holds checked where the queue is.** A report for the person passes the hold gate in `SendReportHandler`; a report to a parent passes it in `SendDirectiveHandler::send_report`, after the room checks, so a no-room refusal takes no rate slot (FR-010).
- **A failed Needs you write does not refuse the report.** The contract's `internal` row names a tree read failure only; a failed notification or item write raises the storage alert, as for every attention event (contract amended).
- **Commits.** T6 tests are two commits (person path, parent path), and the desktop has its own commit.
- **Refused text in visible form (final review L1).** A refused report is journaled with its text in visible form, so a bidirectional format character cannot reorder the record; `a_refused_report_is_journaled_in_visible_form`. A refused directive keeps the 0012 escape of C0 and C1 only.

## Change 0016: Read-only file browser of an agent

### Intent

Reviewing what the agents did is the owner's main job, and the desktop app showed only the terminals. This change adds a read-only file view of one agent's root, the folder its CLI starts in (its working folder, or its project's folder). The view has three parts: Files (a tree and the content of a file), Changes (the files that differ from the last commit, with their diffs), and History (the commits that change files inside the root, with each commit's files and diffs). It writes nothing, stays inside the root, shows a file of a secret-looking name only after the user reveals it, and shows content and diffs up to 512 KiB. Agents cannot use it.

### Requirements

- **0016/FR-001** (US1): WHEN the user opens the file view of an agent, THE SYSTEM SHALL list the entries of the root, and WHEN the user opens a folder, THE SYSTEM SHALL list that folder's entries, folders first and then the other entries, each group in name order, each entry with its name, its kind (folder, file, link, or other), and, for a file, its size, and SHALL list at most 1,000 entries of one folder and say how many more exist.
- **0016/FR-002** (US1): WHEN the user opens a file inside the root, THE SYSTEM SHALL show its content as read-only text with line numbers, IF the file is larger than the size limit THEN THE SYSTEM SHALL show its size instead, and IF the file is a binary file THEN THE SYSTEM SHALL say so instead.
- **0016/FR-003** (US1, US2, US3): THE SYSTEM SHALL resolve every path that the file view asks for, symbolic links included, before it reads, SHALL refuse a path that is absolute, holds a `..` part, or is not inside the root, and SHALL NOT send any part of a file or a folder listing that is not inside the root.
- **0016/FR-004** (US1, US2, US3): THE SYSTEM SHALL NOT send the content or the diff of a secret-looking file, or the diff of a rename whose old or new name is secret-looking, until the user reveals that file, and SHALL hide it again when the user opens another file, another part, or refreshes.
- **0016/FR-005** (US2): WHERE the root is inside a git repository, WHEN the user opens the Changes part, THE SYSTEM SHALL list the changed files inside the root, at most 1,000 with the count of the others, and WHEN the user opens one, SHALL show its diff against the last commit, staged and unstaged changes together, or its size and that it is too large when the diff is over the size limit.
- **0016/FR-006** (US3): WHERE the root is inside a git repository, WHEN the user opens the History part, THE SYSTEM SHALL list the commit history in pages of 50 commits, and WHEN the user opens a commit, SHALL show its details, the files inside the root that it changed, and each file's diff against the commit's first parent.
- **0016/FR-007** (US1, US2, US3): THE SYSTEM SHALL NOT create, change, or delete any file in the root or in its git data, SHALL NOT change any agent, and SHALL NOT run any program that the repository's configuration or folder names, such as a hook, a diff driver, a text conversion, a content filter, a signature check, or a file system monitor, while it serves the file view.
- **0016/FR-008** (US1, US2, US3): THE SYSTEM SHALL show a loading state while a part of the file view waits for the office, an empty state when a folder, the changes, or the history has nothing to show, and an error state with the reason when the office cannot read what was asked, and SHALL show the time of the last load with a refresh action.
- **0016/FR-009** (US1, US2, US3): THE SYSTEM SHALL show every file name, file content, diff, and commit text in the file view as plain text, SHALL NOT render markup, run scripts, load images, or follow links found in them, and SHALL show each control character and bidirectional format character as a visible symbol.

### Preserved behaviors

- **0016/PB-1:** The agent's connection to the office (0002) gets no file view request: the allowed agent requests stay as they are.
- **0016/PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), in the agent's working folder, or in the repository's git data (0010/PB-2) because of the file view.
- **0016/PB-3:** The desktop app still needs the office token and a connection from the same machine for every request (0001/FR-007, FR-008).
- **0016/PB-4:** The file view changes no agent's status, session, terminal, task text, or Needs you item, and adds nothing to the action record.
- **0016/PB-5:** The panel, the terminals, and every existing dialog work as before; the file view adds one way in from an agent and nothing else changes.

### Key entities

None stored. Every answer is read from the root and its git data at the moment of the request; the office keeps no copy.

### Success criteria

- **SC-001:** Of 20 requests for paths outside the root (parent parts, absolute paths, a link to a file outside, a link to a folder outside, a link inside a linked folder, and a link chain that leaves the root), 100% are refused, and 0 bytes of the outside files reach the desktop app.
- **SC-002:** A file of exactly 512 KiB shows in full; a file of 512 KiB plus 1 byte shows its size and 0 bytes of content; a diff over 512 KiB shows its size and 0 lines.
- **SC-003:** In a folder with 10 secret-looking files that are also changed files, opening the tree, each file, the Changes part, and each diff sends 0 bytes of their content to the desktop app until a reveal; after one reveal, exactly that one file's content arrives; after opening another file, it is hidden again. A `.env` renamed to `notes.txt`, in the Changes part and in a commit, sends 0 bytes of its diff until a reveal.
- **SC-004:** After a session that lists every folder, opens every file, the changes, every diff, the history, and every commit of a test repository, the root and its git data are byte for byte the same as before, with the same modification times, no lock file was created, and a hook, a diff driver, a text conversion, a content filter, a signature check program, and a file system monitor that the repository names each wrote 0 marker files.
- **SC-005:** In a repository with one modified, one staged new, one deleted, one renamed, and one untracked file inside the root and one modified file outside it, the Changes part lists exactly the 5 files inside the root, each with its kind, and each diff holds the removed and added lines of that file.
- **SC-006:** With 120 commits that change the root and 10 that change only another folder of the repository, the History part shows 50, then 100, then 120 commits, newest first, none of the 10 others.
- **SC-007:** Each visible state appears in a test with its text, and every action of the flow completes with the keyboard alone.
- **SC-008:** A file and a commit message that hold `<script>`, an image tag, a terminal escape code (U+001B), and a right-to-left override (U+202E) show as text, with U+001B and U+202E as visible symbols, and 0 scripts run and 0 images load.
- **SC-009:** In a working folder of 50,000 files, a folder of 1,000 entries lists within 2 seconds, a 512 KiB file shows within 2 seconds, and the Changes part of 1,000 changed files lists within 5 seconds.

### Notes from Deviations

- **One server commit for the folder reader and the git reader.** They share the port types of the application layer, so they do not build apart.
- **Filter switches reused.** The 0010 function that empties filter drivers lists the configuration with `config --list --name-only -z`, and it now also empties `smudge`. The 0010 worktree status and remove runs get the same switch, which changes nothing for them.
- **Submodules.** Every status and diff passes `--ignore-submodules=dirty`, so git never runs inside a submodule. A submodule shows as changed only when its recorded commit changed.
- **Root commit.** A commit without a parent is compared with the empty tree, in both the file list and the diff.
- **Renames across the root.** git pairs renames only inside the pathspec of the root, so in a subfolder root a file moved across the root's edge shows as added or deleted, never as a rename with an outside old path.
- **Pure rename.** A rename or a mode change without content change has no hunk, so its diff answers `none`, also after a reveal.
- **Read time.** The History part shows the desktop's load time, because the history answer has no read time.
- **Large commit message.** A message over the git runner's 1 MiB output limit answers `internal` instead of a shortened message.
- **Race and hard links.** An agent that swaps a folder for a link between the check and the read, or that adds a hard link to an outside file, can still make the view show that file.
- **Unix only.** The link, named pipe, and hook marker tests run on Linux and macOS, not on Windows. The `.git` refusal compares ASCII case only on Windows.
- **No hands-on run of the packaged app.** The desktop flow ran against the mock core and the server against real repositories, not together on a real agent.
