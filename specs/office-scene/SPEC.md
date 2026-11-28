# Office scene

## Intent

An optional scene draws the agents as people in an office, beside the same list and terminals. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0005-office-scene](../changes/archive/0005-office-scene/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

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

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0005-office-scene - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
