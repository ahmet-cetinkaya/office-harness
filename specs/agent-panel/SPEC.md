# Agent panel

## Intent

One panel lists every agent of every project with its status, last output, and task line, and gives the shortcuts, the notifications, and the switcher around that list. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0003-panel-practicality](../changes/archive/0003-panel-practicality/SPEC.md)
- [0011-task-line](../changes/archive/0011-task-line/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0003/FR-002 | 0011 | Change 0003, Requirements |
| 0003/FR-006 (in part) | 0007, 0015, 0022 | Change 0003, Requirements |
| 0003/SC-004 | 0022 | Change 0003, Success criteria |
| 0003/FR-008 (in part) | 0005 | Change 0003, Requirements |
| 0003 Key entities, Notification | 0025 | Change 0003, Key entities |

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0022: [Idle sleep](../idle-sleep/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0005: [Office scene](../office-scene/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)
- 0025: [Scheduled directives](../scheduled-directives/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)
- 0020: [Temporary agents](../temporary-agents/SPEC.md)

## Change 0003: Panel practicality

### Intent

The owner named switching and searching between agents as his main problem: other tools make him walk through a scene or open many windows to find an agent and its result. This change makes the desktop app one panel in which he sees every agent of every project, opens any agent's terminal in one step, switches projects in one step, and gets a notification when an agent finishes or needs him. Opening the app shows the office where he left it. This change covers R1 and R9 of `docs/PRD.md`.

### Requirements

- **0003/FR-001** (US1): THE SYSTEM SHALL show the agent list, the agent terminals, and the agent details in one window, without a separate window per agent or per project.
- **0003/FR-002** (US1): THE SYSTEM SHALL show, in one list, the name, project, status, last output line, and task line of every agent in every project. (modified by 0011)
- **0003/FR-003** (US1): WHERE the office has a hierarchy from 0002, THE SYSTEM SHALL also show each agent's role and team in the agent list.
- **0003/FR-004** (US1, R9 criterion 1): WHEN the user clicks an agent in the panel or presses that agent's keyboard shortcut, THE SYSTEM SHALL open that agent's live terminal.
- **0003/FR-005** (US1, R9 criterion 3): WHEN the user picks one project or "All projects" in the project switcher, THE SYSTEM SHALL show only the agents of that choice.
- **0003/FR-006** (in part) (US3): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, is put on hold automatically (0007/FR-008), or sends a report for the person (0015/FR-007), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and while an unread report notification of that agent exists, a further report adds none; (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification, and exception (b) does not apply to the automatic hold or to a report, because the terminal does not show them; and (c) an unprompted change to `waiting for input` (0022 Terms) adds no notification and sends no OS notification. (modified by 0007, 0015, 0022)
- **0003/FR-007** (US2): IF a notification occurs while the desktop app is closed, THEN THE SYSTEM SHALL show it as unread when the app opens.
- **0003/FR-008** (US2): WHEN the user opens the desktop app, THE SYSTEM SHALL show the current state of the office with the last view, where the last view now also names the open view (panel or scene), and SHALL open that view. This changes steps 1 and 6 of the 0003 Flow, which now read "the open view" where they read "the panel", and 0003/SC-006, which now measures the open view. (modified by 0005)
- **0003/FR-009** (US2): WHEN the user opens a notification, or opens the terminal of an agent that has unread notifications, THE SYSTEM SHALL show those notifications of that agent as read.

FR-002 states R9 criterion 2. FR-006 notifies only for the four events it lists: a status change that the user caused (`stopped`) and `interrupted` after a server restart notify nobody.
Later additions to the shortcuts and the events (recorded 2026-11-23, ACF-106 archive; no requirement of this spec changed). The fixed shortcuts are `Ctrl+Shift+F1` to `Ctrl+Shift+F10` here. [Change 0008](../needs-you-queue/SPEC.md) added `Ctrl+Shift+F11` (jump to the agent that needs you), and change 0007 added `Ctrl+Shift+H` (the office hold). The server holds 12 reserved shortcuts in all (`src/modules/agent/domain/src/value_objects/settings.rs:33`). [Change 0007](../hold-and-limits/SPEC.md) (FR-008) added the notification event `auto_hold` to the four events of FR-006, so the server knows five (`src/modules/notification/domain/src/enums/notification_event.rs:21`). [Change 0008](../needs-you-queue/SPEC.md) added the Needs you queue. It is a separate queue with its own requests and event, not a notification event. Changed by 0017: the notification event `leftover_kept` is added; see `specs/changes/archive/0017-orphan-sweep/contracts/sweep-protocol.md`. Changed by 0020: the notification event `temp_ended` is added; see `specs/changes/archive/0020-temporary-agents/contracts/temp-protocol.md`.

This change modified: 0001/FR-017. The final text is under [Change 0001](../server-and-terminal/SPEC.md), Requirements.

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

## Change 0011: Task line per agent

### Intent

The PRD asks "what is each agent doing now?". The agent list of 0003 shows each agent's status and last output line, but not what the agent was asked to do. This change adds one short line per agent with the last task the office gave it: the text of the last directive it received, or a task text the user set, with how long ago it was given. The line comes only from text the office itself sent or the user typed, never from the CLI's output, so it is labelled as the task given, not as what the agent does now. This change amends the approved change 0003.

### Terms

| Term | Meaning |
|---|---|
| Agent, new task, action record | As defined in 0001. |
| Directive | As defined in 0002. |
| Agent list | As defined in 0003. |
| Task text | The last task the office gave an agent: the text of the last directive it received, or the text the user set. |
| Receives (a directive) | The office has entered the directive into the agent's terminal (0002/FR-010). A directive that the office refuses is not received. |
| Task line | The task text as shown in the agent list: its first line, shortened, with its age. |

### Requirements

- **0011/FR-001** (US1): THE SYSTEM SHALL show, for each agent with a task text, a task line with the first line of the task text, up to 80 characters followed by an ellipsis when shortened, and the time since the task text was set, labelled as the task given.
- **0011/FR-002** (US1): WHEN an agent receives a directive, THE SYSTEM SHALL set the agent's task text to the directive's text and its time to the time of the directive.
  - Amended by 0012 (`specs/changes/archive/0012-safe-directive-delivery/SPEC.md`): an agent receives a directive when the office submits it, text and Enter. A directive that waits in the queue, is withdrawn, or is left unsubmitted does not set the task text (0012 PB-3).
- **0011/FR-003** (US2): WHEN the user sets, edits, or clears an agent's task text in the agent's details, THE SYSTEM SHALL apply that text and record the change in the action record with the user as actor. IF the text has more than 8,192 characters, THEN THE SYSTEM SHALL refuse it and name the limit.
- **0011/FR-004** (US1): WHEN a new task starts on an agent, THE SYSTEM SHALL clear the agent's task text.
- **0011/FR-005** (US1): THE SYSTEM SHALL show the full task text in the agent's details.
- **0011/FR-006** (US1): THE SYSTEM SHALL show the task line and the task text without applying any control character or terminal escape code in them.
- **0011/FR-007** (US1): THE SYSTEM SHALL NOT change an agent's task text from the agent's terminal output or window title.

This change modified: 0003/FR-002 (the list now shows the task line: "THE SYSTEM SHALL show, in one list, the name, project, status, last output line, and task line of every agent in every project."). The final text is under Change 0003, Requirements.

Modified by later changes: none.

### Preserved behaviors

- **0011/PB-1:** A directive is entered into the target's terminal as before (0002/FR-010); the task text only shows it, and the office types nothing more into the CLI.
- **0011/PB-2:** A new task starts a new CLI session as before (0001/FR-024), and continuing a previous session (0001/FR-025) keeps the task text.
- **0011/PB-3:** The office reads no CLI prompt or title and writes nothing to a CLI's configuration (0001/FR-021).
- **0011/PB-4:** Status and the last output line show as before (0003/FR-002, 0001/FR-014), as text, never color alone.
- **0011/PB-5:** Every desktop app action that this change adds, including setting the task text, is available from the keyboard (0001/FR-028).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | task text, task text time, source (directive or user) | As in 0001. Kept across restarts (0001/FR-003). |

### Success criteria

- **0011/SC-001:** Within 2 seconds after a directive, the target's row shows the directive's first line, up to 80 characters, and its age.
- **0011/SC-002:** While an agent prints output and changes its window title for 5 minutes, its task line changes 0 times except for its age.
- **0011/SC-003:** A task text with newlines and terminal escape codes shows as one line in the row with 0 applied codes.
- **0011/SC-004:** After a new task starts, 100% of affected agents show no task line until the next directive or user edit.
- **0011/SC-005:** After a restart of the background server, 100% of task texts and their ages are the same as before.

### Notes from Deviations

- **Concurrency test size.** The test for two directives at once sends 3 directives, because the per-target limit is 3 per 60 seconds.
- **Action record label.** The action record shows the `task_text` kind as "Changed the task text", in the style of the other labels.
- **Control characters in `visibleText`.** The line feed is kept (it only breaks the line); `taskLine` cuts at it, and the details wrap it. The check for C0, DEL, and C1 uses the `\p{Cc}` property class, because the linter refuses control characters in a regular expression.
- **One order for a stored text and its event.** A shared `TaskOrder` orders a directive's stored text and its `agent.task` event against a user edit, so clients see the events in the order the texts were stored (`agent.task.set` and `agent.new_task` take it too). A row runs its one-minute age clock only while it has a task. An open task editor says so when the task text changes under it; the draft stays and Save still replaces the new text.
- **None of the notes defers or changes a requirement.**

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0003-panel-practicality - 2026-11-23
- 0011-task-line - 2026-11-28
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
