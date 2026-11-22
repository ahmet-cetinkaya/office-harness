---
status: draft
capability: office
plan: specs/changes/0017-orphan-sweep/PLAN.md
supersedes: 0013/FR-008
---

<!-- Draft for the spec review gate. Only the owner's approval, given through the review gate, changes the status. -->

# 0017: End the processes left over from an unclean stop

## Intent

When the background server stops uncleanly (SIGKILL, a crash, or a power loss of the server process only), some agent CLI processes and their helper processes keep running. The next server start does not see them. They keep spending the owner's quota unseen, and they can keep a session busy, which can break the resume of 0013. This change makes the server remember, for each session, which processes belong to it. At the next start, before the user can resume anything, the server ends only the processes that are still exactly those processes, and records each end. The resume warning of 0013 then names only the agents that really had processes left.

## Context

- Source of truth: `docs/PRD.md`. References such as `0001/FR-004` point to requirements of the approved changes. The living spec is `specs/office/SPEC.md`. 0013 is not merged into it yet, so this change refers to `0013/FR-008` in `specs/changes/0013-bulk-resume/SPEC.md`.
- This change depends on 0001 (sessions, the stop and its grace period, the action record, the storage alert), 0003 (notifications), 0007 (the office as an actor), and 0013 (the unclean stop, the `lost` session end, and the "Resume interrupted" action).
- Research R1 of 0013 (`specs/changes/0013-bulk-resume/research.md`) killed the server with SIGKILL. A CLI that ignores the terminal hang-up kept running, and the next server start did not see it. Research R3 found that the background service of `opencode` keeps running in a session of its own after the hang-up. 0013 only warns about such leftovers (0013/FR-008, 0013 A5). This change finds and ends them.
- The idea comes from an MIT-licensed reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, action record | As defined in 0001. |
| Forced kill of the server | SIGKILL of the server process on Linux and macOS, or a forced end of it on Windows. It causes an unclean stop. |
| Unclean stop, `lost` session | As defined in 0013: the server ended without ending its sessions, and the next start ends each session that is still open as `lost`. |
| Grace period | The time between the request to end and the forced end of a stop (0001 A3: 10 seconds by default; the user can change it). |
| Session process | The CLI process of a session, or a process that the office saw descend from that CLI process while the session ran (a helper). A helper stays a session process after it leaves the CLI's process tree, for example when it moves to a session of its own or to a new parent. |
| Process identity | The three values that the OS reports for a running process: its process id, its start time, and its command line (program and arguments). The office also keeps the machine run (boot) in which it saw them. |
| Match | A running process matches a stored process identity when all three values are equal (the whole command line is compared, not the shortened text) and the machine run is the same. A stored identity from another run of the machine never matches. Where the OS gives no exact machine run (macOS and Windows), a boot time that differs by more than 2 seconds does not prove a restart: the office then does not end the process and keeps the warning. |
| Command line in visible form | The command line with each control, escape, and bidirectional-control character shown as an escape sequence (as 0014 and 0015/FR-003 show text), cut to 1,000 characters. A longer line ends with an ellipsis and the first 8 characters of a hash of the whole line. |
| Leftover | A session process of a `lost` session that still runs and matches at the next server start. |
| Sweep | The check, at a server start, of every stored process identity of every `lost` session, and the end of every leftover. |
| Sweep result | What the sweep did with one stored process identity: ended, ended by force, could not end (with the reason), or left (with the reason). |
| Helper limit | The most open helper identities that the office stores for one session: 200. |

## User stories

### US1: Leftovers are ended before the user can resume (Priority: P1)

After an unclean stop, the next server start ends every leftover before the resume list shows and before any agent starts. Each end is in the agent's action record.

**Why this priority:** it is the value of the change. A leftover spends quota unseen and can break the resume of its agent.

**Independent test:** start 3 agents on stand-in CLIs that outlive the server, and give one of them a helper that moves to a session of its own. Kill the server with SIGKILL and start it again. Before the resume list shows, all 4 processes have ended, and each agent's action record has one entry per process with the office as actor, the session, the process id, the command line, and the result.

