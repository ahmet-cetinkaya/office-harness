# Agent and team templates

## Intent

The person saves the definition of an agent or a team as a template and creates agents and teams from it. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0009-agent-templates](../changes/archive/0009-agent-templates/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0009/FR-001 | 0010 | Change 0009, Requirements |
| 0009/FR-003 | 0010 | Change 0009, Requirements |

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0010: [Agent worktrees and dependencies](../agent-worktree/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0009: Agent and team templates

### Intent

The person builds the same kinds of agents and teams again and again: a CEO, a lead with six members, the same CLI command and office additions each time. Without this change each agent is created by hand with every field (0001/FR-012, 0002/FR-002). This change lets the person save an agent definition as a template and create an agent from it in one step, save a team as a template and create a lead with its members in one action, and keep the templates in one plain file that can be exported and imported. Templates are copies: editing or deleting one changes no agent. Templates never carry environment values out of the office.

### Terms

| Term | Meaning |
|---|---|
| Agent, project, office additions, action record | As defined in 0001. |
| Role, team, CEO, team lead, member | As defined in 0002. |
| Agent template | A saved agent definition: template name, agent name pattern, CLI command, optional working folder, office additions, and optional role. |
| Team template | A saved team definition: template name, one lead entry, and one or more member entries. Each entry is an agent template and, for members, a count. |
| Name pattern | The text from which the office makes an agent's name. It can hold `{team}` (the team's name) and `{n}` (the lowest number that makes the name unique). |
| Template file | One plain-text file that holds one or more templates and a format version. |

### Requirements

- **0009/FR-001** (US1): THE SYSTEM SHALL let the user save an agent template with a template name, an agent name pattern, a CLI command, an optional working folder, office additions, an optional role, and the own worktree choice (0010), either as a new template or as a copy of an existing agent's definition.
- **0009/FR-002** (US1): WHEN the user creates an agent from an agent template at a place in the tree, with an optional project, THE SYSTEM SHALL create one agent with the template's values, a name from the name pattern that is unique in the office, and, when the template has no working folder, the project's folder as working folder.
- **0009/FR-003** (US2): THE SYSTEM SHALL let the user save a team template with a template name, one lead entry, and one or more member entries, where each entry is an agent template, with its own worktree choice, and each member entry has a count.
- **0009/FR-004** (US2): WHEN the user creates a team from a team template with a team name, an optional project, and a parent, THE SYSTEM SHALL create the team, its lead under the parent, and each member under the lead, in one action.
- **0009/FR-005** (US1, US2): IF any agent or the team of a template creation cannot be created, THEN THE SYSTEM SHALL create none of them and SHALL name each reason.
- **0009/FR-006** (US1, US2): WHEN the user edits or deletes a template, THE SYSTEM SHALL keep every agent and team created from it unchanged.
- **0009/FR-007** (US3): WHEN the user exports templates, THE SYSTEM SHALL write them to one plain-text file with a format version, with every environment variable as its name and an empty value, and SHALL say before writing that flags and startup prompts are written as they are.
- **0009/FR-008** (US3): WHEN the user imports a template file, THE SYSTEM SHALL show, as plain text, every template in it with its CLI commands, working folders, flags, environment variable names, startup prompts, and the name it will be saved under, and SHALL save the templates only after the user confirms.
- **0009/FR-009** (US3): IF an imported file has an unknown format version, cannot be read as a template file, is larger than 256 KiB, or holds a team template with more than 50 agents, or holds a template that FR-001 or FR-003 would not let the user save (for example an empty CLI command or a member count below 1), THEN THE SYSTEM SHALL refuse the whole file, name the template and the reason, and save no template from it.
- **0009/FR-010** (US2): IF a team template the user saves would create more than 50 agents, THEN THE SYSTEM SHALL refuse to save it and name the limit.
- **0009/FR-011** (US3): WHEN a template holds an environment variable without a value, THE SYSTEM SHALL show that variable as 'no value' in the template and in creation from it, and SHALL not pass a variable without a value to the CLI, so the login environment's value stays in effect (0001/FR-020).

Modified by later changes: 0009/FR-001 (by 0010), 0009/FR-003 (by 0010).

### Preserved behaviors

- **0009/PB-1:** Role placement keeps the rules of 0002: an agent or team created from a template gets a role only at a place that 0002/FR-001 allows, and a refusal names the allowed roles (0002/FR-003).
- **0009/PB-2:** Each agent created from a template is recorded in the action record with the user as actor, like any create (0001/FR-011).
- **0009/PB-3:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021); a template holds only office-harness's own definitions.
- **0009/PB-4:** No office data leaves the user's machine (0001/FR-005). An export writes only the file the user picks.
- **0009/PB-5:** Every desktop app action that this change adds is available from the keyboard (0001/FR-028).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent template | A saved agent definition. | template name, name pattern, CLI command, working folder (optional), office additions, role (optional) | Template name is unique among templates in the office. Not linked to the agents created from it. |
| Team template | A saved team definition. | template name, lead entry, member entries (agent template, count) | Template name is unique among templates. Uses agent templates as entries. |
| Template file | Templates outside the office. | format version, templates | Plain text the user owns. Holds no environment values. |

### Success criteria

- **0009/SC-001:** A user builds a 15-agent, 3-level tree (a CEO and two teams of a lead and 6 members) from two saved templates (one agent template, one team template used twice) in under 3 minutes and at most 3 create actions.
- **0009/SC-002:** Exporting templates and importing the file into an empty office reproduces 100% of each template's values except environment values, and the file holds 0 environment values.
- **0009/SC-003:** Each invalid file kind of FR-009 is refused with its named reason, and 0 templates are saved from it.
- **0009/SC-004:** After a template is edited or deleted, 0 agents and 0 teams created from it change.
- **0009/SC-005:** A team creation that fails at any agent leaves 0 new agents and 0 new teams.

### Assumptions

- **A1:** Creating from a template creates agents like the 0001 and 0002 create actions; it does not start them. The user starts them as in 0001.
- **A6:** 0001 has no "secret" mark on environment variables, so export treats every environment value as possibly secret and leaves all of them empty. Inside the office, templates keep the values, as agents already do (0001/FR-022). An imported variable has no value until the user gives one (FR-011).
- **A10:** Saving, editing, deleting, exporting and importing templates are not recorded in the action record; only the agents created from templates are (PB-2).

### Notes from Deviations

- **Own worktree choice (0010).** 0010 modified 0009/FR-001 and 0009/FR-003, and the final text above is the text of 0010. An agent template and each entry of a team template carry the own worktree choice as one optional value, off by default. The plan of 0009 had said that a build without 0010 refuses a template body that has this value; 0010 shipped, and the build accepts it.
- **Tests beyond the plan.** A later task added tests for 0009/PB-3, A1, and A10 that the plan did not list: saving, creating an agent and a team, and exporting leave the CLI homes and the project folder as they were, every agent made from a template is stopped, and template changes write no row of the action record.
- **Size.** The change has 11 requirements, more than the size rule of ten. It shipped as one change and the split was not done.
- **Gaps stated as PARTIAL or NOT RUN in the QA report.** 0009/SC-001 is PARTIAL: a scripted run on a mock core builds the 15 agents in 3 create actions, and nobody timed the 3-minute run by hand. These hands-on rows were not run: the real file dialogs on KDE Plasma, macOS, and Windows, a screen reader pass, and 200 percent zoom.
- **None of the notes defers or changes a requirement.**
- **Edge cases, user stories, and the user experience** are in the archived spec.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0009-agent-templates - 2026-11-28
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
