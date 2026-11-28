---
status: shipped
capability: agent-hierarchy
plan:
supersedes:
---

<!-- Approved under the overnight directive 2026-11-16; owner may amend. -->

# 0002: Agent hierarchy

## Intent

With 12 or more agents, a flat set of terminals is hard to follow. This change organizes the agents of the office as a company: a CEO, teams with a team lead each, and members under the lead. A parent agent can create agents below it and send them directives, so that the CEO gives directives to team leads and a lead gives work to its members. This change covers R2 of `docs/PRD.md`.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4.
- The MVP is three change specs: `0001-office-server-and-terminal`, `0002-agent-hierarchy` (this one), and `0003-panel-practicality`. This change depends on 0001: it uses 0001's background server, agents, projects, terminals, sessions, office token, and action record. It does not depend on 0003. References such as `0001/FR-012` point to requirements in 0001.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it. The desktop app runs on Linux, macOS, and Windows.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, session, task, office token, action record | As defined in 0001. |
| Role | One of: CEO, team lead, member. An agent without a role is outside the hierarchy. |
| Team | A named group with one team lead and zero or more members, linked to at most one project. |
| Parent, child, descendant | The tree relations between agents: a team lead's parent is the CEO, and a member's parent is its team lead. The descendants of an agent are its children and their children. |
| Directive | Text that a parent agent sends to one of its descendants as input. When the user types into a terminal, that is ordinary input, not a directive. |

## User stories

### US1 — Organize agents as a company (Priority: P1)

The user builds a hierarchy: a CEO, teams with one team lead each, and members under a lead. Each member is an orchestrator; the sub-agents it uses are the CLI's own feature (A3).

**Why this priority:** the hierarchy makes 12 or more agents understandable (R2), and the user can build it by hand without agents that create agents. It is the smallest slice of this change with value on its own, and US2 needs it.

**Independent test:** with no other agents, create a CEO, 2 teams with a lead each, and 2 members per team (7 agents, 3 levels). The desktop app shows the tree with the correct parents. Deleting one team lead asks for confirmation and then removes that lead and its 2 members.

**Scenarios:**

1. Given the office has no CEO, When the user creates a CEO agent, Then the desktop app shows it at the top of the hierarchy.
2. Given a CEO exists, When the user creates team "Backend" for project P with a team lead, Then the lead shows under the CEO and the team shows its project.
3. Given a team lead exists, When the user adds a member to the team, Then the member shows under the lead with its role.
4. Given a team lead has 2 members, When the user deletes the lead, Then the app states that 3 agents will stop and be deleted, and deletes them only after the user confirms.
5. Given the user selects a parent, When he tries to add an agent with a role that is not allowed under that parent, Then the system refuses and names the roles that are allowed there.
6. Given a team lead with 2 running members, When the user stops the lead and chooses to stop its descendants too, Then all 3 agents show `stopped`.

### US2 — Delegate work down the hierarchy (Priority: P2)

A parent agent creates child agents and sends directives to them, as a CEO gives directives to team leads and a lead gives work to members.

**Why this priority:** this turns the tree into a working company, but the user can do the same work by hand with US1 and the terminals from 0001.

**Independent test:** in a CEO > team lead tree, have the lead agent, from inside its own session, create one member and send it a directive. The directive appears as input in the member's terminal, and the action record names the lead as the actor.

**Scenarios:**

1. Given a team lead agent runs, When it asks the office to create a member in its team, Then the member appears under the lead and the action record names the lead as creator.
2. Given a team lead agent runs, When it sends a directive to one of its members, Then the directive text appears in the member's terminal as one input followed by Enter.
3. Given a member agent runs, When it asks to send a directive to an agent outside its own descendants, Then the office refuses, tells the member why, and records the refusal.
4. Given the user types in a member's terminal, When the lead sends a directive to that member at the same time, Then the directive arrives as one unbroken input, not mixed with the user's keys.
5. Given a team lead runs, When it starts a new task on one of its members, Then the member starts a new CLI session.

## Requirements

### ADDED