**Scenarios:**

1. Given a CLI process of a `lost` session still runs and matches, When the server starts, Then the server asks it to end, and the agent's action record shows "ended" with the office as actor, the session, the process id, and the command line.
2. Given a leftover does not end within the grace period, When the grace period ends, Then the server ends it by force and records "ended by force".
3. Given a helper of a `lost` session moved to a session of its own while the session ran, When the server starts, Then the server ends it like the CLI process.
4. Given the sweep runs, When the user opens "Resume interrupted", Then the dialog says that the office ends processes left from the last run, and the list shows only after the sweep ends.
5. Given the sweep runs, When the user starts an agent, starts a new task, or continues a session, Then the request waits and runs after the sweep ends.
6. Given a leftover still runs after the forced end, or the OS does not let the office end it, When the sweep ends, Then the action record shows "could not end" with the reason, and the user gets one notification that names the agent.

### US2: Only the session's own processes are ended (Priority: P1)

The sweep ends a process only when its process id, start time, and command line all still match. It leaves every other process and records why.

**Why this priority:** ending an unrelated process of the user is worse than leaving a leftover. US1 must not ship without it.

**Independent test:** after a SIGKILL of the server, put in place: a process with a stored process id but another command line, a process with a stored command line but another start time, and a process that the office did not start. Start the server. None of them ends, and each stored identity that they reuse has one "left" entry with its reason.

**Scenarios:**

1. Given the process id of a stored identity now belongs to a process with another start time or another command line, When the sweep runs, Then the office does not end that process and records "left" with the reason "other start time" or "other command line".
2. Given no process runs with a stored process id, When the sweep runs, Then the office records "left" with the reason "not running".
3. Given the office cannot read the start time or the command line of a process with a stored process id, When the sweep runs, Then the office does not end it and records "left" with the reason "not readable".
4. Given a leftover ends by itself during the grace period and its process id is then given to another process, When the forced end would follow, Then the office does not end the new process, because it checks the match again before each end.
5. Given the server stopped cleanly, or a session ended normally, When the server starts, Then the sweep ends no process of that session and records nothing for it.

### US3: The resume warning names only the agents with leftovers (Priority: P2)

The "Resume interrupted" list marks each agent of a `lost` session with what the sweep found. The second-copy warning shows only where a process from before may still run.

**Why this priority:** US1 and US2 remove the danger; this story removes a warning that no longer applies. Without it, the dialog of 0013 still warns for every `lost` session, which is safe but less precise.

**Independent test:** run 4 agents. Make the first CLI outlive the server and end on request, the second outlive the server and refuse to end, and the third end on the hang-up. The fourth has a session from before this change, with no stored identity. Kill the server with SIGKILL and start it again, then open "Resume interrupted". The first agent has the note that 1 process was ended. The second and the fourth have the warning. The third has no mark.

**Scenarios:**

1. Given the sweep ended every leftover of an agent, When the list shows, Then that agent has a note that says how many processes from before the stop were ended, and no second-copy warning.
2. Given the sweep could not end a leftover of an agent, or could not read a process with a stored process id, When the list shows, Then that agent is marked with the warning of 0013, and Confirm waits for the acknowledgement while the agent is checked.
3. Given the sweep found no leftover of an agent, When the list shows, Then that agent has no mark and no warning.
4. Given the office has no complete stored process identities for an agent's `lost` session, When the list shows, Then that agent is marked with the warning as 0013 has it.

## Requirements

### ADDED

