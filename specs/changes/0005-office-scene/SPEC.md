---
status: approved (in the MVP, built after 0003)
capability: office
plan:
supersedes: 0003/FR-008 (in part)
---

<!-- Approved by the owner on 2026-11-17; owner may amend. In the MVP, built after 0003. -->

# 0005: Office scene

## Intent

The owner liked the idea of an office that he can look at, but he disliked walking to an agent to see its result and the slow moves between projects. This change adds an optional second view to the panel: a flat picture of the office in which each agent sits at a desk, each project is a floor, and each team is a department. The picture shows what the panel list shows, and the user opens any agent's terminal from a desk in one step. It adds no way to do work that the panel cannot do. This change covers R7 of `docs/PRD.md`.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4. R7 is in the MVP; this change is built after 0003.
- This change depends on 0001 (agents, projects, statuses, terminals, the desktop app) and on 0003 (the panel, from which the scene opens, and the agent list, whose content the scene repeats). It does not depend on 0002 or 0004. Where 0002 is present, the scene groups desks by team (FR-003). References such as `0001/FR-014` and `0003/FR-004` point to requirements in those specs.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it.
- The requirements below describe behavior only. The plan decides how to build them.
- The owner approved this spec on 2026-11-17. The decisions in Clarifications are approved as written; the owner may amend them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, status, quiet period, terminal, desktop app | As defined in 0001. |
| Panel, agent list, project switcher, last output line | As defined in 0003. The scene opens from the panel and does not replace it. |
| Role, team | As defined in 0002. Present only where 0002 is present. |
| Office scene | The second view of the desktop app that shows the agents as desks (see Key entities). |
| Desk | The place of one agent in the scene. It shows the agent's name, its status as text, and its last output line. |
| Floor | The group of desks of one project. Agents without a project share one floor named "No project". |
| Department | The group of desks of one team on a floor. Where 0002 is absent, or for an agent without a team, the floor has one unnamed group. |
| Open (a desk) | The user picks a desk by pointer or by keyboard. This opens the agent's live terminal in the same window, as in 0003/FR-004. |

## User stories

### US1: See the whole office as floors and desks (Priority: P1)

The user opens the scene from the panel and sees every agent of the chosen floors at a desk with its status, and opens any agent's terminal from its desk.

**Why this priority:** this is R7. Without it, the scene has no value.

**Independent test:** with 12 agents in 3 projects, open the scene. It shows 3 floors with 12 desks. Each desk shows a name, a status as text, and a last output line. Open one desk with a click and one with the keyboard. In both cases that agent's live terminal opens, and no second window opens.

**Scenarios:**

1. Given 12 agents run in 3 projects, When the user opens the scene, Then it shows 3 floors, and each floor shows one desk for each agent of its project.
2. Given a desk is shown, When the agent's status changes, Then the desk shows the new status as text within 2 seconds.
3. Given a desk is shown, When the agent produces output, Then the desk's last output line shows the new line within 2 seconds.
4. Given the scene is open, When the user opens a desk, Then that agent's live terminal opens in the same window and has keyboard focus.
5. Given the office has teams from 0002, When the user opens the scene, Then each floor groups its desks by team and names each team as a department, with the team lead's desk first.
6. Given an agent has no project, When the user opens the scene, Then its desk shows on the floor "No project".

### US2: Switch between panel and scene without losing my place (Priority: P2)

The user moves between the panel and the scene in one step and returns to where he was.

**Why this priority:** the scene is only useful if it stays optional and never traps the user. It needs US1.

**Independent test:** in the panel, pick the project "Backend" and select agent A. Open the scene, then return to the panel. The project choice and the selected agent are the same as before. Close the app in the scene; on the next start the app opens the scene.

**Scenarios:**