- **FR-001** (US1, US2): THE SYSTEM SHALL organize agents in a tree with the roles CEO, team lead, and member, where a team lead's parent is the CEO when a CEO exists and a member's parent is its team lead. A member has no child agents in the office.
- **FR-002** (US1): THE SYSTEM SHALL let the user create a team with a name, an optional project, and one team lead.
- **FR-003** (US1): THE SYSTEM SHALL let the user create an agent with a role only at a place in the tree that FR-001 allows, and SHALL name the allowed roles when it refuses another place.
- **FR-004** (US1): THE SYSTEM SHALL show each agent under its parent in the desktop app's hierarchy view.
- **FR-005** (US1): THE SYSTEM SHALL NOT limit the number of agents in an office to a fixed maximum.
- **FR-006** (US1): WHEN the user deletes an agent that has descendants, THE SYSTEM SHALL show how many agents will stop and be deleted, and SHALL delete them only after the user confirms.
- **FR-007** (US1): WHEN the user stops an agent that has descendants, THE SYSTEM SHALL offer to stop all of its descendants in the same action.
- **FR-008** (US2): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, and send directives only within its own descendants and only for the roles that FR-001 allows.
  - Amended by 0015 (`specs/changes/archive/0015-upward-reports/SPEC.md`): a running agent also sends reports, only to its own parent or to the person, with no field that names another agent (0015/FR-002).
- **FR-009** (US2): IF an agent asks for an action on an agent that is not its descendant, or for a role or action that FR-008 does not allow, THEN THE SYSTEM SHALL refuse the action, tell the agent the reason, and record the refusal.
- **FR-010** (US2): WHEN a parent agent sends a directive, THE SYSTEM SHALL enter the directive text into the target agent's terminal as one unbroken input followed by Enter.
  - Amended by 0012 (`specs/changes/archive/0012-safe-directive-delivery/SPEC.md`): the text goes in as one paste only when the target waits for input, no person typed there in the last 30 seconds or left unsent text there, no text the office typed without Enter waits there, and the screen shows no open question; until then the directive waits in the target's queue. Enter follows only when the text, or the CLI's paste placeholder, shows at the input line (0012/FR-001 to FR-003).
- **FR-011** (US1, US2): THE SYSTEM SHALL add directives and refused actions to the action record and SHALL record a named agent as the actor of each action that an agent takes.

### PRESERVED

- **PB-1:** An agent without a role keeps the behavior of 0001: the user creates, starts, attaches to, stops, and deletes it, and it shows outside the hierarchy.
- **PB-2:** Each new task starts a new CLI session, also when a parent agent starts it (0001/FR-024).
- **PB-3:** office-harness creates, changes, or deletes no file in an agent CLI's configuration, also when it lets agents call the office (0001/FR-021).
- **PB-4:** Agents in the hierarchy keep running while no desktop app is open, and the teams and the tree stay across server and machine restarts (0001/FR-001, 0001/FR-004).
- **PB-5:** Every desktop app action is available from the keyboard, including the actions that this change adds (0001/FR-028).

## User experience

**Flow:**

1. The user opens the hierarchy view in the desktop app. It shows the CEO, the teams with their projects, each team lead, and its members. Agents without a role show in a separate group outside the hierarchy.
2. The user creates a CEO, or a team with its lead, or a member under a lead. The create action offers only the roles allowed at the selected place.
3. The user selects any agent in the tree to attach to its terminal or open its details, as in 0001.
4. When the user deletes or stops an agent that has descendants, the app shows the number of affected agents and asks for confirmation.
5. Agents create members and send directives from inside their sessions. The new agents appear in the tree, and the directives appear as input in the target terminals and in the action record.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Empty | The office has no CEO and no team. The hierarchy view says so and offers the actions that create a CEO or a team. | Edge cases |
| Success | The tree with each agent under its parent and its role as text. | FR-004, US1 scenarios 1 to 3 |
| Confirmation | The number of agents that a delete or stop affects, with confirm and cancel actions. | FR-006, FR-007 |
| Error: role not allowed | The refusal, with the roles that are allowed at that place. | FR-003, US1 scenario 5 |
| Unauthorized agent action | The action record shows the refused action, its actor, and the reason. | FR-009, FR-011 |
| Loading, server not reachable, unauthorized client | As in 0001. | 0001/FR-026, 0001/FR-027 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The tree works from the keyboard: move between agents, expand and collapse teams, and open an agent (PB-5).
- Each tree entry has an accessible name that includes the agent's name, role, and status. Role and level are text, never color or indentation alone.
- The confirmation dialog takes focus, states the number of affected agents as text, and closes with Escape as a cancel.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | role (optional), parent agent (optional), team (optional) | Has at most one parent and belongs to at most one team. A member's team is its lead's team. |
| Team | A group of agents for one area or project. | name, project (optional) | Name is unique in the office. Has one team lead and zero or more members. |
| Directive | Text that a parent agent sends to a descendant. | sender agent, target agent, text, time | Recorded as an action record entry. |
| Action record entry (extends 0001) | One recorded action. | action now also `directive` and `refused`; actor can be a named agent | Belongs to the office and shows in the target agent's details; a refused directive also shows in the sender's details. |

