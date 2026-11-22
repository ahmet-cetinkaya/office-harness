---
status: draft
capability: office
plan: specs/changes/0015-upward-reports/PLAN.md
supersedes: 0002/FR-008 (as modified by 0014), 0003/FR-006 (as modified by 0007), 0008/FR-002, 0008/FR-003, 0008/FR-005
---

<!-- Draft for the spec review gate. Only the owner's approval, given through the review gate, changes the status. -->

# 0015: Upward reports

## Intent

Directives go down the tree (0002), and a parent can read its descendants' state (0014), but nothing comes back up as words. A lead that briefed five members learns which one finished, or which one is stuck, only by guessing from a status, and the CEO cannot ask the owner for a decision except by going quiet. This change lets a running agent send a short report to its own parent: that it is done, that it has a question, that it is blocked, or a piece of information. The office types the report into the parent's terminal through the safe delivery of 0012, so a report never presses Enter into a question meant for a person. A report from the CEO, from an agent with no parent, or one that the sender marks for the person, goes to the owner instead: it joins Needs you (0008). The parent answers with a directive (0002); the owner answers in the agent's terminal. The idea comes from an MIT-licensed reference office app; see the pull request.

## Context

- Source of truth: `docs/PRD.md`, R2 (deep hierarchy) and R9 (find the agent that needs the user).
- This change depends on 0001 (agents, statuses, terminals, the action record), 0002 (the tree, parents, the agent's calls from inside its session, directives), 0003 (notifications), 0007 (holds and rate limits), 0008 (Needs you), 0012 (safe directive delivery), 0013 (the resume hold, an automatic hold under 0003/FR-006), and 0014 (the visible form of control characters). References such as `0002/FR-008` point to requirements in those changes.
- 0002 A2 says that the MVP has no reply channel and that a parent reads a child's result from its terminal or its files. This change adds a report channel upward. It does not add a channel between siblings or a reply object: a parent still answers with a directive.
- 0012 is approved and built on its own branch, and not yet on `main` when this spec is written. The delivery rules of this change are the 0012 rules, applied to a report as to a directive.
- The office does not read a CLI's prompts (0001 A1, 0003 A3). The report's kind is what the sender says, not what the office saw.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, status, terminal, action record | As defined in 0001. |
| Parent, role, directive | As defined in 0002. The parent of an agent is the agent directly above it in the tree. |
| Notification | As defined in 0003. |
| Agent-made action, office hold, agent hold, rate limit, automatic hold | As defined in 0007. This change adds sending a report to the agent-made actions (FR-006). |
| Needs you, item, attention event, jump shortcut | As defined in 0008. |
| Queue, typed, submitted, waiting, unsubmitted, withdrawn | As defined in 0012. |
| Visible form | A text in which each control character, escape code, and bidirectional format character is replaced by a visible symbol, as 0014 defines it for names and task lines. |
| Report | A short text that a running agent sends upward from inside its session, with one kind: `done`, `question`, `blocked`, or `info`. |
| Sender | The agent that sends a report. |
| Report for the person | A report that goes to the owner through Needs you, not to an agent: a report from an agent without a parent, or one the sender marks for the person. |
| Report heading | The fixed line that the office puts before a report's text: the kind, the sender's name, and the sender's id. |

## Data flow and trust boundary

- **Boundary:** the agent's connection to the office (0002). The office knows the sender from the call credential of its running session and fixes the sender as the actor. The request carries the kind, the text, and one yes-or-no choice: for the person or not. It names no agent.
- **Into the office:** the kind (one of four words), the text (up to 1,000 characters), and the choice.
- **Out of the office, to the parent's terminal:** the report heading and the report text in visible form, as one line, typed under the rules of 0012.
- **Out of the office, to the owner:** a Needs you item with the sender's name, the kind, and the text in visible form, and one notification with the sender's name and the kind.
- **Into the action record:** the kind, the text in visible form, the sender as the actor, and the parent when there is one.
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent sends a report to a sibling, an ancestor above its parent, or another branch. | The request cannot name an agent; the office picks the parent from the tree at the moment of the report (FR-002, SC-002). |
  | A report's Enter answers a question that the parent's CLI shows to a person. | The report is typed only under the 0012 rules: queue, readiness, question check, and the input-line check before Enter (FR-004, SC-006). |
  | A control character, an escape code, or a line break in the text acts on the parent's terminal, submits part of the text early, or changes the owner's view. | The office turns the text into its visible form once, when it accepts the report, and uses only that form afterwards (FR-003, SC-004). |
  | A bidirectional format character reorders the text the parent or the owner reads. | Shown as a visible symbol (FR-003, SC-004). |
  | A long report floods a parent's context or the owner's list. | 1,000 characters at most; a longer one is refused (FR-001, SC-005). |
  | A looping agent floods its parent or Needs you with reports. | A rate limit of its own, the holds, and the automatic hold of 0007 apply (FR-006, SC-007); Needs you keeps one item per agent (0008), and a further report adds no notification while one is unread (0003/FR-006 as modified). |
  | A parent's children fill its queue with reports, so directives from the parent's own parent are refused as "queue full". | One waiting report per sender, and at most 10 waiting reports per queue, so at least 10 of the 20 places stay for directives (FR-010, SC-011). |
  | A report's text pretends to be the office, for example by repeating a report heading. | The heading comes first on the line and the text follows it on the same line, so a fake heading can only follow the real one. The parent treats the text as data (Residual risk). |
  | A child's report steers its parent (prompt injection). | Not prevented: a report is text from a child by design. The heading names the kind and the sender, so the parent can tell a report from a directive of its own parent (Residual risk). |

- **Residual risk:** an agent CLI runs as the user's OS user and can act outside the office; the parent rule prevents mistakes, not a hostile agent (0002 A8). A parent's CLI reads a report as input; the office marks where it comes from but cannot make the CLI treat it as data. A fake heading inside the text is shown after the real one, not removed.

## User stories

### US1 — A member tells its lead that it is done (Priority: P1)

A running member sends a report to its lead: done, a question, blocked, or a piece of information. The report appears in the lead's terminal as one line that says who sent it and what kind it is, typed safely. The lead answers with a directive to the member, using the id that the report names.

**Why this priority:** this is the gap the change closes. Without it a lead learns that a member finished only by guessing, and the owner relays results by hand.

**Independent test:** build a tree of the CEO, lead L1 with members M1 and M2, and lead L2 with member M3. M1 sends a `done` report. L1's terminal shows one line with the heading, the kind `done`, M1's name and id, and the text; no other terminal shows it. L1 sends a directive to M1's id; M1's terminal shows it.

**Scenarios:**

1. Given M1 runs under lead L1, and L1 waits for input with no question on its screen, When M1 sends a `done` report, Then the office types the heading and the text into L1's terminal as one line, presses Enter after the text shows at L1's input line, and answers M1 that the report was submitted.
2. Given M1 sends a report, When the office delivers it, Then no terminal other than L1's receives it, neither M2's, L2's, M3's, nor the CEO's.
3. Given L1 works, When M1 sends a report, Then the report waits in L1's queue and M1's answer says it waits, with its place.
4. Given L1's screen shows a permission question, When M1 sends a report, Then the office types nothing into L1's terminal while the question shows.
5. Given L1 received M1's report, When L1 sends a directive to the id that the report heading names, Then the directive reaches M1 as any directive does.
6. Given L1 is stopped, When M1 sends a report to its parent, Then the office refuses it, says that the parent is not running, and records the refusal.
7. Given lead L1 runs under the CEO, When L1 sends a report, Then it reaches the CEO's terminal.

### US2 — The CEO, or any agent, reaches the owner through Needs you (Priority: P1)

The CEO has no parent, so its reports go to the owner. Any other agent can mark a report for the person, for example a question only the owner can answer. Such a report joins Needs you with its kind and text, notifies the owner once, and stays there until the owner opens the agent's terminal or dismisses it. The owner answers in the agent's terminal.

**Why this priority:** the CEO cannot ask the owner anything today except by going quiet, and a quiet terminal says nothing about what it needs.

**Independent test:** the CEO sends a `question` report. Needs you shows the CEO's item with the kind and the text, and the window title count grows by one. The CEO keeps working and then goes quiet; the item still shows the question. The owner presses the jump shortcut: the CEO's terminal opens and the item leaves.

**Scenarios:**

1. Given the CEO runs, When it sends a `question` report, Then Needs you shows an item for the CEO with the kind and the text in visible form, and the owner gets one notification.
2. Given member M1 runs under L1, When M1 sends a `question` report marked for the person, Then Needs you shows M1's item with the question, and L1's terminal receives nothing.
3. Given the CEO's item holds a report, When the CEO changes to `working` and later to `waiting for input`, Then the item stays and still shows the report.
4. Given the CEO's item holds a report, When the owner opens the CEO's terminal by any route or dismisses the item, Then the item leaves Needs you.
5. Given the CEO's item holds a report, When the CEO sends a second report, Then the item keeps its wait time and shows the second report.
6. Given an agent has no parent (the CEO, an agent outside the hierarchy, or a lead in an office without a CEO), When it sends a report, Then the report goes to the person.
7. Given the desktop app is closed, When the CEO sends a report, Then the item shows in Needs you when the app opens, as 0008/FR-006 requires.

### US3 — Reports are bounded, safe, and on the record (Priority: P1)

A report is short, its control and bidirectional characters show as visible symbols, holds and rate limits apply to it as to any agent-made action, and each report and each refusal is in the action record.

**Why this priority:** without the bounds a report is a new way to flood a parent or the owner, or to inject terminal codes. It ships with US1 and US2.

**Independent test:** M1 sends a report whose text holds an escape character, a right-to-left override, and a line break. L1's terminal, the action record, and (for a report for the person) Needs you show each of them as a visible symbol. M1 sends 6 reports within 60 seconds: the 6th is refused with the name of the limit, and the refusal is in M1's record.

**Scenarios:**

1. Given a report text of 1,001 characters, When the sender sends it, Then the office refuses it with the limit in the reason and records the refusal.
2. Given a blank report text, When the sender sends it, Then the office refuses it and records the refusal.
3. Given the office is on hold, When an agent sends a report, Then the office refuses it with the reason that the office is on hold, and records the refusal.
4. Given an agent hold covers the sender, When it sends a report, Then the office refuses it with the reason that the agent is on hold.
5. Given the sender sent 5 reports in the last 60 seconds, When it sends a 6th, Then the office refuses it with the limit and its value; the third such refusal within 60 seconds puts the sender on hold automatically (0007/FR-008).
6. Given the office accepts a report to a parent, When the user looks at the action records, Then the parent's record holds the report with the sender as the actor, and a later withdrawal or unsubmitted delivery shows in the sender's and the parent's records as 0012 requires for a directive.
7. Given the office accepts a report for the person, When the user looks at the sender's action record, Then it holds the report with the sender as the actor.

## Requirements

### ADDED

- **FR-001** (US1, US2, US3): WHEN a running agent sends a report with a kind of `done`, `question`, `blocked`, or `info` and a text of 1 to 1,000 characters that is not blank, THE SYSTEM SHALL accept it; otherwise THE SYSTEM SHALL refuse it, tell the sender the reason, and record the refusal.
- **FR-002** (US1, US2): THE SYSTEM SHALL send an accepted report to the sender's parent as the tree is at the moment of the report, or to the person when the sender has no parent or marks the report for the person, and SHALL NOT take any input that names another agent as the report's receiver.
- **FR-003** (US3): WHEN the system accepts a report, THE SYSTEM SHALL replace each control character (C0, DEL, C1, including line breaks), escape code, and bidirectional format character in the text with its visible form, and SHALL use only the visible form in the parent's terminal, in Needs you, and in the action record.
- **FR-004** (US1): WHEN a report goes to a parent, THE SYSTEM SHALL type the report heading and the report text into the parent's terminal as one line, in the parent's queue together with its directives, under the rules of 0012/FR-001 to 0012/FR-005 as they apply to a directive whose target is the parent, and SHALL answer the sender that the report was submitted, waits with its place in the queue, or was left unsubmitted.
- **FR-005** (US1): IF the sender's parent is not running, THEN THE SYSTEM SHALL refuse a report to it, say that the parent is not running, and record the refusal; IF a waiting report's sender is no longer a child of the report's parent, THEN THE SYSTEM SHALL withdraw it as 0012/FR-005 withdraws a directive whose target left the sender's subtree.
- **FR-006** (US3): THE SYSTEM SHALL treat sending a report as an agent-made action under 0007: the office hold and an agent hold that covers the sender refuse it (0007/FR-002, 0007/FR-004), and a rate limit "reports sent by one agent", 5 per rolling 60 seconds by default, refuses a report beyond it (0007/FR-007), counts toward the automatic hold (0007/FR-008), and is changed by the user in the office settings like the other limits (0007/FR-009).
- **FR-007** (US2): WHEN the system accepts a report for the person, THE SYSTEM SHALL record it as an attention event of the sender, notify the owner with the sender's name and the report's kind as 0003/FR-006 (as modified) allows, and answer the sender that the report is in Needs you.
- **FR-008** (US2): WHILE an item in Needs you holds a report, THE SYSTEM SHALL keep showing that report on the item when later attention events change the item's latest event, and SHALL replace it only with the sender's next report for the person.
- **FR-009** (US1, US2, US3): THE SYSTEM SHALL add each accepted report to the action record with the sender as the actor, the kind, and the text in visible form, under the parent for a report to a parent and under the sender for a report for the person, and SHALL record each refusal in the sender's action record and, when the report had a parent, in the parent's.
- **FR-010** (US1): WHILE reports wait in a parent's queue, THE SYSTEM SHALL keep at most one waiting report per sender, replacing a sender's waiting report with its newer report in the same place and recording the replaced one as withdrawn, and SHALL keep at most 10 waiting reports per queue, refusing a further report with the reason that the parent's queue has no room for reports, and SHALL NOT count such a refusal toward any rate limit or automatic hold.
### MODIFIED

- **0002/FR-008** (as modified by 0014) (US1): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that 0002/FR-001 allows, and send reports only to its own parent or to the person.
- **0003/FR-006** (as modified by 0007) (US2): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, is put on hold automatically (0007/FR-008), or sends a report for the person (0015/FR-007), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and while an unread report notification of that agent exists, a further report adds none, and (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification. Exception (b) does not apply to the automatic hold or to a report, because the terminal does not show them.
- **0008/FR-002** (US2): THE SYSTEM SHALL show each item with the agent's name, project, latest event as the office saw it (quiet for the quiet period, bell, exit with its exit code, put on hold by the office with the limit it hit, or a report with its kind), and wait time, as text, and, while the item holds a report, the report's kind and its whole text in visible form.
- **0008/FR-003** (US2): THE SYSTEM SHALL order the items in groups by their latest event, `failed` first, then `waiting for input`, bell, automatic hold (the resume hold of 0013 included), and report, then `exited`, and inside each group by wait time, longest first, and items with the same wait time by agent name.
- **0008/FR-005** (US2): WHEN an agent with an item is deleted or changes to `stopped`, or, for an item that holds no report, changes to `starting` or `working`, or, for an item that holds no report and whose latest event is an automatic hold (the resume hold of 0013 included), has its hold released, THE SYSTEM SHALL remove that agent's item from Needs you.

### PRESERVED

- **PB-1:** The 0002 tree rules stay: directives go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007). A report goes up one level only, or to the person.
- **PB-2:** A directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals. Reports share the queue and do not jump it; they take at most 10 of its places (FR-010), so directives always keep at least 10.
- **PB-3:** The task line shows the last submitted directive (0011/FR-002, 0012 PB-3). A report changes no task text, neither the sender's nor the parent's.
- **PB-4:** Needs you keeps one item per agent, its order inside a group, the jump shortcut, the count, and dismiss (0008). An item without a report behaves as before.
- **PB-5:** The office reads no CLI prompt and writes nothing into a CLI's configuration (0001/FR-021).
- **PB-6:** The existing rate limits, their defaults, and their settings stay as they are (0007 A3); the reports limit is a new one beside them.
- **PB-7:** The agent's connection to the office still gets no events and no terminals (0002); a report's answer is the only thing it returns.

## User experience

**Flow (agent to parent):**

1. A member finishes its task and sends a `done` report with a one-line summary.
2. The office checks it (running sender, kind, length, holds, limit), turns the text into its visible form, and puts it in the lead's queue.
3. When the lead waits for input with no question showing and nobody typing, the office types one line: the heading, then the text, and presses Enter once the line shows.
4. The member's answer says submitted, waiting with its place, or unsubmitted.
5. The lead reads the line and answers with a directive to the member's id, or sends its own report to the CEO.

**Flow (agent to the owner):**

1. The CEO sends a `question` report.
2. Needs you shows the CEO's item with "question" and the text; the owner gets one notification; the window title count grows.
3. The owner presses the jump shortcut or picks the item; the CEO's terminal opens, and the item leaves.
4. The owner answers in the CEO's terminal.

**Visible states:**

| State | What the sender, the parent, or the owner sees | Covered by |
|---|---|---|
| Submitted | The sender's answer says submitted; the parent's terminal shows the line. | FR-004 |
| Waiting | The sender's answer says waiting, with the place in the parent's queue. | FR-004 |
| Unsubmitted | The answer or the action records say unsubmitted; the line stays in the parent's terminal. | FR-004 |
| In Needs you | The sender's answer says so; the owner sees the item with the kind and the text. | FR-007, 0008/FR-002 |
| Refused | The sender's answer gives the reason; the refusal is in the record. | FR-001, FR-005, FR-006, FR-009 |
| Withdrawn | The sender's and the parent's records show the text and the reason. | FR-004, FR-005 |
| Replaced | The sender's newer report waits in the older one's place; the records show the older one as withdrawn because it was replaced. | FR-010 |
| No room for reports | The sender's answer gives the reason; the refusal is in the record and counts toward no limit. | FR-010 |
| Needs you item with a long report | The item shows the first lines of the text and a control that shows the whole text. | 0008/FR-002 |

**Design:** no new screen. Needs you gets the report's kind and text on an item; the plan must not add other visuals without a design that the owner approves.

**Accessibility:**

- The report's kind and text are text, never color or an icon alone (0001/FR-014).
- An item's accessible name keeps the 0008 parts (name, project, latest event, wait time); the report text is the item's accessible description, so a screen reader reads the name first and the text on request.
- The control that shows a long text works from the keyboard, has a visible focus, and says whether the text is shown in full.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text).
- A new report item is announced only through its 0003 notification and the count, as 0008 requires.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Report | One report an agent sent. | sender, kind, text in visible form, time sent, receiver (a parent or the person) | A report to a parent waits in the parent's queue like a directive until it is typed or withdrawn (0012). A report for the person lives on the sender's Needs you item. Both are in the action record. |
| Item (0008), extended | One agent that waits for the owner. | as in 0008, plus the latest report for the person (kind, text, time), if any | One per agent (0008). |

