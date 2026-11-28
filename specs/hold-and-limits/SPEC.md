# Hold switch and agent action limits

## Intent

A hold switch stops agents from acting, and rate limits and action records keep a looping agent from running away. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0007-hold-and-limits](../changes/archive/0007-hold-and-limits/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0007/FR-007 | 0026 | Change 0007, Requirements |
| 0007/FR-009 | 0026 | Change 0007, Requirements |
| 0007/FR-010 | 0025 | Change 0007, Requirements |

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0026: [Secrets that reach running agents](../agent-secrets/SPEC.md)
- 0025: [Scheduled directives](../scheduled-directives/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

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

This change modified: 0003/FR-006, 0002/FR-009. The final text is under the change that owns each id. [Change 0026](../agent-secrets/SPEC.md) later modified 0007/FR-007 and 0007/FR-009; the text above is final.

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

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0007-hold-and-limits - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
