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
- [0017-orphan-sweep](../changes/archive/0017-orphan-sweep/SPEC.md)
- [0018-idle-sleep](../changes/archive/0018-idle-sleep/SPEC.md)
- [0019-cli-sign-in-check](../changes/archive/0019-cli-sign-in-check/SPEC.md)
- [0020-temporary-agents](../changes/archive/0020-temporary-agents/SPEC.md)
- [0021-worktree-dependencies](../changes/archive/0021-worktree-dependencies/SPEC.md)
- [0022-waiting-item-sleep](../changes/archive/0022-waiting-item-sleep/SPEC.md)
- [0023-drop-ended-commands](../changes/archive/0023-drop-ended-commands/SPEC.md)
- [0025-scheduled-directives](../changes/archive/0025-scheduled-directives/SPEC.md)
- [0026-agent-secrets](../changes/archive/0026-agent-secrets/SPEC.md)

Changes 0008 to 0011 merge in when they are archived. 0004 and 0006 are not merged.

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.
- Ids of 0001 keep the form `0001/FR-nnn`. The requirements of 0001 that a merged change modified are 0001/FR-014 and 0001/FR-017.

Modified requirements (0008 is not merged yet: 0022 modified 0008/FR-001 and 0008/FR-005, and their final text is in the MODIFIED section of [0022](../changes/archive/0022-waiting-item-sleep/SPEC.md)):

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0001/FR-014 | 0018 | Change 0001, Requirements |
| 0001/FR-017 | 0003 | Change 0001, Requirements |
| 0002/FR-008 | 0014, 0015, 0026 | Change 0002, Requirements |
| 0002/FR-009 | 0007 | Change 0002, Requirements |
| 0002/FR-010 | 0012 | Change 0002, Requirements |
| 0003/FR-006 (in part) | 0007, 0015, 0022 | Change 0003, Requirements |
| 0003/SC-004 | 0022 | Change 0003, Success criteria |
| 0003/FR-008 (in part) | 0005 | Change 0003, Requirements |
| 0003 Key entities, Notification | 0025 | Change 0003, Key entities |
| 0007/FR-007 | 0026 | Change 0007, Requirements |
| 0007/FR-009 | 0026 | Change 0007, Requirements |
| 0007/FR-010 | 0025 | Change 0007, Requirements |
| 0012/FR-003 | 0025 | Change 0012, Requirements |
| 0012/FR-005 | 0025 | Change 0012, Requirements |
| 0013/FR-008 | 0017 | Change 0013, Requirements |
| 0015/FR-005 | 0020 | Change 0015, Requirements |
| 0017/FR-008 | 0023 | Change 0017, Requirements |
| 0018/FR-002 | 0022 | Change 0018, Requirements |
| 0018/FR-004 | 0025 | Change 0018, Requirements |
| 0018/FR-009 | 0022 | Change 0018, Requirements |
| 0018 Terms, Idle and Idle since | 0022 | Change 0022, Terms |

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
- **0001/FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`, or `asleep`, `waiting for input` means that the CLI runs and produced no output during the quiet period, and `asleep` means that the office ended the CLI after an idle time and keeps its last session for a wake (0018/FR-003). (modified by 0018)
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

- **FR-011 lists four actions; the record holds more.** Requirement: record each create, start, stop, and delete action. Shipped: the record also holds `new_task` and `continue` (0001, T7), `team_create`, `team_delete`, `directive`, and `refused` (0002), `hold` and `release` (0007), and `task_text` (0011), 13 kinds in all (`src/modules/audit/domain/src/action_kind.rs:9`). Reason: each later change records its own actions in the same record, and the user sees them in the same details. The data model lists them. Recorded 2026-11-23 (ACF-106 archive). Changed by 0017: two more kinds, `end_leftover` and `helper_limit`, are in `specs/changes/archive/0017-orphan-sweep/contracts/sweep-protocol.md`.
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
- **0002/FR-008** (US1, US2): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that FR-001 allows, send reports only to its own parent or to the person, and read only the secrets that the person granted to that agent. (modified by 0014, 0015, 0026)
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
- **0003/FR-006** (in part) (US3): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, is put on hold automatically (0007/FR-008), or sends a report for the person (0015/FR-007), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and while an unread report notification of that agent exists, a further report adds none; (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification, and exception (b) does not apply to the automatic hold or to a report, because the terminal does not show them; and (c) an unprompted change to `waiting for input` (0022 Terms) adds no notification and sends no OS notification. (modified by 0007, 0015, 0022)
- **0003/FR-007** (US2): IF a notification occurs while the desktop app is closed, THEN THE SYSTEM SHALL show it as unread when the app opens.
- **0003/FR-008** (US2): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view, where the last view now also names the open view (panel or scene), and SHALL open that view. This changes steps 1 and 6 of the 0003 Flow, which now read "the open view" where they read "the panel", and 0003/SC-006, which now measures the open view. (modified by 0005)
- **0003/FR-009** (US2): WHEN the user opens a notification, or opens the terminal of an agent that has unread notifications, THE SYSTEM SHALL show those notifications of that agent as read.

FR-002 states R9 criterion 2. FR-006 notifies only for the four events it lists: a status change that the user caused (`stopped`) and `interrupted` after a server restart notify nobody.
Later additions to the shortcuts and the events (recorded 2026-11-23, ACF-106 archive; no requirement of this spec changed). The fixed shortcuts are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` here. Change 0008 added `Ctrl+Shift+F11` (jump to the agent that needs you), and change 0007 added `Ctrl+Shift+H` (the office hold). The server holds 12 reserved shortcuts in all (`src/modules/agent/domain/src/value_objects/settings.rs:33`). Change 0007 (FR-008) added the notification event `auto_hold` to the four events of FR-006, so the server knows five (`src/modules/notification/domain/src/enums/notification_event.rs:21`). Change 0008 added the Needs you queue. It is a separate queue with its own requests and event, not a notification event. Changed by 0017: the notification event `leftover_kept` is added; see `specs/changes/archive/0017-orphan-sweep/contracts/sweep-protocol.md`. Changed by 0020: the notification event `temp_ended` is added; see `specs/changes/archive/0020-temporary-agents/contracts/temp-protocol.md`.

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
| Notification | An event that the user must see. | agent (optional), schedule (optional), event (`exited`, `failed`, `waiting for input`, bell, and the events that later changes add, a schedule run result and missed runs of 0025 included), time, read or unread | Belongs to the office. Refers to one agent, or to one schedule of 0025 and to its target while the target exists, or, for the missed runs of one minute (0025/FR-006), to no agent and no schedule. (modified by 0025) |
| Last view | Where the user was when the desktop app closed. | project choice, selected agent, open terminal | One per desktop app on the machine. |

### Success criteria

- **0003/SC-001:** With 30 agents across 3 projects that run at the same time, the agent list in one window shows all 30, and each status change and each change of the last output line shows in the list within 2 seconds.
- **0003/SC-002:** From the panel, the user opens any agent's live terminal with 1 click or 1 shortcut, and the terminal shows live output within 1 second.
- **0003/SC-003:** The user switches from one project to another, or to "All projects", with 1 action.
- **0003/SC-004:** While the desktop app runs, the user gets a notification within 10 seconds after an agent exits, fails, rings the bell, or makes a prompted change to `waiting for input` (that is, within 10 seconds after the quiet period ends). (modified by 0022)
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
- **0007/FR-007** (US3): IF an agent's agent-made action would exceed one of the rate limits in the table under Assumption A3, with the row added by 0015 and the row added by 0026, THEN THE SYSTEM SHALL refuse the action with a reason that names the limit and its value. (modified by 0026)
- **0007/FR-008** (US3): WHEN an agent is refused by a rate limit for the third time within 60 seconds, THE SYSTEM SHALL put that agent on hold and notify the user once, naming the agent.
- **0007/FR-009** (US3): THE SYSTEM SHALL let the user change each rate limit in the office settings, the limit of reads of secrets by one agent (`limit_secret_reads_per_agent`) included, and SHALL apply a changed value to the next agent-made action. (modified by 0026)
- **0007/FR-010** (US1, US2, US3): THE SYSTEM SHALL add every hold, release, and refusal of this change to the action record, with the user as the actor of a hold or release that the user takes, the office as the actor of an automatic hold, the refused agent as the actor of a refusal of an agent-made action, and the schedule (its name, its id, and the note that the person made it) as the actor of a refusal of a schedule run (0025/FR-007). A refused directive of an agent appears in the action record of the sender and in the action record of the target; a refused run of a schedule appears once, in the action record of its target. (modified by 0025)
- **0007/FR-011** (US1, US2): THE SYSTEM SHALL show in the desktop app, as text, whether the office is on hold and which agents are on hold.

This change modified: 0003/FR-006, 0002/FR-009. The final text is under the change that owns each id. Change 0026 later modified 0007/FR-007 and 0007/FR-009; the text above is final.

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
| Reads of secrets by one agent (added by 0026) | 20 | An agent reads each secret once per task, and a loop stops. |

## Change 0012: Safe directive delivery

### Intent

A parent agent's directive was typed into the target's terminal at once, with Enter, whenever the target ran. When the target's CLI showed a question for a person, the Enter or a letter of the text could answer it, and nobody saw it. This change puts each directive in a queue per target and types it only when the target waits for input, no person types or has unsent text there, no text the office typed without Enter waits there, and the screen shows no question that the office recognises. Enter follows only after the text, or the CLI's placeholder for it, shows at the input line. The sender learns whether its directive was submitted, waits, or was left unsubmitted, and a directive that cannot be typed in time is withdrawn and recorded. This change modifies 0002/FR-010 and amends 0011 (the task line shows the last submitted directive). What the screen checks cannot cover is under Residual risk in the change spec.

### Requirements

- **0012/FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one left the queue and the conditions of 0012/FR-002 hold again.
- **0012/FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, no unsent person text waits at its input line, no pending office text (a startup prompt or an unsubmitted directive) waits in the target's current session, and the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **0012/FR-003** (US1): WHEN the system has typed a directive, THE SYSTEM SHALL press Enter only IF the end of the directive's text, or a new paste placeholder, appears at the input line of the target's screen within 2 seconds, and otherwise SHALL press no Enter, leave the typed text in the terminal, and record the directive as unsubmitted with the reason in the sender's and the target's action records; for a scheduled directive (0025), THE SYSTEM SHALL record it once, in the target's action record, with the schedule as the actor, and as an `unsubmitted` run result in the schedule's history. (modified by 0025)
- **0012/FR-004** (US2): WHEN the system accepts a directive, THE SYSTEM SHALL answer the sender that the directive was submitted, that it waits with its place in the target's queue, or that it was left unsubmitted; IF 20 directives already wait for the target, THEN THE SYSTEM SHALL refuse the directive, say that the target's queue is full, and record the refusal.
- **0012/FR-005** (US2): IF a waiting directive was not typed within 10 minutes, its target stopped running, the target is no longer the sender's descendant, a hold would now refuse the directive (0007/FR-002, 0007/FR-004, and for a scheduled directive 0025/FR-007), or the background server stops normally, THEN THE SYSTEM SHALL withdraw the directive without typing it and SHALL record the withdrawal with the directive text and the reason in the sender's and the target's action records; for a scheduled directive (0025), the condition that the target is no longer the sender's descendant does not apply, and THE SYSTEM SHALL record the withdrawal once, in the target's action record, with the schedule as the actor, and as a `withdrawn` run result in the schedule's history. (modified by 0025)

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
- **Gap found after ship (2026-11-23).** 0012/FR-002 is not met for an unnumbered choice list, such as the start question of `claude` in a new folder: the office typed the directive after the quiet period, without Enter (0012/FR-003 held). #127 closed it: the check now reads such a list as an open question.

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
- **0013/FR-008** (US1): WHERE the last session of a listed agent was still open when the server stopped uncleanly, THE SYSTEM SHALL mark that agent in the list with what the sweep (0017/FR-003) found for it, and SHALL keep Confirm unavailable while an agent with the warning is checked, until the user acknowledges the warning. (modified by 0017) The mark is one of these:
  - every leftover ended (ended or ended by force): a note with the number of processes from before the stop that the office ended, and no warning;
  - a leftover that could not be ended, a stored process identity that was not readable, a process that matched except for a boot time that changed, or no complete stored process identities for the session (a session from before 0017, a failed store, a session over the helper limit, or a sweep that could not run): the warning, shown before the user confirms, that a CLI process of that agent from before the stop may still run and that resuming can start a second copy of a CLI next to it;
  - no leftover found: no mark and no warning.

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
- **0015/FR-005** (US1): IF the sender's parent is not running, THEN THE SYSTEM SHALL refuse a report to it, say that the parent is not running, and record the refusal; IF a waiting report's sender is no longer a child of the report's parent, THEN THE SYSTEM SHALL withdraw it as 0012/FR-005 withdraws a directive whose target left the sender's subtree, EXCEPT a waiting report or end notice of a temp that the system ended, which stays in the queue. (modified by 0020)
- **0015/FR-006** (US3): THE SYSTEM SHALL treat sending a report as an agent-made action under 0007: the office hold and an agent hold that covers the sender refuse it (0007/FR-002, 0007/FR-004), and a rate limit "reports sent by one agent", 5 per rolling 60 seconds by default, refuses a report beyond it (0007/FR-007), counts toward the automatic hold (0007/FR-008), and is changed by the user in the office settings like the other limits (0007/FR-009).
- **0015/FR-007** (US2): WHEN the system accepts a report for the person, THE SYSTEM SHALL record it as an attention event of the sender, notify the owner with the sender's name and the report's kind as 0003/FR-006 (as modified) allows, and answer the sender that the report is in Needs you.
- **0015/FR-008** (US2): WHILE an item in Needs you holds a report, THE SYSTEM SHALL keep showing that report on the item when later attention events change the item's latest event, and SHALL replace it only with the sender's next report for the person.
- **0015/FR-009** (US1, US2, US3): THE SYSTEM SHALL add each accepted report to the action record with the sender as the actor, the kind, and the text in visible form, under the parent for a report to a parent and under the sender for a report for the person, and SHALL record each refusal in the sender's action record and, when the report had a parent, in the parent's.
- **0015/FR-010** (US1): WHILE reports wait in a parent's queue, THE SYSTEM SHALL keep at most one waiting report per sender, replacing a sender's waiting report with its newer report in the same place and recording the replaced one as withdrawn, and SHALL keep at most 10 waiting reports per queue, refusing a further report with the reason that the parent's queue has no room for reports, and SHALL NOT count such a refusal toward any rate limit or automatic hold.

This change modified: 0002/FR-008, 0003/FR-006, 0008/FR-002, 0008/FR-003, 0008/FR-005. The final text of 0002/FR-008 and 0003/FR-006 is under the change that owns each id. 0008 is not in this file yet: the final text of 0008/FR-002 and FR-003 is in the MODIFIED section of [0015](../changes/archive/0015-upward-reports/SPEC.md). Change 0022 modified 0008/FR-005 again, so its final text is in the MODIFIED section of [0022](../changes/archive/0022-waiting-item-sleep/SPEC.md). The fold of 0008 takes that text.

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

- **0016/SC-001:** Of 20 requests for paths outside the root (parent parts, absolute paths, a link to a file outside, a link to a folder outside, a link inside a linked folder, and a link chain that leaves the root), 100% are refused, and 0 bytes of the outside files reach the desktop app.
- **0016/SC-002:** A file of exactly 512 KiB shows in full; a file of 512 KiB plus 1 byte shows its size and 0 bytes of content; a diff over 512 KiB shows its size and 0 lines.
- **0016/SC-003:** In a folder with 10 secret-looking files that are also changed files, opening the tree, each file, the Changes part, and each diff sends 0 bytes of their content to the desktop app until a reveal; after one reveal, exactly that one file's content arrives; after opening another file, it is hidden again. A `.env` renamed to `notes.txt`, in the Changes part and in a commit, sends 0 bytes of its diff until a reveal.
- **0016/SC-004:** After a session that lists every folder, opens every file, the changes, every diff, the history, and every commit of a test repository, the root and its git data are byte for byte the same as before, with the same modification times, no lock file was created, and a hook, a diff driver, a text conversion, a content filter, a signature check program, and a file system monitor that the repository names each wrote 0 marker files.
- **0016/SC-005:** In a repository with one modified, one staged new, one deleted, one renamed, and one untracked file inside the root and one modified file outside it, the Changes part lists exactly the 5 files inside the root, each with its kind, and each diff holds the removed and added lines of that file.
- **0016/SC-006:** With 120 commits that change the root and 10 that change only another folder of the repository, the History part shows 50, then 100, then 120 commits, newest first, none of the 10 others.
- **0016/SC-007:** Each visible state appears in a test with its text, and every action of the flow completes with the keyboard alone.
- **0016/SC-008:** A file and a commit message that hold `<script>`, an image tag, a terminal escape code (U+001B), and a right-to-left override (U+202E) show as text, with U+001B and U+202E as visible symbols, and 0 scripts run and 0 images load.
- **0016/SC-009:** In a working folder of 50,000 files, a folder of 1,000 entries lists within 2 seconds, a 512 KiB file shows within 2 seconds, and the Changes part of 1,000 changed files lists within 5 seconds.

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