## Success criteria

- **SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each member and each lead sends one report. 100% of the reports reach exactly the sender's parent's terminal, and 0 reach any other terminal.
- **SC-002:** In 10 attempts to point a report at another agent, by a field naming an agent, a team, or a branch, 100% are refused and recorded, and 0 reach any terminal or Needs you.
- **SC-003:** A report for the person shows in Needs you, with its kind and text, and the window title count changes, within 2 seconds after the agent sends it.
- **SC-004:** A report text that holds U+001B, U+009B, U+202E, and a line feed: 0 of these characters reach the parent's terminal, Needs you, or the action record as themselves, and the parent's terminal receives the report as one line.
- **SC-005:** A report of 1,000 characters is accepted; one of 1,001 characters and a blank one are refused and recorded (2 of 2).
- **SC-006:** Of 10 reports sent to a parent whose screen shows a permission question, 0 characters are typed into the parent's terminal while the question shows.
- **SC-007:** Under an office hold, 100% of 5 reports are refused with the hold reason. With no hold, the 6th report within 60 seconds is refused with the limit, and after the third such refusal the sender is on hold automatically.
- **SC-008:** A lead that received a member's report sends a directive to the id in the report heading, and the member's terminal receives it (1 of 1).
- **SC-009:** A report item stays in Needs you while its agent changes to `working` and back to `waiting for input`, and leaves when the owner opens the agent's terminal (1 of 1, and 1 of 1 for dismiss).
- **SC-010:** 100% of accepted, refused, withdrawn, and unsubmitted reports have an entry in the action records that FR-009 names.
- **SC-011:** With 10 reports from 10 children waiting in a working lead's queue, a directive from the CEO to that lead is accepted (1 of 1), an 11th report from another child is refused, and a second report from a child whose report waits leaves exactly one waiting report of that child.
- **SC-012:** 5 reports for the person from one agent, with the first notification unread, add exactly 1 notification, and the item shows the 5th report.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The sender has no parent: the CEO, an agent outside the hierarchy, or a lead in an office without a CEO. | Boundary | The report goes to the person (FR-002). |
| The CEO marks a report for its parent. | Boundary | The CEO has no parent; the report goes to the person (FR-002). |
| The sender's parent is stopped, exited, failed, interrupted, or never started. | Failure | The report is refused, with the reason that the parent is not running; the sender can mark it for the person (FR-005). |
| The request names an agent, a team, a target, or any field other than the kind, the text, and the person choice. | Invalid | The office refuses it, tells the sender, and records the refusal (FR-002). |
| The request names an actor or a caller. | Unauthorized | Refused and recorded, as for every agent request (0002). |
| The kind is not one of the four words. | Invalid | Refused and recorded (FR-001). |
| The text is blank, or longer than 1,000 characters. | Invalid | Refused and recorded with the reason (FR-001). |
| The text holds line breaks, escape codes, C1 controls, or bidirectional format characters. | Boundary | Each shows as a visible symbol everywhere, and the parent's terminal receives one line (FR-003). |
| The text holds a fake report heading. | Boundary | It shows after the real heading on the same line (Residual risk). |
| The sender sends a second report while its first waits in the parent's queue. | Boundary | The newer report takes the first one's place; the first is withdrawn and recorded as replaced (FR-010). |
| 10 reports from different children already wait in the parent's queue. | Boundary | A further report is refused with the reason that the parent's queue has no room for reports; the refusal counts toward no rate limit or automatic hold (FR-010). A directive to the parent is still accepted. |
| The parent's queue holds 20 entries, at least 10 of them directives. | Boundary | A report or a directive is refused with the reason that the queue is full; the refusal counts toward no rate limit (0012/FR-004, 0012 A6). |
| The parent shows a question, works, or a person types in its terminal. | Concurrent | The report waits in the queue (0012/FR-002). |
| A waiting report is not typed within 10 minutes, the parent stops, a hold starts, or the server stops. | Failure | It is withdrawn and recorded in the sender's and the parent's records (0012/FR-005). |
| The sender is moved under another parent, or deleted, while its report waits. | Concurrent | The report is withdrawn and recorded (FR-005). |
| A report for the person arrives while the sender has an item for `waiting for input`. | Concurrent | The item keeps its wait time, its latest event becomes the report, and it shows the report (0008/FR-001, FR-008). |
| A second report for the person arrives. | Boundary | The item shows the second one and keeps its wait time; the first stays in the action record (FR-008). |
| The agent goes back to `working` after its report for the person. | Concurrent | The item stays (0008/FR-005 as modified). |
| The agent stops or is deleted while its item holds a report. | Concurrent | The item leaves (0008/FR-005). |
| The owner has the agent's terminal open in the focused window when the report arrives. | Boundary | The item and the notification still appear; the terminal does not show the report (0003/FR-006 as modified). |
| An agent sends a second report for the person while its first report notification is unread. | Boundary | The item shows the second report; no second notification is added (0003/FR-006 as modified). |
| The report text is long. | Boundary | Needs you shows its first lines and a control that shows the whole text (0008/FR-002 as modified). |
| The sender's session ends while its report waits in the parent's queue. | Concurrent | The report stays in the queue; the sender's end does not withdraw it, because the parent can still read it. |
| The background server crashes while reports wait. | Failure | They are lost, as directives are (0012 A4); their acceptance stays in the record. |
| The desktop app sends a report. | Invalid | The office does not offer reports to the desktop app; it is the owner's own client. |

