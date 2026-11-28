# Resume all interrupted agents

## Intent

After an unclean stop the person resumes all interrupted agents in one action. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0013-bulk-resume](../changes/archive/0013-bulk-resume/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0013/FR-008 | 0017 | Change 0013, Requirements |

Changes of other capabilities that this file names:

- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0013: Resume all interrupted agents

### Intent

After a server restart, the user must resume each interrupted agent one by one: open it, pick its last session, and continue it. When a CLI cannot continue its session, the agent ends at once and the user must notice it. This change adds one confirmed action that resumes every interrupted agent that the user checks. When a CLI refuses to continue, the office starts that agent once in a new session. An agent that still fails is put on hold. The office still starts no agent by itself (0001/FR-004). This change extends 0001 and 0007.

### Requirements

- **0013/FR-001** (US1): WHILE at least one agent of the office is `interrupted`, THE SYSTEM SHALL offer a "Resume interrupted" action that shows the number of interrupted agents.
- **0013/FR-002** (US1, US3): WHEN the user opens the "Resume interrupted" action, THE SYSTEM SHALL list every interrupted agent in start order, with what will happen to it (continue its last session, shown with that session's start time, or start a new session, shown with the reason) and with whether it is on hold, SHALL check every listed agent that is not on hold, and SHALL start nothing before the user confirms.
- **0013/FR-003** (US1, US2): WHEN the user confirms, THE SYSTEM SHALL, for each checked agent that is still `interrupted`, continue its last session where the agent's CLI supports continuing it (0001/FR-025) and otherwise start a new session (0001/FR-024), SHALL start every member before its lead and every lead before the CEO, and SHALL skip each checked agent that is no longer `interrupted`, with that reason.
- **0013/FR-004** (US2): IF a session that this action continued ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL start the agent once in a new session and SHALL record that start in the agent's action record with the office as actor and the reason (which session the CLI refused, and how).
- **0013/FR-005** (US3): IF a new session that this action started ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL NOT start the agent again, SHALL put the agent on hold (0007/FR-003) with the office as actor and the reason, and SHALL notify the user once, naming the agent.
- **0013/FR-006** (US1, US2, US3): THE SYSTEM SHALL show the result of each checked agent as text as soon as it is known: checking (during the check period), continued, new session (with the reason), on hold (with the reason), or skipped (with the reason).
- **0013/FR-007** (US2): WHEN this action starts a new session instead of continuing the last one, for any reason, THE SYSTEM SHALL keep the reason with that start's entry in the agent's action record, so the user sees in the agent's details that the CLI's earlier context was not carried over.
- **0013/FR-008** (US1): WHERE the last session of a listed agent was still open when the server stopped uncleanly, THE SYSTEM SHALL mark that agent in the list with what the sweep (0017/FR-003) found for it, and SHALL keep Confirm unavailable while an agent with the warning is checked, until the user acknowledges the warning. (modified by 0017) The mark is one of these:
  - every leftover ended (ended or ended by force): a note with the number of processes from before the stop that the office ended, and no warning;
  - a leftover that could not be ended, a stored process identity that was not readable, a process that matched except for a boot time that changed, or no complete stored process identities for the session (a session from before 0017, a failed store, a session over the helper limit, or a sweep that could not run): the warning, shown before the user confirms, that a CLI process of that agent from before the stop may still run and that resuming can start a second copy of a CLI next to it;
  - no leftover found: no mark and no warning.

### Preserved behaviors

- **0013/PB-1:** No agent starts without an explicit action of the user; the server starts no interrupted agent by itself at its start (0001/FR-004).
- **0013/PB-2:** The user can still continue any previous session of an agent from its details, also a session that the CLI refused before (0001/FR-025).
- **0013/PB-3:** A hold refuses only agent-made actions; the user's own actions, this action included, work as before (0007 PB-1, PB-2).
- **0013/PB-4:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).
- **0013/PB-5:** Each start, continue, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).
- **0013/PB-6:** The notification of FR-005 is a notification of an automatic hold under 0003/FR-006 (as modified by 0007): it names the agent and the event, and the focused-terminal exception does not apply to it.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Resume result | What the action did to one checked agent. | agent, result (checking, continued, new session, on hold, skipped), reason | Shown while the dialog is open; not stored. What happened stays in the action record and the hold list. |
| Hold (extends 0007) | An agent hold. | reason, which can now also be a failed resume | Set by the office (FR-005). |
| Session (extends 0001) | One run of an agent CLI. | end result: also "lost", when the session was still open at an unclean stop | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | reason (optional): why a start began a new session instead of a continue | Belongs to the agent it names (0001/FR-011). |

### Success criteria

- **0013/SC-001:** With 30 interrupted agents, the user resumes all of them with 2 activations after a clean stop (open the action, confirm) and 3 after an unclean stop (open, acknowledge the warning, confirm), and 100% of them have a started session within 60 seconds of the confirm.
- **0013/SC-002:** With a CLI that ends within 1 second of every continue, 100% of the checked agents run a new session within 30 seconds of the confirm, and each has exactly 1 action record entry for that start with the office as actor and a reason.
- **0013/SC-003:** An agent whose CLI ends within 1 second of every start is started at most 2 times by one bulk action, is on hold after it, and causes exactly 1 notification.
- **0013/SC-004:** After 10 server starts that follow a SIGKILL of the server, 0 agents start without an action of the user.
- **0013/SC-005:** In a tree of 1 CEO, 2 leads, and 4 members, all interrupted, 100% of the members start before their lead and both leads start before the CEO.
- **0013/SC-006:** After a SIGKILL of the server while agents run, the warning of FR-008 shows and marks 100% of those agents, and 0 confirms are possible before the acknowledgement while a marked agent is checked; after a clean stop, the warning shows 0 times.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0013-bulk-resume - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
