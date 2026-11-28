# Agent state and reports in the hierarchy

## Intent

A parent reads the state of its descendants, and a member sends reports up to its parent and to the person. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0014-subtree-status](../changes/archive/0014-subtree-status/SPEC.md)
- [0015-upward-reports](../changes/archive/0015-upward-reports/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0015/FR-005 | 0020 | Change 0015, Requirements |

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003, 0011: [Agent panel](../agent-panel/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)
- 0020: [Temporary agents](../temporary-agents/SPEC.md)

## Change 0014: A parent reads the state of its descendants

### Intent

A parent agent sends directives down the tree (0002), but it cannot see what its team does. This change lets a running agent read the state of its own descendants from inside its session: name, place in the tree, status, last output time, Needs you, hold, and task line. The agent reads state, never terminal content, and only below itself. This change amends the approved 0002.

### Requirements

- **0014/FR-001** (US1): WHEN a running agent reads its subtree, THE SYSTEM SHALL answer with one entry for each of its descendants, each entry with the descendant's name, parent, role, status, last output time, whether it has an item in Needs you, whether an office hold or an agent hold covers it under the rules of 0007/FR-004, and its task line with the time the task text was set.
- **0014/FR-002** (US1): THE SYSTEM SHALL include in the answer only the descendants of the reading agent, as the tree is at the moment of the read, and SHALL NOT include the reading agent itself, its ancestors, or any other agent.
- **0014/FR-003** (US1): THE SYSTEM SHALL NOT include terminal content, the full task text past its task line, the CLI command, the working folder, environment values, or office additions in the answer.
- **0014/FR-004** (US1): THE SYSTEM SHALL answer a read while the office or an agent is on hold as without the hold, and SHALL NOT count a read toward any rate limit or automatic hold of 0007.
- **0014/FR-005** (US1): THE SYSTEM SHALL NOT add a read that it answers to the action record, and SHALL record a read that it refuses as 0002/FR-009 and 0002/FR-011 require.

This change modified: 0002/FR-008. The final text is under the change that owns each id.

### Preserved behaviors

- **0014/PB-1:** The 0002 tree rules stay as they are: an agent creates, starts, stops, gives new tasks to, and sends directives to its own descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007).
- **0014/PB-2:** Holds and rate limits refuse agent-made actions as before (0007/FR-002, FR-004, FR-007, FR-008). A read is not an agent-made action, so it adds no new way past a hold.
- **0014/PB-3:** A read changes no state: no status, task text, Needs you item, hold, notification, or action record entry changes because an agent read it (0008, 0011/FR-007).
- **0014/PB-4:** The office reads no CLI prompt and creates, changes, or deletes no file in a CLI's configuration (0001/FR-021).
- **0014/PB-5:** The desktop app shows the same as before, and the agent's connection to the office still gets no events and no terminals (0002).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Subtree entry | The state of one descendant at the moment of a read. Not stored. | agent, name, parent, role, status, last output time, in Needs you (yes or no), covered by a hold (yes or no), task line and the time of its task text | One per descendant of the reading agent, in one answer. |

### Success criteria

- **0014/SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each of the 9 agents reads once. 100% of the answers list exactly the reader's descendants: 0 entries for the reader itself, a sibling, an ancestor, or an agent in another branch.
- **0014/SC-002:** Each descendant prints a known marker text in its terminal before the read. 0 answers hold any part of a marker text.
- **0014/SC-003:** After a member's status changes to `working`, then to `waiting for input`, then to `stopped`, its lead's next read after each change shows the new status (3 of 3).
- **0014/SC-004:** While the office is on hold, 100% of 10 reads are answered. With no hold, after 30 reads within 60 seconds, the reader's next directive is not refused by a rate limit, and 0 automatic holds occur.
- **0014/SC-005:** With 30 agents in the office, a read answers within 2 seconds.
- **0014/SC-006:** In 10 attempts to point a read at a sibling, an ancestor, or an agent of another branch, 100% are refused and recorded, and 0 answers hold data of that agent.
- **0014/SC-007:** A descendant whose task text holds an escape character (U+001B), a one-byte control sequence introducer (U+009B), a right-to-left override (U+202E), and a second line: the reader's answer holds only the first line, and 0 control or bidirectional format characters reach the reader as themselves.

## Change 0015: Upward reports

### Intent

