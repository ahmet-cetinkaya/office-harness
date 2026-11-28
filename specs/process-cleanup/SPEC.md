# Leftover processes and ended commands

## Intent

After an unclean stop the office ends the processes left over from the earlier run, and it keeps no command line of an ended session. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0017-orphan-sweep](../changes/archive/0017-orphan-sweep/SPEC.md)
- [0023-drop-ended-commands](../changes/archive/0023-drop-ended-commands/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0017/FR-008 | 0023 | Change 0017, Requirements |

Changes of other capabilities that this file names:

- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0019: [CLI sign-in check before a start](../cli-sign-in-check/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0017: End the processes left over from an unclean stop

### Intent

When the background server stops uncleanly (SIGKILL, a crash, or a power loss of the server process only), some agent CLI processes and their helper processes keep running. The next server start does not see them. They keep spending the owner's quota unseen, and they can keep a session busy, which can break the resume of 0013. This change makes the server remember, for each session, which processes belong to it. At the next start, before the user can resume anything, the server ends only the processes that are still exactly those processes, and records each end. The resume warning of 0013 then names only the agents that really had processes left.

### Requirements

- **0017/FR-001** (US1): WHEN the office starts a session, THE SYSTEM SHALL store the process identity of the session's CLI process before the session shows as started, and SHALL make it readable only by the OS user who owns the office (as 0001/FR-009). THE SYSTEM SHALL store the command line as a hash of the whole line, for the match, and as a display text of at most 1,000 characters.
- **0017/FR-002** (US1): WHILE a session runs, THE SYSTEM SHALL store the process identity of each process that descends from the session's CLI process within 10 seconds after that process starts, SHALL keep it while that process runs, also after it leaves the CLI's process tree, and SHALL drop it when that process ends.
- **0017/FR-003** (US1, US2): WHEN the server starts and finds `lost` sessions, THE SYSTEM SHALL run the sweep before it shows the list of the "Resume interrupted" action and before it starts any agent, SHALL ask each leftover to end and end by force each leftover that still runs after the grace period, and SHALL check the match again just before each end request. Where the OS lets the office hold a process (Linux 5.3 and later, and Windows), THE SYSTEM SHALL send each end request only to the process that it just checked, so that a process that gets the same process id later is never ended. On macOS, and on a Linux kernel without this, the time between the check and the end request is a known risk (see Notes from Deviations).
- **0017/FR-004** (US1): WHEN the sweep ends a leftover, THE SYSTEM SHALL add an entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the result "ended" or "ended by force".
- **0017/FR-005** (US2): IF a stored process identity of a `lost` session does not match a running process, THEN THE SYSTEM SHALL NOT end any process for it and SHALL add a "left" entry to the agent's action record with the office as actor, the session, the process id, the command line in visible form, and the reason: not running, machine restarted, boot time changed, other start time, other command line, or not readable.
- **0017/FR-006** (US1): IF a leftover still runs after the forced end, or the OS refuses to end it, THEN THE SYSTEM SHALL add a "could not end" entry with the reason and the command line in visible form to the agent's action record and SHALL notify the user once per server start, naming each agent with such a leftover.
- **0017/FR-007** (US1): WHILE the sweep runs, THE SYSTEM SHALL show, in the panel and in the "Resume interrupted" dialog, that the office ends processes left from the last run, and SHALL keep each start, new task, and continue request of the user waiting until the sweep ends, then run it.
- **0017/FR-008** (US2): WHEN a session ends while the server runs (a stop, an end of its CLI, a start failure, an end at the sign-in screen, or a clean server stop), THE SYSTEM SHALL remove every stored process identity of that session in the same step as the session end, so that no later sweep ends a process for it or records anything for it, and the office keeps no command line of that session's processes. (modified by 0023)
- **0017/FR-009** (US1, US2): WHEN the sweep records the sweep result of a stored process identity, THE SYSTEM SHALL mark that identity as ended in the same step, so that a later sweep, also after the server is killed during this sweep, neither ends a process for it nor records a second sweep result for it.
- **0017/FR-010** (US1): WHILE a session runs, THE SYSTEM SHALL store at most 200 open helper identities for it (the helper limit). WHEN a new helper would pass the limit, THE SYSTEM SHALL stop storing helpers of that session, SHALL add one entry to the agent's action record that names the limit, and the session SHALL count as having no complete stored process identities.
- **0017/FR-011** (US1): WHILE a session runs and a stored session process keeps its process id and start time but runs another command line (it replaced itself with another program), THE SYSTEM SHALL replace the stored command line with the new one within 10 seconds, and SHALL NOT mark that identity as ended.

This change modified: 0013/FR-008. The final text is under the change that owns that id.

Change 0023 modified 0017/FR-008. The text above is final.

### Preserved behaviors

- **0017/PB-1:** The server starts no agent by itself at its start (0001/FR-004). The sweep only ends processes; every `lost` agent stays `interrupted` until the user acts.
- **0017/PB-2:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021). The sweep reads process identities and ends processes; it does not touch any CLI file.
- **0017/PB-3:** The stop action ends the CLI and every process that descends from it at the time of the stop, first with a request and then by force after the grace period (0001/FR-012, 0001 A3), as today.
- **0017/PB-4:** The office never ends a process that it did not start: a process that is not a session process of a `lost` session is never ended by the sweep, also when its command line is the same as an agent's.
- **0017/PB-5:** The user's own actions work as before once the sweep ends, and the "Resume interrupted" action behaves as 0013/FR-001 to 0013/FR-007 define.
- **0017/PB-6:** After an unclean stop, the open sessions still end as `lost`, and the user can still continue a `lost` session (0013 Key entities, Session; 0001/FR-025).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity | One session process that the office knows. | process id, start time, hash of the whole command line, display text of the command line (at most 1,000 characters), machine run, kind (CLI or helper), state (open or ended). At most 200 open helpers per session (FR-010). | Belongs to one session of one agent. Readable only by the office owner (FR-001). |
| Sweep result | What one sweep did with one stored process identity. | result (ended, ended by force, could not end, left), reason | Exactly one per stored process identity of a `lost` session (FR-009). Kept as an action record entry. |
| Session (extends 0013) | One run of an agent CLI. | sweep finding of a `lost` session: leftovers ended (with the number), may still run, or none found | Drives the mark of 0013/FR-008 (modified). |
| Action record entry (extends 0001, 0007) | One recorded action. | action now also "end leftover"; result ended, ended by force, could not end, or left; process id; command line in visible form; session | Actor is the office. Shows in the details of the agent that owns the session. |

### Success criteria

- **0017/SC-001:** In a test where 30 stand-in CLIs, each with one helper in a session of its own, outlive a forced kill of the server, 100% of the 60 processes have ended before the "Resume interrupted" list shows, on Linux, macOS, and Windows in CI.
- **0017/SC-002:** In the same test, 0 unrelated processes end: a process with a stored command line and another start time, a process that got a stored process id with another command line, and a process that the office did not start all still run after the sweep, and each stored identity that they meet has exactly 1 "left" entry with its reason.
- **0017/SC-003:** With 30 `lost` sessions and the default grace period, the "Resume interrupted" list shows within 30 seconds after the server start.
- **0017/SC-004:** 100% of ended leftovers have exactly 1 action record entry with the office as actor, the agent, the session, the process id, and the command line in visible form (control characters shown as escapes, at most 1,000 characters, then an ellipsis and the hash start). A stand-in that ignores the request to end is ended by force after the grace period and has the result "ended by force".
- **0017/SC-005:** With 4 agents after a forced kill of the server (one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, and one session from before this change), the list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent.
- **0017/SC-006:** After 10 clean server stops and 10 normal agent stops, each followed by a server start, the sweep ends 0 processes and 0 agents start without an action of the user.
- **0017/SC-007:** When the server gets a forced kill during a sweep and started again, 100% of the leftovers have ended after the second start, and each stored process identity of the `lost` sessions has exactly 1 sweep result in the action record.
- **0017/SC-008:** With a session whose CLI starts 500 helper processes that stay alive, the office stores at most 200 open helper identities for it, adds exactly 1 limit entry to the action record, and after a forced kill of the server the agent shows the 0013 warning.
- **0017/SC-009:** With a stand-in CLI that replaces itself with another program while the session runs, after a forced kill of the server 100% of such processes end at the next start with the new command line in the action record.

### Notes from Deviations

- **One notification for each agent, one OS notification for each server start (0017/FR-006).** The server writes one `leftover_kept` notification for each agent with a leftover that could not be ended, because a notification refers to one agent and each agent keeps its own Needs you item. The desktop app sends one OS notification for each server start that names all those agents. It sends it when `office.sweep` says `done`, or 1.5 seconds after the first row for a client that connected during the sweep. A client that is not connected during the sweep shows the rows in its list, but no OS notification.
- **Platforms of the 60-process test (0017/SC-001).** The test with real stand-in processes runs on Linux and on macOS. On Linux it reads process start times from `/proc`. On macOS it reads them from `ps`, in whole seconds, the same unit the office stores there. On Windows the cover is the process inspection tests, which read the identity of real child processes and end them through the handle in CI on all three systems, and the sweep unit tests with a fake inspector.
- **Race on macOS and on a Linux kernel before 5.3 (assumption A10 of 0017).** There the OS gives no handle to a process, so the office checks the match and then sends the end request to the process id. A process that exits and whose id is reused in that short time can be ended wrongly. The office accepts this risk only on these systems. On macOS the start time is in whole seconds, so the recorder also cannot tell a process that replaced itself by `exec` from a new process that took the same id within the same second; the recorder then takes the new command line for the stored row.

## Change 0023: Keep no command lines of ended sessions

### Intent

Change 0017 stores the identity of each session process: its process id, its start time, a hash of its whole command line, and a display text of up to 1,000 characters of that command line. When a session ends normally, the office marks these identities as ended but keeps them. Nothing reads them after that, but a helper command line can hold a secret, for example `curl -H "Authorization: ..."`. This change makes the office remove the stored identities of a session when it ends normally, remove identities that an earlier build left behind, and remove the identities of an interrupted session once the "Resume interrupted" list no longer reads them. It also makes sure that, after the next server start, the database files keep no copy of a command line that left the store: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identities, also when an earlier build freed it. The sweep, its match, and the action record stay as they are.

### Requirements

- **0023/FR-001** (US3): WHEN the office stores a new session of an agent, THE SYSTEM SHALL remove each ended stored process identity of every earlier `lost` session of that agent.
- **0023/FR-002** (US2): WHEN the server starts, THE SYSTEM SHALL, after it ends the sessions that are still open as `lost`, remove each stored process identity of a session that ended normally, and each ended stored process identity of a `lost` session that is not the last session of its agent.
- **0023/FR-003** (US2, US3): THE SYSTEM SHALL keep each open stored process identity of a `lost` session, and each stored process identity of a `lost` session that is the last session of its agent, until FR-001 or FR-002 removes it or its agent is deleted.
- **0023/FR-004** (US1, US2): WHEN the server starts, THE SYSTEM SHALL leave in the office database no copy of a display text freed before that start (Terms: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identity), other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **0023/FR-005** (US2): WHEN the server starts for the first time under this build, THE SYSTEM SHALL leave in the office database no copy of a display text that an earlier build freed, other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **0023/FR-006** (US2): IF the server cannot finish FR-004 or FR-005 at a start, THEN THE SYSTEM SHALL write the reason to the server's log, SHALL start as it would otherwise, and SHALL try again at each later start until it finishes.

This change modified: 0017/FR-008. The final text is under the change that owns that id.

### Preserved behaviors

- **0023/PB-1:** After an unclean stop, the sweep still checks every open stored process identity of every `lost` session, ends each leftover with the same match, and records exactly one sweep result for each identity before the "Resume interrupted" list shows (0017/FR-003 to 0017/FR-007, 0017/FR-009).
- **0023/PB-2:** The "Resume interrupted" list still marks an agent whose last session is `lost` with the same finding: a note with the number of ended leftovers, the warning that a CLI process may still run (could not end, not readable, boot time changed, no complete stored identities, helper limit, not swept), or no mark (0013/FR-008 as modified by 0017).
- **0023/PB-3:** Each action record entry of the sweep keeps its text: the office as actor, the session, the process id, the command line in visible form, and the result (0017/FR-004 to 0017/FR-006).
- **0023/PB-4:** After clean server stops and normal agent stops, the next server start ends 0 processes and starts 0 agents without an action of the person (0017/SC-006, 0001/FR-004).
- **0023/PB-5:** While a session runs, the office still stores the CLI identity before the session shows as started, stores helper identities within 10 seconds, drops a helper identity when the helper ends, replaces the command line after an `exec`, and applies the helper limit of 200 (0017/FR-001, 0017/FR-002, 0017/FR-010, 0017/FR-011).
- **0023/PB-6:** The person can still continue any previous session of an agent from its details, also a `lost` one (0013/PB-2, 0001/FR-025).

### Terms

Terms that this change defines. The other terms are as in 0001, 0013, 0017, and 0019.

| Term | Meaning |
|---|---|
| Session ended normally | A session that ended while the server ran: a stop, an end of its CLI, a start failure, an end at the sign-in screen (0019), or a clean server stop (0017/FR-008). Every session end except `lost`. |
| Remove | Delete the stored process identity with all its values. Blanking only the display text is not a remove (A2). |
| Office database | The owner-only storage of the office (0001/FR-009): the database file and its write log and shared-memory files (`-wal`, `-shm`), with the freed space inside them. Not the disk blocks that the OS freed when one of these files shrank (A4). |
| Freed display text | The display text of a stored process identity after it leaves the store: the identity is removed (FR-001, FR-002, 0017/FR-008), a helper identity is dropped when its helper ends (0017/FR-002), the old command line is replaced after an `exec` (0017/FR-011), or the identity is deleted with its agent. |

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity (changes 0017) | One session process that the office knows. | As in 0017. The lifetime changes: it exists while its session runs; after a `lost` end, while it is open or while its session is the last session of its agent. | Belongs to one session of one agent. Removed with its session end, with a new session of its agent, or at a server start, as FR-001, FR-002, and 0017/FR-008 define. Once its display text is freed (Terms), no copy of it stays in the office database after the next server start (FR-004). |
| Session (extends 0013, 0017) | One run of an agent CLI. | end result: ended normally or `lost` | A session that ended normally has no stored process identity. Only the last session of an agent, when it is `lost`, keeps ended identities. |
| Action record entry of the sweep (0017) | What the sweep did with one stored identity. | unchanged | Keeps its text after the identity it names is removed. |

### Success criteria

- **0023/SC-001:** After 20 sessions end normally (10 stops by the person, 5 ends of the CLI, 5 by clean server stops), each with a CLI and a helper whose command line holds a unique marker that no terminal output shows, the office holds 0 stored process identities of those 20 sessions right after each end.
- **0023/SC-002:** After the next server start that follows those 20 ends, a byte search of the database file and its `-wal` and `-shm` files finds 0 of the 20 markers.
- **0023/SC-003:** With an office database from the 0017 build that holds 10 sessions that ended normally, each with a marker in a stored command line, plus 10 markers in helper identities that the 0017 build dropped and 10 markers in command lines that it replaced after `exec`, the first start of this build leaves 0 identities of those sessions, the byte search of the database file and its `-wal` and `-shm` files finds 0 of the 30 markers, and the sweep ends 0 processes.
- **0023/SC-004:** In the 0017/SC-005 test (4 agents after a forced kill of the server: one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, one session from before 0017), the "Resume interrupted" list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent, also after a second server start without a resume.
- **0023/SC-005:** After the person resumes the 4 agents of SC-004, the office holds 0 ended identities of their `lost` sessions, and 100% of the sweep entries of those sessions in the action record show the same text as before the resume.
- **0023/SC-006:** In the 0017/SC-001 test (30 stand-in CLIs, each with one helper, outlive a forced kill of the server), 100% of the 60 processes still end before the "Resume interrupted" list shows.
- **0023/SC-007:** In 10 sessions, each with a helper whose command line holds a unique marker (as in SC-001) and that ends 15 seconds after it starts while its session still runs, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 markers.
- **0023/SC-008:** In 10 sessions whose CLI replaces itself with another program after the office stored it, with a unique marker only in the old command line, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 old markers.

### Notes from Deviations

- **No requirement is deferred or changed.** The Deviations section of 0023 is empty.
- **One checkpoint for two steps (plan D4 and D5).** The checkpoint after the start deletes and the checkpoint after the rebuild share one `wal_checkpoint(TRUNCATE)`, which runs after the `VACUUM` when the rebuild is pending. Two checkpoints would log a busy result twice on the first start. One checkpoint after the rebuild still leaves the write log empty.
- **Linux only for the sweep test (0023/SC-004, 0023/SC-005).** These criteria need the sweep of 0017, which runs on Linux only, so their test runs on Linux only. The other tests of the change run on Linux and macOS in CI.
- **The VACUUM error path is only logged.** No test makes the `VACUUM` fail. The busy checkpoint is tested (0023/FR-006).
- **Assumptions that the text names.** The bare `A2` and `A4` in the text above name these assumptions of 0023:
  - **A2:** The office removes the whole stored identity (option a), and does not only blank the display text and keep the hash (option b). Reasons: an ended identity of a session that ended normally has no reader at all, so the hash, process id, and start time have no use either; a hash of a short secret can be found by trying values; a remove also stops the store from growing with each session. Nothing reads these identities, so the remove changes no visible behavior.
  - **A4:** Between the time a display text is freed and the next server start, a copy of it can stay in the office database, readable only by the owner (0001/FR-009). FR-004 bounds this to the next server start (Clarifications). The byte search of FR-004, FR-005, and the criteria covers the database file and its `-wal` and `-shm` files, with the freed space inside them. It does not cover disk blocks that the OS freed when one of these files shrank or was truncated, nor swap or file system journals. The office cannot reach them, as A8 says for backup copies, so a test does not scan the disk.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0017-orphan-sweep - 2026-11-23
- 0023-drop-ended-commands - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