## Change 0017: End the processes left over from an unclean stop

### Intent

When the background server stops uncleanly (SIGKILL, a crash, or a power loss of the server process only), some agent CLI processes and their helper processes keep running. The next server start does not see them. They keep spending the owner's quota unseen, and they can keep a session busy, which can break the resume of 0013. This change makes the server remember, for each session, which processes belong to it. At the next start, before the user can resume anything, the server ends only the processes that are still exactly those processes, and records each end. The resume warning of 0013 then names only the agents that really had processes left.

### Requirements

- **0017/FR-001** (US1): WHEN the office starts a session, THE SYSTEM SHALL store the process identity of the session's CLI process before the session shows as started, and SHALL make it readable only by the OS user who owns the office (as 0001/FR-009). THE SYSTEM SHALL store the command line as a hash of the whole line, for the match, and as a display text of at most 1,000 characters.
- **0017/FR-002** (US1): WHILE a session runs, THE SYSTEM SHALL store the process identity of each process that descends from the session's CLI process within 10 seconds after that process starts, SHALL keep it while that process runs, also after it leaves the CLI's process tree, and SHALL drop it when that process ends.
- **0017/FR-003** (US1, US2): WHEN the server starts and finds `lost` sessions, THE SYSTEM SHALL run the sweep before it shows the list of the "Resume interrupted" action and before it starts any agent, SHALL ask each leftover to end and end by force each leftover that still runs after the grace period, and SHALL check the match again just before each end request. Where the OS lets the office hold a process (Linux 5.3 and later, and Windows), THE SYSTEM SHALL send each end request only to the process that it just checked, so that a process that gets the same process id later is never ended. On macOS, and on a Linux kernel without this, the time between the check and the end request is a known risk (see Notes from Deviations).
- **0017/FR-004** (US1): WHEN the sweep ends a leftover, THE SYSTEM SHALL add an entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the result "ended" or "ended by force".
- **0017/FR-005** (US2): IF a stored process identity of a `lost` session does not match a running process, THEN THE SYSTEM SHALL NOT end any process for it and SHALL add a "left" entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the reason: not running, machine restarted, boot time changed, other start time, other command line, or not readable.
- **0017/FR-006** (US1): IF a leftover still runs after the forced end, or the OS refuses to end it, THEN THE SYSTEM SHALL add a "could not end" entry with the reason and the command line in visible form to the agent's action record and SHALL notify the user once per server start, naming each agent with such a leftover.
- **0017/FR-007** (US1): WHILE the sweep runs, THE SYSTEM SHALL show, in the panel and in the "Resume interrupted" dialog, that the office ends processes left from the last run, and SHALL keep each start, new task, and continue request of the user waiting until the sweep ends, then run it.
- **0017/FR-008** (US2): WHEN a session ends while the server runs (a stop, an end of its CLI, a start failure, an end at the sign-in screen, or a clean server stop), THE SYSTEM SHALL remove every stored process identity of that session in the same step as the session end, so that no later sweep ends a process for it or records anything for it, and the office keeps no command line of that session's processes. (modified by 0023)
- **0017/FR-009** (US1, US2): WHEN the sweep records the sweep result of a stored process identity, THE SYSTEM SHALL mark that identity as ended in the same step, so that a later sweep, also after the server is killed during this sweep, neither ends a process for it nor records a second sweep result for it.
- **0017/FR-010** (US1): WHILE a session runs, THE SYSTEM SHALL store at most 200 open helper identities for it (the helper limit). WHEN a new helper would pass the limit, THE SYSTEM SHALL stop storing helpers of that session, SHALL add one entry to the agent's action record that names the limit, and the session SHALL count as having no complete stored process identities.
- **0017/FR-011** (US1): WHILE a session runs and a stored session process keeps its process id and start time but runs another command line (it replaced itself with another program), THE SYSTEM SHALL replace the stored command line with the new one within 10 seconds, and SHALL NOT mark that identity as ended.

This change modified: 0013/FR-008. The final text is under the change that owns that id.

Change 0023 modified 0017/FR-008. The text above is final.

### Preserved behaviors

- **0017/PB-1:** The server starts no agent by itself at its start (0001/FR-004). The sweep only ends processes; every `lost` agent stays `interrupted` until the user acts.
- **0017/PB-2:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021). The sweep reads process identities and ends processes; it does not touch any CLI file.
- **0017/PB-3:** The stop action ends the CLI and every process that descends from it at the time of the stop, first with a request and then by force after the grace period (0001/FR-012, 0001 A3), as today.
- **0017/PB-4:** The office never ends a process that it did not start: a process that is not a session process of a `lost` session is never ended by the sweep, also when its command line is the same as an agent's.
- **0017/PB-5:** The user's own actions work as before once the sweep ends, and the "Resume interrupted" action behaves as 0013/FR-001 to 0013/FR-007 define.
- **0017/PB-6:** After an unclean stop, the open sessions still end as `lost`, and the user can still continue a `lost` session (0013 Key entities, Session; 0001/FR-025).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity | One session process that the office knows. | process id, start time, hash of the whole command line, display text of the command line (at most 1,000 characters), machine run, kind (CLI or helper), state (open or ended). At most 200 open helpers per session (FR-010). | Belongs to one session of one agent. Readable only by the office owner (FR-001). |
| Sweep result | What one sweep did with one stored process identity. | result (ended, ended by force, could not end, left), reason | Exactly one per stored process identity of a `lost` session (FR-009). Kept as an action record entry. |
| Session (extends 0013) | One run of an agent CLI. | sweep finding of a `lost` session: leftovers ended (with the number), may still run, or none found | Drives the mark of 0013/FR-008 (modified). |
| Action record entry (extends 0001, 0007) | One recorded action. | action now also "end leftover"; result ended, ended by force, could not end, or left; process id; command line in visible form; session | Actor is the office. Shows in the details of the agent that owns the session. |

### Success criteria

- **0017/SC-001:** In a test where 30 stand-in CLIs, each with one helper in a session of its own, outlive a forced kill of the server, 100% of the 60 processes have ended before the "Resume interrupted" list shows, on Linux, macOS, and Windows in CI.
- **0017/SC-002:** In the same test, 0 unrelated processes end: a process with a stored command line and another start time, a process that got a stored process id with another command line, and a process that the office did not start all still run after the sweep, and each stored identity that they meet has exactly 1 "left" entry with its reason.
- **0017/SC-003:** With 30 `lost` sessions and the default grace period, the "Resume interrupted" list shows within 30 seconds after the server start.
- **0017/SC-004:** 100% of ended leftovers have exactly 1 action record entry with the office as actor, the agent, the session, the process id, and the command line in visible form (control characters shown as escapes, at most 1,000 characters, then an ellipsis and the hash start). A stand-in that ignores the request to end is ended by force after the grace period and has the result "ended by force".
- **0017/SC-005:** With 4 agents after a forced kill of the server (one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, and one session from before this change), the list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent.
- **0017/SC-006:** After 10 clean server stops and 10 normal agent stops, each followed by a server start, the sweep ends 0 processes and 0 agents start without an action of the user.
- **0017/SC-007:** When the server gets a forced kill during a sweep and started again, 100% of the leftovers have ended after the second start, and each stored process identity of the `lost` sessions has exactly 1 sweep result in the action record.
- **0017/SC-008:** With a session whose CLI starts 500 helper processes that stay alive, the office stores at most 200 open helper identities for it, adds exactly 1 limit entry to the action record, and after a forced kill of the server the agent shows the 0013 warning.
- **0017/SC-009:** With a stand-in CLI that replaces itself with another program while the session runs, after a forced kill of the server 100% of such processes end at the next start with the new command line in the action record.

### Notes from Deviations

- **One notification for each agent, one OS notification for each server start (0017/FR-006).** The server writes one `leftover_kept` notification for each agent with a leftover that could not be ended, because a notification refers to one agent and each agent keeps its own Needs you item. The desktop app sends one OS notification for each server start that names all those agents. It sends it when `office.sweep` says `done`, or 1.5 seconds after the first row for a client that connected during the sweep. A client that is not connected during the sweep shows the rows in its list, but no OS notification.
- **Platforms of the 60-process test (0017/SC-001).** The test with real stand-in processes runs on Linux and on macOS. On Linux it reads process start times from `/proc`. On macOS it reads them from `ps`, in whole seconds, the same unit the office stores there. On Windows the cover is the process inspection tests, which read the identity of real child processes and end them through the handle in CI on all three systems, and the sweep unit tests with a fake inspector.
- **Race on macOS and on a Linux kernel before 5.3 (assumption A10 of 0017).** There the OS gives no handle to a process, so the office checks the match and then sends the end request to the process id. A process that exits and whose id is reused in that short time can be ended wrongly. The office accepts this risk only on these systems. On macOS the start time is in whole seconds, so the recorder also cannot tell a process that replaced itself by `exec` from a new process that took the same id within the same second; the recorder then takes the new command line for the stored row.

## Change 0018: Put idle agents to sleep

### Intent

With 20 to 30 agents, many of them wait for input for hours. Each idle CLI, and the helper servers that it starts, keeps its memory all that time. This change adds an "Idle sleep" setting, off by default. When the person turns it on, the office puts an agent to sleep after it stays idle longer than a set time: it ends the CLI with the normal stop, keeps the session, and keeps the last screen visible. One key or one button wakes the agent, and the office continues the same session. When the CLI cannot continue it, the office uses the fallback and the hold of 0013.

### Requirements

- **0018/FR-001** (US1): THE SYSTEM SHALL let the person turn idle sleep on and off in the office settings, with idle sleep off by default and an idle time of 30 minutes when the person first turns it on; SHALL accept an idle time only as a whole number of minutes from 5 to 1440, and refuse any other value with that rule and keep the previous value; SHALL keep the setting in the background server across server and machine restarts; and SHALL apply each change at once, without a restart.
- **0018/FR-002** (US1): WHILE idle sleep is on, THE SYSTEM SHALL begin to put an agent to sleep within 60 seconds of the first moment at which the agent shows `waiting for input`, the idle time has passed since its idle-since moment, and none of these blocking conditions holds: (a) the agent does not show `waiting for input`; (b) it has an item in Needs you that is not a plain waiting item, that is, an item that recorded a bell, a failure, an exit, an automatic hold, a resume hold, or a kept leftover, an item that holds a report for the person, or a waiting item whose agent's screen shows an open question under the question check of 0012/FR-002; (c) a hold covers it (0007/FR-004); (d) a directive or a report waits in its queue (0012, 0015); (e) pending office text or unsent person text waits at its input line (0012); (f) one of its children runs. The idle-since moment is the latest of the agent's last prompted change to `waiting for input`, the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time; an unprompted change does not move it. THE SYSTEM SHALL check all six conditions again right before it ends the CLI, and SHALL NOT end it when one of them holds. (modified by 0022)
- **0018/FR-003** (US1): WHEN the system puts an agent to sleep, THE SYSTEM SHALL end the agent's CLI as the person's stop does (0001 A3), SHALL show the agent as `asleep` once the CLI has ended, SHALL keep the last session as the session that a wake continues, SHALL keep the recent output of that session with the lines that the CLI printed while it ended, SHALL add one action record entry for the sleep, and no separate stop entry, with the office as actor and the reason "idle for N minutes", and SHALL NOT notify the person or add a Needs you item for the sleep or for the end of the CLI that the sleep caused.
- **0018/FR-004** (US1, US2): WHILE an agent is asleep, THE SYSTEM SHALL keep it `asleep` across server and machine restarts and after idle sleep is turned off, until the person wakes it, a run of a schedule wakes it (0025/FR-010), the person or an agent-made action stops it (it then shows `stopped`), deletes it, starts a new task on it (0001/FR-024), or continues one of its sessions (0001/FR-025), or an agent-made action starts it as 0002/FR-008 allows for an agent that is not running; and SHALL NOT list it in "Resume interrupted" (0013/FR-001). (modified by 0025)
- **0018/FR-005** (US4): WHILE an agent is going to sleep or asleep, THE SYSTEM SHALL refuse a directive to it and a report to it, SHALL answer the sender that the receiver is not running because it is asleep, and SHALL record the refusal as 0002/FR-011 and 0015/FR-009 require; and WHEN a directive or a report still waits in the queue of an agent that is going to sleep, THE SYSTEM SHALL withdraw it with the reason that the receiver is asleep and record the withdrawal as 0012/FR-005 requires.
- **0018/FR-006** (US2): WHILE an agent is asleep, THE SYSTEM SHALL offer the person a Wake button in the agent's details and in its terminal view; WHILE an agent is asleep and its terminal view has keyboard focus, THE SYSTEM SHALL wake the agent when the person presses Enter and SHALL send no key from that view to any CLI; and THE SYSTEM SHALL carry out a wake while the office or the agent is on hold.
- **0018/FR-007** (US2, US3): WHEN the person wakes an agent, THE SYSTEM SHALL continue the agent's last session where its CLI supports it (0001/FR-025) and otherwise start a new session at once with the reason, SHALL apply the fallback of 0013/FR-004 and the hold of 0013/FR-005 to this start as 0013 applies them to a start of its action, and SHALL add one action record entry for the wake with the person as actor, with the reason when the wake started a new session; IF the agent is not asleep when the system carries out the wake, THEN THE SYSTEM SHALL start nothing and tell the person that the agent is no longer asleep.
- **0018/FR-008** (US2, US3): THE SYSTEM SHALL show the result of each wake as text in the agent's details and terminal view as soon as it is known: waking, checking (during the check period), continued, new session (with the reason), on hold (with the reason), or no longer asleep.
- **0018/FR-009** (US1): IF the agent's CLI does not end when the system puts the agent to sleep (the 0001 stop fails), THEN THE SYSTEM SHALL leave the agent's status as the CLI shows it, SHALL record the failure once in the agent's action record with the office as actor and the reason, and SHALL NOT try to put that agent to sleep again until it has made a prompted change to `waiting for input` after the failure and the idle time has passed since that change; an unprompted change SHALL NOT allow a new try. (modified by 0022)

This change modified: 0001/FR-014. The final text is under the change that owns that id.

Change 0022 modified 0018/FR-002, 0018/FR-009, and the Terms Idle and Idle since. The text above is final. The Terms are under Change 0022, Terms.

### Preserved behaviors

- **0018/PB-1:** No agent starts without an explicit action. The office starts no agent by itself, at a server start or at any other time (0001/FR-004, 0013 PB-1). The fallback of FR-007 follows only the person's wake.
- **0018/PB-2:** While idle sleep is off, the office behaves as before this change: it ends no CLI and starts nothing because of idle time.
- **0018/PB-3:** A hold refuses only agent-made actions. The office carries out the person's own actions, Wake included, as before (0007 PB-1, PB-2). A hold stops no CLI by itself; a hold keeps an agent awake (FR-002 c).
- **0018/PB-4:** The person's stop, new task, continue, and delete do what they did before (0001/FR-012, FR-024, FR-025). Each start, continue, stop, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).
- **0018/PB-5:** Person input reaches the terminal of every agent whose CLI runs at once and unchanged, and the app shortcuts still leave a focused terminal (0001/FR-017 as modified by 0003, 0012 PB-5).
- **0018/PB-6:** The notification of a wake hold is a notification of an automatic hold under 0003/FR-006 as modified by 0007: it names the agent and the event, and the focused-terminal exception does not apply to it (0013 PB-6).
- **0018/PB-7:** Every status, `asleep` included, shows as text, never color alone, in the agent list, the office scene, and the subtree read of 0014/FR-001 (0001/FR-014, 0005 PB-3, 0014/FR-001).
- **0018/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Idle sleep setting | Whether the office puts idle agents to sleep. | on or off, idle time in minutes, when it was last turned on or changed | One per office; kept by the background server. |
| Agent (extends 0001) | One agent. | status: also `asleep`; idle since (kept only while the background server runs) | As in 0001. |
| Session (extends 0001) | One run of an agent CLI. | the last session of an asleep agent is the one a wake continues | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | actions now also sleep, failed sleep, and wake; reason (optional) | Belongs to the agent it names (0001/FR-011). |
| Wake result | What a wake did. | agent, result (waking, checking, continued, new session, on hold, no longer asleep), reason | Shown in the agent's details and terminal view; not stored; it shows until the agent's next sleep, the person's next action on the agent, or a server restart. |