1. Given the panel is open, When the user chooses the scene action, Then the scene opens in the same window.
2. Given the scene is open, When the user chooses the panel action, Then the panel opens with the same project choice and the same selected agent as before.
3. Given the user used the scene when the app closed, When the user opens the app, Then the app opens the scene (0003 "Last view" now also holds the view that was open).
4. Given the scene is open, When the user picks a project in the project switcher, Then the scene shows only that project's floor, or all floors for "All projects".
5. Given the scene is open, When the user looks for an agent by name, Then the scene shows which desk it is and focuses it.

### US3: Read the scene without a pointer or motion (Priority: P3)

The user who works by keyboard, with a screen reader, or with reduced motion reads and uses the scene as fully as a user with a pointer.

**Why this priority:** the rules of 0001 demand it for every view, but the scene works for pointer users first, so this can follow.

**Independent test:** with only the keyboard, move over all 12 desks, open one, and return to the panel. With the system setting "reduce motion" on, the scene shows no moving element.

**Scenarios:**

1. Given the scene is open, When the user presses Tab and the arrow keys, Then keyboard focus moves between desks in reading order, floor by floor, and a visible focus mark shows the desk that has focus.
2. Given a desk has keyboard focus, When the user presses Enter, Then the desk opens (US1 scenario 4).
3. Given the user's system asks to reduce motion, When the scene is open, Then no element of the scene moves or flashes.
4. Given a screen reader runs, When a desk's status changes, Then the reader announces the agent's name and new status.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL offer an office scene as a second view that the user opens from the panel and leaves for the panel in one step each.
- **FR-002** (US1): WHILE the scene is open, THE SYSTEM SHALL show one desk for every agent of the project choice, each with the agent's name, its status as text, and its last output line.
- **FR-003** (US1): THE SYSTEM SHALL group the desks by floor (one per project) and, where 0002 is present, by department (one per team), with the team lead's desk first in its department.
- **FR-004** (US1): WHEN the status or the last output line of an agent changes, THE SYSTEM SHALL update the desk of that agent without a user action.
- **FR-005** (US1): WHEN the user opens a desk, THE SYSTEM SHALL open that agent's live terminal in the same window, with keyboard focus, as 0003/FR-004 does for the agent list.
- **FR-006** (US2): WHEN the user returns from the scene to the panel, THE SYSTEM SHALL show the panel with the project choice and the selected agent that it had before the scene opened.
- **FR-007** (US2): WHILE the scene is open, THE SYSTEM SHALL apply the project switcher and a search for an agent by name to the scene, with the same result as in the panel.
- **FR-008** (US3): THE SYSTEM SHALL make every scene action available from the keyboard, with a visible focus mark, and SHALL announce each change of a desk's status to screen readers.
- **FR-009** (US3): WHERE the user's system asks to reduce motion, THE SYSTEM SHALL show the scene with no moving or flashing element.

### MODIFIED

- **0003/FR-008** (US2): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view, where the last view now also names the open view (panel or scene), and SHALL open that view. This changes steps 1 and 6 of the 0003 Flow, which now read "the open view" where they read "the panel", and 0003/SC-006, which now measures the open view.

### PRESERVED

- **PB-1:** The scene shows data and starts no action of its own: it does not create, start, stop, or delete agents, and it sends no input to any terminal except through the terminal that the user opens (0001/FR-017).
- **PB-2:** The panel stays the main interface; it keeps every function it has, and the scene adds none that the panel lacks (`docs/PRD.md` section 4.1; 0003 Terms).
- **PB-3:** Every state shows as text, never color alone (0001/FR-014), and every scene action is available from the keyboard (0001/FR-028).
- **PB-4:** Opening or closing the scene starts, stops, or changes no agent. Agents keep running while no desktop app is open (0001/FR-019).
- **PB-5:** office-harness sends no office data outside the user's machine; the scene reads the same data as the panel and stores nothing else (0001/FR-005).
- **PB-6:** The scene writes no CLI configuration file and no project file (0001/FR-021).

## User experience

**Flow:**

