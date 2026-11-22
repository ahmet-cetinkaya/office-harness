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

- [0002-agent-hierarchy](../changes/archive/0002-agent-hierarchy/SPEC.md)
- [0003-panel-practicality](../changes/archive/0003-panel-practicality/SPEC.md)
- [0005-office-scene](../changes/archive/0005-office-scene/SPEC.md)
- [0007-hold-and-limits](../changes/archive/0007-hold-and-limits/SPEC.md)

Changes 0001 and 0008 to 0011 merge in when they ship. 0004, 0006, 0012 to 0014 are not merged.

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.
- Change 0001 is not merged yet. Ids of 0001 that other changes name keep the form `0001/FR-nnn`. The one requirement of 0001 that a merged change modified is under "Pending base change 0001".

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0001/FR-017 | 0003 | Pending base change 0001 |
| 0002/FR-009 | 0007 | Change 0002, Requirements |
| 0003/FR-006 (in part) | 0007 | Change 0003, Requirements |
| 0003/FR-008 (in part) | 0005 | Change 0003, Requirements |

## Pending base change 0001

Change 0001 (office server and terminal) is the base of every change in this file. It is not merged yet. It merges in a later pull request. Until then this file holds only the one requirement of 0001 that a merged change modified. The text is the one that change 0003 wrote.

- **0001/FR-017** (US1): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts: the shortcut that moves focus back to the panel, the agent shortcuts, and the switcher shortcut (A1). (modified by 0003)

Change 0007 widened this list by one shortcut. See the note after the success criteria of change 0007.


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
- **0002/FR-008** (US2): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, and send directives only within its own descendants and only for the roles that FR-001 allows.
  - Amended by 0015 (`specs/changes/0015-upward-reports/SPEC.md`): a running agent also sends reports, only to its own parent or to the person, with no field that names another agent (0015/FR-002).
- **0002/FR-009** (US1, US2, US3): IF an agent asks for an action on an agent that is not its descendant, for a role or action that 0002/FR-008 does not allow, for an action while it, its target, or the office is on hold, or for an action beyond a rate limit, THEN THE SYSTEM SHALL refuse the action, tell the agent the reason in the answer to its request, and record the refusal. (modified by 0007)
- **0002/FR-010** (US2): WHEN a parent agent sends a directive, THE SYSTEM SHALL enter the directive text into the target agent's terminal as one unbroken input followed by Enter.
  - Amended by 0012 (`specs/changes/0012-safe-directive-delivery/SPEC.md`): the text goes in as one paste only when the target waits for input, no person typed there in the last 30 seconds or left unsent text there, no text the office typed without Enter waits there, and the screen shows no open question; until then the directive waits in the target's queue. Enter follows only when the text, or the CLI's paste placeholder, shows at the input line (0012/FR-001 to FR-003).
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
- **0003/FR-006** (in part) (US3): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, or is put on hold automatically (0007/FR-008), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification. Exception (b) does not apply to the automatic hold, because the terminal does not show it. (modified by 0007)
  - Amended by 0015 (`specs/changes/0015-upward-reports/SPEC.md`): a report for the person is a notified event too; a further report adds no notification while one is unread, and the focused-terminal exception does not apply to it.
- **0003/FR-007** (US2): IF a notification occurs while the desktop app is closed, THEN THE SYSTEM SHALL show it as unread when the app opens.
- **0003/FR-008** (US2): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view, where the last view now also names the open view (panel or scene), and SHALL open that view. This changes steps 1 and 6 of the 0003 Flow, which now read "the open view" where they read "the panel", and 0003/SC-006, which now measures the open view. (modified by 0005)
- **0003/FR-009** (US2): WHEN the user opens a notification, or opens the terminal of an agent that has unread notifications, THE SYSTEM SHALL show those notifications of that agent as read.

FR-002 states R9 criterion 2. FR-006 notifies only for the four events it lists: a status change that the user caused (`stopped`) and `interrupted` after a server restart notify nobody.
Later additions to the shortcuts and the events (recorded 2026-11-23, ACF-106 archive; no requirement of this spec changed). The fixed shortcuts are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` here. Change 0008 added `Ctrl+Shift+F11` (jump to the agent that needs you), and change 0007 added `Ctrl+Shift+H` (the office hold). The server holds 12 reserved shortcuts in all (`src/modules/agent/domain/src/value_objects/settings.rs:33`). Change 0007 (FR-008) added the notification event `auto_hold` to the four events of FR-006, so the server knows five (`src/modules/notification/domain/src/enums/notification_event.rs:21`). Change 0008 added the Needs you queue. It is a separate queue with its own requests and event, not a notification event.

This change modified: 0001/FR-017. The final text is under "Pending base change 0001".

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

Amended by 0015: a fifth limit, "reports sent by one agent", 5 per 60 seconds, counts reports (0015/FR-006).