Directives go down the tree (0002), and a parent can read its descendants' state (0014), but nothing comes back up as words. A lead that briefed five members learns which one finished, or which one is stuck, only by guessing from a status, and the CEO cannot ask the owner for a decision except by going quiet. This change lets a running agent send a short report to its own parent: that it is done, that it has a question, that it is blocked, or a piece of information. The office types the report into the parent's terminal through the safe delivery of 0012, so a report never presses Enter into a question meant for a person. A report from the CEO, from an agent with no parent, or one that the sender marks for the person, goes to the owner instead: it joins Needs you (0008). The parent answers with a directive (0002); the owner answers in the agent's terminal.

### Requirements

- **0015/FR-001** (US1, US2, US3): WHEN a running agent sends a report with a kind of `done`, `question`, `blocked`, or `info` and a text of 1 to 1,000 characters that is not blank, THE SYSTEM SHALL accept it; otherwise THE SYSTEM SHALL refuse it, tell the sender the reason, and record the refusal.
- **0015/FR-002** (US1, US2): THE SYSTEM SHALL send an accepted report to the sender's parent as the tree is at the moment of the report, or to the person when the sender has no parent or marks the report for the person, and SHALL NOT take any input that names another agent as the report's receiver.
- **0015/FR-003** (US3): WHEN the system accepts a report, THE SYSTEM SHALL replace each control character (C0, DEL, C1, including line breaks), escape code, and bidirectional format character in the text with its visible form, and SHALL use only the visible form in the parent's terminal, in Needs you, and in the action record.
- **0015/FR-004** (US1): WHEN a report goes to a parent, THE SYSTEM SHALL type the report heading and the report text into the parent's terminal as one line, in the parent's queue together with its directives, under the rules of 0012/FR-001 to 0012/FR-005 as they apply to a directive whose target is the parent, and SHALL answer the sender that the report was submitted, waits with its place in the queue, or was left unsubmitted.
- **0015/FR-005** (US1): IF the sender's parent is not running, THEN THE SYSTEM SHALL refuse a report to it, say that the parent is not running, and record the refusal; IF a waiting report's sender is no longer a child of the report's parent, THEN THE SYSTEM SHALL withdraw it as 0012/FR-005 withdraws a directive whose target left the sender's subtree, EXCEPT a waiting report or end notice of a temp that the system ended, which stays in the queue. (modified by 0020)
- **0015/FR-006** (US3): THE SYSTEM SHALL treat sending a report as an agent-made action under 0007: the office hold and an agent hold that covers the sender refuse it (0007/FR-002, 0007/FR-004), and a rate limit "reports sent by one agent", 5 per rolling 60 seconds by default, refuses a report beyond it (0007/FR-007), counts toward the automatic hold (0007/FR-008), and is changed by the user in the office settings like the other limits (0007/FR-009).
- **0015/FR-007** (US2): WHEN the system accepts a report for the person, THE SYSTEM SHALL record it as an attention event of the sender, notify the owner with the sender's name and the report's kind as 0003/FR-006 (as modified) allows, and answer the sender that the report is in Needs you.
- **0015/FR-008** (US2): WHILE an item in Needs you holds a report, THE SYSTEM SHALL keep showing that report on the item when later attention events change the item's latest event, and SHALL replace it only with the sender's next report for the person.
- **0015/FR-009** (US1, US2, US3): THE SYSTEM SHALL add each accepted report to the action record with the sender as the actor, the kind, and the text in visible form, under the parent for a report to a parent and under the sender for a report for the person, and SHALL record each refusal in the sender's action record and, when the report had a parent, in the parent's.
- **0015/FR-010** (US1): WHILE reports wait in a parent's queue, THE SYSTEM SHALL keep at most one waiting report per sender, replacing a sender's waiting report with its newer report in the same place and recording the replaced one as withdrawn, and SHALL keep at most 10 waiting reports per queue, refusing a further report with the reason that the parent's queue has no room for reports, and SHALL NOT count such a refusal toward any rate limit or automatic hold.

This change modified: 0002/FR-008, 0003/FR-006, 0008/FR-002, 0008/FR-003, 0008/FR-005. The final text of 0002/FR-008 and 0003/FR-006 is under the change that owns each id. The final text of 0008/FR-002, 0008/FR-003, and 0008/FR-005 is under [Change 0008](../needs-you-queue/SPEC.md).

### Preserved behaviors

