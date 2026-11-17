---
status: draft
capability: office
plan:
supersedes:
---

# 0004: Meeting room

## Intent

The owner liked meeting rooms in other agent offices: he calls chosen agents together on one topic with a preset, and he gets a decision and a summary. This change adds that to the panel. The user picks a topic, the agents, and a preset. The system briefs each agent in its own terminal, collects the answers, asks one agent to write the result, and saves the whole meeting as a record. This change covers R8 of `docs/PRD.md` and settles open decision D-11.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4. R8 has the priority "Later"; this spec exists so that the owner can approve it before the work starts.
- This change depends on 0001 (agents, statuses, terminals, the desktop app) and on 0003 (the panel, from which the meeting room opens and in which the user opens an agent's terminal). It does not depend on 0002. References such as `0001/FR-014` and `0003/FR-004` point to requirements in those specs.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it.
- The requirements below describe behavior only. The plan decides how to build them.
- This spec is a draft. The decisions in Clarifications are reversible defaults that settle D-11; the owner approves or amends them with the spec.

## Terms

| Term | Meaning |
|---|---|
| Agent, status, quiet period, terminal, desktop app | As defined in 0001. |
| Panel | As defined in 0003. The meeting room opens from the panel and does not replace it. |
| Meeting | One run of a topic with a set of agents and a preset, from start to a saved record. |
| Participant | An agent that the user chose for the meeting. A meeting has 2 to 8 participants. |
| Facilitator | The one participant that writes the result. The user picks it at the start; the default is the first participant that he chose. |
| Preset | A fixed kind of meeting. It sets the instruction in the brief and the format that the facilitator uses for the result. The presets are listed below. |
| Brief | The text that the system sends to each participant: the topic, the user's optional extra instruction, and the preset's instruction. |
| Answer | The text that a participant's terminal produced after the brief, until the participant's status became `waiting for input` again, without color and cursor codes. |
| Meeting record | The saved result of a meeting (see Key entities). |
| Time limit | The longest time that a meeting waits for one participant's answer: 10 minutes. |

**Presets:**

| Preset | What the participants do | Format of the facilitator's result |
|---|---|---|
| Decision | Each participant states a recommendation on the topic with reasons. | Summary, decision (or "no decision" with the reason), action items. |
| Brainstorm | Each participant lists ideas on the topic. | Summary, the grouped ideas, action items. |
| Review | Each participant reviews the work that the topic names and lists findings. | Summary, findings by importance, action items. |
| Status round | Each participant reports what it did, what it does next, and what blocks it. | Summary, blockers, action items. |

## User stories

### US1: Hold a meeting and get a decision (Priority: P1)

The user picks agents, a preset, and a topic, and gets a saved record with each participant's answer and the facilitator's result.

**Why this priority:** this is R8. Without it, the meeting room has no value.

**Independent test:** with 3 agents that wait for input, start a "Decision" meeting with a topic. Each of the 3 agents receives the brief in its terminal, answers, and the facilitator writes the result. The panel shows a meeting record with 3 answers and the result.

**Scenarios:**

1. Given 3 agents have the status `waiting for input`, When the user picks them, the "Decision" preset, and a topic, and starts the meeting, Then each of the 3 terminals receives the brief.
2. Given every participant answered, When the last answer is complete, Then the system sends the facilitator all answers and the preset's result format, and the facilitator's reply is the result.
3. Given the facilitator replied, When the reply is complete, Then the panel shows the meeting record with the topic, the preset, each participant's answer, and the result.
4. Given one of the chosen agents has the status `working`, When the user starts the meeting, Then the system refuses, names that agent, and starts no meeting.
5. Given one participant produces no answer within the time limit, When the time limit ends, Then the meeting goes on without it, and the record marks it "no answer".

### US2: Follow and stop a running meeting (Priority: P2)

While a meeting runs, the user sees where each participant is, opens any participant's terminal, and can stop the meeting.

**Why this priority:** a meeting can take minutes, and the user needs to see that it works and be able to end it. It needs US1.

**Independent test:** start a meeting with 3 participants. The panel shows each participant's state as text. Open one participant's terminal in one step. Stop the meeting. The record is saved with the answers so far.

**Scenarios:**

1. Given a meeting runs, When the user looks at the meeting view, Then each participant shows one state: `briefed`, `answering`, `answered`, `no answer`, or `left`, as text.
2. Given a meeting runs, When the user opens a participant (0003/FR-004), Then that participant's live terminal opens in the same window.
3. Given a meeting runs, When the user stops it, Then no further input goes to any participant, and the system saves a record with the outcome `stopped` and the answers so far.
4. Given a participant exits or fails during the meeting, When its status changes, Then the meeting goes on and the record marks the participant `left`.

### US3: Find and reuse past meetings (Priority: P3)

The user opens the list of past meetings, reads one, and copies it as text.

**Why this priority:** the record only has value if the user finds it again, but the user can read each result in the first meeting view, so this can follow.

**Independent test:** hold 3 meetings. The list shows them newest first. Open the oldest and copy it. The copied text holds the topic, the preset, the answers, and the result.

**Scenarios:**

1. Given 3 meetings are saved, When the user opens the meeting list, Then the list shows each meeting's topic, preset, time, and outcome, newest first.
2. Given a meeting record, When the user opens it, Then the panel shows all of its fields.
3. Given a meeting record is open, When the user chooses the copy action, Then the record's text is on the clipboard as Markdown.
4. Given a participant of a saved meeting was deleted, When the user opens the record, Then the record still shows the participant's name and answer.

## Requirements

### ADDED

- **FR-001** (US1): WHEN the user starts a meeting with a topic, 2 to 8 participants, and one preset, THE SYSTEM SHALL send the brief to each participant's terminal as one unbroken input that no key of the user can split.
- **FR-002** (US1): THE SYSTEM SHALL offer the presets Decision, Brainstorm, Review, and Status round, each with the instruction and the result format in Terms.
- **FR-003** (US1): IF any chosen participant does not have the status `waiting for input`, THEN THE SYSTEM SHALL refuse to start the meeting and name each such participant and its status.
- **FR-004** (US1): WHEN every participant has answered or its time limit has ended, THE SYSTEM SHALL send the facilitator the answers and the preset's result format, and take the facilitator's answer as the result.
- **FR-005** (US1): WHEN the result is complete, THE SYSTEM SHALL save a meeting record with the topic, the preset, the participants, each answer as the terminal produced it, the result as the facilitator wrote it, the start and end times, and the outcome (`completed`, `partial`, or `stopped`).
- **FR-006** (US2): WHILE a meeting runs, THE SYSTEM SHALL show each participant's state as text, and open any participant's live terminal in one step.
- **FR-007** (US2): WHEN the user stops a meeting, THE SYSTEM SHALL send no further input for it, leave every agent running, and save the record with the outcome `stopped`.
- **FR-008** (US2): IF a participant exits or fails during a meeting, THEN THE SYSTEM SHALL go on with the other participants and mark that participant `left` in the record.
- **FR-009** (US3): THE SYSTEM SHALL list the meeting records newest first, open any record in the panel, and copy a record as Markdown text.

The outcome is `completed` when every participant answered and the facilitator replied, `partial` when at least one participant has `no answer` or `left`, and `stopped` when the user stopped the meeting.

### PRESERVED

- **PB-1:** The meeting sends input only to the participants' terminals, and writes no CLI configuration file and no project file (0001/FR-021).
- **PB-2:** office-harness sends no office data outside the user's machine; meeting records stay local (0001/FR-005). What an agent CLI does with the brief it receives is outside office-harness.
- **PB-3:** Every meeting-room action is available from the keyboard (0001/FR-028), and every state shows as text, never color alone (0001/FR-014).
- **PB-4:** Closing the desktop app keeps every agent and every running meeting going; the record is saved when the meeting ends (0001/FR-019).
- **PB-5:** The user can type into a participant's terminal at any time (0001/FR-017); the meeting does not lock it.

## User experience

**Flow:**

1. The user opens the meeting room from the panel.
2. The user enters a topic, optionally adds an extra instruction, picks 2 to 8 agents, picks one preset, and confirms the facilitator.
3. The user starts the meeting. If a participant is not `waiting for input`, the system says which one and why.
4. The meeting view shows each participant's state as text and an action to open its terminal. A stop action is always visible.
5. When the facilitator replies, the meeting view shows the record. The user can copy it.
6. The meeting list in the meeting room shows past records.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading, server not reachable, unauthorized | As in 0001 and 0003. | PB-3, 0003/PB-5 |
| Empty list | No meeting yet, with the action that starts one. | US3, Edge cases |
| Fewer than two agents | The meeting room says that a meeting needs 2 agents and offers the create action (0001). | Edge cases |
| Refused start | The agent names and statuses that block the start. | FR-003 |
| Running | The participants with their states, the time left for the answers, and the stop action. | FR-006 |
| Finished | The meeting record. | FR-005, US1 scenario 3 |
| Stopped or partial | The record with the outcome shown as text and the missing answers marked. | FR-005, FR-007, FR-008 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The agent picker, the preset picker, the participant states, and the record list work from the keyboard. Keyboard focus is always visible (PB-3).
- Participant states and the outcome are text, optionally with an icon, never color alone.
- A change of a participant's state or of the meeting's outcome is announced to screen readers.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Meeting | One run of a topic. | topic, extra instruction, preset, facilitator, start and end times, outcome, state while it runs | One per run. Has 2 to 8 participants. |
| Participant | An agent's seat in one meeting. | agent name (kept after the agent is deleted), state, answer | Belongs to one meeting. Refers to one agent while it exists. |
| Meeting record | The saved result. | all attributes of the meeting, each answer, the result text | Is the meeting after it ended; one per meeting. |
| Preset | A fixed kind of meeting. | name, brief instruction, result format | Four fixed presets. |

## Success criteria

- **SC-001:** With 5 agents that wait for input, a user starts a meeting (topic, agents, preset, start) in under 60 seconds.
- **SC-002:** After the user starts a meeting with 8 participants, all 8 terminals have the brief within 5 seconds.
- **SC-003:** With fake agents that print known answers, 100% of the saved answers equal the text that the terminals produced, and the result equals the facilitator's reply.
- **SC-004:** With one participant that never answers, the meeting ends with the outcome `partial` within the time limit plus 10 seconds.
- **SC-005:** After the user stops a meeting, no further input reaches any participant, and the record is saved within 2 seconds.
- **SC-006:** With 50 saved meetings, a user opens the meeting list, finds one by topic, and copies it within 30 seconds.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The topic is empty or only spaces. | Invalid | The system refuses to start and says that a topic is required. |
| Fewer than 2 or more than 8 agents are chosen. | Invalid | The system refuses to start and names the limit. |
| The facilitator is not one of the participants. | Invalid | Not possible; the picker offers only the participants. |
| The same agent is chosen twice. | Invalid | The picker lists each agent once. |
| An agent in another project is chosen. | Boundary | Allowed. A meeting may mix projects. |
| A participant is already in another running meeting. | Concurrent | The system refuses the second meeting for that agent and names the running one (FR-003 also holds: its status is `working`). |
| Two meetings with different agents run at the same time. | Concurrent | Both run; each has its own view and record. |
| The user types into a participant's terminal during the meeting. | Concurrent | The input goes to the agent (PB-5); the answer then holds that text as well. |
| A participant answers with no text. | Boundary | The record shows the answer as empty and the state as `answered`. |
| An answer is very long. | Boundary | The record keeps the answer up to the terminal output limit of 0001 and says where it was cut. |
| The facilitator exits, fails, or gives no reply within the time limit. | Failure | The meeting ends with the outcome `partial`, shows the answers, and says that no result was written. |
| The server restarts during a meeting. | Failure | The meeting ends with the outcome `partial`, and the record holds the answers so far. |
| A participant's agent is deleted during the meeting. | Failure | The participant is marked `left` (FR-008). |
| The desktop app is closed during the meeting. | Failure | The meeting goes on (PB-4); on the next start the panel shows the record. |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show. |
| An agent's CLI does not return to `waiting for input` after the brief. | Failure | The time limit ends the wait for that participant (US1 scenario 5). |

## Assumptions

- **A1:** The answer is the text that the participant's terminal produced after the brief until its status became `waiting for input` again, so a meeting works with any agent CLI that office-harness runs, without a CLI-specific protocol (0001 A1 sets the quiet period).
- **A2:** One round only: each participant answers once, and the facilitator writes the result. The MVP has no debate rounds and no agent-to-agent messages.
- **A3:** The facilitator uses the same terminal as before; its answer to the result request is the result. The system does not parse or check the result's structure: the record stores the text as written.
- **A4:** The presets are fixed. The user cannot create or edit presets in this change.
- **A5:** The time limit is 10 minutes per participant answer and is not changeable in this change.
- **A6:** The meeting list has no search in this change; it is a newest-first list (SC-006 counts on a list of 50).
- **A7:** Later releases stay open: R7 can show a meeting from the same records, and R5 can include agents on other machines in the same way.

## Clarifications

### Session 2026-11-17 (overnight directive)

Decisions with reversible defaults, so that the owner can approve the spec. They settle D-11 in `docs/PRD.md`.

- Q: Which presets? → A: Four fixed presets: Decision, Brainstorm, Review, and Status round (Terms). Reversible: a preset is one name, one instruction, and one result format; more can be added later.
- Q: Which output format? → A: One meeting record with the topic, preset, participants, every answer, and the facilitator's result, which has a summary, a decision or the preset's equivalent, and action items (Terms). The record copies as Markdown text. It does not write a file into a project folder (PB-1). Reversible: export to a file can be added later.
- Q: Who writes the result? → A: One participant, the facilitator, that the user picks (default: the first chosen). The system does not use a separate hidden agent. Reversible: a later change can add a built-in facilitator.
- Q: Can agents with the status `working` join? → A: No (FR-003). Their terminals take the user's or the CLI's work, and a brief would mix into it.
- Q: How many rounds? → A: One (A2).
- Q: Are presets user-defined? → A: No (A4).
- Q: Does a meeting need agents of one project? → A: No (Edge cases).

## Out of scope

- Agent-to-agent messages, debate rounds, and voting.
- User-defined presets and changing the time limit.
- Search across meeting records, and export to a file.
- A built-in facilitator that is not one of the user's agents.
- The R7 office scene, R5 distributed machines, and the agent hierarchy of 0002 (for example, inviting a whole team in one action).
- Scheduling meetings in advance or repeating them.