### Success criteria

- **0018/SC-001:** With idle sleep never turned on, 10 agents that wait for input for 24 hours cause 0 sleeps, 0 starts by the office, and 0 sleep entries in the action record.
- **0018/SC-002:** With an idle time of 5 minutes and 10 idle agents for which no blocking condition holds, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment (5 minutes of idle time, at most 60 seconds to begin, at most 10 seconds of stop grace time).
- **0018/SC-003:** For each of the blocking conditions (b) to (f) of FR-002, an idle agent for which only that condition holds is put to sleep 0 times in 3 times the idle time; for condition (a), an agent that changes to `working` between the first and the last check is stopped 0 times in 10 tries.
- **0018/SC-004:** After the person turns idle sleep on with 30 minutes, 10 agents that were idle for 3 hours cause 0 sleeps in the first 30 minutes.
- **0018/SC-005:** For an asleep agent whose CLI can continue its session, 1 key press (Enter) or 1 button press wakes it, and in 100% of 20 wakes the agent runs its last session again within 10 seconds of the press.
- **0018/SC-006:** Over 20 sleep and wake cycles, each sleep adds exactly 1 action record entry (office as actor) and each successful wake adds exactly 1 (person as actor), and the sleeps cause 0 notifications and 0 Needs you items.
- **0018/SC-007:** A wake of an agent whose CLI ends within 1 second of every start starts the agent at most 2 times; then the agent is on hold and exactly 1 notification names it.
- **0018/SC-008:** After 10 server restarts, 5 of them after a SIGKILL of the server, 100% of the agents that were asleep still show `asleep`, 0 of them start without a person action, and "Resume interrupted" lists 0 of them.
- **0018/SC-009:** 100% of the directives and reports sent to an asleep agent are refused, and each answer says that the receiver is asleep.

### Notes from Deviations

- **A sleep whose CLI ended on its own (0018/FR-009).** A sleep whose CLI ended on its own just before the stop records one failed `sleep` entry with the reason "the CLI had already ended", answers `conflict` with that text, and changes no status: the exit event of the CLI sets it as for any exit. The agent is not asleep then. The office does not bar the agent, because the stop did not fail, and it has left `waiting for input` anyway.
- **A client cannot wake an agent past the watch.** A user client cannot send `agent.wake`. It gets `invalid` "agent.wake is made by wake.start only", as for `agent.sleep`, because a bare wake skips the 0013 watch. `wake.start` and the bulk resume run it inside the server. The desktop app sends only `wake.start`.
- **Refused directive to an asleep agent (0018/FR-005).** The action record shows the refusal with the same `invalid: ` prefix that the not-running refusal has. The wire message is the contract text "the agent is not running, it is asleep: <name>".
- **Idle time check in two places.** The desktop app checks the idle time (0, or 5 to 1440) before it sends `settings.set`, and the server checks it again. The server decides.
- **Sign-in check before a wake.** `agent.wake` is a starting request, so the sign-in check of 0019 also runs before a wake, and a stopping server refuses a wake as it refuses any start.
- **The wake result is not stored (assumption A13 of 0018).** It shows until the agent's next sleep, the person's next action on the agent, or a server restart. The action record, the status, and the hold list keep what happened.

## Change 0019: Check that the CLI is signed in before a start

### Intent

When an agent's CLI is not signed in, a start today looks like a success: the agent shows `starting`, then `waiting for input`, and the CLI waits at its sign-in screen. Nobody is there to sign in. The user finds out only when the agent does no work, and a parent agent or a bulk resume cannot tell at all. This change asks the CLI before each start whether it is signed in, with the CLI's own read-only status command, and also watches the first seconds of the CLI's screen for a sign-in screen. When the CLI is not signed in, the start ends with one clear reason that says so and what to do. When the check cannot tell, the start works as it does today.

### Requirements

- **0019/FR-001** (US1, US2): WHEN a session of an agent is about to start (a start, a new task, a continue, an agent-made start, or a start of a bulk resume) AND the agent's CLI has a known status command (A1) AND none of the conditions of FR-010 holds, THE SYSTEM SHALL run that status command before it starts the CLI, and SHALL start the CLI only when the answer is not "not signed in".
- **0019/FR-002** (US1): IF the status command ends with a non-zero exit AND its output contains the CLI's not-signed-in words (A2), THEN THE SYSTEM SHALL NOT start the CLI, and SHALL record the request in the action record with the not-signed-in reason as its failure; for an agent that does not run, it SHALL store the session as one that never started with that reason and show the agent as `failed`; for an agent that runs (a new task or a continue), it SHALL refuse the request with that reason and leave the running session as it is.
- **0019/FR-003** (US2): IF the status command has not ended 5 seconds after it began, THEN THE SYSTEM SHALL end the status command and every process it started, and SHALL go on with the start as if no status command were known.
- **0019/FR-004** (US2): IF the sign-in check gives no answer (A2), or the agent's command, working folder, or office additions changed between the check and the start, THEN THE SYSTEM SHALL go on with the start as if no status command were known, and SHALL note in the start's action record entry that the sign-in check gave no answer and why: timed out, could not run, unclear answer, or the agent changed during the check.
- **0019/FR-005** (US3): WHILE a session is in its watch period AND neither a person nor the office typed into its terminal (the startup prompt aside, A5), WHEN the bottom of the CLI's screen shows the sign-in screen words of that CLI (A5), THE SYSTEM SHALL end the CLI, SHALL end the session as not signed in with the not-signed-in reason, SHALL show the agent as `failed`, SHALL record the end in the action record with the office as actor and the not-signed-in reason, and SHALL notify the user once as for a failed agent (0003/FR-006).
- **0019/FR-006** (US1, US2): THE SYSTEM SHALL run the status command without input and without a terminal, with the agent's program, working folder, login environment, and office additions (0001/FR-020), without the office's agent-call credential (0002), SHALL keep at most 64 KiB of its output in memory only, and SHALL read and write no file for the sign-in check.
- **0019/FR-007** (US1, US3): THE SYSTEM SHALL use one not-signed-in reason that names the CLI, says that it is not signed in, says how the office found it (the status command it ran, or the sign-in screen), says to sign in to the CLI in the user's own terminal and then start the agent again, and quotes at most one line of what the CLI showed, in visible form and at most 200 characters long.
- **0019/FR-008** (US1, US2, US3): THE SYSTEM SHALL NOT show, record, store, or log any output of a status command except the one quoted line of FR-007 after a not-signed-in answer, and SHALL NOT show, record, store, or log any screen text of the watch except the one quoted row of FR-007 after a match.
- **0019/FR-009** (US2): WHILE a status command runs, THE SYSTEM SHALL let other start, stop, new task, continue, and exit handling go on without waiting for it.
- **0019/FR-010** (US2): IF the agent's command has arguments of its own, OR the office additions hold flags, OR a sign-in key variable of the CLI (A11) is set in the office additions or the login environment, THEN THE SYSTEM SHALL run no status command for that start and SHALL start the CLI as if no status command were known.
- **0019/FR-011** (US3): IF the startup prompt holds one of the CLI's sign-in screen words, THEN THE SYSTEM SHALL NOT watch that session for a sign-in screen.

### Preserved behaviors

- **0019/PB-1:** office-harness creates, changes, or deletes no file in an agent CLI's configuration (0001/FR-021). A status command is the CLI's own program; what it writes into its own folders is the same as at any start of that CLI (A4).
- **0019/PB-2:** A start of an agent whose CLI has no known status command, and that shows no sign-in screen, works as before: no command runs before it and nothing ends it.
- **0019/PB-3:** The startup prompt is typed in after the first output and is never submitted (0001, D5); the check does not change this.
- **0019/PB-4:** A start of a bulk resume that ends as not signed in counts as a start that does not start (0013/FR-004, 0013/FR-005): the fallback start is tried once, and then the agent is put on hold with the reason.
- **0019/PB-5:** No agent starts without an action of the user or of an agent (0001/FR-004). The sign-in check starts no CLI session.
- **0019/PB-6:** Each start, stop, and failure is recorded in the action record with its actor (0001/FR-011).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Sign-in check result | What the check found for one start. | answer (signed in, not signed in, no answer), reason of no answer (timed out, could not run, unclear answer), the quoted line | Not stored by itself. A not-signed-in answer is kept as the never-started reason of the session and the failure of the action record entry; a no answer is kept as a note on the entry. |
| Session (extends 0001) | One run of an agent CLI. | end result: also "not signed in", with the reason, when the office ended it at a sign-in screen | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | note (optional): the sign-in check gave no answer, and why | Belongs to the agent it names (0001/FR-011). |

### Success criteria

- **0019/SC-001:** With a test CLI whose status command says it is not signed in, 100% of 12 starts of agents that do not run (3 each of start, new task, continue, and agent-made start) end `failed` with the not-signed-in reason in the answer, the session list, and the action record, and 0 CLI sessions run.
- **0019/SC-002:** With a test status command that never ends, 100% of 5 starts begin their CLI within 6 seconds of the request, and 1 second after each timeout 0 processes of the status command run.
- **0019/SC-003:** With a test CLI that shows a known sign-in screen within its first 2 seconds, 100% of 5 starts end within 3 seconds of the screen, each with exactly 1 action record entry with the office as actor and exactly 1 notification.
- **0019/SC-004:** With a test CLI that shows the sign-in words after the person typed into its terminal, or after its first 20 seconds, 0 of 5 sessions are ended by the office.
- **0019/SC-005:** With test CLIs whose status command and screen print 10 KiB that hold control characters, bidirectional marks, and a line shaped like a secret key, 0 bytes of a signed-in or no-answer output appear in an answer, the action record, the session list, or the server log, and 100% of shown reasons quote at most 200 characters with 0 raw control or bidirectional characters.
- **0019/SC-006:** A sign-in check against a test CLI whose status command writes nothing leaves its home folder and the office's data folder with 0 created, changed, or deleted files.
- **0019/SC-007:** With a real `claude` and a real `codex`, each signed in and each not signed in, 100% of the 4 starts behave as US1 says (checked by hand on Linux; the result goes into the QA report).
- **0019/SC-008:** While a test status command hangs for one agent, 100% of 5 stops of another running agent end within 1 second of the request.
- **0019/SC-009:** With a test status command that always says not signed in, 0 of 9 starts are refused when the agent's command has its own arguments (3), the office additions hold a flag (3), or a sign-in key variable is set (3), and each of them starts its CLI.
- **0019/SC-010:** With a test CLI that shows the sign-in words only inside the startup prompt or inside a directive typed in its first 20 seconds, 0 of 6 sessions are ended by the office.

### Notes from Deviations

- **A stop between the mark and the exit.** A person's stop that comes after the watch marked the run but before the run exits leaves the agent `stopped`: the exit then gives no `failed` status and the office adds no `stop` entry. The office records its `stop` entry only after the status followed.
- **Group signal before the child is reaped.** Where the system can look at an exit without reaping it (Linux, Android, FreeBSD), the group of a status command or git child is signalled before the child is reaped, and the caller takes its status after that. On other systems the process id of a reaped child could be reused before the signal; the window needs a helper that holds a pipe after the child exited and then left the group.
- **Assumptions that the requirements name.** A bare `A1`, `A2`, `A4`, `A5`, or `A11` in the requirements above names an assumption of 0019, kept here:
  - **A1:** The known status commands are `claude auth status --text` for `claude` and `codex login status` for `codex`. `gemini` and every other command have none. The CLI is known by the program name of the agent's command.
  - **A2:** The answer is "not signed in" only when the status command ends with a non-zero exit and its output contains the CLI's not-signed-in words: `not logged in` for both `claude` and `codex`, compared without case. Exit 0 is "signed in". Every other result is "no answer" (unclear answer).
  - **A4:** The status command is the CLI's own program, run with the agent's environment. Like any start of the CLI, it can write the CLI's own files in the CLI's own folders. The office itself opens no file of a CLI for the check and writes none.
  - **A5:** The watch period is the first 20 seconds of a session. The office reads the bottom 15 rows that show anything, as 0012 does, and compares the sign-in words of the CLI without case. The watch ends at the first key the person types into the terminal, at the first directive the office types into it, at the end of the watch period, or when the session ends. The startup prompt does not end the watch, because a prompt that holds a sign-in word turns the watch off for that session (0019/FR-011).
  - **A11:** The sign-in key variables are the names that the archived 0019 spec lists, for `claude` and for `codex`. With one of them, or with arguments of its own, a CLI can work while its status command says it is not signed in. The office skips the check (0019/FR-010). Only a variable's name is checked, never its value.
  - The timeout of 5 seconds (0019/FR-003) is not a setting.

## Change 0020: Temporary agents

### Intent

A lead that gives a member one small job keeps that member in the tree after the job is done: only the person deletes agents (0002 A4), so helpers pile up, each with its CLI, its memory, and its place in the tree. This change adds the temporary agent, a "temp": an agent that the person or a parent agent creates for one job, with an objective. A temp starts at once, works in its own branch and worktree by default when its folder is in a git repository (0010), and reports `done` through the upward report channel (0015). The office then ends it: it stops the CLI, removes the agent, and keeps the results. A worktree or branch that holds work that is not merged stays, and the done report stays visible in a list of finished temps until the person dismisses it. A cap limits how many temps exist at once, an idle limit ends a temp that stops producing output, a temp whose parent goes away ends with it, and the person sees and can end every temp in one view.

### Requirements

