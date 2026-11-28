---
status: shipped
capability: agent-panel
plan: specs/changes/archive/0011-task-line/PLAN.md
supersedes: 0003/FR-002
---

<!-- Approved on 2026-11-19 through the review gate; shipped in PR #94. -->

# 0011: Task line per agent

## Intent

The PRD asks "what is each agent doing now?". The agent list of 0003 shows each agent's status and last output line, but not what the agent was asked to do. This change adds one short line per agent with the last task the office gave it: the text of the last directive it received, or a task text the user set, with how long ago it was given. The line comes only from text the office itself sent or the user typed, never from the CLI's output, so it is labelled as the task given, not as what the agent does now. This change amends the approved change 0003.

## Context

- Source of truth: `docs/PRD.md`.
- This change depends on 0001 (agents, new task, the action record), 0002 (directives, 0002/FR-010), and 0003 (the agent list, 0003/FR-002). References such as `0003/FR-002` point to requirements in those changes.
- A new task in 0001 starts a new CLI session and carries no text (0001/FR-024). The texts the office knows are the directives of 0002 and what the user types. The office does not read the CLI's prompts or title (0001 A1, 0003 A3).
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, new task, action record | As defined in 0001. |
| Directive | As defined in 0002. |
| Agent list | As defined in 0003. |
| Task text | The last task the office gave an agent: the text of the last directive it received, or the text the user set. |
| Receives (a directive) | The office has entered the directive into the agent's terminal (0002/FR-010). A directive that the office refuses is not received. |
| Task line | The task text as shown in the agent list: its first line, shortened, with its age. |

## User stories

### US1 — See the task each agent was given (Priority: P1)

The user sees, in the agent list, the last task each agent was given and how long ago, without opening its terminal.

**Why this priority:** it answers "what is each agent doing now?" for 30 agents at a glance, and it is the whole value of the change.

**Independent test:** let a lead send member M the directive "Fix the login test. Then report." M's row shows "Task given 0 min ago: Fix the login test. Then report." and the age grows. M prints output for one minute: the line does not change. Start a new task on M: the line is empty.

**Scenarios:**

1. Given lead L sends member M a directive, When the user looks at the agent list, Then M's row shows the directive's first line, up to 80 characters, and the time since it was given, labelled as the task given.
2. Given the task text has more than 80 characters in its first line or more than one line, When the user looks at the row, Then the row shows the first 80 characters of the first line with an ellipsis, and M's details show the full text.
3. Given M has a task text, When M's CLI prints output or changes its window title, Then the task line does not change.
4. Given M has a task text, When a new task starts on M, Then M's task line is empty.
5. Given a task text holds control characters or terminal escape codes (for example text the user pasted), When the user looks at the row and the details, Then they show the text without applying those codes.

### US2 — Set or correct the task text myself (Priority: P2)

The user sets or edits an agent's task text in its details, for agents that get their tasks by typing and not by directive.

**Why this priority:** agents without a parent never receive directives; without this, their line stays empty. It needs US1's line to show anything.

**Independent test:** in agent A's details, set the task text "Review PR 12". A's row shows it with age 0 min, and A's action record shows the edit with the user as actor.

**Scenarios:**

1. Given agent A has no task text, When the user sets "Review PR 12" in A's details, Then A's row shows "Task given 0 min ago: Review PR 12".
2. Given the user edits A's task text, When he opens A's action record, Then the edit is recorded with the user as actor.
3. Given the user clears A's task text, When he looks at the agent list, Then A's task line is empty.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL show, for each agent with a task text, a task line with the first line of the task text, up to 80 characters followed by an ellipsis when shortened, and the time since the task text was set, labelled as the task given.
- **FR-002** (US1): WHEN an agent receives a directive, THE SYSTEM SHALL set the agent's task text to the directive's text and its time to the time of the directive.
  - Amended by 0012 (`specs/changes/archive/0012-safe-directive-delivery/SPEC.md`): an agent receives a directive when the office submits it, text and Enter. A directive that waits in the queue, is withdrawn, or is left unsubmitted does not set the task text (0012 PB-3).
- **FR-003** (US2): WHEN the user sets, edits, or clears an agent's task text in the agent's details, THE SYSTEM SHALL apply that text and record the change in the action record with the user as actor. IF the text has more than 8,192 characters, THEN THE SYSTEM SHALL refuse it and name the limit.
- **FR-004** (US1): WHEN a new task starts on an agent, THE SYSTEM SHALL clear the agent's task text.
- **FR-005** (US1): THE SYSTEM SHALL show the full task text in the agent's details.
- **FR-006** (US1): THE SYSTEM SHALL show the task line and the task text without applying any control character or terminal escape code in them.
- **FR-007** (US1): THE SYSTEM SHALL NOT change an agent's task text from the agent's terminal output or window title.

### MODIFIED

- **0003/FR-002** (US1): THE SYSTEM SHALL show, in one list, the name, project, status, last output line, and task line of every agent in every project.

### PRESERVED