## Assumptions

- **A1:** The four kinds are words the sender picks. The office does not check that a `done` report means the work is done; it only shows the word. The kinds help the parent and the owner sort what they read.
- **A2:** 1,000 characters, counted as the text stores them (Unicode scalar values, as 0014 A9), is enough for a summary, a question, or the reason for a block. Longer results go into files that the parent reads (0002 A2). The limit is fixed in this change, not a setting.
- **A3:** The visible form replaces a line break, too, so a report is always one line in the parent's terminal and cannot submit part of itself early in a CLI that does not mark pastes (0012 A10). The visible form is the one of 0014, used for names and task lines.
- **A4:** The report heading is `[report from <name> (<agent id>): <kind>]`, followed by one space and the text. The name is in visible form. The full agent id is there so that the parent can answer with a directive (0002). The contract fixes the exact characters.
- **A5:** The default rate limit of 5 reports per 60 seconds lets an agent report each step of a short task and stops a loop. It does not count toward the directive limits, and the directive limits do not count reports: a lead whose eight members finish at once gets eight reports, which wait in its queue (0012).
- **A6:** A report for the person is an attention event of the sender (0008), so it sorts with `waiting for input` and keeps the 0008 wait-time rule. Its item stays through status changes to `starting` and `working`, because the owner has not read it yet; it leaves when the owner opens the terminal or dismisses it, or the agent stops or is deleted.
- **A7:** Any agent may mark a report for the person, not only the CEO. A member that needs a decision that only the owner can make asks the owner directly; its lead learns of it only through the action record. Limiting this to the CEO was considered and not chosen: a member's question would then have to pass through every level, each of which can only relay it.
- **A8:** The owner answers a report for the person by typing in the agent's terminal, which the jump shortcut opens. There is no answer form in Needs you.
- **A9:** A parent answers a report with a directive to the sender's id, under the rules of 0002 and 0012. A report has no id of its own and no thread; the heading's agent id is the reply address.
- **A10:** Like the 0002 descendant rule, the parent rule prevents mistakes. It is not a security boundary against a hostile agent that runs as the same OS user (0002 A8).
- **A11:** The action record keeps the report's text in visible form, as the office accepted it. The text as the sender wrote it is not stored.