- **FR-001** (US1): WHEN the office starts a session, THE SYSTEM SHALL store the process identity of the session's CLI process before the session shows as started, and SHALL make it readable only by the OS user who owns the office (as 0001/FR-009). THE SYSTEM SHALL store the command line as a hash of the whole line, for the match, and as a display text of at most 1,000 characters.
- **FR-002** (US1): WHILE a session runs, THE SYSTEM SHALL store the process identity of each process that descends from the session's CLI process within 10 seconds after that process starts, SHALL keep it while that process runs, also after it leaves the CLI's process tree, and SHALL drop it when that process ends.
- **FR-003** (US1, US2): WHEN the server starts and finds `lost` sessions, THE SYSTEM SHALL run the sweep before it shows the list of the "Resume interrupted" action and before it starts any agent, SHALL ask each leftover to end and end by force each leftover that still runs after the grace period, and SHALL check the match again just before each end request. Where the OS lets the office hold a process (Linux 5.3 and later, and Windows), THE SYSTEM SHALL send each end request only to the process that it just checked, so that a process that gets the same process id later is never ended. On macOS, and on a Linux kernel without this, the time between the check and the end request is a known risk (A10).
- **FR-004** (US1): WHEN the sweep ends a leftover, THE SYSTEM SHALL add an entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the result "ended" or "ended by force".
- **FR-005** (US2): IF a stored process identity of a `lost` session does not match a running process, THEN THE SYSTEM SHALL NOT end any process for it and SHALL add a "left" entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the reason: not running, machine restarted, boot time changed, other start time, other command line, or not readable.
- **FR-006** (US1): IF a leftover still runs after the forced end, or the OS refuses to end it, THEN THE SYSTEM SHALL add a "could not end" entry with the reason and the command line in visible form to the agent's action record and SHALL notify the user once per server start, naming each agent with such a leftover.
- **FR-007** (US1): WHILE the sweep runs, THE SYSTEM SHALL show, in the panel and in the "Resume interrupted" dialog, that the office ends processes left from the last run, and SHALL keep each start, new task, and continue request of the user waiting until the sweep ends, then run it.
- **FR-008** (US2): WHEN a session ends while the server runs (a stop, an end of its CLI, or a clean server stop), THE SYSTEM SHALL mark every stored process identity of that session as ended, so that no later sweep ends a process for it or records anything for it.
- **FR-009** (US1, US2): WHEN the sweep records the sweep result of a stored process identity, THE SYSTEM SHALL mark that identity as ended in the same step, so that a later sweep, also after the server is killed during this sweep, neither ends a process for it nor records a second sweep result for it.
- **FR-010** (US1): WHILE a session runs, THE SYSTEM SHALL store at most 200 open helper identities for it (the helper limit). WHEN a new helper would pass the limit, THE SYSTEM SHALL stop storing helpers of that session, SHALL add one entry to the agent's action record that names the limit, and the session SHALL count as having no complete stored process identities.
- **FR-011** (US1): WHILE a session runs and a stored session process keeps its process id and start time but runs another command line (it replaced itself with another program), THE SYSTEM SHALL replace the stored command line with the new one within 10 seconds, and SHALL NOT mark that identity as ended.

### MODIFIED

- **0013/FR-008** (US3): WHERE the last session of a listed agent was still open when the server stopped uncleanly, THE SYSTEM SHALL mark that agent in the list with what the sweep (0017/FR-003) found for it:
  - every leftover ended (ended or ended by force): a note with the number of processes from before the stop that the office ended, and no warning;
  - a leftover that could not be ended, a stored process identity that was not readable, a process that matched except for a boot time that changed, or no complete stored process identities for the session (a session from before 0017, a failed store, a session over the helper limit, or a sweep that could not run): the warning, shown before the user confirms, that a CLI process of that agent from before the stop may still run and that resuming can start a second copy of a CLI next to it;
  - no leftover found: no mark and no warning.

  THE SYSTEM SHALL keep Confirm unavailable while an agent with the warning is checked, until the user acknowledges the warning.

### PRESERVED

- **PB-1:** The server starts no agent by itself at its start (0001/FR-004). The sweep only ends processes; every `lost` agent stays `interrupted` until the user acts.
- **PB-2:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021). The sweep reads process identities and ends processes; it does not touch any CLI file.
- **PB-3:** The stop action ends the CLI and every process that descends from it at the time of the stop, first with a request and then by force after the grace period (0001/FR-012, 0001 A3), as today.
- **PB-4:** The office never ends a process that it did not start: a process that is not a session process of a `lost` session is never ended by the sweep, also when its command line is the same as an agent's.
- **PB-5:** The user's own actions work as before once the sweep ends, and the "Resume interrupted" action behaves as 0013/FR-001 to 0013/FR-007 define.
- **PB-6:** After an unclean stop, the open sessions still end as `lost`, and the user can still continue a `lost` session (0013 Key entities, Session; 0001/FR-025).

