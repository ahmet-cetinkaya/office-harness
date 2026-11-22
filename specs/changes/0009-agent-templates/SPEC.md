---
status: approved
capability: office
plan: specs/changes/0009-agent-templates/PLAN.md
supersedes:
---

<!-- Approved by the owner on 2026-11-19, through Efe's review gate. -->

# 0009: Agent and team templates

## Intent

The owner builds the same kinds of agents and teams again and again: a CEO, a lead with six members, the same CLI command and office additions each time. Today each agent is created by hand with every field (0001/FR-012, 0002/FR-002). This change lets him save an agent definition as a template and create an agent from it in one step, save a team as a template and create a lead with its members in one action, and keep his templates in one plain file that he can export and import. Templates are copies: editing or deleting one changes no agent. Templates never carry environment values out of the office.

## Context

- Source of truth: `docs/PRD.md`. The 15-agent, 3-level tree of acceptance criterion AC-06 is the size this change must make fast to build.
- This change depends on 0001 (agents, office additions, the action record) and 0002 (roles, teams, and where each role may be created). References such as `0002/FR-003` point to requirements in those changes.
- Importing a template file is the first way for text from outside the office to become a CLI command that the office runs. The file is untrusted data: the user sees every command before any template is saved, and importing creates no agent.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, office additions, action record | As defined in 0001. |
| Role, team, CEO, team lead, member | As defined in 0002. |
| Agent template | A saved agent definition: template name, agent name pattern, CLI command, optional working folder, office additions, and optional role. |
| Team template | A saved team definition: template name, one lead entry, and one or more member entries. Each entry is an agent template and, for members, a count. |
| Name pattern | The text from which the office makes an agent's name. It can hold `{team}` (the team's name) and `{n}` (the lowest number that makes the name unique). |
| Template file | One plain-text file that holds one or more templates and a format version. |

## User stories

### US1 — Create an agent from a saved agent template (Priority: P1)

The user saves an agent definition as an agent template, new or copied from an existing agent, and creates an agent from it in one step at a place he picks.

**Why this priority:** it removes retyping the CLI command and the office additions for each agent, and it is the base the team template builds on.

**Independent test:** save agent A as a template. Create two agents from it with one action each. Both have A's CLI command and office additions and names from the pattern. Edit the template; the two agents stay as they were.

**Scenarios:**

1. Given agent A exists, When the user saves A as an agent template, Then the template holds A's CLI command, working folder, office additions, and role, and A does not change.
2. Given an agent template with name pattern `reviewer-{n}` and no working folder, When the user creates an agent from it in project P, Then the office creates `reviewer-1` with the template's values and P's folder as working folder.
3. Given `reviewer-1` exists, When the user creates another agent from the same template, Then the new agent is `reviewer-2`.
4. Given an agent template with role member, When the user creates an agent from it at a place where 0002 allows no member, Then the office creates nothing and names the allowed roles.
5. Given agents were created from a template, When the user edits or deletes that template, Then none of those agents changes.

### US2 — Create a team from a team template in one action (Priority: P1)

The user saves a team template (a lead and members with counts) and creates a full team from it, with one typed team name, under a parent he picks.

**Why this priority:** the owner's success measure is a 15-agent, 3-level tree from two saved templates in under 3 minutes. Team templates do most of that work. It needs US1's agent templates for its entries.

**Independent test:** save a team template with one lead entry and one member entry with count 6. Create team "api" from it under the CEO. The office creates team "api" with lead `api-lead` and members `api-member-1` to `api-member-6`, all in the tree under the CEO.

**Scenarios:**

1. Given a team template with a lead and 6 members, When the user creates team "api" from it under the CEO, Then the office creates team "api", its lead under the CEO, and 6 members under the lead, in one action.
2. Given a team template, When the user creates two teams from it with different team names, Then each team gets its own lead and members with names from the patterns.
3. Given team "api" exists, When the user creates a team named "api" from a template, Then the office creates nothing and says that the team name is taken.
4. Given one member's working folder does not exist, When the user creates a team from the template, Then the office creates no team and no agent and names the missing folder.

### US3 — Keep templates in a file I own (Priority: P2)

The user exports his templates to one plain-text file, keeps it in his own repository, and imports it into an office. Import shows what the file holds before it saves anything.

**Why this priority:** it lets the owner keep and reuse templates across machines and office rebuilds. US1 and US2 already have value inside one office.

**Independent test:** export all templates from office 1. The file holds every template with environment variable names but no values. Import the file into an empty office 2: the preview lists each template with its commands and folders; after confirm, office 2 has the same templates and no agent.

**Scenarios:**