## Success criteria

- **SC-001:** A user builds a 15-agent, 3-level hierarchy (1 CEO, 2 team leads, 12 members) from the desktop app alone, without editing any file, in under 10 minutes.
- **SC-002:** With 30 agents in 3 levels that run at the same time, the hierarchy view shows 100% of them under their correct parents.
- **SC-003:** In a test of 20 agent requests that FR-008 does not allow, 100% are refused, and 100% show in the action record with the actor and the reason.
- **SC-004:** In a test of 100 directives sent while the user types in the target terminal, 100% arrive as one unbroken input followed by Enter.
  - Amended by 0012: while the user types, or leaves typed text unsent, the directives wait in the target's queue and are typed after the user's input, each as one unbroken input followed by Enter (0012/FR-002, 0012/SC-003, 0012/SC-008).
- **SC-005:** Deleting a team lead with N members removes exactly N + 1 agents after the user confirms, and 0 agents when the user cancels.
- **SC-006:** A team lead agent creates a member and gives it a directive with 0 user actions between the two steps.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The office has no CEO and no team. | Empty | The hierarchy view says so and offers the create actions. |
| A team has a lead but no members. | Empty | The team shows its lead and says that it has no members. |
| The user creates a second CEO. | Invalid | The system refuses and names the existing CEO. |
| The user creates a team without a team lead. | Invalid | The system refuses; a team needs its lead. |
| Two teams get the same name. | Invalid | The system refuses the second one and names the conflict. |
| A role is not allowed under the selected parent, for example a member under the CEO. | Invalid | The system refuses and names the allowed roles (FR-003). |
| An agent sends a directive to an agent that is not running. | Invalid | The system refuses, tells the sender that the target is not running, and records the refusal. |
| A directive is very long or contains control characters. | Boundary | The system enters the text as one input and does not run control characters in it as keys. |
| An agent creates many agents in a loop. | Boundary | The system adds no fixed cap (FR-005). Each new agent shows in the tree and the action record, and the user stops the parent and its descendants in one action (FR-007). |
| The user types while a directive arrives. | Concurrent | The user's input reaches the terminal at once; the directive waits until 30 seconds after the user's last input, then arrives as one unbroken input (FR-010 as amended by 0012). |
| Two parent agents' directives arrive at the same target at the same time. | Concurrent | Each directive arrives as one unbroken input, one after the other, in the order the office accepted them (0012/FR-001). |
| The user deletes an agent while its parent sends it a directive. | Concurrent | The system refuses the directive, tells the parent that the target was deleted, and records the refusal. |
| An agent acts outside its own descendants, on itself, or asks to delete an agent. | Unauthorized | The system refuses and records it (FR-009). |
| A parent agent stops, fails, or is interrupted while its children run. | Failure | The children keep running and keep their place in the tree. |
| The user deletes the CEO. | Failure | The confirmation counts every agent in the tree, and the system deletes the whole tree only after the user confirms (FR-006). |

## Assumptions

- **A1:** The user creates agents of every role from the desktop app (US1). A running parent agent creates agents only within its own descendants (US2).
- **A2:** A directive is text that a parent agent enters into a descendant's terminal. The MVP has no separate message channel and no reply channel. A parent reads a child's result from the child's terminal output or from the files that the child writes.
  - Amended by 0015: an agent also reports upward to its parent, or to the person through Needs you; a parent still answers with a directive, and there is no reply request (0015 A9).