## Clarifications

### Session 2026-11-23 (drafted under the owner's delegation)

- Q: Can a sender pick the receiver? → A: No agent: the parent comes from the tree (FR-002). The only choice is "for the person" (A7).
- Q: Which reports join Needs you? → A: Reports from an agent with no parent, the CEO among them, and reports marked for the person (FR-002, FR-007). To be confirmed at the spec review gate.
- Q: How does a parent answer? → A: With a directive to the sender's id from the heading; there is no reply request (A9).
- Q: Are control characters refused, as in a directive? → A: No; they are shown as visible symbols, line breaks included, so a report never fails for a stray character and never acts on a terminal (FR-003, A3).
- Q: Do holds and rate limits apply? → A: Yes; a report is an agent-made action with its own limit (FR-006, A5).

### Session 2026-11-23 (spec review r1)

- Q: Can a parent's children lock its queue against directives from above? → A: No; one waiting report per sender (a newer one replaces it, recorded) and at most 10 waiting reports per queue, so directives keep at least 10 of the 20 places (FR-010, SC-011).
- Q: Does each report for the person notify? → A: Only while no report notification of that agent is unread, like the bell rule (0003/FR-006 as modified, SC-012).
- Q: Does a report survive the release of a resume hold (0013)? → A: Yes; the resume hold is an automatic hold, and an item that holds a report does not leave on a hold release (0008/FR-005 as modified).
- Judgment calls A3, A5, and A7 were agreed at the review gate.

## Out of scope

- Reports to siblings, to ancestors above the parent, to another branch, or to several receivers (FR-002).
- Reports down the tree; a parent sends directives (0002).
- A report id, threads, or a reply request; a parent answers with a directive (A9).
- An answer form for the owner in Needs you (A8).
- A history view of reports beyond the action record and the parent's terminal.
- Attachments and files in a report; the parent reads files the sender wrote (0002 A2).
- Keeping waiting reports across a server crash (0012 A4).
- Settings for the 1,000-character limit (A2).