1. Given the office has templates whose office additions hold environment variables with values, When the user exports them, Then the file holds each variable's name with an empty value.
2. Given a valid template file, When the user imports it, Then the office shows every template with its CLI commands, working folders, flags, environment variable names, startup prompts, and the name it will be saved under, and saves nothing yet.
3. Given the import preview shows, When the user confirms, Then the office saves the templates and creates no agent.
4. Given the import preview shows, When the user cancels, Then the office saves nothing.
5. Given a file with an unknown format version, When the user imports it, Then the office refuses the whole file, names the reason, and saves nothing.
6. Given an imported template holds variable `API_KEY` without a value, When the user creates an agent from it and starts it, Then the CLI runs with the login environment's `API_KEY`, and the agent's details show `API_KEY` as 'no value'.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let the user save an agent template with a template name, an agent name pattern, a CLI command, an optional working folder, office additions, and an optional role, either as a new template or as a copy of an existing agent's definition.
- **FR-002** (US1): WHEN the user creates an agent from an agent template at a place in the tree, with an optional project, THE SYSTEM SHALL create one agent with the template's values, a name from the name pattern that is unique in the office, and, when the template has no working folder, the project's folder as working folder.
- **FR-003** (US2): THE SYSTEM SHALL let the user save a team template with a template name, one lead entry, and one or more member entries, where each entry is an agent template and each member entry has a count.
- **FR-004** (US2): WHEN the user creates a team from a team template with a team name, an optional project, and a parent, THE SYSTEM SHALL create the team, its lead under the parent, and each member under the lead, in one action.
- **FR-005** (US1, US2): IF any agent or the team of a template creation cannot be created, THEN THE SYSTEM SHALL create none of them and SHALL name each reason.
- **FR-006** (US1, US2): WHEN the user edits or deletes a template, THE SYSTEM SHALL keep every agent and team created from it unchanged.
- **FR-007** (US3): WHEN the user exports templates, THE SYSTEM SHALL write them to one plain-text file with a format version, with every environment variable as its name and an empty value, and SHALL say before writing that flags and startup prompts are written as they are.
- **FR-008** (US3): WHEN the user imports a template file, THE SYSTEM SHALL show, as plain text, every template in it with its CLI commands, working folders, flags, environment variable names, startup prompts, and the name it will be saved under, and SHALL save the templates only after the user confirms.
- **FR-009** (US3): IF an imported file has an unknown format version, cannot be read as a template file, is larger than 256 KiB, or holds a team template with more than 50 agents, or holds a template that FR-001 or FR-003 would not let the user save (for example an empty CLI command or a member count below 1), THEN THE SYSTEM SHALL refuse the whole file, name the template and the reason, and save no template from it.
- **FR-010** (US2): IF a team template the user saves would create more than 50 agents, THEN THE SYSTEM SHALL refuse to save it and name the limit.
- **FR-011** (US3): WHEN a template holds an environment variable without a value, THE SYSTEM SHALL show that variable as 'no value' in the template and in creation from it, and SHALL not pass a variable without a value to the CLI, so the login environment's value stays in effect (0001/FR-020).

### PRESERVED

- **PB-1:** Role placement keeps the rules of 0002: an agent or team created from a template gets a role only at a place that 0002/FR-001 allows, and a refusal names the allowed roles (0002/FR-003).
- **PB-2:** Each agent created from a template is recorded in the action record with the user as actor, like any create (0001/FR-011).
- **PB-3:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021); a template holds only office-harness's own definitions.
- **PB-4:** No office data leaves the user's machine (0001/FR-005). An export writes only the file the user picks.
- **PB-5:** Every desktop app action that this change adds is available from the keyboard (0001/FR-028).

## User experience

**Flow:**

1. In an agent's details, the user saves the agent as a template. Or he opens the templates view and creates a template.
2. To create an agent, he picks a template, a place in the tree, and an optional project. One action creates the agent.
3. To create a team, he picks a team template, types the team name, and picks the parent and an optional project. One action creates the team, its lead, and its members.
4. To export, he picks templates and a file. The office says that environment values are left out and that flags and startup prompts are written as they are, then writes the file.
5. To import, he picks a file. The office shows each template with its commands, folders, flags, environment variable names, and startup prompts. He confirms or cancels.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| No templates | The templates view says that no template exists, with the actions that save one and that import a file. | Edge cases |
| Templates | Each template with its name, kind (agent or team), CLI command, and role or agent count. | FR-001, FR-003 |
| Creation refused | Nothing is created; each reason is named (taken name, place not allowed, missing folder). | FR-005, US1 scenario 4, US2 scenarios 3 and 4 |
| Export notice | Environment values are left out; flags and startup prompts are written as they are. | FR-007 |
| Import preview | Every template with its commands, folders, flags, environment variable names, startup prompts, and saved name; confirm and cancel. | FR-008 |
| Import refused | The reason; no template saved. | FR-009 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The templates view, creation from a template, export, and the import preview with confirm and cancel work from the keyboard (PB-5). Keyboard focus is always visible.
- The import preview is readable by screen readers in a fixed order: template name, kind, command, folder, flags, environment variable names, startup prompt.
- Refusal reasons are text and are announced to assistive technology.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent template | A saved agent definition. | template name, name pattern, CLI command, working folder (optional), office additions, role (optional) | Template name is unique among templates in the office. Not linked to the agents created from it. |
| Team template | A saved team definition. | template name, lead entry, member entries (agent template, count) | Template name is unique among templates. Uses agent templates as entries. |
| Template file | Templates outside the office. | format version, templates | Plain text the user owns. Holds no environment values. |