- **A3:** office-harness does not manage sub-agents. A member is an orchestrator that uses its CLI's own sub-agent feature. Those sub-agents run inside the member's session, show only in its terminal output, and are not office agents. This is an owner decision (see Clarifications).
- **A4:** The CEO can create team leads, and a team lead can create members of its own team. A member creates no office agents. Agents cannot delete agents; only the user deletes.
- **A5:** The office has at most one CEO. Teams can exist without a CEO; when the user creates a CEO later, the existing team leads move under it.
- **A6:** A member's project is its team's project, and its working folder is that project's folder by default (0001 A6).
- **A7:** office-harness gives each agent a way to call the office from inside its session. It does so through the agent's start environment, flags, or prompt, never through the CLI's configuration files (PB-3).
- **A8:** The descendant rule (FR-008, FR-009) prevents mistakes. It does not stop a hostile agent that runs as the same OS user and reads the user's office token (0001 A7).
- **A9:** Moving an agent to a new parent is not in the MVP. The user deletes the agent and creates it again.
- **A10:** Later releases stay open: R7 can show the roles, teams, and projects of this change as desks and departments, and R8 can use directives to talk to chosen agents.

## Clarifications

### Session 2026-11-15

- Q: How many levels does the hierarchy have, and which roles may create which roles? → A: Three levels, CEO > team lead > member. office-harness has no sub-agent role, because members use their CLI's own sub-agent feature (FR-001, A3, A4).
- Q: Should the MVP spec be split? → A: Yes, into `0001-office-server-and-terminal`, `0002-agent-hierarchy`, and `0003-panel-practicality`. This change depends on 0001.
- Q: Where is a refused directive recorded, in the sender's action record or the target's? → A: In both, one entry each, one entry when they are the same agent (owner decision, 2026-11-20).

## Out of scope

- Office-managed sub-agents below a member (A3).
- Moving agents between parents (A9).
- A message or reply channel between agents beyond directives (A2).
- Limits on how many agents an agent may create.
- The one-list overview, the project switcher, and notifications: change 0003.
- R5 distributed machines, R7 office scene, and R8 meeting room.
- **Split:** done by owner decision on 2026-11-15 (see Clarifications).

## Deviations

- **A5 was not shipped until (closed): leads did not move under a CEO that the user creates later.** Requirement (A5): when the user creates a CEO later, the existing team leads move under it. Shipped: a lead takes the CEO as its parent only when it is placed, and only if a CEO exists then (`resolved_parent` in `src/modules/hierarchy/domain/src/services/tree_rules.rs:128`). No code rewrites `parent_agent_id` of the leads when a CEO is placed; the only write of the place is for the one agent that is placed (`src/modules/hierarchy/infrastructure/src/services/agent_places.rs:55`). A lead that was placed before the CEO keeps no parent, so the descendant rule (`is_descendant_of`, `tree_rules.rs:171`) does not make it a descendant of the CEO. The tree view still shows the team. Reason: not recorded. This is a gap in the code, tracked as a code bug (see the QA report), not a decision to drop A5. Recorded 2026-11-23. Fixed on 2026-11-23: placing a CEO now moves every team lead that has no parent under the CEO, in the same transaction as the CEO's own place write (`AgentPlaces::adopt_parentless_leads`, called from `HierarchyRepository::place_agent`). Tests: `placing_a_ceo_moves_the_parentless_leads_under_it` and `a_failed_ceo_placement_moves_no_lead` (`hierarchy_repository_tests.rs`), and the wire test `a_ceo_created_after_a_team_takes_the_team_lead_under_it` (`tests/agent_subtree.rs`). An office saved before the fix is repaired once at each start: when exactly one CEO exists, every lead without a parent moves under it (`HierarchyRepository::repair_parentless_leads`; tests in `H/tests/startup.rs`). Shipped behavior now matches A5; this Deviation is closed.
- **`team.delete` has no requirement.** The contract has the request `team.delete`, and no FR of this spec names it. It is an extra request that serves the team part of FR-002 (a team the user can create, the user can also remove). It removes only the team record. Recorded 2026-11-23.
- **0007 changed FR-009.** Change 0007 (hold switch and agent action limits) supersedes this FR-009 in part: a hold or a rate limit also refuses an agent action, and the refusal is recorded. See `supersedes:` in `specs/changes/archive/0007-hold-and-limits/SPEC.md` and the amendment note in the contract. Recorded 2026-11-23.
- **The `plan:` front matter is empty.** The plan of this change is `.agents/work/0002-agent-hierarchy/plan.md` in the owner's checkout. The folder `.agents/` is git-ignored, so the file is not in the repository. The contract names decision D6 of that plan. Recorded 2026-11-23.
