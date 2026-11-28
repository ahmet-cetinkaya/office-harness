---
status: shipped
capability: agent-reports
plan: specs/changes/archive/0014-subtree-status/PLAN.md
supersedes: 0002/FR-008
---

<!-- Approved under the owner's spec delegation on 2026-11-23; shipped in PR #116. -->

# 0014: A parent reads the state of its descendants

## Intent

A parent agent sends directives down the tree (0002), but it cannot see what its team does. A lead that briefed five members has to guess which of them is still working, which one waits, and which one stopped, because nothing comes back up except the members' terminals, which agents cannot read. This change lets a running agent read the state of its own descendants from inside its session: for each one, the name, the place in the tree, the status, when its terminal last showed output, whether it waits in the owner's Needs you list, whether a hold covers it, and the task line it was given. The agent reads state, never terminal content, and only below itself, because terminal output shown to another agent is a path for injected instructions and leaked secrets. This is the first slice of an upward report channel; the channel itself, in which a child tells its parent that it is done or asks a question, is the next change. The idea comes from an MIT-licensed reference office app; see the pull request.

## Context

- Source of truth: `docs/PRD.md`, R2 (deep hierarchy).
- This change depends on 0001 (agents, statuses, terminals, the action record), 0002 (the tree, descendants, the agent's calls from inside its session, 0002/FR-008, 0002/FR-009), 0007 (holds and rate limits), 0008 (Needs you), and 0011 (the task text). References such as `0002/FR-008` point to requirements in those changes.
- 0002 A2 says that a parent reads a child's result from the child's terminal output or from the files the child writes. This change does not change that: it adds the child's state, not its result.
- The office does not read a CLI's prompts (0001 A1, 0003 A3), so a status says what the office saw (`working`, `waiting for input`, and so on), never what the agent meant.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, status, terminal, action record | As defined in 0001. |
| Parent, descendant, role, directive | As defined in 0002. |
| Agent-made action, office hold, agent hold, rate limit | As defined in 0007. |
| Needs you, item | As defined in 0008. |
| Task text, task line | As defined in 0011. The task line is the first line of the task text, up to 80 characters, with an ellipsis when shortened. |
| Read (of the subtree) | A request that a running agent sends from inside its session to get the state of its descendants. It changes nothing in the office. |
| Last output time | The time at which the agent's CLI last wrote to its terminal in its current session. |
| Terminal content | Anything the CLI wrote to its terminal: output, the last output line, the screen, the scrollback, and the window title. |

## Data flow and trust boundary

- **Boundary:** the agent's connection to the office (0002). The office knows the caller from the call credential of its running session and fixes the caller as the actor; the request names nothing else.
- **Into the office:** the request type only. A read takes no input, so it cannot name an agent, a branch, or a field.
- **Out of the office, to the reading agent:** for each descendant, its id, name, parent id, role, status, last output time, two yes-or-no flags (Needs you, hold), and its task line with one time. The only free text is the name (up to 64 characters, set when the agent was created) and the task line (up to 80 characters, from a directive of the descendant's parent or from the user). No text that a CLI wrote to its terminal crosses.
- **Never crosses:** terminal content, the full task text, the CLI command, the working folder, environment values (which can hold keys), office additions, the office token, and any agent's call credential (FR-003).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | A descendant's CLI output carries instructions that steer the reader (prompt injection), or a secret it printed. | No terminal content crosses (FR-003). |
  | An agent reads a sibling, an ancestor, or another branch. | The office picks the descendants from the tree at the moment of the read; the request cannot name an agent (FR-002, SC-001, SC-006). |
  | A descendant lead writes a directive whose text later reaches the CEO as its member's task line. | Only one line of at most 80 characters crosses, as data (FR-003, SC-007). This is the remaining upward text path; the reader treats it as data. |
  | A control character, an escape code, or a bidirectional format character in a name or a task line acts on the reader's terminal or reorders the text it reads. | Each one is shown as a visible symbol, as 0011's display does (Edge cases, SC-007). |
  | A field added to agents later leaks through the read. | The answer names its fields one by one (FR-003). |
  | A looping reader floods the action record. | Answered reads are not recorded (FR-005, A3). |
| A looping reader slows the office down. | A read takes no lock that orders office changes, so it blocks no other action (A11). |

- **Residual risk:** an agent CLI runs as the user's OS user and can read the office's files directly; the read rule prevents mistakes, not a hostile agent (A8).

## User stories

### US1 — A parent sees the state of its team (Priority: P1)

A running lead, or the CEO, reads the state of every agent below it and picks its next directive from that state: it briefs the member that waits, leaves the working ones alone, and restarts the one that stopped.

**Why this priority:** this is the whole change. Without it a parent picks its next directive by guessing, and the owner has to relay the state by hand.

**Independent test:** build a tree of the CEO, lead L1 with members M1 and M2, and lead L2 with member M3. Let M1 print output continuously, let M2 go quiet until it shows `waiting for input`, and let M1 print a marker text. L1 reads. The answer lists M1 and M2, each with its status, its last output time, its flags, and its task line, and it lists no other agent and no part of the marker text.

**Scenarios:**

1. Given lead L1 has members M1 (`working`) and M2 (`waiting for input`), When L1 reads, Then the answer lists M1 and M2, each with its name, parent, role, status, last output time, whether it has an item in Needs you, whether a hold covers it, and its task line with the time the task text was set.
2. Given the CEO has leads L1 and L2 with their members, When the CEO reads, Then the answer lists L1, L2, and every member, each one after its parent.
3. Given L1 and L2 are leads under the same CEO, When L1 reads, Then the answer lists neither L1 itself, nor the CEO, nor L2, nor L2's members.
4. Given M1 printed the text "marker-7Q" in its terminal, When L1 reads, Then no part of the answer holds that text or any other terminal content of M1.
5. Given member M1 has no descendants, When M1 reads, Then the answer is an empty list.
6. Given the office is on hold, When L1 reads, Then the answer is the same as without the hold, and the read counts toward no rate limit.
7. Given M2 has an item in Needs you, When L1 reads, Then M2's entry says so, and M2's item stays in Needs you.

## Requirements

### ADDED

- **FR-001** (US1): WHEN a running agent reads its subtree, THE SYSTEM SHALL answer with one entry for each of its descendants, each entry with the descendant's name, parent, role, status, last output time, whether it has an item in Needs you, whether an office hold or an agent hold covers it under the rules of 0007/FR-004, and its task line with the time the task text was set.
- **FR-002** (US1): THE SYSTEM SHALL include in the answer only the descendants of the reading agent, as the tree is at the moment of the read, and SHALL NOT include the reading agent itself, its ancestors, or any other agent.
- **FR-003** (US1): THE SYSTEM SHALL NOT include terminal content, the full task text past its task line, the CLI command, the working folder, environment values, or office additions in the answer.
- **FR-004** (US1): THE SYSTEM SHALL answer a read while the office or an agent is on hold as without the hold, and SHALL NOT count a read toward any rate limit or automatic hold of 0007.
- **FR-005** (US1): THE SYSTEM SHALL NOT add a read that it answers to the action record, and SHALL record a read that it refuses as 0002/FR-009 and 0002/FR-011 require.

### MODIFIED

- **0002/FR-008** (US1): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that 0002/FR-001 allows.

### PRESERVED

- **PB-1:** The 0002 tree rules stay as they are: an agent creates, starts, stops, gives new tasks to, and sends directives to its own descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009 as modified by 0007).
- **PB-2:** Holds and rate limits refuse agent-made actions as before (0007/FR-002, FR-004, FR-007, FR-008). A read is not an agent-made action, so it adds no new way past a hold.
- **PB-3:** A read changes no state: no status, task text, Needs you item, hold, notification, or action record entry changes because an agent read it (0008, 0011/FR-007).
- **PB-4:** The office reads no CLI prompt and creates, changes, or deletes no file in a CLI's configuration (0001/FR-021).
- **PB-5:** The desktop app shows the same as before, and the agent's connection to the office still gets no events and no terminals (0002).

## User experience

None: this change adds no screen to the desktop app. The reading agent gets the answer as text in its own session, through the same call that it uses for directives (0002 A7).

**Visible states (for the reading agent):**

| State | What the agent gets | Covered by |
|---|---|---|
| Descendants exist | One entry per descendant, each after its parent. | FR-001, FR-002, US1 scenarios 1 and 2 |
| No descendants | An empty list. | US1 scenario 5, Edge cases |
| Refused | An error with the reason; the refusal is in the action record. | FR-005, Edge cases |
| Office cannot read its state | An error that says the state cannot be read now. | Edge cases |

**Design:** none; no visual change. The plan must not add visuals.

**Accessibility:** no change to the desktop app; its keyboard and screen reader behavior stays as before (0001/FR-028).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Subtree entry | The state of one descendant at the moment of a read. Not stored. | agent, name, parent, role, status, last output time, in Needs you (yes or no), covered by a hold (yes or no), task line and the time of its task text | One per descendant of the reading agent, in one answer. |

## Success criteria

- **SC-001:** In a tree of 1 CEO, 2 leads, and 6 members, each of the 9 agents reads once. 100% of the answers list exactly the reader's descendants: 0 entries for the reader itself, a sibling, an ancestor, or an agent in another branch.
- **SC-002:** Each descendant prints a known marker text in its terminal before the read. 0 answers hold any part of a marker text.
- **SC-003:** After a member's status changes to `working`, then to `waiting for input`, then to `stopped`, its lead's next read after each change shows the new status (3 of 3).
- **SC-004:** While the office is on hold, 100% of 10 reads are answered. With no hold, after 30 reads within 60 seconds, the reader's next directive is not refused by a rate limit, and 0 automatic holds occur.
- **SC-005:** With 30 agents in the office, a read answers within 2 seconds.
- **SC-006:** In 10 attempts to point a read at a sibling, an ancestor, or an agent of another branch, 100% are refused and recorded, and 0 answers hold data of that agent.
- **SC-007:** A descendant whose task text holds an escape character (U+001B), a one-byte control sequence introducer (U+009B), a right-to-left override (U+202E), and a second line: the reader's answer holds only the first line, and 0 control or bidirectional format characters reach the reader as themselves.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The reading agent has no descendants (a member, or an agent outside the hierarchy). | Empty | The answer is an empty list, not a refusal (FR-002). |
| A descendant is stopped, exited, failed, interrupted, or never started. | Empty | Its entry shows that status and no last output time. |
| A descendant has no task text. | Empty | Its entry shows no task line. |
| A descendant's task text has 8,192 characters or several lines. | Boundary | The entry carries only the task line: the first line, up to 80 characters, with an ellipsis when shortened (FR-003). |
| A task line or a name holds control characters, terminal escape codes, or bidirectional format characters. | Boundary | The answer shows each such character as a visible symbol, as 0011's display does, so nothing is applied to the reader's terminal and the text keeps its order (0011/FR-006). |
| A descendant has not written any output yet in its current session. | Boundary | Its last output time is the time its session started. |
| The request names an agent, or carries any other input. | Invalid | The office refuses it, tells the agent the reason, and records the refusal (FR-005). A read takes no input, so it can never point at an agent outside the subtree. |
| The request names an actor or a caller. | Unauthorized | The office refuses it and records the refusal, as for every agent request (0002). |
| A descendant is placed, moved out, or deleted during or between reads. | Concurrent | Each answer shows the tree of one moment; the next read shows the change (FR-002). |
| Two agents read at the same time. | Concurrent | Each gets its own subtree. |
| A team lead was placed before the office had a CEO. | Boundary | The read follows the parent the office stored. Since the 0002 A5 fix, such a lead moves under the CEO when the CEO is created, and an office saved before the fix is repaired at start, so the CEO's read lists that lead and its members (A10). |
| The reading agent's session has ended, so its call credential is gone. | Unauthorized | The office refuses the connection, as for every agent request (0002). |
| An agent reads in a tight loop. | Boundary | Each read is answered; reads change nothing, so no hold or limit applies (FR-004, A2). The user stops or holds the agent as for any loop. |
| The office cannot read the tree, the agents, or Needs you (for example disk failure). | Failure | The agent gets an error that says so, the refusal is recorded, and nothing changes. |
| The desktop app sends the read. | Invalid | The office does not offer the read to the desktop app; the panel already shows every agent (0003). |

## Assumptions

- **A1:** The office knows the last output time only while the CLI runs, in memory. After a server restart the agents are `interrupted` (0001/FR-004) and show no last output time until they run again.
- **A2:** A read is not an agent-made action under 0007: it changes nothing in the office, the same as an agent asking who it is. Holds do not refuse it, and it counts toward no rate limit. Counting it was considered and not chosen: a lead that polls its team would then reach a limit and put itself on hold automatically (0007/FR-008), which blocks its next directive for no reason.
- **A3:** An answered read is not added to the action record, the same as an agent asking who it is today. A lead that polls its team would otherwise bury the record's real actions under reads.
- **A4:** "Covered by a hold" is the office hold, or an agent hold on the descendant or on one of its ancestors, the reading agent included: the same coverage that refuses agent-made actions under 0007/FR-004. The answer does not say which hold covers it or why.
- **A5:** "Has an item in Needs you" says only whether the item exists, not its event or its wait time.
- **A6:** The answer shows the state at the moment of the read. The office sends no update to the agent when a state changes; the agent reads again.
- **A7:** The task line of a task text set by the user (0011/FR-003) is readable by the agent's ancestors. Hiding task text is out of scope, as in 0011.
- **A8:** Like the 0002 descendant rule, the read rule prevents mistakes. It is not a security boundary against a hostile agent that runs as the same OS user (0002 A8, 0001 A7).
- **A9:** The task line counts characters as the text stores them, so a character built from several parts can count as more than one. The reader is an agent, not a person, so the line does not need the on-screen count of 0011 A3.
- **A10:** The read uses the descendant rule as the office applies it (0002/FR-008). The 0002 A5 gap is closed: existing team leads move under a CEO created later, and an office saved before the fix is repaired at start, so the CEO sees those leads in a read.
- **A11:** A read's cost grows with the size of the office only. It answers from stored state (the tree, the agents, Needs you, the holds) and from each running session's last output time in memory; it reads no terminal content and takes no lock that orders office changes. A looping reader therefore costs processor time but never blocks a create, a delete, a directive, or a start.

## Clarifications

### Session 2026-11-23 (drafted under the owner's delegation)

- Q: Does the reading agent appear in its own answer? → A: No. The answer holds its descendants only (FR-002); the agent already knows who it is.
- Q: Do holds and rate limits apply to the read? → A: No (FR-004, A2). The read is not an agent-made action. To be confirmed at the spec review gate.
- Q: What is "last activity"? → A: The last output time: when the CLI last wrote to its terminal. The time is not terminal content; the output itself never leaves the office (FR-003).
- Q: Does the reader get the full task text? → A: No, only the task line (FR-001, FR-003). The task text can come from a directive that a descendant lead wrote, so a short single line keeps that upward text small (review guidance, 2026-11-23).

## Out of scope

- **The upward report channel:** a child telling its parent that it is done or asking it a question, and the CEO reaching the owner. This is the next change.
- Sending state changes to agents as they happen; the agent reads again (A6).
- Reading terminal content of any agent: output, last output line, screen, or title (FR-003).
- Reading the state of siblings, ancestors, other branches, agents outside the hierarchy, or the whole office (FR-002).
- The Needs you item's event and wait time, and the hold's reason and setter (A4, A5).
- A rate limit or a hold rule for reads (A2).
- Any change to the desktop app.