## Success criteria

- **SC-001:** A user builds a 15-agent, 3-level tree (a CEO and two teams of a lead and 6 members) from two saved templates (one agent template, one team template used twice) in under 3 minutes and at most 3 create actions.
- **SC-002:** Exporting templates and importing the file into an empty office reproduces 100% of each template's values except environment values, and the file holds 0 environment values.
- **SC-003:** Each invalid file kind of FR-009 is refused with its named reason, and 0 templates are saved from it.
- **SC-004:** After a template is edited or deleted, 0 agents and 0 teams created from it change.
- **SC-005:** A team creation that fails at any agent leaves 0 new agents and 0 new teams.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| No template exists. | Empty | The templates view says so and offers save and import. |
| An imported template has the name of an existing template. | Boundary | The preview shows the name it will be saved under, with the lowest free number appended (A3); nothing is replaced. |
| A name pattern has no `{n}` and the name exists. | Boundary | The office adds `-{n}` to make the name unique (A2). |
| A team template's lead entry has role CEO, or a member entry has role team lead. | Invalid | The position sets the role (A4); the entry's own role is ignored for team creation. |
| An agent template with role CEO is used when a CEO exists. | Invalid | Nothing is created; 0002's refusal names the allowed roles (PB-1). |
| The template's working folder, or the chosen project's folder, does not exist on this machine. | Invalid | Nothing is created; the folder is named (FR-005). |
| Neither the template nor the creation has a working folder or a project. | Invalid | Nothing is created; the office says a working folder is needed (FR-005). |
| An imported startup prompt or command holds control characters or escape codes. | Invalid | The preview shows them as visible text, not as terminal codes (FR-008). |
| An imported file is larger than 256 KiB, or holds a team template with 51 agents. | Boundary | Refused with the reason (FR-009). |
| An agent template used in a team template is deleted. | Concurrent | The team template keeps its own copy of the entry and still works (A5). |
| Two creations from one template at the same moment. | Concurrent | Each gets a different unique name; neither fails because of the other. |
| The file cannot be written on export (no permission, disk full). | Failure | The office says the file was not written; no template changes. |
| A flag or startup prompt holds a secret. | Failure | Export writes it as is; the export notice says so before writing (FR-007). |
| An agent template whose name pattern holds `{team}` is used outside a team creation. | Invalid | `{team}` is replaced by the team of the place picked, or, without one, the creation is refused and names the pattern. |

## Assumptions

- **A1:** Creating from a template creates agents like the 0001 and 0002 create actions; it does not start them. The user starts them as in 0001.
- **A2:** Defaults for name patterns: an agent template copied from agent `x` gets `x-{n}`; a team lead entry gets `{team}-lead`; a member entry gets `{team}-member-{n}`. A pattern without `{n}` gets `-{n}` added only when the name is taken.
- **A3:** An imported template whose name exists is saved under that name with ` (2)`, ` (3)`, and so on, the lowest free one. Import never replaces a template.
- **A4:** In a team template, the position sets the role: the lead entry creates a team lead, each member entry creates members. An agent template's own role applies only when an agent is created from it directly.
- **A5:** A team template holds copies of its entries, not links to agent templates. Editing or deleting an agent template does not change a team template.
- **A6:** 0001 has no "secret" mark on environment variables, so export treats every environment value as possibly secret and leaves all of them empty. Inside the office, templates keep the values, as agents already do (0001/FR-022). An imported variable has no value until the user gives one (FR-011).
- **A7:** The 50-agent limit per team template guards one confirmation against creating an unexpected number of agents; it is not an office size cap (0002/FR-005 holds). The 256 KiB file limit is far above any real template file.
- **A8:** Templates are kept by the background server in the office, so they survive restarts like the rest of the office (0001/FR-003).
- **A9:** Only the user uses templates in this change. Agents that create agents under 0002 do not use templates.
- **A10:** Saving, editing, deleting, exporting and importing templates are not recorded in the action record; only the agents created from templates are (PB-2).

## Clarifications

### Session 2026-11-19 (review edits)

- Q: What does an environment variable without a value do after import? → A: It shows as 'no value' and is not passed to the CLI, so the login environment's value stays in effect (FR-011, A6).
- Q: What happens to an imported template that breaks FR-001 or FR-003? → A: The whole file is refused and the template and the reason are named (FR-009).
- Q: What does `{team}` become when an agent template is used outside a team creation? → A: The team of the place picked; without one, the creation is refused and names the pattern (Edge cases).
- Q: Are template changes recorded in the action record? → A: No; only the agents created from templates are (A10).

## Out of scope

- A template marketplace or sharing templates with other people.
- Templates for meetings (R8).
- Agents creating agents from templates (A9).
- Updating existing agents from a changed template; templates are copies (FR-006).
- A "secret" mark on environment variables; it would change 0001 (A6).
- Templates that pick a machine (0006).
- Creating agents during import; import saves templates only.