- **0020/FR-001** (US1, US2): WHEN the person, or a running agent as 0002/FR-008 allows, creates an agent with the temp choice on and an objective that follows the text rules of a directive (0002), THE SYSTEM SHALL create the agent as a temp at the place the 0002 rules allow, start it at once, and put the objective into the temp's queue as its first entry, typed under the rules of 0012 as for a directive whose sender is the creator; IF the objective is missing or breaks those rules, THEN THE SYSTEM SHALL create nothing, give the reason, and record the refusal; and WHEN a temp creates an agent, THE SYSTEM SHALL create it as a temp.
- **0020/FR-002** (US1): WHERE the working folder of a new temp is inside a git repository, THE SYSTEM SHALL create the temp with its own branch and worktree as 0010/FR-002 does, unless the creator turns own worktree off in the create; WHERE the folder is not inside a git repository, THE SYSTEM SHALL create the temp without one.
- **0020/FR-003** (US3): IF a create would make the number of temps in the office greater than the temp cap, THEN THE SYSTEM SHALL create nothing, refuse it with a reason that names the cap and its value, record the refusal, and SHALL NOT count the refusal toward any rate limit or the automatic hold (0007/FR-008); THE SYSTEM SHALL let the person set the temp cap in the office settings as a whole number from 1 to 100, 8 by default, and SHALL apply a changed value to the next create without ending any temp.
- **0020/FR-004** (US3): WHILE a temp is running or asleep, no child of it runs, no hold covers it, and it has no item in Needs you (0008), THE SYSTEM SHALL end the temp within 60 seconds after the time since its quiet-since moment reaches the idle limit, with the reason that it produced no output for that many minutes; THE SYSTEM SHALL let the person set the idle limit in the office settings as a whole number of minutes from 5 to 1440, 30 by default, and SHALL apply a changed value at the next check.
- **0020/FR-005** (US1, US2, US3, US4): THE SYSTEM SHALL end a temp, with the reason in brackets, WHEN it accepts a `done` report from the temp (done), WHEN FR-004 applies (no output for N minutes), WHEN the temp's CLI exits or fails other than by a stop or a sleep (exited with its exit code, or failed with its reason), WHEN the person or an agent stops the temp, alone or with an ancestor (stopped by the actor), WHEN the person ends it from the temps view (ended by the person), WHEN the person deletes it, alone or with an ancestor (deleted by the person, or its parent was deleted), and WHEN its parent temp ends (its parent ended); SHALL decide each end on the temp's status as it is under the office's order of status changes, so that a status that a new task or a continued session (0001/FR-024, 0001/FR-025) passes through ends no temp; and SHALL NOT end a temp only because it is `interrupted`.
- **0020/FR-006** (US1, US4): WHEN the system ends a temp, THE SYSTEM SHALL end its child temps first, stop its CLI as the person's stop does when it runs, apply the keep rule to its own worktree (or, for a delete, the person's keep or remove answer of 0010/FR-005), remove the agent as a delete does, add one finished temp entry, and add one action record entry for the end with the reason and with the person, the stopping agent, or the office as the actor; and IF the office cannot remove a worktree or a branch that the keep rule removes, THEN THE SYSTEM SHALL keep it and name it and the failure in the finished entry.
- **0020/FR-007** (US3): WHEN the system ends a temp because of the idle limit or because its CLI exited or failed, THE SYSTEM SHALL put one end notice, with the temp's name in visible form, its id, and the reason, into the parent's queue under 0015/FR-004 and 0015/FR-010 as a report of that temp WHERE the parent runs and its queue has room; and, for an end because of the idle limit whose notice reaches no queue, THE SYSTEM SHALL notify the person once with the temp's name and the reason (an exit or a failure notifies the person under 0003/FR-006 as before).
- **0020/FR-008** (US1, US2): THE SYSTEM SHALL keep each finished temp entry across server and machine restarts until the person dismisses it, and a dismiss SHALL remove only the entry, never a worktree, a branch, or a file.
- **0020/FR-009** (US2): THE SYSTEM SHALL show the person, in one temps view of the desktop app, every temp with its name, its creator, its parent, its objective, its status, its run time, and its quiet time, and every finished temp entry, newest first; SHALL let the person end each temp and dismiss each entry from that view with the keyboard alone, ending after one confirmation that says the temp will be removed, that work not merged stays, and that files git ignores are removed with a removed worktree, and SHALL ask the same confirmation when the person stops a single temp from its details or its terminal view; and SHALL mark a temp as "temp", as text, in its details.

This change modified: 0015/FR-005. The final text is under the change that owns that id.

### Preserved behaviors

- **0020/PB-1:** An agent that is not a temp keeps every behavior it has: the office does not end, remove, or idle-limit it, and an agent-made create of an agent that is not a temp has no own worktree (0010 A7).
- **0020/PB-2:** The 0002 tree rules stay as they are: the CEO creates team leads, a team lead creates members of its team, a member creates no agents, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009). A temp create follows them.
- **0020/PB-3:** Holds and rate limits apply to a temp create as to any agent-made create (0007); the person's own actions are never held or rate-limited (0007/PB-1). The temp cap is not a rate limit; it applies to every create of a temp.
- **0020/PB-4:** A delete by the person still asks once whether to keep or remove own worktrees, with keep as the default, and asks again for a worktree with uncommitted changes (0010/FR-005, 0010/FR-006). The office removes no worktree with uncommitted changes and no branch with commits that its base lacks without that answer.
- **0020/PB-5:** A temp's reports follow every 0015 rule: the parent from the tree, the visible form, the queue and its limits, the rate limit, and Needs you for a report for the person. An objective and an end notice are typed under the 0012 rules: queue, readiness, question check, and the input-line check before Enter.
- **0020/PB-6:** The office starts no agent by itself after a restart (0001/FR-004, 0013/PB-1); an `interrupted` temp waits until the person resumes, ends, or deletes it.
- **0020/PB-7:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Temp (an agent, extended) | An agent for one job. | the temp mark, objective, creator, created time | An agent of 0001 and 0002; its children are temps; removed when it ends. |
| Finished temp entry | What stays of a temp after it ends. | temp name and id, objective, creator, parent, start and end time, end reason, last `done` report (kind, text), kept worktree folder and branch or none, a removal failure | One per ended temp; kept until the person dismisses it; no link to a live agent. |
| Temp settings | The office bounds of temps. | temp cap (1 to 100, default 8), idle limit (5 to 1440 minutes, default 30) | One set per office, with the 0007 settings. |

### Success criteria

- **0020/SC-001:** The person creates a temp with an objective in a git repository; 1 of 1 starts in its own worktree, and its terminal receives the objective with Enter once its CLI waits for input.
- **0020/SC-002:** A lead creates 3 member temps; each sends a `done` report. 3 of 3 reports reach the lead's terminal, 3 of 3 temps are no longer agents within 10 seconds after their report, and 3 finished entries show the reports.
- **0020/SC-003:** Of 4 temps that end with a clean worktree, an uncommitted change, a new commit, and both, the clean one's worktree and branch are removed and 3 of 3 others keep both, with 0 changed files lost.
- **0020/SC-004:** With the cap at 3 and 3 temps in the office, a 4th create by an agent and one by the person are refused with the cap and its value in the reason (2 of 2), 0 agents are created, and 5 such refusals of one agent within 60 seconds put it on 0 holds.
- **0020/SC-005:** With the idle limit at 5 minutes, a temp with no output and no Needs you item ends within 6 minutes, its finished entry says "no output for 5 minutes", and its running parent's terminal shows 1 end notice. A temp that prints one line every 60 seconds for 15 minutes, a temp under a hold for 10 minutes, and a temp whose Needs you item stays for 10 minutes, are not ended (0 of 3).
- **0020/SC-006:** The person deletes a lead with 3 member temps and keeps the worktrees: 3 of 3 finished entries name the deleted parent, and 3 of 3 worktrees and branches stay.
- **0020/SC-007:** A lead temp with 2 running member temps sends `done`: both members end before the lead, with the reason that their parent ended (2 of 2).
- **0020/SC-008:** 5 finished entries survive a server restart (5 of 5); dismissing 1 removes the entry and 0 files, worktrees, or branches.
- **0020/SC-009:** With 10 temps in the office at several places in the tree, the temps view lists 10 of 10, and the person ends 1 with the keyboard alone.
- **0020/SC-010:** A temp's `done` report that waits in a working parent's queue when the temp ends is typed into the parent's terminal once the parent is ready (1 of 1).
- **0020/SC-011:** A temp whose CLI exits with code 1 before any `done` report ends within 5 seconds, and its finished entry names exit code 1 (1 of 1).
- **0020/SC-012:** Of 10 new tasks and 10 continued sessions on running temps, 0 end a temp.

### Notes from Deviations

- **Dismiss record.** A dismiss adds a `dismiss` entry (actor `user`, detail the entry id) to the ended temp's action record.
- **Refused from the role path.** An agent's create through the role path returns the cap refusal as `refused`, not as `internal`.
- **Temp create and the 0017 sweep gate.** A person's temp create starts its CLI inside the create, so it waits for the sweep gate like `agent.start`: no CLI starts while the sweep runs. An agent cannot create a temp during the sweep, because no agent CLI runs before a gated start.

## Change 0021: Installed packages in new agent worktrees

### Intent

When the office creates an agent's own worktree (0010), the new worktree has no installed packages, because git does not check out the files that it ignores. The agent must run a full install before it can build or test, and many agents mean many installs. This change gives a new worktree links to the installed Node.js packages of the base checkout, so the agent's CLI can build and test at once. The links never count as work: git ignores them, the delete question does not count them, and the office removes them without following them, so it never changes the base checkout. A new office setting, "Worktree dependencies", on by default, lets the person turn this off.

### Requirements

- **0021/FR-001** (US3): THE SYSTEM SHALL let the person turn "Worktree dependencies" on and off in the office settings, on by default, with a help text that says that new own worktrees share the checkout's installed Node.js packages and that an install, update, or removal of packages in such a worktree changes the checkout's packages; SHALL keep the setting in the background server across server and machine restarts; SHALL apply a change to the creates that begin after it and leave the dependency links of existing worktrees as they are; and SHALL show the state of the setting as text: loading, on, off, the reason when it cannot be loaded, and the reason when a change cannot be saved, with the previous value kept.
- **0021/FR-002** (US1): WHEN the office creates an agent's own worktree (0010/FR-002, 0010/FR-009) WHILE Worktree dependencies is on, THE SYSTEM SHALL make dependency links at each dependency place that meets the conditions of FR-003, before the agent's CLI starts, so that the worktree's dependency folder at that place gives the installed packages of the base checkout at the same place.
- **0021/FR-003** (US1, US2): THE SYSTEM SHALL make dependency links at a dependency place only when all of these hold: (a) Worktree dependencies is on; (b) the office does not run on Windows; (c) the base checkout has a dependency folder at that place; (d) git ignores that dependency folder in the base checkout; (e) git, inside the new worktree, ignores a dependency folder at that place; (f) the new worktree has nothing (no file, folder, or link) named `node_modules` at that place; and at any other place THE SYSTEM SHALL make, change, and remove nothing, and SHALL create the agent as 0010 does.
- **0021/FR-004** (US2): IF a dependency link would reach an outside target after every link on its way is followed, THEN THE SYSTEM SHALL NOT make that link, SHALL write the place and the reason to the background server's log, and SHALL go on with the other dependency links and with the create.
- **0021/FR-005** (US2): WHILE a worktree has dependency links, THE SYSTEM SHALL keep them out of every list of changes: git inside the worktree ignores them, so the delete question (0010/FR-005, 0010/FR-006) counts none of them and the agent's own `git status` lists none of them; and THE SYSTEM SHALL write nothing for them into the repository's git data or configuration beyond the branch and the worktree record of 0010 (no ignore or exclude entries), and nothing into the base checkout.
- **0021/FR-006** (US2): WHEN the office removes a worktree (0010/FR-007) or undoes a create that failed (0010/FR-004), THE SYSTEM SHALL first remove each dependency link that it made in that worktree and that still points into the base checkout, without following it, also after a restart of the background server; SHALL then remove the worktree as 0010 does; and SHALL follow no link during the removal, so that it changes and removes no file or folder in the base checkout or anywhere else outside the worktree folder.
- **0021/FR-007** (US1): IF the office cannot make a dependency link at a dependency place (for example, the disk is full or the file system has no links), THEN THE SYSTEM SHALL remove the dependency links that it made at that place and leave nothing else that it made there, SHALL write the place and the reason to the background server's log, SHALL go on with the other places, and SHALL create the agent; THE SYSTEM SHALL NOT fail a create because of a dependency link.
- **0021/FR-008** (US3): WHILE own worktree is on in the create form, THE SYSTEM SHALL show a note that says that untracked files, such as local environment files, are not copied (0010 A8), and, WHILE Worktree dependencies is on and the office does not run on Windows, that installed Node.js packages are shared from the checkout and that an install, update, or removal of packages in the worktree changes the checkout's packages; otherwise, that installed packages are not shared, so the agent installs them.

### Preserved behaviors

- **0021/PB-1:** An agent created without own worktree starts in its chosen folder as the person's shell would, with no dependency link (0010 PB-1, 0001/FR-020).
- **0021/PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), and nothing in the person's own repository folder; it adds only a branch and a worktree record to the repository's git data (0010 PB-2).
- **0021/PB-3:** The delete question still lists each worktree with whether it has uncommitted changes, and the second confirmation still counts every changed, added, deleted, or untracked file that git does not ignore (0010/FR-005, 0010/FR-006).
- **0021/PB-4:** IF an agent's worktree folder is missing when the agent starts, THEN the office does not start the CLI and says that the worktree folder is missing (0010/FR-008).
- **0021/PB-5:** Other files that git ignores in a worktree are still not counted as uncommitted changes and are still removed with the worktree (0010 A9).
- **0021/PB-6:** A create that fails for a reason of 0010 still creates no agent and leaves no branch, folder, or dependency link behind (0010/FR-004).
- **0021/PB-7:** Each create and delete is recorded in the action record with its actor, as before (0001/FR-011, 0010 PB-4). The dependency links add no entry.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Worktree dependencies setting | Whether the office makes dependency links in new own worktrees. | on or off | One per office; kept by the background server. |
| Dependency link | One link that the office made in an agent worktree to installed packages of the base checkout. | place in the worktree, target in the base checkout | Belongs to exactly one agent worktree; removed before that worktree. In a kept worktree, it belongs to no agent. |
| Agent worktree (extends 0010) | The branch and folder of an agent with own worktree. | the dependency links that the office made in it, kept across restarts | As in 0010. |

### Success criteria

- **0021/SC-001:** In a repository whose base checkout has installed, ignored dependency folders at the top and in one workspace folder, an agent in a new own worktree runs the project's test command with 0 installs before it, and the command passes.
- **0021/SC-002:** Right after the create, `git status` in the worktree lists 0 entries, and the delete question counts 0 changed files for the worktree.
- **0021/SC-003:** After the office removes 10 such worktrees, 5 of them after a restart of the background server, and undoes 3 failed creates, 0 files in the base checkout's dependency folders are changed or missing, compared by name, size, and content with a listing made before the creates.
- **0021/SC-004:** With Worktree dependencies off, 10 creates with own worktree make 0 dependency links.
- **0021/SC-005:** In a base checkout where one package of the top dependency folder is a link to a folder outside the base checkout, and the dependency folder of a second place is itself a link to a folder outside it, a create makes 0 dependency links that reach an outside target, makes a link for every other package, and creates the agent.
- **0021/SC-006:** On Windows, 10 creates with own worktree make 0 dependency links, and 100% of them create the agent as 0010 does.
- **0021/SC-007:** On a file system without links, 10 creates with own worktree create 10 agents, leave 0 dependency links, and write 1 log line with the reason for each place that met the conditions of FR-003.

### Notes from Deviations

- **Log line.** The reasons of 0021/FR-004 and 0021/FR-007 reach the background server's log as plain lines. Each line starts with `dependency links: ` and then names the place and the reason. No storage alert shows in the app.
- **Where a place's links are found at removal (0021/FR-006).** The office finds the dependency places again from the worktree's tracked `package.json` files. A place whose `package.json` the agent deleted keeps its links until git removes the worktree. Git's removal deletes a link without following it, so the base checkout is still never changed.
- **None of the notes defers or changes a requirement.**

## Change 0022: Idle agents sleep past their waiting item, and one wait gives one item

### Intent

A hands-on run of idle sleep (0018) with a real `claude` showed two faults. First, each change from `working` to `waiting for input` adds a Needs you item, and any item keeps the agent awake, so an idle agent sleeps only after the person opens its terminal or dismisses the item. Second, `claude` redraws a hint line now and then while it waits. The office reads that output as `working` for a moment, the item leaves, and the return to `waiting for input` adds a new item and a new notification: the run saw 4 to 8 of them per agent with no input. With 20 to 30 agents, an idle `claude` that nobody looks at never sleeps, and the person learns to ignore the alerts. This change lets an item that only says "waiting for input" stop keeping an agent awake, keeps every other item kind as a blocker, and makes one wait give one item and one notification.

### Requirements

- **0022/FR-001** (US1): WHEN the system puts an agent that has a plain waiting item to sleep, THE SYSTEM SHALL keep that item in Needs you with its wait time and its latest event, SHALL record no event on it for the sleep, and SHALL remove it only as 0008/FR-004 and 0008/FR-005 say, for example when the person opens the agent's terminal, dismisses the item, or wakes the agent and it changes to `starting`.
- **0022/FR-002** (US1): IF the system cannot read the screen of an agent that has a waiting item at a check of 0018/FR-002, THEN THE SYSTEM SHALL treat that item as not plain and SHALL NOT put the agent to sleep at that check.

This change modified: 0003/FR-006, 0003/SC-004, 0008/FR-001, 0008/FR-005, 0018/FR-002, 0018/FR-009, and the 0018 Terms Idle and Idle since. The final text of 0003/FR-006, 0003/SC-004, 0018/FR-002, 0018/FR-009, and the Terms is under the change that owns each id, or under Terms below. 0008 is not in this file yet: the final text of 0008/FR-001 and 0008/FR-005 is in the MODIFIED section of [0022](../changes/archive/0022-waiting-item-sleep/SPEC.md), and the fold of 0008 takes that text.

