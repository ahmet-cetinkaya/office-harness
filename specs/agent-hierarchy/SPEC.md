# Agent hierarchy and directives

## Intent

Agents sit in a hierarchy of leads and members. A lead creates and directs other agents, and a directive reaches an agent as one safe input. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0002-agent-hierarchy](../changes/archive/0002-agent-hierarchy/SPEC.md)
- [0012-safe-directive-delivery](../changes/archive/0012-safe-directive-delivery/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0002/FR-008 | 0014, 0015, 0026 | Change 0002, Requirements |
| 0002/FR-009 | 0007 | Change 0002, Requirements |
| 0002/FR-010 | 0012 | Change 0002, Requirements |
| 0012/FR-002 | 0024 | Change 0012, Requirements |
| 0012/FR-003 | 0025 | Change 0012, Requirements |
| 0012/FR-005 | 0025 | Change 0012, Requirements |

Changes of other capabilities that this file names:

- 0011: [Agent panel](../agent-panel/SPEC.md)
- 0014, 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0026: [Secrets that reach running agents](../agent-secrets/SPEC.md)
- 0024: [Exact agent status from CLI hooks](../agent-status/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0025: [Scheduled directives](../scheduled-directives/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

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

## Change 0012: Safe directive delivery

### Intent

A parent agent's directive was typed into the target's terminal at once, with Enter, whenever the target ran. When the target's CLI showed a question for a person, the Enter or a letter of the text could answer it, and nobody saw it. This change puts each directive in a queue per target and types it only when the target waits for input, no person types or has unsent text there, no text the office typed without Enter waits there, and the screen shows no question that the office recognises. Enter follows only after the text, or the CLI's placeholder for it, shows at the input line. The sender learns whether its directive was submitted, waits, or was left unsubmitted, and a directive that cannot be typed in time is withdrawn and recorded. This change modifies 0002/FR-010 and amends 0011 (the task line shows the last submitted directive). What the screen checks cannot cover is under Residual risk in the change spec.

### Requirements

- **0012/FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one left the queue and the conditions of 0012/FR-002 hold again.
- **0012/FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, no unsent person text waits at its input line, no pending office text (a startup prompt or an unsubmitted directive) waits in the target's current session, the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval), and, WHILE the target's session is exact (0024), no permission ask is open and no compaction runs (0024/FR-016); otherwise THE SYSTEM SHALL keep the directive in the queue. (modified by 0024)
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

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0002-agent-hierarchy - 2026-11-23
- 0012-safe-directive-delivery - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