1. The user opens the panel and chooses the scene action.
2. The scene shows the floors that the project switcher selects. Each floor is a labeled area with its project name and the number of agents. Each desk shows the agent's name, its status, and its last output line.
3. The user opens a desk by click or by keyboard, and the agent's live terminal opens in the same window.
4. The user chooses the panel action to return, or leaves the scene open; the app opens it again on the next start (0003/FR-008, modified).

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading, server not reachable, unauthorized | As in 0001 and 0003. | PB-3, 0003/PB-5 |
| No agents | The scene says that the office has no agent and offers the panel action, where the user creates the agent (0001). | Edge cases |
| Success | The floors with their desks. | FR-002, FR-003 |
| A project without agents | The floor shows its name and says that it has no agents. | Edge cases |
| No agent matches the search | The scene says that no desk matches the search. | FR-007, Edge cases |
| Desk status | Each desk shows its status as text. | FR-002, PB-3 |
| Many agents | The scene stays readable and usable; it scrolls, or groups folded by the user, instead of shrinking desks below a readable size. | SC-004, Edge cases |

**Design:** no fixed design exists. The scene is flat and two-dimensional (see Clarifications). The plan must not add visuals beyond this flow, and no moving figures, without a design that the owner approves.

**Accessibility:**

- The scene works from the keyboard: move between desks, open a desk, return to the panel (FR-008). Keyboard focus is always visible.
- Each desk has an accessible name that includes the agent's name, project, and status. Floor and department are text, not a picture alone.
- A change of a desk's status is announced to screen readers.
- Status is text, optionally with an icon, never color alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text), and the scene stays usable at 200% zoom without losing any text.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Desk | One agent in the scene. | agent name, status, last output line | One per agent. Shows an existing agent; it is not stored. |
| Floor | The desks of one project. | project name, agent count | One per project, plus "No project". Not stored. |
| Department | The desks of one team on a floor. | team name | One per team. Exists only where 0002 is present. Not stored. |
| Last view (extends 0003) | Where the user left the app. | open view (panel or scene), in addition to the 0003 attributes | One per desktop app on the machine, as in 0003 A4. |

## Success criteria

- **SC-001:** With 12 agents in 3 projects, a user who sees the panel opens the scene, finds a named agent's desk, and opens its terminal in under 10 seconds.
- **SC-002:** After an agent's status changes, the desk shows the new status, and after an agent's last output line changes, the desk shows the new line, within 2 seconds in 100% of 50 changes of each kind.
- **SC-003:** In the scene and in the panel, the same 12 agents show the same status text and the same last output line in 100% of 50 samples.
- **SC-004:** With 100 agents in 10 projects, the scene opens in under 2 seconds, and the user finds and opens a named agent's terminal in under 15 seconds.
- **SC-005:** With only the keyboard, a user opens the scene, opens a desk, and returns to the panel with 0 pointer actions.
- **SC-006:** After a round trip panel, scene, panel, the project choice and the selected agent equal their earlier values in 100% of 20 trials.
- **SC-007:** With the system setting "reduce motion" on, a scan of the scene while 12 agents change status finds 0 moving or flashing elements.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The office has no agent. | Empty | The scene says so and offers the panel action; the user creates the agent in the panel (0001). |
| A project has no agent. | Empty | Its floor shows with the text that it has no agents. |
| The search matches no desk. | Empty | The scene says that no desk matches. |
| The search text is only spaces. | Invalid | The scene treats it as no search. |
| An agent has no project. | Boundary | Its desk shows on the floor "No project" (US1 scenario 6). |
| An agent has a project but no team, where 0002 is present. | Boundary | Its desk shows on the floor in a group without a department name. |
| A last output line is very long, or the agent has no output yet. | Boundary | A long line is cut at the desk's width with a visible sign of the cut. No output shows as an empty line, never as an old line from another agent. |
| An agent name is very long. | Boundary | The name is cut with a visible sign, and the accessible name holds the full name. |
| 100 or more agents. | Boundary | The scene shows every desk at a readable size and scrolls (SC-004). |
| Agents are created, stopped, or deleted while the scene is open. | Concurrent | The scene adds, updates, or removes the desk without a user action (FR-004), and keyboard focus stays on a desk that still exists. |
| The agent under the focused desk is deleted. | Concurrent | Keyboard focus moves to the next desk, to the previous desk when none follows, and to the empty-state heading when no desk is left; the change is announced. |
| Two views would show different data. | Concurrent | Not allowed: the panel and the scene read the same state (SC-003). |
| 0002 is not present. | Boundary | The scene groups by floor only (FR-003). |
| The desktop app cannot reach the background server, or its token is rejected. | Unauthorized | The 0001 states show. |
| The user opens a desk of an agent whose status is `exited`, `failed`, or `stopped`. | Failure | The terminal opens with the agent's recent output, as in 0001/FR-016. |
| The server stops while the scene is open. | Failure | The scene shows the 0001 state for an unreachable server, as the panel does (PB-2). |
| The user's system asks to reduce motion. | Boundary | FR-009. |