## User experience

**Flow:**

1. The server starts after an unclean stop. While the sweep runs, the panel shows the text "Ending processes left from the last run".
2. If the user opens "Resume interrupted" during the sweep, the dialog shows the same text in place of the list, then shows the list when the sweep ends.
3. If the user starts an agent, starts a new task, or continues a session during the sweep, the request waits and runs when the sweep ends.
4. If a leftover could not be ended, the user gets one notification that names each such agent.
5. The list of "Resume interrupted" marks each agent of a `lost` session with the sweep's finding: a note that leftovers were ended, the second-copy warning, or nothing.
6. In an agent's details, the action record shows one entry for each stored process identity of the `lost` session: ended, ended by force, could not end, or left, with the reason, the session, the process id, and the command line in visible form.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Sweep runs | The panel text "Ending processes left from the last run". | FR-007, US1 scenario 5 |
| Dialog opened during the sweep | The same text in the dialog in place of the list; no Confirm. | FR-007, US1 scenario 4 |
| No `lost` session | No sweep text; the panel is as before. | FR-003, US2 scenario 5 |
| Leftovers ended | In the list, a note with the number of ended processes; no warning. | 0013/FR-008 (modified), US3 scenario 1 |
| Leftover could not be ended, or not readable, or no stored identity | In the list, the 0013 mark and warning, and the acknowledgement that Confirm waits for. | 0013/FR-008 (modified), US3 scenarios 2 and 4 |
| No leftover found | In the list, no mark and no warning. | 0013/FR-008 (modified), US3 scenario 3 |
| Could not end | One notification naming the agents. | FR-006, US1 scenario 6 |
| Agent details | The action record entries of the sweep, with actor, session, process id, command line in visible form, and result or reason. | FR-004, FR-005, FR-006 |
| Desktop app cannot reach the server or its token is rejected | The 0001 states. | Edge cases |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves. The note and the warning use the mark and the warning area of the 0013 dialog.

**Accessibility:**

- The sweep text, the note, the warning, and the action record entries are text, never color alone.
- The sweep text is announced to assistive technology when it shows and when the sweep ends.
- The dialog keeps the keyboard behavior of 0013: every control works from the keyboard (0001/FR-028), focus is always visible, and Escape closes the dialog.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity | One session process that the office knows. | process id, start time, hash of the whole command line, display text of the command line (at most 1,000 characters), machine run, kind (CLI or helper), state (open or ended). At most 200 open helpers per session (FR-010). | Belongs to one session of one agent. Readable only by the office owner (FR-001). |
| Sweep result | What one sweep did with one stored process identity. | result (ended, ended by force, could not end, left), reason | Exactly one per stored process identity of a `lost` session (FR-009). Kept as an action record entry. |
| Session (extends 0013) | One run of an agent CLI. | sweep finding of a `lost` session: leftovers ended (with the number), may still run, or none found | Drives the mark of 0013/FR-008 (modified). |
| Action record entry (extends 0001, 0007) | One recorded action. | action now also "end leftover"; result ended, ended by force, could not end, or left; process id; command line in visible form; session | Actor is the office. Shows in the details of the agent that owns the session. |

## Success criteria