### Preserved behaviors

- **0022/PB-1:** An item that recorded a bell, an automatic hold, a resume hold, or a kept leftover, an item that holds a report for the person, and a waiting item whose agent's screen shows an open question under the 0012 question check keep the agent awake (0018/FR-002 b, as modified). Holds, waiting directives and reports, pending office text, unsent person text, and running children keep it awake as before (0018/FR-002 c to f).
- **0022/PB-2:** When the person opens an agent's terminal by any route or dismisses its item, the item leaves Needs you at once (0008/FR-004), and opening the terminal marks the agent's notifications read (0003/FR-009). Dismiss changes no status and is not recorded (0008 A4).
- **0022/PB-3:** An agent that works after a wait end, such as a person input, a directive, a report, or office text, and then changes back to `waiting for input` gets a new item and 1 notification, under the read rule of 0003/FR-006 (b), and its earlier item leaves when it changes to `working` (0008 US1 scenario 5).
- **0022/PB-4:** A bell, an exit, a failure, an automatic hold, a resume hold, a kept leftover, and a report for the person notify and change the item as before, with the coalescing of 0003/FR-006 (a). This change touches only the change between `working` and `waiting for input`.
- **0022/PB-5:** The status that the office shows does not change: an unprompted change still shows `working` for as long as the CLI prints, in the agent list, the office scene, and the subtree read of 0014 (0001/FR-014).
- **0022/PB-6:** While idle sleep is off, the office puts no agent to sleep and starts nothing because of idle time (0018 PB-2). A sleep adds no notification and no item (0018/FR-003).
- **0022/PB-7:** A failed sleep leaves the agent's status as the CLI shows it and adds exactly 1 action record entry, with the office as actor and the reason (0018/FR-009, as modified).
- **0022/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration, and the question check reads only the screen copy that the office keeps (0001/FR-021, 0012 PB-4).

### Terms

Terms that this change defines. The other terms are as in 0001, 0003, 0007, 0008, 0012, 0013, 0015, 0017, and 0018.

| Term | Meaning |
|---|---|
| Work bound | The quiet period plus 10 seconds: 15 seconds with the default quiet period of 5 seconds (A10). |
| Wait end | One of these events for one agent: (1) a person input reaches its terminal; (2) the office types text into its terminal, such as a directive, a report, a startup prompt, or other office text, with or without Enter; (3) a session of its CLI starts, continues, or ends for any reason (exit, failure, stop, sleep, sign-in check, server stop); (4) the agent has shown `working` without a break for longer than the work bound. |
| Unprompted change | A change of an agent's status from `waiting for input` to `working`, or from `working` back to `waiting for input`, when no wait end occurred since the agent last changed to `waiting for input` in its current session. A redraw of a hint line is one example. A change to `working` is unprompted at first; when that stretch of `working` passes the work bound, the stretch is a wait end (4), and the return to `waiting for input` is prompted. |
| Prompted change | A change of status between `working` and `waiting for input` that is not unprompted. The first change to `waiting for input` in a session is always prompted, because the start of the session is a wait end. |
| Wait | The time from an agent's prompted change to `waiting for input` until its next wait end. Unprompted changes stay inside the same wait. |
| Waiting item | An item in Needs you that recorded only changes to `waiting for input` since it was added and that holds no report for the person. |
| Plain waiting item | A waiting item for which the question check finds no open question on the agent's screen at a check of 0018/FR-002. |

Modified terms of 0018 (final text):

- **Idle:** The agent's status is `waiting for input`. The idle time counts from the idle-since moment, which an unprompted change (0022 Terms) does not move.
- **Idle since:** The latest of three moments: the agent's last prompted change to `waiting for input` (0022 Terms), the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time. An unprompted change does not move it.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Wait | The time from a prompted change to `waiting for input` until the next wait end. | agent, start time, whether a wait end has occurred, start of the current stretch of `working` | At most one open wait per agent. It ends at a wait end. It is not shown and lives only while the session runs; a server stop is a wait end. |
| Item (extends 0008) | One agent that waits for the person. | adds: whether it recorded any event other than a change to `waiting for input` | As in 0008: at most one per agent, kept by the background server. Whether it is a waiting item follows from this attribute and its report; whether a waiting item is plain follows from the question check at each sleep check. |

### Success criteria

- **0022/SC-001:** With an idle time of 5 minutes and 10 idle agents that each have a plain waiting item and no other blocking condition, and with no person action, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment, and 100% of their items are still in Needs you afterwards.
- **0022/SC-002:** For each of these item kinds alone, a recorded bell, a recorded automatic hold, a recorded resume hold, a report for the person, and an open question on the screen (a yes-or-no choice, a choice list with a selection mark, and a request for permission), an idle agent whose only blocking condition is that item is put to sleep 0 times in 3 times the idle time.
- **0022/SC-003:** A test CLI that answers one prompt and then prints one line on its own every 30 seconds for 20 minutes, with no input, causes exactly 1 `waiting for input` notification and exactly 1 item, and the item's wait time at the end is at least 20 minutes.
- **0022/SC-004:** With a real `claude` that answers one prompt and then gets no input for 30 minutes, the person gets exactly 1 `waiting for input` notification for that agent (the 0018 hands-on run saw 4 to 8), and with an idle time of 5 minutes the agent shows `asleep` within 6 minutes and 10 seconds of its prompted change, with 0 dismisses and 0 terminal opens.
- **0022/SC-005:** In 10 rounds in which a directive reaches an agent that has an item, 10 of 10 items leave when the agent changes to `working`, and 10 of 10 returns to `waiting for input` each add exactly 1 new item and 1 notification.
- **0022/SC-006:** In 10 sleep checks of an agent that has a waiting item and whose screen cannot be read, the agent is put to sleep 0 times.
- **0022/SC-007:** In 10 rounds in which the person opens an agent's item and the agent then works for 2 minutes on its own, with no input, and returns to `waiting for input`, 10 of 10 returns each add exactly 1 new item and 1 notification, and with an idle time of 5 minutes the agent is put to sleep 0 times before 5 minutes have passed since that return.

### Notes from Deviations

- **No requirement is deferred or changed.** The Deviations section of 0022 is empty.
- **Criteria that tests ran with fewer rounds or shorter times (QA report).** 0022/SC-001, 0022/SC-003, 0022/SC-005, and 0022/SC-007 ran with fewer agents or rounds, a shortened quiet period, a shortened idle time, or a shorter run than the criterion states. 0022/SC-004 passed in a 30-minute run with a real signed-in `claude`: 1 notification, and a sleep 311.4 seconds after the prompted wait, with no dismiss and no terminal view open.
- **A failing stop (0018/FR-009, as modified).** The fake CLIs cannot make a stop fail, so a unit test covers the rule that a failed sleep bars the agent until a prompted wait, and no integration test covers it.
- **Assumption that the text names.** The bare `A10` in the Terms above names this assumption of 0022:
  **A10:** The work bound is the quiet period plus 10 seconds. The redraw output of a hint line lasts well under 1 second, but the status stays `working` until one quiet period passes with no output, so a redraw shows `working` for about one quiet period. A fixed 10 seconds would make every redraw count as work once the person sets a quiet period of 10 seconds or more. 10 more seconds keep redraws inside the bound and still end the wait for real work within seconds. A CLI that prints by itself without a break for longer than the bound while it waits, such as a long animation, gives 1 new item and 1 notification at its return.

## Change 0023: Keep no command lines of ended sessions

### Intent

Change 0017 stores the identity of each session process: its process id, its start time, a hash of its whole command line, and a display text of up to 1,000 characters of that command line. When a session ends normally, the office marks these identities as ended but keeps them. Nothing reads them after that, but a helper command line can hold a secret, for example `curl -H "Authorization: ..."`. This change makes the office remove the stored identities of a session when it ends normally, remove identities that an earlier build left behind, and remove the identities of an interrupted session once the "Resume interrupted" list no longer reads them. It also makes sure that, after the next server start, the database files keep no copy of a command line that left the store: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identities, also when an earlier build freed it. The sweep, its match, and the action record stay as they are.

### Requirements

- **0023/FR-001** (US3): WHEN the office stores a new session of an agent, THE SYSTEM SHALL remove each ended stored process identity of every earlier `lost` session of that agent.
- **0023/FR-002** (US2): WHEN the server starts, THE SYSTEM SHALL, after it ends the sessions that are still open as `lost`, remove each stored process identity of a session that ended normally, and each ended stored process identity of a `lost` session that is not the last session of its agent.
- **0023/FR-003** (US2, US3): THE SYSTEM SHALL keep each open stored process identity of a `lost` session, and each stored process identity of a `lost` session that is the last session of its agent, until FR-001 or FR-002 removes it or its agent is deleted.
- **0023/FR-004** (US1, US2): WHEN the server starts, THE SYSTEM SHALL leave in the office database no copy of a display text freed before that start (Terms: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identity), other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **0023/FR-005** (US2): WHEN the server starts for the first time under this build, THE SYSTEM SHALL leave in the office database no copy of a display text that an earlier build freed, other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **0023/FR-006** (US2): IF the server cannot finish FR-004 or FR-005 at a start, THEN THE SYSTEM SHALL write the reason to the server's log, SHALL start as it would otherwise, and SHALL try again at each later start until it finishes.

This change modified: 0017/FR-008. The final text is under the change that owns that id.

### Preserved behaviors

- **0023/PB-1:** After an unclean stop, the sweep still checks every open stored process identity of every `lost` session, ends each leftover with the same match, and records exactly one sweep result for each identity before the "Resume interrupted" list shows (0017/FR-003 to 0017/FR-007, 0017/FR-009).
- **0023/PB-2:** The "Resume interrupted" list still marks an agent whose last session is `lost` with the same finding: a note with the number of ended leftovers, the warning that a CLI process may still run (could not end, not readable, boot time changed, no complete stored identities, helper limit, not swept), or no mark (0013/FR-008 as modified by 0017).
- **0023/PB-3:** Each action record entry of the sweep keeps its text: the office as actor, the session, the process id, the command line in visible form, and the result (0017/FR-004 to 0017/FR-006).
- **0023/PB-4:** After clean server stops and normal agent stops, the next server start ends 0 processes and starts 0 agents without an action of the person (0017/SC-006, 0001/FR-004).
- **0023/PB-5:** While a session runs, the office still stores the CLI identity before the session shows as started, stores helper identities within 10 seconds, drops a helper identity when the helper ends, replaces the command line after an `exec`, and applies the helper limit of 200 (0017/FR-001, 0017/FR-002, 0017/FR-010, 0017/FR-011).
- **0023/PB-6:** The person can still continue any previous session of an agent from its details, also a `lost` one (0013/PB-2, 0001/FR-025).

### Terms

Terms that this change defines. The other terms are as in 0001, 0013, 0017, and 0019.

| Term | Meaning |
|---|---|
| Session ended normally | A session that ended while the server ran: a stop, an end of its CLI, a start failure, an end at the sign-in screen (0019), or a clean server stop (0017/FR-008). Every session end except `lost`. |
| Remove | Delete the stored process identity with all its values. Blanking only the display text is not a remove (A2). |
| Office database | The owner-only storage of the office (0001/FR-009): the database file and its write log and shared-memory files (`-wal`, `-shm`), with the freed space inside them. Not the disk blocks that the OS freed when one of these files shrank (A4). |
| Freed display text | The display text of a stored process identity after it leaves the store: the identity is removed (FR-001, FR-002, 0017/FR-008), a helper identity is dropped when its helper ends (0017/FR-002), the old command line is replaced after an `exec` (0017/FR-011), or the identity is deleted with its agent. |

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity (changes 0017) | One session process that the office knows. | As in 0017. The lifetime changes: it exists while its session runs; after a `lost` end, while it is open or while its session is the last session of its agent. | Belongs to one session of one agent. Removed with its session end, with a new session of its agent, or at a server start, as FR-001, FR-002, and 0017/FR-008 define. Once its display text is freed (Terms), no copy of it stays in the office database after the next server start (FR-004). |
| Session (extends 0013, 0017) | One run of an agent CLI. | end result: ended normally or `lost` | A session that ended normally has no stored process identity. Only the last session of an agent, when it is `lost`, keeps ended identities. |
| Action record entry of the sweep (0017) | What the sweep did with one stored identity. | unchanged | Keeps its text after the identity it names is removed. |

### Success criteria

- **0023/SC-001:** After 20 sessions end normally (10 stops by the person, 5 ends of the CLI, 5 by clean server stops), each with a CLI and a helper whose command line holds a unique marker that no terminal output shows, the office holds 0 stored process identities of those 20 sessions right after each end.
- **0023/SC-002:** After the next server start that follows those 20 ends, a byte search of the database file and its `-wal` and `-shm` files finds 0 of the 20 markers.
- **0023/SC-003:** With an office database from the 0017 build that holds 10 sessions that ended normally, each with a marker in a stored command line, plus 10 markers in helper identities that the 0017 build dropped and 10 markers in command lines that it replaced after `exec`, the first start of this build leaves 0 identities of those sessions, the byte search of the database file and its `-wal` and `-shm` files finds 0 of the 30 markers, and the sweep ends 0 processes.
- **0023/SC-004:** In the 0017/SC-005 test (4 agents after a forced kill of the server: one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, one session from before 0017), the "Resume interrupted" list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent, also after a second server start without a resume.
- **0023/SC-005:** After the person resumes the 4 agents of SC-004, the office holds 0 ended identities of their `lost` sessions, and 100% of the sweep entries of those sessions in the action record show the same text as before the resume.
- **0023/SC-006:** In the 0017/SC-001 test (30 stand-in CLIs, each with one helper, outlive a forced kill of the server), 100% of the 60 processes still end before the "Resume interrupted" list shows.
- **0023/SC-007:** In 10 sessions, each with a helper whose command line holds a unique marker (as in SC-001) and that ends 15 seconds after it starts while its session still runs, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 markers.
- **0023/SC-008:** In 10 sessions whose CLI replaces itself with another program after the office stored it, with a unique marker only in the old command line, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 old markers.

### Notes from Deviations

- **No requirement is deferred or changed.** The Deviations section of 0023 is empty.
- **One checkpoint for two steps (plan D4 and D5).** The checkpoint after the start deletes and the checkpoint after the rebuild share one `wal_checkpoint(TRUNCATE)`, which runs after the `VACUUM` when the rebuild is pending. Two checkpoints would log a busy result twice on the first start. One checkpoint after the rebuild still leaves the write log empty.
- **Linux only for the sweep test (0023/SC-004, 0023/SC-005).** These criteria need the sweep of 0017, which runs on Linux only, so their test runs on Linux only. The other tests of the change run on Linux and macOS in CI.
- **The VACUUM error path is only logged.** No test makes the `VACUUM` fail. The busy checkpoint is tested (0023/FR-006).
- **Assumptions that the text names.** The bare `A2` and `A4` in the text above name these assumptions of 0023:
  - **A2:** The office removes the whole stored identity (option a), and does not only blank the display text and keep the hash (option b). Reasons: an ended identity of a session that ended normally has no reader at all, so the hash, process id, and start time have no use either; a hash of a short secret can be found by trying values; a remove also stops the store from growing with each session. Nothing reads these identities, so the remove changes no visible behavior.
  - **A4:** Between the time a display text is freed and the next server start, a copy of it can stay in the office database, readable only by the owner (0001/FR-009). FR-004 bounds this to the next server start (Clarifications). The byte search of FR-004, FR-005, and the criteria covers the database file and its `-wal` and `-shm` files, with the freed space inside them. It does not cover disk blocks that the OS freed when one of these files shrank or was truncated, nor swap or file system journals. The office cannot reach them, as A8 says for backup copies, so a test does not scan the disk.

## Change 0026: Secrets that reach running agents

### Intent

Today the person gives an agent an API key as an environment variable in the agent's office additions (0001/FR-022). The key sits in the office's storage as plain text, it is applied only at the next session start (0001/FR-023), and a key that changes means a restart of every agent that uses it. This change adds a secret store. The person saves a secret once in the desktop app. The office keeps it sealed at rest, with a key that the OS keyring or an owner-only file protects. A running agent reads the secrets that the person granted to it, at the moment it needs them, through an office command it can call. Replacing a secret reaches every running agent at its next read, with no restart. This change also fixes what may never hold a secret value: logs, the action record, notifications, exports, crash files, and stored terminal output.