## Assumptions

- **A1:** The scene is a flat, two-dimensional picture. The user does not move a figure, and no avatar walks between desks. This follows the owner's dislike of walking to an agent (Context, Intent). Reversible: see Clarifications.
- **A2:** The scene shows what the panel's agent list shows (name, status, last output line) and nothing that the server does not already hold. It needs no new data from the server.
- **A3:** A floor stands for a project and a department for a team. The scene does not let the user place, move, or rename desks; the layout follows the data.
- **A4:** The scene is optional: the app works as a whole without it, and a user who never opens it loses no function (PB-2).
- **A5:** The scene shows live meetings or a meeting record only if a later change adds them; 0004 does not require this change. Reversible: a later change can add it.
- **A6:** The 2-second update time (FR-004, SC-002) counts from the moment the server reports the change to the desktop app.

## Clarifications

### Session 2026-11-17 (overnight directive)

The owner approved these decisions on 2026-11-17.

- Q: 2D or 3D? → A: Flat 2D (A1). The owner tried a 3D scene and disliked walking to results. Reversible: a later change can add a 3D layer over the same desks.
- Q: Does an avatar or a figure walk, work, or react? → A: No. A desk only shows text status. Reversible: a later change can add motion that honors FR-009.
- Q: Is the scene the main view? → A: No. It is the second view and opens from the panel (FR-001, PB-2; `docs/PRD.md` section 4.1).
- Q: Can the user act on agents in the scene? → A: Only open a terminal (FR-005, PB-1). Create, start, stop, and delete stay in the panel, so that the scene never duplicates a function.
- Q: What do floors and departments stand for? → A: Floors stand for projects, departments for the teams of 0002 (A3). The user does not edit the layout.
- Q: Does the app reopen the scene after a restart? → A: Yes, as part of the last view (0003/FR-008, modified), to follow "resume where you left off" in `docs/PRD.md` section 4.1. Reversible: the panel can be the only start view.
- Q: Does the scene depend on 0002 or 0004? → A: No. It uses teams where 0002 is present (FR-003) and does not show meetings (A5).

### Open questions from review (2026-11-17)

Reversible defaults, so that the owner can amend them.

- Q: Where does the CEO's desk go, when 0002 is present and the CEO has no team? → A: On the CEO's own project floor (or "No project"), in the group without a department name, shown first.
- Q: Where does a team lead's desk go when the lead's project differs from its team's project? → A: On the lead's own floor, in the department of its team; the department then shows on each floor where one of its agents sits, and the lead is first only on the floor where it sits.
- Q: Does the scene mark stale data after a server stop? → A: No. The scene shows the 0001 state as the panel does (PB-2); a stale-data marker would be a new function for both views in a later change.

## Out of scope

- A 3D scene, moving avatars, and animation beyond the status text (A1).
- Editing the layout: placing, moving, or renaming desks, floors, and departments (A3).
- Creating, starting, stopping, or deleting agents from the scene (PB-1).
- Showing meetings or meeting records of 0004 (A5).
- Agents on other machines (R5).
- Search across terminal output of all agents.
- Notifications beyond those of 0003.