- **SC-001:** In a test where 30 stand-in CLIs, each with one helper in a session of its own, outlive a forced kill of the server, 100% of the 60 processes have ended before the "Resume interrupted" list shows, on Linux, macOS, and Windows in CI.
- **SC-002:** In the same test, 0 unrelated processes end: a process with a stored command line and another start time, a process that got a stored process id with another command line, and a process that the office did not start all still run after the sweep, and each stored identity that they meet has exactly 1 "left" entry with its reason.
- **SC-003:** With 30 `lost` sessions and the default grace period, the "Resume interrupted" list shows within 30 seconds after the server start.
- **SC-004:** 100% of ended leftovers have exactly 1 action record entry with the office as actor, the agent, the session, the process id, and the command line in visible form (control characters shown as escapes, at most 1,000 characters, then an ellipsis and the hash start). A stand-in that ignores the request to end is ended by force after the grace period and has the result "ended by force".
- **SC-005:** With 4 agents after a forced kill of the server (one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, and one session from before this change), the list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent.
- **SC-006:** After 10 clean server stops and 10 normal agent stops, each followed by a server start, the sweep ends 0 processes and 0 agents start without an action of the user.
- **SC-007:** When the server gets a forced kill during a sweep and started again, 100% of the leftovers have ended after the second start, and each stored process identity of the `lost` sessions has exactly 1 sweep result in the action record.
- **SC-008:** With a session whose CLI starts 500 helper processes that stay alive, the office stores at most 200 open helper identities for it, adds exactly 1 limit entry to the action record, and after a forced kill of the server the agent shows the 0013 warning.
- **SC-009:** With a stand-in CLI that replaces itself with another program while the session runs, after a forced kill of the server 100% of such processes end at the next start with the new command line in the action record.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| A `lost` session has no stored process identity (an office from before 0017). | Empty | The sweep ends nothing for it. The list keeps the 0013 mark and warning for its agent (0013/FR-008, modified). |
| No `lost` session at start. | Empty | No sweep runs and no sweep text shows (US2 scenario 5). |
| The process id of a stored identity belongs to another process now (process id reused). | Invalid | Not ended; "left" with "other start time" or "other command line" (FR-005). |
| A process runs the same command line as a stored identity but started at another time. | Invalid | Not ended; "left" with "other start time" (FR-005). |
| The CLI changed its own command line after the office stored it. | Invalid | Not ended; "left" with "other command line" (FR-005, A4). The agent shows no warning, because the office cannot tell it apart from an unrelated process. |
| The machine restarted since the unclean stop, and the OS gives an exact machine run (Linux). | Boundary | No stored identity matches; each is "left" with "machine restarted" (FR-005). No warning shows, because no process can survive a restart. |
| The boot time differs by more than 2 seconds, but the process id, start time, and command line all match (macOS, Windows; a clock correction can cause this). | Boundary | Not ended; "left" with "boot time changed" (FR-005). The agent keeps the warning, because a restart is not proven (0013/FR-008, modified). When the process id is not running or has another start time, the usual reason applies and no warning shows. |
| A leftover ends by itself during the sweep. | Concurrent | Recorded as "ended". Before each end request the office checks the match again, so a process id that is reused in the meantime is not ended (FR-003, US2 scenario 4). |
| The OS does not let the office read a process (permission denied). | Unauthorized | Not ended; "left" with "not readable" (FR-005). The agent keeps the warning (0013/FR-008, modified). |
| The OS does not let the office end a leftover (permission denied). | Unauthorized | "could not end" with the reason, one notification, and the warning in the list (FR-006). |
| A helper moved to a new parent or a session of its own while the session ran. | Boundary | It is still a session process and the sweep ends it when it matches (US1 scenario 3). |
| A helper started less than 10 seconds before the unclean stop. | Boundary | It can be missing from the stored identities and is then not ended (A2). |
| The server is killed again during the sweep. | Failure | The next start sweeps again over the stored identities that have no sweep result yet. A process that already ended is "left" with "not running". Each stored identity has exactly one sweep result (FR-009, SC-007). |
| The office cannot store a process identity during a session. | Failure | The session keeps running and the 0001 storage alert shows. At the next unclean stop, the session counts as having no complete stored identities, so its agent keeps the warning (0013/FR-008, modified). |
| The sweep cannot read the list of running processes at all. | Failure | The sweep ends nothing. Each stored identity is "left" with "not readable" (FR-005), and every agent of a `lost` session keeps the 0013 warning (0013/FR-008, modified). |
| 30 agents had `lost` sessions. | Boundary | All leftovers are asked to end at the same time, and the list shows within 30 seconds with the default grace period (SC-003). |
| The user starts an agent or opens the resume list during the sweep. | Concurrent | The request waits and runs after the sweep; the list shows after the sweep (FR-007). |
| A helper that the user's own CLI started to use after the unclean stop (a shared service). | Boundary | It still matches as a session process and is ended (A6). |
| The office is on hold (0007). | Boundary | The sweep runs; it is an action of the office, not an agent-made action. |
| A session process replaces itself with another program while the session runs (for example `sh -c 'exec node ...'`, or an npm wrapper). | Concurrent | The stored command line is replaced and the identity stays open (FR-011). After an unclean stop the process matches and is ended. |
| A session starts more than 200 helpers. | Boundary | The office stops storing helpers of that session, adds one entry, and the agent keeps the warning (FR-010, SC-008). Helpers already stored are still swept. |
| A command line is very long, or holds control, escape, or bidirectional-control characters (the agent chooses it). | Invalid | The match uses the whole line. The action record and the desktop show it in visible form, cut to 1,000 characters with an ellipsis and the hash start (FR-004, FR-005). The full line is not stored. |
| A leftover ends and its process id goes to another process between the last check and the end request (macOS, or a Linux kernel before 5.3). | Concurrent | A residual risk with no exception made for it (A10). Windows and Linux 5.3 and later end through a held process, so they do not have it. |
| A command line holds a secret, for example a token in an argument. | Boundary | It is stored readable only by the office owner (FR-001) and shown only in the owner's action record. It never leaves the machine (0001/FR-005). |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). The sweep runs without the app. |