- **PB-1:** A directive is entered into the target's terminal as before (0002/FR-010); the task text only shows it, and the office types nothing more into the CLI.
- **PB-2:** A new task starts a new CLI session as before (0001/FR-024), and continuing a previous session (0001/FR-025) keeps the task text.
- **PB-3:** The office reads no CLI prompt or title and writes nothing to a CLI's configuration (0001/FR-021).
- **PB-4:** Status and the last output line show as before (0003/FR-002, 0001/FR-014), as text, never color alone.
- **PB-5:** Every desktop app action that this change adds, including setting the task text, is available from the keyboard (0001/FR-028).

## User experience

**Flow:**

1. A parent agent sends a directive. The target's row in the agent list shows "Task given 0 min ago:" and the directive's first line.
2. The user reads the list: each agent shows its status, last output line, and task line.
3. He opens an agent's details to read the full task text, or to set, edit, or clear it.
4. A new task on the agent clears its line.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| No task text | No task line in the row. | FR-004, US2 scenario 3 |
| Task line | "Task given N min ago:" and up to 80 characters. | FR-001, US1 scenario 1 |
| Shortened | The first 80 characters with an ellipsis; the full text in details. | FR-001, FR-005 |
| Details | The full task text with set, edit, and clear actions. | FR-003, FR-005 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- Set, edit, and clear work from the keyboard (PB-5). Keyboard focus is always visible.
- Screen readers read the task line as part of the row's description, after the name, project, and status that 0003 puts in its accessible name.
- The age is text, never color alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | task text, task text time, source (directive or user) | As in 0001. Kept across restarts (0001/FR-003). |

## Success criteria

- **SC-001:** Within 2 seconds after a directive, the target's row shows the directive's first line, up to 80 characters, and its age.
- **SC-002:** While an agent prints output and changes its window title for 5 minutes, its task line changes 0 times except for its age.
- **SC-003:** A task text with newlines and terminal escape codes shows as one line in the row with 0 applied codes.
- **SC-004:** After a new task starts, 100% of affected agents show no task line until the next directive or user edit.
- **SC-005:** After a restart of the background server, 100% of task texts and their ages are the same as before.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| An agent never got a directive and the user set nothing. | Empty | No task line (FR-001). |
| The user sets a task text that is empty or only spaces. | Empty | The task text is cleared; no line shows. |
| A user task text of exactly 8,192 characters is accepted; 8,193 is refused. | Boundary | The refusal names the limit (FR-003). |
| The first line is exactly 80 characters. | Boundary | Shown whole, without an ellipsis (FR-001). |
| A task text holds escape codes or control characters (for example text the user pasted). | Invalid | Shown as text without applying them (FR-006). |
| Two directives arrive at the same moment. | Concurrent | The task text is the one the office entered into the terminal last. |
| A directive that the office refuses (for example under 0007 or 0002/FR-009, or blank text or text with control characters, 0002 as built). | Invalid | The task text does not change; only directives the agent received set it. |
| The user edits the task text while a directive arrives. | Concurrent | The later of the two is kept; both are in the action record. |
| The agent is deleted. | Boundary | Its task text goes with it. |
| The task text holds sensitive text and the screen is shared. | Failure | The office shows it like any other row text; hiding it is out of scope. |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0003 PB-5). |

## Assumptions

- **A1:** The age shows in whole minutes ("0 min ago", "12 min ago") and in hours past 60 minutes ("2 h ago").
- **A2:** Setting the task text in details only labels the agent; it does not type the text into the CLI. To send work to an agent, the user types into its terminal or a parent sends a directive.
- **A3:** The line counts characters as the user sees them, so a wide character or an emoji counts as one.
- **A4:** Directives are already in the action record (0002/FR-011), so FR-002 adds no record entry of its own.

## Clarifications

### Session 2026-11-19 (review edits)

- Q: Can a directive be multi-line, blank, or hold escape codes? → A: No; 0002 as built refuses those directives, so the multi-line and escape-code cases come from the user's own task text (US1 scenarios 2 and 5, SC-003, Edge cases).
- Q: Is the user's task text limited in length? → A: Yes, to 8,192 characters, the same bound as a directive (FR-003).
- Q: Does the office scene show the task line? → A: No; the desks keep name, status, and last output line (Out of scope).
- Q: When does an agent receive a directive? → A: When the office has entered it into the agent's terminal; a refused directive is not received (Terms).
- Q: Does starting a stopped agent (`agent.start`, which also starts a new session) clear the task text? → A: No; only the new-task action (`agent.new_task`) clears it, so a text set on a stopped agent survives its first start (FR-004). Decided under the owner's spec delegation, 2026-11-19.

## Out of scope

- Reading the task from the CLI's output, prompt, or window title (FR-007).
- Showing the task line in the needs-you queue (0008, draft); a later amendment can add it.
- A task history per agent beyond the action record.
- Hiding or blurring task text on a shared screen.
- Giving the new-task action a text that the office types into the CLI.
- Showing the task line on the office scene's desks (0005); the desks keep name, status, and last output line.