- **0015/PB-1:** The 0002 tree rules stay: directives go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007). A report goes up one level only, or to the person.
- **0015/PB-2:** A directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals. Reports share the queue and do not jump it; they take at most 10 of its places (FR-010), so directives always keep at least 10.
- **0015/PB-3:** The task line shows the last submitted directive (0011/FR-002, 0012 PB-3). A report changes no task text, neither the sender's nor the parent's.
- **0015/PB-4:** Needs you keeps one item per agent, its order inside a group, the jump shortcut, the count, and dismiss (0008). An item without a report behaves as before.
- **0015/PB-5:** The office reads no CLI prompt and writes nothing into a CLI's configuration (0001/FR-021).
- **0015/PB-6:** The existing rate limits, their defaults, and their settings stay as they are (0007 A3); the reports limit is a new one beside them.
- **0015/PB-7:** The agent's connection to the office still gets no events and no terminals (0002); a report's answer is the only thing it returns.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Report | One report an agent sent. | sender, kind, text in visible form, time sent, receiver (a parent or the person) | A report to a parent waits in the parent's queue like a directive until it is typed or withdrawn (0012). A report for the person lives on the sender's Needs you item. Both are in the action record. |
| Item (0008), extended | One agent that waits for the owner. | as in 0008, plus the latest report for the person (kind, text, time), if any | One per agent (0008). |

### Success criteria

- **0015/SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each member and each lead sends one report. 100% of the reports reach exactly the sender's parent's terminal, and 0 reach any other terminal.
- **0015/SC-002:** In 10 attempts to point a report at another agent, by a field naming an agent, a team, or a branch, 100% are refused and recorded, and 0 reach any terminal or Needs you.
- **0015/SC-003:** A report for the person shows in Needs you, with its kind and text, and the window title count changes, within 2 seconds after the agent sends it.
- **0015/SC-004:** A report text that holds U+001B, U+009B, U+202E, and a line feed: 0 of these characters reach the parent's terminal, Needs you, or the action record as themselves, and the parent's terminal receives the report as one line.
- **0015/SC-005:** A report of 1,000 characters is accepted; one of 1,001 characters and a blank one are refused and recorded (2 of 2).
- **0015/SC-006:** Of 10 reports sent to a parent whose screen shows a permission question, 0 characters are typed into the parent's terminal while the question shows.
- **0015/SC-007:** Under an office hold, 100% of 5 reports are refused with the hold reason. With no hold, the 6th report within 60 seconds is refused with the limit, and after the third such refusal the sender is on hold automatically.
- **0015/SC-008:** A lead that received a member's report sends a directive to the id in the report heading, and the member's terminal receives it (1 of 1).
- **0015/SC-009:** A report item stays in Needs you while its agent changes to `working` and back to `waiting for input`, and leaves when the owner opens the agent's terminal (1 of 1, and 1 of 1 for dismiss).
- **0015/SC-010:** 100% of accepted, refused, withdrawn, and unsubmitted reports have an entry in the action records that FR-009 names.
- **0015/SC-011:** With 10 reports from 10 children waiting in a working lead's queue, a directive from the CEO to that lead is accepted (1 of 1), an 11th report from another child is refused, and a second report from a child whose report waits leaves exactly one waiting report of that child.
- **0015/SC-012:** 5 reports for the person from one agent, with the first notification unread, add exactly 1 notification, and the item shows the 5th report.

### Notes from Deviations

- **Visible form in the use case, not the gateway (PLAN D3).** The plan made the visible form in `agent_call.rs`. The use case (`SendReportHandler`) makes it instead, so the length check and the visible form sit in one place and every receiver gets the same form. The 0014 `visible_text` moved from the server app to the hierarchy domain for that; the subtree read imports it from there.
- **Ports bundle.** `SendReportHandler::new` takes a `ReportPorts` bundle, as `SendDirectiveHandler::new` takes `DirectivePorts`, and holds the directive handler whose queue a report to a parent shares.
- **Holds checked where the queue is.** A report for the person passes the hold gate in `SendReportHandler`; a report to a parent passes it in `SendDirectiveHandler::send_report`, after the room checks, so a no-room refusal takes no rate slot (FR-010).
- **A failed Needs you write does not refuse the report.** The contract's `internal` row names a tree read failure only; a failed notification or item write raises the storage alert, as for every attention event (contract amended).
- **Commits.** T6 tests are two commits (person path, parent path), and the desktop has its own commit.
- **Refused text in visible form (final review L1).** A refused report is journaled with its text in visible form, so a bidirectional format character cannot reorder the record; `a_refused_report_is_journaled_in_visible_form`. A refused directive keeps the 0012 escape of C0 and C1 only.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0014-subtree-status - 2026-11-23
- 0015-upward-reports - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