## Assumptions

- **A1:** The sweep runs only for sessions that the server finds still open at its start, that is after an unclean stop (0013 `lost`). A clean stop and a normal session end use the stop of 0001 and need no sweep (FR-008).
- **A2:** The office sees a new descendant of the CLI, and a change of a stored command line, within 10 seconds (FR-002, FR-011). A helper that starts and leaves the tree, or a process that replaces itself, in the last 10 seconds before an unclean stop can be missed. The office does not end processes that it has no stored identity for, also when their parent is a leftover, so that the rule "all three values match" has no exception.
- **A3:** The grace period of the sweep is the grace period of a stop (0001 A3), so the user has one setting for both.
- **A4:** The command line is the one that the OS reports when the office stores the identity, not the one that the office used to start the CLI. A process that rewrites its own command line after the unclean stop no longer matches and is left. While the session runs, a changed command line of the same process is stored again (FR-011).
- **A5:** The sweep ends all leftovers in parallel, so its time depends on the grace period and not on the number of agents.
- **A6:** A helper that a session started, such as the `opencode` background service, belongs to that session even when another program uses it after the unclean stop. The sweep ends it when it matches; the CLI that needs it starts a new one.
- **A7:** The sweep writes one action record entry for each stored process identity of a `lost` session, also for "not running", so that the agent's details show what the office checked.
- **A8:** The notification of FR-006 comes once per server start and names all agents with a leftover that could not be ended. It is a notification of the office, like the automatic hold of 0013/FR-005.
- **A9:** One background server runs one office on one machine (0001 A8). Agents on other machines (0006, deferred) are not swept.
- **A10:** macOS has no handle to a process, and Linux before 5.3 has none either. There the office checks the match and then sends the end request to the process id. A process that exits and whose id is reused in that short time can be ended wrongly. The office accepts this risk only on these systems; the research records it.
- **A11:** The helper limit of 200 is fixed. A setting for it is out of scope.

## Out of scope

- Ending helpers that outlive a normal end of a session or a clean server stop, for example a service that a CLI keeps for its next start. The stop of 0001 stays as it is (PB-3).
- Ending a process that has no stored identity, also when it descends from a leftover (A2).
- Sweeping without a `lost` session, for example on a timer or by a user action.
- Agents on other machines (A9).
- A setting for the 10 seconds of FR-002 or for the helper limit (A11).
- Reading CLI output or CLI files to find out whether a session is busy.