### Requirements

- **0026/FR-001** (US1): THE SYSTEM SHALL let the person save a secret with a name and a value in the desktop app, and SHALL show it in a list with its name, its version, the time of its last change, and the agents that hold a grant.
- **0026/FR-002** (US1, US5): THE SYSTEM SHALL keep every secret value sealed, and SHALL NOT write a value as readable text to the office database file, its write log, its shared-memory file, or any other file the office creates. THE SYSTEM SHALL keep the store key outside the office database file.
- **0026/FR-003** (US1, US4): THE SYSTEM SHALL keep the store key in the OS keyring where the OS offers one for the owner (macOS Keychain, Windows Credential Manager, Linux Secret Service) and otherwise in an owner-only key file, SHALL decide this once, when the store key is created, SHALL show the key custody as text in the Secrets dialog, and SHALL NOT move or recreate the store key without the person's reset (FR-015).
- **0026/FR-004** (US1, US5): THE SYSTEM SHALL NOT return a stored value to a user client in any answer, event, export, or screen. The person can replace a value and delete a secret and cannot read a value back.
- **0026/FR-005** (US1, US2): THE SYSTEM SHALL let the person grant a secret to an agent and revoke the grant, SHALL give a grant to no agent until the person does, SHALL NOT pass a grant to an agent's parent, children, or later agents, and SHALL let an agent read only the secrets it holds a grant for.
- **0026/FR-006** (US2): WHEN a running agent asks the office for its secrets, THE SYSTEM SHALL answer the names and versions of the secrets that agent may read. WHEN it reads a granted secret by name, THE SYSTEM SHALL answer the current value and version, in the same session, without a restart of the agent.
- **0026/FR-007** (US2, US4): IF an agent reads a secret that does not exist, was deleted, or was never granted to it, THEN THE SYSTEM SHALL answer `not_found` with one text that is the same in all three cases, so that an agent learns nothing about a secret it may not read. A user client cannot call the read request, and an agent cannot save, replace, delete, grant, or revoke; both are refused as any request outside the caller's allowlist (0002).
- **0026/FR-008** (US3): WHEN the person saves a value under the name of an existing secret, THE SYSTEM SHALL replace the value, raise the version by 1, keep the grants, and answer every later read with the new value. THE SYSTEM SHALL show for each grant the version that the agent read last and the time, or `not read yet`.
- **0026/FR-009** (US4): WHEN the person deletes a secret, THE SYSTEM SHALL remove its sealed value and all its grants. WHEN an agent is deleted, THE SYSTEM SHALL remove its grants.
- **0026/FR-010** (US2, US5): THE SYSTEM SHALL record every read of an agent, accepted or refused, in the action record with the agent as actor, the secret name, and the version when there was one, and SHALL record every save, replace, delete, grant, revoke, and reset of the person with the person as actor and the name, and SHALL NOT put a value in any of these entries.
- **0026/FR-011** (US2): THE SYSTEM SHALL refuse a read while the office or the agent is on hold (0007/FR-004), and SHALL limit the reads of one agent to the number in the setting `limit_secret_reads_per_agent` per rolling 60 seconds (1 to 1000, default 20), as 0007 limits the other actions of an agent. A refusal names the hold or the limit.
- **0026/FR-012** (US5): BEFORE the office stores or sends agent-supplied text, THE SYSTEM SHALL replace each exact occurrence of any stored secret value, also an occurrence that is split across two chunks of terminal output, by the placeholder `[secret NAME]`. This covers every place below, and no place is left to a caller's judgment:
  - (a) stored recent output of a session and the replay shown when the person attaches;
  - (b) action record texts, including the entries of the 0017 sweep and the entries of directives, reports, refusals, and reads;
  - (c) the stored display text of every process identity and every helper identity (0017), where the hash of the whole command line is computed from the original command line before the replacement, so that the sweep still matches the process;
  - (d) notification texts, Needs you items, and reports (0015), including the report line typed into a parent's terminal;
  - (e) the stored task text of an agent (0011);
  - (f) a directive (0012): its entry in a target's queue, the text typed into the target's terminal, and its record entries;
  - (g) the name, objective, and startup prompt that an agent gives to a new agent or a temp (0002, 0020), as stored and as typed, and the finished temp records (0020);
  - (h) events and answers that the office sends to a client and that carry text from (a) to (g).

  It does not cover the live stream to an attached terminal (A3), text that the person typed in the desktop app, or text that the office stored before the secret was saved (A13).
- **0026/FR-013** (US5): THE SYSTEM SHALL NOT put a value, or any part of it, in the server log, in an error text, in a panic message, in a refusal reason, or in a debug print, and SHALL ask the OS not to write a memory dump of the server process where the OS lets a process do so (A5).
- **0026/FR-014** (US5): THE SYSTEM SHALL NOT include a secret name, value, or grant in a template, in an export of templates (0009), or in any other file the person can export.
- **0026/FR-015** (US4): WHEN the store is locked or its store key is lost, THE SYSTEM SHALL let the person reset the store after a confirmation that names how many secrets it will remove. A reset removes every secret, grant, and the store key, and the next save creates a new store key under FR-003.
- **0026/FR-016** (US1): THE SYSTEM SHALL accept a secret name of 1 to 64 characters that starts with a letter A to Z and holds only the letters A to Z, digits, and underscore; a value of 8 to 8192 bytes of UTF-8 text without a NUL character; and at most 100 secrets. It SHALL refuse anything else with the reason named.
- **0026/FR-017** (US1, US4): THE SYSTEM SHALL make every action of the Secrets dialog available from the keyboard (0001/FR-028), SHALL show the state of every grant and every version as text and not only as color, and SHALL confirm a delete and a reset before it acts.
- **0026/FR-018** (US1, US5): THE SYSTEM SHALL show the value field of the dialog as masked input, SHALL empty it when the person saves, cancels, or closes the dialog, and SHALL NOT keep a typed value in the app's saved state, its URL, its logs, or an error text.
- **0026/FR-019** (US2, US4): WHILE the store is locked, THE SYSTEM SHALL refuse every read and every save with the reason `the secret store is locked: <custody>`, SHALL try the store key again at the next request without a server restart, and SHALL NOT overwrite or drop a sealed record.

This change modified: 0002/FR-008, 0007/FR-007, 0007/FR-009. The final text is under the change that owns each id. The table of rate limits under Assumption A3 of 0007 gained one row, which is in Rate limits above.

### Preserved behaviors

- **0026/PB-1:** The environment variables in office additions keep their behavior: stored as before, applied at the next session start (0001/FR-022, 0001/FR-023), blanked in an export (0009 A6).
- **0026/PB-2:** The agent connection keeps its proof and its allowlist. This change adds `secret.available` and `secret.get` to it and nothing else (0002).
- **0026/PB-3:** The office token never appears in an agent's environment, and a user client still proves the office token (0001/FR-007, 0002).
- **0026/PB-4:** A session's recent output is still stored and replayed in order, capped by lines and by bytes (0001/FR-003, 0001/FR-016); only a secret value in it changes to the placeholder (FR-012).
- **0026/PB-5:** The server log keeps its rule: ids, status, and error kinds, and now never a secret value (FR-013).
- **0026/PB-6:** The action record keeps its entry fields and its display in the agent's details (0001/FR-011); this change adds two kinds (the contract lists them).
- **0026/PB-7:** A hold and every rate limit of 0007 keep their text and order for the actions they cover; reads join them as one more agent-made action (FR-011).
- **0026/PB-8:** The 0017 sweep still matches a process by the hash of its whole original command line, and ends it as before, also when the stored display text holds a placeholder (0017/FR-003 to 0017/FR-007).

### Terms

Terms that this change defines. The other terms are as in 0001, 0002, 0007, 0009, 0011, 0012, 0015, 0017, and 0020.

| Term | Meaning |
|---|---|
| Secret | One named value that the person saved in the office: a name, a value, a version, and a list of grants. |
| Value | The text of a secret. Only the person's save request and a granted agent's read carry it. |
| Sealed | Stored encrypted and authenticated, so that the stored bytes show nothing about the value, and a stored record cannot be moved to another secret without detection. |
| Store key | The key that seals every secret. It is never kept in the office database file. |
| Key custody | Where the store key lives: `os_keyring` (macOS Keychain, Windows Credential Manager, Linux Secret Service) or `key_file` (an owner-only file in the office's private data folder). Shown to the person as text. |
| Locked | The state in which the office has sealed secrets and cannot get the store key at this moment, for example a keyring that is locked or not running. |
| Grant | The person's permission for one agent to read one secret. It belongs to the pair of that secret and that agent. |
| Version | A whole number that starts at 1 for a new secret and grows by 1 each time the person replaces the value. |
| Read | One accepted `secret.get` of an agent: it returns the value to that agent. |
| Placeholder | The text `[secret NAME]` that stands for a value in anything the office stores or sends (FR-012). |
| Agent-supplied text | Text that an agent or its processes produced or sent, and that the office stores or sends on: terminal output, a command line of a process (0017), a directive (0012), a report (0015), a task text (0011), a name or objective that an agent gives to a new agent or a temp (0002, 0020), and the texts the office builds from them. Text that the person typed in the desktop app is not agent-supplied text. |
| Agent, action record, hold, limit, office token | As defined in 0001, 0002, and 0007. |

### Threat model

Assets: the secret values, and the store key. Trust boundary: the office process and its owner-only folder, with the OS keyring, on the owner's machine (0001/FR-005, 0001/FR-008, 0001/FR-009).

| # | Actor and attack | Answer in this change | Left over, stated plainly |
|---|---|---|---|
| T1 | Another OS user reads the office's files or the keyring. | Owner-only files (0001/FR-009); the keyring item belongs to the owner's login (FR-002, FR-003). | None above 0001/FR-009. |
| T2 | A copy of the office database leaves the machine alone: a backup, a sync folder, a support attachment. | Sealed values; the store key is not in the database (FR-002). With `os_keyring` the copy is useless without the keyring. With `key_file` the copy is useless without the key file, which sits in another file of the same folder. | With `key_file` a copy of the whole data folder opens the secrets. The dialog shows the custody, so the person knows. |
| T3 | A program that runs as the owner reads the keyring item or the key file, or the office token file. | Not stopped. Same boundary as 0001: the office token file is also owner-readable. | A hostile program with the owner's rights can read every secret. This change does not claim otherwise (A2). |
| T4 | An agent that is prompt-injected or hostile reads a secret it should not, or all of them. | Per-agent grants, no inheritance (FR-005); the same `not_found` for denied and missing names (FR-007); holds and a rate limit (FR-011); every read recorded (FR-010). | An agent that holds a grant can use or leak that one value (A1). Revoke the grant and rotate the value. |
| T5 | An agent changes or grants secrets, or reads another agent's secrets. | The agent allowlist has no write request; a user request is not offered (FR-007). | None. |
| T6 | A value reaches a log, an action record entry, a notification, a Needs you item, an event, or an export. | Placeholders at every storing point (FR-012); no value in logs or errors (FR-013); exports hold no secrets (FR-014); events and entries carry names only (FR-010). | None for an exact value. A changed form of a value (base64, split by an agent over several lines) is not recognized (A3). The replacement is a weak guessing oracle (A14). |
| T7 | A value reaches the stored terminal output, because the agent printed it. | Placeholder in stored and replayed output (FR-012). | The live stream to the attached terminal shows what the agent printed (A3). The CLI's own transcript, which the CLI keeps in its own folders, can hold it (Out of scope). |
| T8 | A crash or a panic writes memory or a message to disk. | No value in panic text or errors; the server asks the OS for no memory dump (FR-013). | An OS that writes a dump anyway (A5). |
| T9 | A sealed record is copied over another, or edited. | A sealed record is bound to its secret and its version, so a moved, old, or edited record fails to open and the read is refused with an internal error that holds no value (contract). | None. |
| T10 | The keyring is missing, locked, or replaced. | A locked state, no silent new key (FR-003, FR-019); a reset by the person only (FR-015). | The secrets are unreadable until the keyring returns or the person resets. |
| T11 | A leaked or stale client keeps using a grant. | A grant is checked at every request, so a revoke applies at the next read (FR-005). | A value that the agent already read stays with it (A1). |
| T12 | The desktop app keeps the typed value. | Masked input, cleared at save, cancel, and close (FR-018); the app's saved state and logs never hold it. | A screenshot tool or an input logger on the machine (T3). |

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Secret | One named value the person saved. | name, version, sealed value, created, last changed | Name is unique in the office. Has zero or more grants. The value never leaves the office except in a read answer. |
| Grant | One agent's permission to read one secret. | last read version, last read time | One per pair of secret and agent. Deleted with its secret or its agent. |
| Store key | The key that seals every secret. | custody (`os_keyring` or `key_file`) | One per office. Created at the first save. Replaced only by a reset. |
| Action record entry (extends 0001) | A read or a change. | two new kinds: `secret_read`, `secret_change` | Names a secret and a version, never a value. |

### Success criteria

- **0026/SC-001:** After 20 secrets are saved, each with a unique marker, and the server stops, a byte search of the database file, its `-wal` and `-shm` files, and every file under the office's data folder (the key file excluded, which holds only the store key) finds 0 of the 20 markers.
- **0026/SC-002:** With 5 agents that read 3 secrets each, print every value to their terminals, send one in a report and one in a directive to a child, give one to a new agent as its task text and one to a temp as its objective, run one helper process whose command line carries a value for more than 10 seconds and then kill the server and start it again, and cause one failed read, a byte search of the same files, of the action record, of every notification and Needs you item, and of every message that a recording client received finds 0 of the markers in any of them, including the stored process identities, the sweep entries, the directive queue, the stored task texts, and the stored temp objectives. The stored output and the records show `[secret NAME]` at 100% of the occurrences that an agent printed whole, and the sweep ends the helper process that carried a value in its command line (0017/SC-005 test).
- **0026/SC-003:** With 30 running agents that each hold a grant and read the secret once, replacing the value makes 100% of the 30 next reads return the new value and version 2, with 0 sessions restarted and 0 environment changes.
- **0026/SC-004:** In 100 reads by agents that hold no grant, that hold a grant for another name, or that name a deleted secret, 100% are answered `not_found` with the same text and none is answered with a value; a read by a user client is not offered.
- **0026/SC-005:** After a delete, a read of the deleted name is `not_found` for 100% of the agents that held a grant, the office holds 0 sealed records and 0 grants of it, and the database file and its `-wal` and `-shm` files hold 0 copies of its sealed bytes after the next server start.
- **0026/SC-006:** With the keyring unavailable at the first save on Linux, the store key is in the key file, the dialog says `key file`, the key file is readable only by the owner (mode 0600 on Linux and macOS, an owner-only access list on Windows), and a later start with the keyring available does not change the custody.
- **0026/SC-007:** With the keyring locked after secrets were saved, every read and every save is refused with the locked reason within 1 second, the sealed records stay unchanged byte for byte, and the first request after the keyring unlocks succeeds without a server restart.
- **0026/SC-008:** In a template export of an office that has 5 secrets with grants, a byte search finds 0 secret names, 0 values, and 0 grants.
- **0026/SC-009:** A person who has the dialog open saves a secret, grants it to an agent, replaces the value, revokes the grant, and deletes the secret, using the keyboard only, in under 2 minutes, and the value field is empty after each save.
- **0026/SC-010:** An agent that reads more than the limit within 60 seconds gets a refusal that names `limit_secret_reads_per_agent` on read 21 (default), and a read while the agent is on hold is refused with the hold reason; both are recorded with the agent as actor.

### Assumptions

The bare `A1` to `A14` in the text above and in the threat model name these assumptions of 0026.

- **A1:** A value that an agent reads is in that agent's hands. The office cannot take it back, cannot see where the agent puts it, and cannot stop the agent from writing it to a file or sending it to a tool. Revoke and rotate is the answer (FR-005, FR-008). Granting is the person's decision, per agent.
- **A2:** The office defends against copies of its files, other OS users, and agents. It does not defend against a hostile program that runs as the owner, which can already read the office token and the keyring (T3).
- **A3:** Replacement of values (FR-012) is exact-match and best effort: it finds each occurrence of the value as it is, also across chunk borders, and not an encoded or re-split form. It runs at the points where the office stores or sends text. The live stream to an attached terminal is the person's own screen, and changing it would break escape sequences, so it is not changed. The end of a chunk that could be the start of a value is held back for at most 100 ms of silence and then goes to the screen and the stored output as it is, so a screen and an input-line check never wait for more output; a value split across a longer pause is not replaced.
- **A4:** FR-012 replaces text by value. A value of 8 bytes or more is the minimum (FR-016), because a one- or two-character value would replace unrelated text across all stored output, which the person cannot undo. A value that is a common word of 8 or more characters can still match unrelated text; the person chooses the value.
- **A5:** Asking the OS for no memory dump means: core dump size 0 on Linux and macOS for the server process, and on Linux also marking the process not dumpable, which stops a same-user debugger or a read of its memory through `/proc`. Windows has no per-process switch that the office can rely on; the plan records what it can do there, and a dump written by the OS anyway is outside the office's reach.
- **A6:** Key custody per OS: macOS uses the Keychain; Windows uses the Credential Manager, whose items are protected for the logged-in user; Linux uses the Secret Service on the session D-Bus when it answers (GNOME Keyring, KWallet), and falls back to the key file when it does not, for example on a headless machine. The key file sits in the office's private data folder with the same owner-only rule as the office token (0001/FR-009).
- **A7:** The value reaches the server over the office connection, which accepts only the same machine and a valid token (0001/FR-007, 0001/FR-008). It is not encrypted again on that hop.
- **A8:** The value reaches an agent only in the answer of `secret.get`. Nothing is added to the agent's environment or to its startup prompt.
- **A9:** The Secrets dialog is the person's "Settings" for this change. A full settings screen is a separate change.
- **A10:** The person who reads the dialog is the owner. The office has one owner, so no per-person rights exist (0001).
- **A12:** The office knows the values only while it holds them open. After a server start with a locked store it knows none, so it cannot replace a value in text that an agent produced during that time (FR-012). No agent can read a secret while the store is locked (FR-019), so such an agent holds a value only from an earlier read. Revoke and rotate answers it (A1).
- **A13:** The office does not rewrite text that it stored before a secret was saved, or text of a value that was replaced by a newer one: the replacement works on what the office stores from then on. A person who saves a value that an agent already printed should rotate it (A1).
- **A14:** The replacement is a weak oracle: an agent that holds no grant can send a guess in a directive to its own child and see whether it arrives as `[secret NAME]`. For a high-entropy key this tells it nothing in practice, and the minimum length (FR-016) keeps short values out.
- **A11:** The sealing method, the crates, the table layout, and how the placeholder search runs on a stream are for the plan.

### Out of scope

- A secret mark on environment variables, and moving existing environment values into the store.
- A secret that an agent saves, grants, or shares with another agent.
- Grants through templates, teams, or inheritance (0009, 0002).
- Placing a secret in an agent's environment or startup prompt at session start.
- Changing the transcripts that the CLIs keep in their own folders (0001/FR-021).
- Rotating the store key itself while secrets exist (a reset is the way, FR-015).
- Moving the store key between the keyring and the key file.
- Syncing secrets to another machine (0006), and import or export of secrets.
- A settings screen for anything else, and a master password.
- Replacing values in the live terminal stream, or recognizing encoded forms (A3).
- Defending against a hostile program that runs as the owner (A2).

### Notes from Deviations

- **No requirement is deferred or changed.** The Deviations section of 0026 is empty.
- **Held tail (0026/A3).** The redaction releases the end of a chunk that could begin a value after 100 ms of silence, so the screen and the 0012 input-line check never wait for more output. A value split across a longer pause is not replaced. Spec review r3 of PR #141 asked for this.
- **One keyring account for each office.** The account is `secret-store-key-` and a random id that the office keeps in `secret-office-id` in its data folder, so two offices of one OS user never share a key. A keyring that already holds a key for the account is never written over: the key goes to the key file. This was added after spec review r3 and is not a change of a requirement text.
- **Gaps stated as PARTIAL or NOT RUN in the QA report.** 0026/SC-006 is PARTIAL (the owner-only access list on Windows and the keyring on macOS and Windows were not run). 0026/SC-007 is PARTIAL (the 1-second bound was not timed, and the lock is a fake keyring). 0026/FR-013 is PARTIAL (no Windows memory dump switch, A5). A hands-on run with a real signed-in CLI was NOT RUN.
- **Edge cases, user stories, and the user experience** are in the archived spec.

## Change 0025: Scheduled directives

### Intent

The person gives some directives again and again at the same time: a nightly "run the full test suite and report", a Monday "write the status of the week", a check every four hours. Today the person must be at the desk to type each one, and a job that should start while the person sleeps does not start. This change adds the schedule: the person sets, in the desktop app, a directive text for a chosen agent and when it goes: once, daily at a time, weekly on chosen days at a time, or every N hours. A schedule is armed the moment the person saves it and switches it on, with no restart. Each run obeys the holds and rate limits of 0007, each directive goes through the safe delivery queue of 0012, and the action record names the schedule as the actor. A run that the office could not make at its time is recorded as missed and is not made late, so the office never surprises the person with old work.

### Requirements

- **0025/FR-001** (US1, US4): WHEN the person saves a schedule from the desktop app, THE SYSTEM SHALL keep it only IF its name, after the spaces at its ends are removed and the text is put in Unicode normalization form C, is not blank, has at most 100 characters, has no control characters, and is used by no other schedule of the office, compared without case; a weekday that the repeat names twice counts once; its target is an existing agent; its text follows the text rules of a directive; and its repeat is one of the Terms, a weekly repeat has at least one day, an every-N-hours repeat has a whole N from 1 to 168, and a once repeat of a schedule that is saved on names a future local time; IF any field breaks a rule, THEN THE SYSTEM SHALL keep nothing, name each failing field and its rule, and leave a schedule that the person edited as it was.
- **0025/FR-002** (US4): IF a save of a new schedule would make the number of schedules in the office greater than the schedule cap, THEN THE SYSTEM SHALL keep nothing and refuse the save with a reason that names the cap and its value; THE SYSTEM SHALL let the person set the schedule cap in the office settings as a whole number from 1 to 200, 50 by default, SHALL refuse any other value with that rule and keep the previous value, and SHALL apply a changed value to the next save without deleting or switching off any schedule.
- **0025/FR-003** (US1, US4): WHEN the person saves a schedule that is on, edits one, or switches one on, THE SYSTEM SHALL compute its next due time at once, without a restart, as the first due time that is later than both that moment and the schedule's last handled due time, SHALL show it in the schedules view in local time with the date and the day of the week, and SHALL clear the off reason at a save; and THE SYSTEM SHALL keep every schedule, its switch, its next and last handled due times, its off reason, and its history across server and machine restarts, readable only by the OS user who owns the office (0001/FR-009).
- **0025/FR-004** (US2): THE SYSTEM SHALL compute the due times of a once, daily, or weekly repeat in local time, in the time zone that the machine has at that moment, and recompute the next due time within 60 seconds after the machine's time zone changes; WHERE a set time does not exist on a day because daylight saving time starts, THE SYSTEM SHALL use the first minute after the skipped hour; WHERE a set time occurs twice on a day because daylight saving time ends, THE SYSTEM SHALL use only its first occurrence; and THE SYSTEM SHALL compute the due times of an every-N-hours repeat as the anchor plus N, 2N, 3N, and each further whole multiple of N hours of elapsed time, which no change of time zone or daylight saving time moves.
- **0025/FR-005** (US1, US2): WHILE a schedule is on, THE SYSTEM SHALL start a run within 60 seconds after each due time, and WHILE the sweep runs (0017/FR-007), SHALL keep a due run waiting until the sweep ends; IF the background server did not run at a due time, or the office can start the run only more than 2 minutes (the grace) after its due time, THEN THE SYSTEM SHALL NOT make that run and SHALL handle the due time as missed under FR-006; THE SYSTEM SHALL handle each due time of a schedule at most once, also when the clock is set back; and WHEN the system handles the due time of a once schedule, THE SYSTEM SHALL switch that schedule off.
- **0025/FR-006** (US2): WHEN the system handles due times as missed, THE SYSTEM SHALL add, for each schedule, one missed entry for all its due times that it finds missed at the same moment, with their count and the first and last of them in local time, to the schedule's history and to the action record; SHALL show the last missed time in the schedules view; SHALL set the next due time to the first future one; SHALL switch a once schedule off with the off reason that its time was missed; and SHALL notify the person with one notification for all the missed entries that it adds in the same minute, which says how many schedules missed runs.
- **0025/FR-007** (US3): THE SYSTEM SHALL treat each run as an agent-made action for the holds of 0007 and SHALL check the holds when the run starts and again when its directive enters the target's queue: WHILE the office is on hold, or an agent hold covers the target, THE SYSTEM SHALL refuse the run with the hold as the reason; IF a scheduled directive would make the target's schedule window exceed the value of the rate limit "directives sent to one target agent" (0007 A3), THEN THE SYSTEM SHALL refuse the run with a reason that names the limit and its value; THE SYSTEM SHALL count a scheduled directive only in the target's schedule window and never in the count of directives that agents send to that target, so that no agent's directive is refused because of a schedule; and THE SYSTEM SHALL NOT make a refused run later and SHALL NOT count a refused run toward the automatic hold of any agent (0007/FR-008).
- **0025/FR-008** (US3): IF a schedule's latest run has no result yet (for example while its target wakes), or its directive still waits in a queue, when a new run of that schedule starts, THEN THE SYSTEM SHALL skip the new run with the reason "previous run still waiting".
- **0025/FR-009** (US1, US3): WHEN a run passes FR-007 and FR-008 and its target is running, THE SYSTEM SHALL put the text into the target's queue as a scheduled directive whose sender is the schedule itself, never the target and never an agent, typed under 0012/FR-001 to 0012/FR-005 as modified by this change; SHALL record the run result as 0012 answers it (`submitted`, `waiting` with the place in the queue, or `unsubmitted`), or as `refused` with the reason when the queue is full (0012/FR-004); and SHALL change a `waiting` result to `submitted`, `unsubmitted`, or `withdrawn` with the reason when 0012 settles that directive.
- **0025/FR-010** (US3): WHEN a run passes FR-007 and FR-008 and its target is asleep, THE SYSTEM SHALL wake the target as 0018/FR-007 wakes it for the person, with the sign-in check of 0019/FR-001, the fallback of 0013/FR-004, and the hold of 0013/FR-005, and with the schedule as the actor of the wake entry, and SHALL then go on under FR-009 WHEN the wake result is `continued` or `new session`; WHERE the target is going to sleep, THE SYSTEM SHALL wait until it is asleep and then wake it; IF the wake ends `on hold`, or the session does not start, THEN THE SYSTEM SHALL send no directive and record the run as `failed` with the wake's reason; IF the target is `stopped`, `exited`, `failed`, or `interrupted`, THEN THE SYSTEM SHALL start nothing and record the run as `skipped` with the target's status as the reason; and WHEN the target of a schedule is deleted by the person, or removed because it was a temp that ended (0020/FR-006), THE SYSTEM SHALL switch that schedule off at once and show the off reason.
- **0025/FR-011** (US3): WHEN a run ends `skipped` because its target does not run, `failed`, `refused` for a reason other than a hold, `unsubmitted`, or `withdrawn` for a reason other than a stop of the background server, or WHEN the system switches a schedule off under FR-010, THE SYSTEM SHALL notify the person with the schedule's name, its target, the run result, and the reason, and SHALL add no further such notification for that schedule until a run of it succeeds; THE SYSTEM SHALL add no Needs you item for a schedule notification; and WHEN the person opens a schedule notification, THE SYSTEM SHALL show that schedule in the schedules view, or the schedules view itself for a notification of missed runs (FR-006), and SHALL open no terminal.
- **0025/FR-012** (US1, US3, US4): THE SYSTEM SHALL add each run result and each schedule-made action, refusal, unsubmitted directive, and withdrawal to the action record once, in the target's details, with the schedule as the actor, named by its name and its id with the note that the person made it; SHALL add each save, edit, switch, Run now, and delete of a schedule with the person as the actor, in the details of its target while the target exists; and SHALL keep the last 20 run results of each schedule in its history, newest first, each with its due time, the time the run started, whether it was due or Run now, the result, and the reason.
- **0025/FR-013** (US1, US4): THE SYSTEM SHALL show the person, in one schedules view of the desktop app, every schedule with its name, its target, its repeat in words, its switch as text, its next due time, its last run result with its time, its last missed time, and its off reason; SHALL let the person, with the keyboard alone, make, edit, switch off and on, run now, open the history of, and delete each schedule, a delete only after one confirmation that says that the history goes, that the action record stays, and that a waiting directive stays; SHALL apply an edit and a switch at once under FR-003, where a switch on makes no run for a time the schedule was off and is refused for a once schedule whose time has passed or a schedule with an off reason until the person edits it; and SHALL make a Run now a run under FR-007 to FR-012 that changes neither the next due time nor the switch.
- **0025/FR-014** (US4): THE SYSTEM SHALL let only the person, through the desktop app, make, edit, switch, run, or delete a schedule or change the schedule cap; and IF an agent asks the office for any of these actions, THEN THE SYSTEM SHALL refuse the request, tell the agent the reason, and record the refusal as 0002/FR-009 requires.

This change modified: 0003 Key entities (Notification), 0007/FR-010, 0012/FR-003, 0012/FR-005, 0018/FR-004. The final text is under the change that owns each id.

### Preserved behaviors

- **0025/PB-1:** The office starts no agent at a server start, and no agent without an explicit action (0001/FR-004, 0013/PB-1, 0018/PB-1). An `interrupted` agent stays `interrupted` until the person acts; a run skips it. The only start that a schedule makes is the wake of its own asleep target, at a due time while the server runs or at the person's Run now.
- **0025/PB-2:** The person's own direct actions from the desktop app (create, start, stop, delete, new task, wake, and typing into a terminal) are never held or rate-limited (0007/PB-1, 0018/FR-006). Holds and the per-target limit apply to runs, Run now included.
- **0025/PB-3:** A scheduled directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals other than the subtree one. Person input reaches the terminal at once and unchanged (0012/PB-5).
- **0025/PB-4:** The rate limits of agents count as before (0007 A3): a scheduled directive adds nothing to any count of directives that agents send, and an agent is refused or put on automatic hold only for its own actions (0007/FR-007, 0007/FR-008).
- **0025/PB-5:** The agent's connection to the office gets no new request, and the 0002 tree rules stay as they are: directives from agents go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009).
- **0025/PB-6:** Idle sleep stays as before: a scheduled directive that waits in a queue keeps its target awake as any waiting directive does (0018/FR-002 d), and a directive or report from an agent to an asleep agent is still refused (0018/FR-005).
- **0025/PB-7:** Every desktop app action is available from the keyboard, the actions of this change included, and every status shows as text (0001/FR-014, 0001/FR-028).
- **0025/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Terms

Terms that this change defines. The other terms are as in 0001, 0002, 0003, 0007, 0008, 0012, 0013, 0017, 0018, and 0020.

| Term | Meaning |
|---|---|
| Agent, status, session, stop, action record, desktop app, background server | As defined in 0001. Running means the status `starting`, `working`, or `waiting for input`. |
| Directive, text rules of a directive, agent's connection | As defined in 0002. The text rules: not blank, at most 8,192 characters, and no control characters (0020 A3). |
| Notification | As defined in 0003, as modified by this change: it can refer to a schedule. |
| Agent-made action, office hold, agent hold, rate limit, automatic hold, office settings | As defined in 0007. A hold covers an agent under the rules of 0007/FR-004. |
| Needs you, item | As defined in 0008. |
| Queue | As defined in 0012: the entries that wait to be typed into one agent's terminal. |
| Check period | As defined in 0013: the 20 seconds after a start in which the office watches whether the session ends by itself. |
| Sweep | As defined in 0017. While it runs, starts wait (0017/FR-007). |
| Asleep, going to sleep, wake, wake result | As defined in 0018. |
| Temp | As defined in 0020. A temp can be the target of a schedule. |
| Schedule | A rule that the person makes in the desktop app. It has a name, a target, a directive text, a repeat, and a switch that is on or off. |
| Scheduled directive | The directive that a run of a schedule puts into its target's queue. Its sender is the schedule itself, never the target and never an agent. |
| Target | The agent that a schedule sends its directive to. The schedule follows the agent by its id, not by its name or its place in the tree. |
| Repeat | When a schedule runs: once (a date and a time), daily (a time), weekly (one or more days of the week and a time), or every N hours (N is a whole number from 1 to 168). Times are in hours and minutes. |
| Local time | The wall-clock time of the machine's current time zone, with its daylight saving rules. |
| Due time | A moment at which the repeat says that the schedule runs. |
| Anchor | For an every-N-hours repeat: the moment at which the person last saved a change of the repeat, or switched the schedule on. An edit of other fields does not move it. |
| Run | One attempt of a schedule to send its directive, at a due time or when the person presses Run now. |
| Run result | What one run did: `submitted`, `waiting` (with the place in the queue), `unsubmitted`, `withdrawn`, `refused`, `skipped`, `failed`, or `missed`. Each result other than `submitted` and `waiting` has a reason. |
| Successful run | A run whose directive was submitted. |
| Handled due time | A due time for which the office recorded a run result, `missed` included. |
| Grace | 2 minutes: the longest time after a due time at which the office still starts the run. |
| Missed entry | One entry in a schedule's history for all the due times of that schedule that the office found missed at the same moment: their count, the first one, and the last one. |
| Schedule-made action | An action that a run takes: a wake or a directive. |
| Schedule window | For one target: the scheduled directives that went to it in the last 60 seconds. It is counted apart from the directives of agents (FR-007). |
| Schedule cap | An office setting: the highest number of schedules, on and off, that the office keeps. |
| Schedule history | The last 20 run results of one schedule, newest first. |
| Off reason | Why the office switched a schedule off: its target was deleted or removed, or its one-time due time was missed. |
| Schedules view | The desktop app view that lists every schedule (FR-013). |

### Data flow and trust boundary

- **Boundaries:** the person's desktop app connection, which presents the office token (0001/FR-007), and the agent's connection to the office (0002). Only the first can make, change, or run a schedule.
- **Into the office:** from the person, each schedule's fields, a switch on or off, an edit, a delete, a Run now, and the schedule cap. The office checks every field when the person saves (FR-001).
- **Out of the office, to a target's terminal:** the directive text, typed under the rules of 0012 as a directive's text is (FR-009).
- **Out of the office, to the person:** the schedules view, the schedule history, and notifications (FR-006, FR-011).
- **Into the action record:** each run result, each schedule-made action, each refusal, and each save, switch, edit, delete, and Run now of the person (FR-012).
- **Kept by the background server:** every schedule and its history, readable only by the OS user who owns the office (FR-003).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent makes or changes a schedule, so that it acts later outside the tree rules or outside a hold. | The agent's connection has no schedule request; any such request is refused and recorded (FR-014, PB-5). |
  | A schedule floods a terminal. | The shortest repeat is 1 hour (FR-001); one schedule never has two runs at once (FR-008); the per-target limit in the schedule window and the 0012 queue limit apply (FR-007, FR-009); the schedule cap bounds the count (FR-002). |
  | Schedules use up an agent's rate limit, so a parent's directive is refused or the agent is put on automatic hold. | Scheduled directives count only in the schedule window, never in the window of agents' directives, and a refused run counts toward no automatic hold (FR-007, PB-4). |
  | A schedule acts while the person has held the office or an agent. | A hold refuses each run that it covers, and the office does not make the run later (FR-007). |
  | A directive text carries control characters or escape codes that act on a terminal. | The text rules of a directive refuse them at save (FR-001); the text is typed only under the 0012 rules (PB-3). |
  | A directive presses Enter into a question meant for the person. | The 0012 question check and input-line check apply to every scheduled directive (FR-009, PB-3). |
  | After a restart, a sleep of the machine, or a clock change, old due times all run at once. | A run more than 2 minutes late is missed, not run; missed due times give one entry per schedule (FR-005, FR-006). |
  | A deleted agent's name is reused, and the schedule sends to the new agent. | The schedule follows its target by id; when the target is deleted or removed, the office switches the schedule off (FR-010). |
  | A schedule text holds private words, and other users of the machine read it. | Schedules are kept readable only by the office owner (FR-003, 0001/FR-009). |

- **Residual risk:** an agent CLI runs as the person's OS user and can act outside the office (0002 A8), for example on the office's own files; the office cannot stop that. A schedule that the person switches on keeps acting while the person is away; the person switches it off, holds the office, or deletes it. A run wakes an asleep agent and starts its CLI, which uses the CLI's quota.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Schedule | One rule of the person. | name, target, text, repeat (kind, date, time, days, N, anchor), switch, next due time, last handled due time, last missed time, off reason, whether a schedule notification waits for a successful run | Unique id; name unique in the office. Refers to one agent by id. Kept by the background server. |
| Run result | What one run did. | due time, start time, due or Run now, result, reason, place in the queue, for a missed entry the count and the first and last missed times | Belongs to one schedule; the last 20 are kept in its history and removed with the schedule. Each is also an action record entry. |
| Schedule window | The scheduled directives that went to one target in the last 60 seconds. | target, the times of those directives | One per target. Counted apart from the directives of agents; not shown. |
| Schedule settings | The office bound of schedules. | schedule cap (1 to 200, default 50) | One per office, with the 0007 and 0020 settings. |
| Action record entry (extends 0001, 0002, 0007) | One recorded action. | actor: the user, a named agent, the office, or now a schedule (its name, its id, and the note that the person made it); actions now also the run results and the person's schedule actions | Belongs to the office and shows in the details of the agent it names. An entry of a run is recorded once, in its target's details. Stays after its schedule is deleted. |
| Notification (changes 0003) | An event that the user must see. | can now refer to a schedule and its target, or to the missed runs of one minute | As in MODIFIED, 0003 Key entities. Adds no Needs you item. |

### Success criteria

- **0025/SC-001:** The person saves a schedule that is on, with a once repeat 3 minutes ahead, for a running agent that waits for input: with 0 restarts, the agent's terminal receives the text with Enter within 60 seconds after the due time (1 of 1), and the action record entry names the schedule, its id, and that the person made it.
- **0025/SC-002:** A daily schedule at a set time runs on 10 days in a row that include a start and an end of daylight saving time: 10 of 10 runs start within 60 seconds after the set wall-clock time. On the day the clock skips 02:00 to 03:00, a schedule at 02:30 runs once at 03:00; on the day the clock repeats 01:00 to 02:00, a schedule at 01:30 runs once.
- **0025/SC-003:** An every-2-hours schedule across a start and an end of daylight saving time makes runs whose starts are 2 hours of elapsed time apart, within 60 seconds, in 100% of 12 runs.
- **0025/SC-004:** With an every-1-hour schedule and a once schedule, a server stop that spans 3 due times of the first and the due time of the second leads to 0 runs of those due times, exactly 1 missed entry with count 3 for the first, the second switched off with the off reason "missed", and exactly 1 notification for both, because the office adds both missed entries in the same minute. The first then runs at its next future due time.
- **0025/SC-005:** A due run that waits for the sweep (0017) runs when the sweep ends 90 seconds after the due time (1 of 1), and is missed when the sweep ends 150 seconds after the due time (1 of 1).
- **0025/SC-006:** After a run, the clock is set back 30 minutes: that due time runs 0 more times. The clock is set forward 3 hours over 3 due times of an every-1-hour schedule: 0 of those runs are made, and 1 missed entry with count 3 is added.
- **0025/SC-007:** Under an office hold, 10 of 10 due runs are refused with the hold reason; under an agent hold on the target, 10 of 10 are refused; 100% have an action record entry with the schedule as actor; after the release, 0 of them run later, and 0 agents are on automatic hold.
- **0025/SC-008:** With the limit "directives sent to one target agent" at 3, 4 schedules due in the same minute for one target give 3 directives in the queue and 1 refusal that names the limit and its value; in the same 60 seconds, a directive from the target's parent agent is accepted (1 of 1), and 0 agents are on automatic hold.
- **0025/SC-009:** For an asleep target whose CLI can continue its session, the schedule wakes it and the directive is submitted within 2 minutes after the due time in 10 of 10 runs, each with 1 wake entry with the schedule as actor.
- **0025/SC-010:** For a stopped target, 5 due runs give 5 `skipped` results with the status as reason, 0 starts, and exactly 1 notification; after the person starts the target and 1 run succeeds, the next skip gives exactly 1 new notification.
- **0025/SC-011:** When the person deletes the target of a schedule that is on, the view shows the schedule off with the off reason within 2 seconds, the person gets 1 notification, and 0 runs follow.
- **0025/SC-012:** The person presses Run now twice within 5 seconds while the target is working: the first run waits in the queue and the second is skipped with "previous run still waiting" (1 of 1).
- **0025/SC-013:** With the schedule cap at 3 and 3 schedules, a 4th save is refused with the cap and its value, and 0 schedules are added. 200 schedules with the cap at 200 are accepted.
- **0025/SC-014:** 10 schedules (5 on, 5 off), each with history, survive 5 server restarts and 1 machine restart with 100% of their fields and history unchanged, and the 5 that are on run at their next due time with 0 person actions after the restart.
- **0025/SC-015:** 50 schedules that are on and due in the same minute for 50 running agents that wait for input start 50 of 50 runs within 60 seconds after the due time.
- **0025/SC-016:** Of 10 agent requests to make, edit, switch, run, or delete a schedule, 10 are refused and recorded, and 0 schedules change.
- **0025/SC-017:** With the keyboard alone, the person makes, switches off, edits, switches on, runs now, and deletes a schedule with 0 pointer actions, and each step adds 1 action record entry with the person as actor.
- **0025/SC-018:** The person deletes a schedule whose directive waits in the target's queue: the directive is still typed when the target is ready (1 of 1), the schedule and its history are gone, and 100% of its action record entries stay.
- **0025/SC-019:** A scheduled directive left unsubmitted (the text does not show at the input line) and one withdrawn after 10 minutes in the queue each give exactly 1 action record entry, in the target's action record, with the schedule as actor (2 of 2), and the matching `unsubmitted` and `withdrawn` results in the schedule's history.

### Assumptions

The bare `A1` to `A15` in the text above and in the data flow name these assumptions of 0025.

- **A1:** A run that the office could not make at its time is never made late. A nightly job that fires when the person turns the machine on surprises the person and can collide with the person's work; a burst of catch-up runs is worse; and the office starts nothing by itself at a server start (0001/FR-004, 0013/PB-1, 0018/PB-1). One missed entry per schedule per moment, and one notification per minute, keep a long absence from flooding the history and the person.
- **A2:** The grace of 2 minutes lets a busy server, or the sweep at a start, delay a run without losing it, and is short enough that a run never comes at a time the person did not expect. A due time before the server start is missed even if it is less than 2 minutes before it, because the server did not run then.
- **A3:** A schedule is the person's explicit action, set ahead of time and switched on. So a run may wake the target, as the person's wake does, and 0018/PB-1 stays true. A run never starts an agent that is `stopped`, `exited`, `failed`, or `interrupted`: the person stopped it, or it ended, or the office must not start it after a restart (0001/FR-004).
- **A4:** A run counts as an agent-made action for the holds, because the person is not present when it acts. A hold means "nothing acts without me", so it refuses runs, Run now included. Of the rate limits, only "directives sent to one target agent" applies, with the same value, because one terminal is one input stream whoever sends; the others count actions per sending agent, and a schedule is not an agent. Scheduled directives fill a count of their own per target, so a schedule never makes a parent's directive fail and never brings an agent near an automatic hold. A refused run is not retried, so a hold release does not set off a burst, and it counts toward no automatic hold (0007/FR-008).
- **A5:** The office notifies the person when a run did not deliver for a reason the person may not know: the target does not run, a failure, a limit, an unsubmitted or withdrawn directive, or a switch off by the office. It does not notify for a hold refusal (the person set the hold), an overlap skip (the previous run is still on its way, and its own result notifies when it fails), or a withdrawal at a server stop (the person stopped the server). One notification per schedule until a run succeeds keeps a nightly failure from adding one notification per night.
- **A6:** A schedule notification adds no Needs you item: Needs you lists agents that wait for the person (0008), and a schedule is not an agent. The notification shows as unread when the desktop app opens (0003/FR-007).
- **A7:** Once, daily, and weekly times are wall-clock times, because the person thinks of "02:00 every night". When the time zone changes, the next run follows the new zone, so a daily run can come sooner or later than 24 hours after the last one; at most one run per due time stays true. Every-N-hours counts elapsed time, because the person thinks of "every 4 hours"; it starts N hours after the anchor, so a new or switched-on schedule does not run at once.
- **A8:** The default schedule cap of 50 is far above the number of repeated jobs that one person keeps, and low enough that a mistake cannot make thousands. The range goes to 200. The history keeps the last 20 results per schedule; the action record keeps every entry (0001/FR-003).
- **A9:** A run begins within 60 seconds after its due time. Schedules are for jobs that run hours or days apart; a minute of delay does not change what the person gets.
- **A10:** A directive's text follows the text rules of a directive (0002). The text is checked at save, so a run never fails on its text.
- **A11:** The sender of a scheduled directive is the schedule itself, never the target and never an agent. A schedule has no action record of its own, so each result that 0012 records "in the sender's and the target's action records" is recorded once, in the target's, with the schedule as actor, and also as a run result in the schedule's history. The 0012 withdrawal for a target that left the sender's subtree does not apply, because the person, not an agent, chose the target. The task line (0011) shows a scheduled directive once it is submitted, as for any directive (0012/PB-3).
- **A12:** Run now exists so that the person can try a schedule. It makes a normal run, so it obeys holds, the limit, and the overlap rule, and its result shows in the history. It never changes the next due time, so it does not move an every-N-hours anchor.
- **A13:** A delete removes the schedule and its history, but no action record entry, because the action record is the office's lasting trail (0001/FR-011). It does not withdraw a waiting directive, because it already left the schedule; the person withdraws nothing by deleting a rule.
- **A14:** A schedule follows its target by id. A target that is deleted, or a temp target that ends, switches the schedule off, rather than letting it pick a new agent with the same name, which could be a different job.
- **A15:** Every schedule field can be edited, the target included. An edit never changes a run that already started.

### Out of scope

- Schedules made, changed, or run by an agent, and any agent request about schedules (FR-014).
- A schedule that starts a temp (0020). A follow-up change adds it and reuses the schedule, the repeat and missed-run rules, the schedule as actor, and the schedule notifications of this change.
- Catch-up runs, late runs, and retries of a refused, skipped, or failed run (A1, A4).
- Waking the machine, or running a schedule while the background server is off.
- Other repeats: monthly, cron-style rules, an end date, a run count, skipped dates, or a time zone per schedule.
- One schedule for many targets, or for a team.
- Conditions on a run, such as "only if the target is idle".
- Other actions than a directive, such as a report, a start, a stop, or a new task.
- Starting an agent that is not running; a run only wakes an asleep target (A3).
- Sending schedule results outside the machine (0001/FR-005).
- Import, export, or copying of schedules.
- Split: proposed and done at spec review r1, scheduled temps to a follow-up change. A further split of the 14 ADDED requirements (for example the schedules view apart from the runs) was considered; kept as one, because a schedule without its missed-run, hold, and view rules is not safe to ship alone. The owner can still ask for it at approval.

### Notes from Deviations

- **No requirement is deferred or changed.** 0025 has no Deviations section.
- **Success criteria that the QA report marks PARTIAL (automated).** 0025/SC-002 and 0025/SC-003 prove the daylight saving rules in the domain tests with fixed zones, not over real days. 0025/SC-005 has no wire test of the wait for a running sweep. 0025/SC-007, 0025/SC-009, 0025/SC-015 and 0025/SC-016 run on fewer cases than the criterion names (3 of 10 refused runs, 1 of 10 wakes, 12 agents instead of 50, 5 of 10 agent requests). 0025/SC-014 runs one restart, not 5 restarts and a machine restart. 0025/SC-019 proves the withdraw by a stop of the target, not by the 10-minute queue limit.
- **Edge cases, user stories, the user experience, and the Clarifications** are in the archived spec.
