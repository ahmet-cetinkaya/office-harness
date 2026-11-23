---
status: shipped
capability: office
plan: specs/changes/archive/0023-drop-ended-commands/PLAN.md
supersedes: 0017/FR-008
---

# 0023: Keep no command lines of ended sessions

<!-- Approved for coding on 2026-11-23 through the review gate (r2 at 5901646), under the owner's spec delegation; shipped in PR #136. -->

## Intent

Change 0017 stores the identity of each session process: its process id, its start time, a hash of its whole command line, and a display text of up to 1,000 characters of that command line. When a session ends normally, the office marks these identities as ended but keeps them. Nothing reads them after that, but a helper command line can hold a secret, for example `curl -H "Authorization: ..."`. This change makes the office remove the stored identities of a session when it ends normally, remove identities that an earlier build left behind, and remove the identities of an interrupted session once the "Resume interrupted" list no longer reads them. It also makes sure that, after the next server start, the database files keep no copy of a command line that left the store: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identities, also when an earlier build freed it. The sweep, its match, and the action record stay as they are.

## Context

- Source of truth: `docs/PRD.md`. The living spec is `specs/office/SPEC.md`; 0017 is folded into it under "Change 0017". References such as `0017/FR-008` point to requirements there.
- The problem comes from the review of PR #126 (0017), note N-d. The reviewer did not block on it, because the office database is readable only by its owner (0017/FR-001, 0001/FR-009), and asked for a follow-up.
- This change depends on 0001 (sessions, the stop, the clean server stop, the action record), 0013 (the `lost` session end, the last session, the "Resume interrupted" list), and 0017 (stored process identities, the sweep, the sweep finding).
- Readers and writers of the stored identities on main at 5272d6f:
  - Store at session start and during the session: `src/modules/agent/infrastructure/src/persistence/repositories/session_process_repository.rs:35-64` (the insert refuses a session that has ended, line 50). The display text is the first 1,000 characters of the raw command line: `src/modules/terminal/application/src/common/models/process_identity.rs:7`, `:60`. Table: `src/modules/agent/infrastructure/src/persistence/migrations/0008_session_processes.sql:11-28`.
  - Change while the session runs: a new command line after `exec` (`session_process_repository.rs:103-116`, 0017/FR-011) and the delete of an ended helper (`session_process_repository.rs:122-130`, 0017/FR-002).
  - Normal session end: `session_repository.rs:179-203` marks every open identity of the session as ended (`session_process_repository.rs:226-232`) in the same transaction as the session row. Callers: the stop (`src/modules/agent/application/src/common/session_ender.rs:103`), an end of the CLI (`session_launcher.rs:418`), the clean server stop (`src/modules/agent/application/src/commands/interrupt_agents/interrupt_agents_handler.rs:115`), and the end-session request (`end_session_handler.rs:33`). These rows keep `command_display` and get no sweep result.
  - Unclean stop: at server start, `src/apps/office-harnessd/src/bootstrap/services.rs:123-124` ends every open session as `lost` and leaves its identities open (`session_repository.rs:213-221`).
  - Sweep: reads only the open identities of `lost` sessions (`src/apps/office-harnessd/src/sweep/orphan_sweep.rs:113`, `session_process_repository.rs:88-97`). It copies the command line in visible form into the action record entry (`src/apps/office-harnessd/src/sweep/leftover_sweep.rs:158-170`) and marks the identity as ended with its sweep result in the same transaction (`src/apps/office-harnessd/src/sweep/leftover_ledger.rs:44-48`, `session_process_repository.rs:147-158`).
  - "Resume interrupted" list (`resume.plan`): reads the sweep finding only when the agent's last session is `lost` (`src/apps/office-harnessd/src/server/resume_requests.rs:193-213`, last session at `:215`). The finding counts sweep results, open identities, CLI identities, and the helper limit flag (`session_process_repository.rs:164-196`). Zero CLI identities gives the warning "no stored process" (`src/apps/office-harnessd/src/server/unclean_stop.rs:32-47`). It never reads the command line.
  - Recorder: reads only the open identities of a running session (`src/modules/agent/application/src/common/session_process_recorder.rs:118`).
  - No other reader: the desktop app does not read the command line of a stored identity, and `end_for_session` (`session_process_repository.rs:202`) has callers in tests only.
  - The database runs in write-ahead log mode with no secure delete (`src/apps/office-harnessd/src/bootstrap/database.rs:12`). A removed record can stay readable in freed space or in the log until it is written over. This already holds for each helper identity dropped at the end of its helper and each command line replaced after `exec`, also in a database that the 0017 build wrote.
- Result: an ended identity of a session that did not end as `lost` has no reader. An ended identity of a `lost` session has a reader only while that session is the last session of its agent. An open identity of a `lost` session has a reader: the next sweep.
- Spec review r1 of PR #136 found that FR-004 first covered only the identities that this change removes, and not the texts that a helper drop, an `exec` replacement, or an agent delete frees (M1). It also asked which bytes the search covers (L1). FR-004 to FR-006, the Terms, A4, A8, and A9 hold the result.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, action record | As defined in 0001. |
| Unclean stop, `lost` session, last session | As defined in 0013. The last session is the newest session of an agent. |
| Stored process identity, sweep, sweep result, command line in visible form | As defined in 0017. |
| Session ended normally | A session that ended while the server ran: a stop, an end of its CLI, a start failure, an end at the sign-in screen (0019), or a clean server stop (0017/FR-008). Every session end except `lost`. |
| Remove | Delete the stored process identity with all its values. Blanking only the display text is not a remove (A2). |
| Office database | The owner-only storage of the office (0001/FR-009): the database file and its write log and shared-memory files (`-wal`, `-shm`), with the freed space inside them. Not the disk blocks that the OS freed when one of these files shrank (A4). |
| Freed display text | The display text of a stored process identity after it leaves the store: the identity is removed (FR-001, FR-002, 0017/FR-008), a helper identity is dropped when its helper ends (0017/FR-002), the old command line is replaced after an `exec` (0017/FR-011), or the identity is deleted with its agent. |

## User stories

### US1: A session leaves no command line behind once the office no longer needs it (Priority: P1)

The person stops an agent whose CLI ran a helper with a secret in its command line. After the stop, the office keeps no part of that command line. A helper that ended earlier in the session, or a command line that a process replaced, leaves no copy either after the next server start.

**Why this priority:** it is the value of the change. Every normal session end today keeps command lines that can hold secrets, and nothing uses them.

**Independent test:** start an agent on a stand-in CLI that starts a helper whose command line holds a unique marker. Stop the agent. The office holds 0 stored process identities of that session. Restart the server. A byte search of the office database for the marker finds 0 matches. Repeat with a helper that ends while its session runs, and with a process that replaces itself by `exec`: after the restart, the search finds 0 copies of the old texts.

**Scenarios:**

1. Given a running session with a stored CLI identity and a stored helper identity, When the person stops the agent, Then the office removes both identities in the same step as the session end.
2. Given a running session, When its CLI ends by itself, Then the office removes every stored identity of that session in the same step as the session end.
3. Given running sessions, When the person stops the server cleanly, Then each session ends as interrupted, and the office removes every stored identity of those sessions.
4. Given a session that ended normally, When the server starts again, Then the sweep finds no identity of that session and ends no process for it.
5. Given a removed identity whose command line held a marker, When the server has started again after the removal, Then a byte search of the office database finds no copy of the marker.
6. Given a running session whose helper, with a marker in its command line, was stored and then ended, When the server has started again, Then a byte search of the office database finds no copy of the marker.
7. Given a stored session process that replaced itself with another program, so that the office replaced its stored command line (0017/FR-011), When the server has started again, Then a byte search of the office database finds no copy of the old command line.

### US2: Command lines stored by an earlier build are removed (Priority: P2)

The person installs this build over an office that already stored identities under 0017. At the next server start, the office removes every stored identity that no reader needs.

**Why this priority:** without it, every command line stored before the update stays forever. It comes after US1 because it is a one-time catch-up of the same rule.

**Independent test:** take an office database from the 0017 build with 10 sessions that ended normally and 1 agent whose last session is `lost`. The 0017 build also dropped helper identities and replaced command lines, each with a marker. Start this build. 0 identities of the 10 sessions remain, a byte search of the office database finds 0 of the markers, and the "Resume interrupted" list shows the same mark for the `lost` agent as before.

**Scenarios:**

1. Given ended identities of sessions that ended normally, When the server starts, Then the office removes them.
2. Given ended identities of a `lost` session that is not the last session of its agent, When the server starts, Then the office removes them.
3. Given open identities of a `lost` session, When the server starts, Then the office keeps them, and the sweep checks them as 0017 defines.
4. Given the ended identities of a `lost` session that is the last session of its agent, When the server starts, Then the office keeps them, and the "Resume interrupted" list shows the same mark as before.
5. Given an office database in which the 0017 build dropped helper identities and replaced command lines, When this build starts for the first time, Then a byte search of the office database finds no copy of those freed display texts.
6. Given the first start of this build could not clear those copies, When the server starts again, Then it tries again, until a start clears them.

### US3: An interrupted session's identities go once the agent runs again (Priority: P3)

After an unclean stop, the person resumes an agent. The office no longer needs the identities of the interrupted session, so it removes them.

**Why this priority:** the action record already holds the command line of each swept identity, so the exposure is small, and US2 also removes these identities at the next server start. This story only shortens the time that they stay.

**Independent test:** after a forced kill of the server and a sweep, resume one agent. The office holds 0 ended identities of that agent's `lost` session, and the action record still shows each sweep entry of that session with its command line.

**Scenarios:**

1. Given the last session of an agent is `lost` and the sweep recorded a result for each of its identities, When the office stores a new session of that agent (resume, continue, new task, or start), Then the office removes the ended identities of that `lost` session.
2. Given a `lost` session that still has an open identity (the sweep could not record it), When the office stores a new session of that agent, Then the office keeps the open identity for the next sweep.
3. Given the office removed the identities of a `lost` session, When the person opens the agent's details, Then each sweep entry of that session still shows the process id, the session, the command line in visible form, and the result.

## Requirements

### ADDED

- **FR-001** (US3): WHEN the office stores a new session of an agent, THE SYSTEM SHALL remove each ended stored process identity of every earlier `lost` session of that agent.
- **FR-002** (US2): WHEN the server starts, THE SYSTEM SHALL, after it ends the sessions that are still open as `lost`, remove each stored process identity of a session that ended normally, and each ended stored process identity of a `lost` session that is not the last session of its agent.
- **FR-003** (US2, US3): THE SYSTEM SHALL keep each open stored process identity of a `lost` session, and each stored process identity of a `lost` session that is the last session of its agent, until FR-001 or FR-002 removes it or its agent is deleted.
- **FR-004** (US1, US2): WHEN the server starts, THE SYSTEM SHALL leave in the office database no copy of a display text freed before that start (Terms: a removed identity, a dropped helper, a replaced command line, or a deleted agent's identity), other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **FR-005** (US2): WHEN the server starts for the first time under this build, THE SYSTEM SHALL leave in the office database no copy of a display text that an earlier build freed, other than a copy inside an action record entry or inside a stored process identity that the office still keeps.
- **FR-006** (US2): IF the server cannot finish FR-004 or FR-005 at a start, THEN THE SYSTEM SHALL write the reason to the server's log, SHALL start as it would otherwise, and SHALL try again at each later start until it finishes.

### MODIFIED

- **0017/FR-008** (US1): WHEN a session ends while the server runs (a stop, an end of its CLI, a start failure, an end at the sign-in screen, or a clean server stop), THE SYSTEM SHALL remove every stored process identity of that session in the same step as the session end, so that no later sweep ends a process for it or records anything for it, and the office keeps no command line of that session's processes.

### PRESERVED

- **PB-1:** After an unclean stop, the sweep still checks every open stored process identity of every `lost` session, ends each leftover with the same match, and records exactly one sweep result for each identity before the "Resume interrupted" list shows (0017/FR-003 to 0017/FR-007, 0017/FR-009).
- **PB-2:** The "Resume interrupted" list still marks an agent whose last session is `lost` with the same finding: a note with the number of ended leftovers, the warning that a CLI process may still run (could not end, not readable, boot time changed, no complete stored identities, helper limit, not swept), or no mark (0013/FR-008 as modified by 0017).
- **PB-3:** Each action record entry of the sweep keeps its text: the office as actor, the session, the process id, the command line in visible form, and the result (0017/FR-004 to 0017/FR-006).
- **PB-4:** After clean server stops and normal agent stops, the next server start ends 0 processes and starts 0 agents without an action of the person (0017/SC-006, 0001/FR-004).
- **PB-5:** While a session runs, the office still stores the CLI identity before the session shows as started, stores helper identities within 10 seconds, drops a helper identity when the helper ends, replaces the command line after an `exec`, and applies the helper limit of 200 (0017/FR-001, 0017/FR-002, 0017/FR-010, 0017/FR-011).
- **PB-6:** The person can still continue any previous session of an agent from its details, also a `lost` one (0013/PB-2, 0001/FR-025).

## User experience

none: no user interface

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Stored process identity (changes 0017) | One session process that the office knows. | As in 0017. The lifetime changes: it exists while its session runs; after a `lost` end, while it is open or while its session is the last session of its agent. | Belongs to one session of one agent. Removed with its session end, with a new session of its agent, or at a server start, as FR-001, FR-002, and 0017/FR-008 define. Once its display text is freed (Terms), no copy of it stays in the office database after the next server start (FR-004). |
| Session (extends 0013, 0017) | One run of an agent CLI. | end result: ended normally or `lost` | A session that ended normally has no stored process identity. Only the last session of an agent, when it is `lost`, keeps ended identities. |
| Action record entry of the sweep (0017) | What the sweep did with one stored identity. | unchanged | Keeps its text after the identity it names is removed. |

## Success criteria

- **SC-001:** After 20 sessions end normally (10 stops by the person, 5 ends of the CLI, 5 by clean server stops), each with a CLI and a helper whose command line holds a unique marker that no terminal output shows, the office holds 0 stored process identities of those 20 sessions right after each end.
- **SC-002:** After the next server start that follows those 20 ends, a byte search of the database file and its `-wal` and `-shm` files finds 0 of the 20 markers.
- **SC-003:** With an office database from the 0017 build that holds 10 sessions that ended normally, each with a marker in a stored command line, plus 10 markers in helper identities that the 0017 build dropped and 10 markers in command lines that it replaced after `exec`, the first start of this build leaves 0 identities of those sessions, the byte search of the database file and its `-wal` and `-shm` files finds 0 of the 30 markers, and the sweep ends 0 processes.
- **SC-004:** In the 0017/SC-005 test (4 agents after a forced kill of the server: one leftover that ends, one that cannot be ended, one CLI that ended on the hang-up, one session from before 0017), the "Resume interrupted" list shows the note for exactly 1 agent, the warning for exactly 2 agents, and no mark for exactly 1 agent, also after a second server start without a resume.
- **SC-005:** After the person resumes the 4 agents of SC-004, the office holds 0 ended identities of their `lost` sessions, and 100% of the sweep entries of those sessions in the action record show the same text as before the resume.
- **SC-006:** In the 0017/SC-001 test (30 stand-in CLIs, each with one helper, outlive a forced kill of the server), 100% of the 60 processes still end before the "Resume interrupted" list shows.
- **SC-007:** In 10 sessions, each with a helper whose command line holds a unique marker (as in SC-001) and that ends 15 seconds after it starts while its session still runs, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 markers.
- **SC-008:** In 10 sessions whose CLI replaces itself with another program after the office stored it, with a unique marker only in the old command line, the byte search of the database file and its `-wal` and `-shm` files after the next server start finds 0 of the 10 old markers.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| A session ends normally and had no stored identity (the store failed, or it ended before the CLI identity was stored). | Empty | Nothing to remove; the session end is as before (0017/FR-008). |
| The office database holds no stored identity at a server start. | Empty | FR-002 removes nothing; the start is as before. |
| A stored display text is empty or holds control characters. | Invalid | The identity is removed like any other; the content of the text does not matter. |
| A session over the helper limit ends normally. | Boundary | All its stored identities, at most 200 helpers and the CLI, are removed (0017/FR-008). |
| The last session of an agent is `lost`, and the person never resumes it. | Boundary | Its ended identities stay, because the "Resume interrupted" list reads them (FR-003). The action record already holds the same command line in visible form for each of them (A3). |
| The last session of an agent is `lost`, and the sweep could not run for it. | Boundary | Its open identities stay, the list warns "not swept" as before (PB-2), and the next sweep checks them (FR-003). |
| An earlier `lost` session of an agent still has an open identity when a new session starts. | Boundary | The open identity stays for the next sweep (FR-003); FR-001 removes only ended identities. |
| A helper is seen, or its command line changes, at the same moment that its session ends. | Concurrent | No stored identity of that session remains after the end (0017/FR-008). The office stores no identity for a session that has ended, as before. |
| The person starts an agent while the sweep runs. | Concurrent | The start waits for the sweep (0017/FR-007), so FR-001 never runs before the sweep records its results. |
| The server is killed during a session end. | Failure | The session end and the removal are one step: either both happened, or the session is still open and ends as `lost` at the next start, with its identities open for the sweep. |
| The server is killed during the removal at its start. | Failure | The next start removes what is left (FR-002). |
| The removal at a server start fails, for example a storage failure. | Failure | The server writes the reason to its log and starts as before; the next start tries again. The sweep and the "Resume interrupted" list are not affected. |
| The removal at a session end fails. | Failure | The session end fails as a whole, as a failed session end does today (0017/FR-008). |
| An agent is deleted. | Boundary | Its sessions and their stored identities are deleted with it, as before. Their display texts are freed display texts; after the next server start, FR-004 leaves no copy of them. |
| A helper ends seconds after the office stored it, while its session runs. | Boundary | Its identity is dropped as before (0017/FR-002). Its display text is a freed display text; after the next server start, FR-004 leaves no copy of it. |
| A helper ends before the office could store it. | Empty | Nothing was stored, so there is nothing to clear (0017 A2). |
| A stored process replaces its command line several times while its session runs. | Boundary | Each old command line is a freed display text; FR-004 covers each of them. |
| The server runs for days without a restart. | Boundary | Freed display texts can stay in the office database until the next start (A4, Clarifications). |
| Another program holds the office database open at a server start, so the clearing cannot finish. | Concurrent | The server writes the reason to its log, starts as before, and tries again at the next start (FR-006). |
| The first start of this build cannot clear the copies that an earlier build freed, for example because the disk has no room for it. | Failure | The server writes the reason to its log, starts as before, and tries again at each later start until it finishes (FR-006). |
| A person or program other than the owner tries to read the office database. | Unauthorized | Refused by the OS, as before (0001/FR-009). This change only shortens what is there to read. |

## Assumptions

- **A1:** "Ended normally" covers every session end except `lost`, also the `interrupted` end of a clean server stop. These sessions get no sweep (0017 A1), so their identities have no reader after the end.
- **A2:** The office removes the whole stored identity (option a), and does not only blank the display text and keep the hash (option b). Reasons: an ended identity of a session that ended normally has no reader at all, so the hash, process id, and start time have no use either; a hash of a short secret can be found by trying values; a remove also stops the store from growing with each session. Nothing reads these identities, so the remove changes no visible behavior.
- **A3:** The ended identities of a `lost` session that is still the last session of its agent keep their display text until FR-001 or FR-002 removes them. Each of them has a sweep entry in the action record with the same command line in visible form (0017 A7), so they add no new exposure. Blanking their text would change the sweep's own step, which is out of scope.
- **A4:** Between the time a display text is freed and the next server start, a copy of it can stay in the office database, readable only by the owner (0001/FR-009). FR-004 bounds this to the next server start (Clarifications). The byte search of FR-004, FR-005, and the criteria covers the database file and its `-wal` and `-shm` files, with the freed space inside them. It does not cover disk blocks that the OS freed when one of these files shrank or was truncated, nor swap or file system journals. The office cannot reach them, as A8 says for backup copies, so a test does not scan the disk.
- **A5:** The removal at a server start runs at each start, not once. It finds nothing after the first start unless an earlier removal failed or a `lost` session stopped being the last session, so it costs little and needs no record that it already ran. The clearing of copies that an earlier build freed (FR-005) is different: it runs once, at the first start of this build that finishes it, because its cost grows with the size of the database.
- **A6:** A removal adds no action record entry, no notification, and no log line on success. The action record is a record of actions on agents, and the removal changes nothing that the person sees.
- **A7:** An action record entry of the sweep names the stored identity by its number. After the identity is removed, a new identity can get the same number. The entry also names the session and the process id, so entries stay distinct, and nothing reads that number back.
- **A8:** Copies of the office database that the owner or a backup tool made before the first start of this build keep the old command lines, and so do the disk blocks outside the database files (A4). The office cannot reach them.
- **A9:** How to build FR-004 to FR-006 is for the plan. As non-binding context, spec review r1 of PR #136 suggests: secure delete on for the office connection (`secure_delete`), so that each delete or update overwrites the freed bytes; a checkpoint that truncates the write log (`wal_checkpoint(TRUNCATE)`) after the removal at each start; and, once at the first start of this build, a rebuild of the database (`VACUUM`) followed by the same checkpoint. It does not suggest a rebuild at every start, because its cost grows with the database.

## Out of scope

- Any change to the sweep, the match, the sweep results, or the text of action record entries (PB-1, PB-3).
- Blanking the display text of identities when the sweep records them (A3).
- Removing or shortening command lines that the action record already holds.
- Stored terminal output, and the transcripts that the CLIs claude, codex, and gemini keep in their own folders.
- Encryption of the office database.
- A setting for how long ended identities stay.
- Command line secrets in running sessions: the stored identity of a running process keeps its display text while the process runs, because the sweep needs it after an unclean stop.
- Freed space of other stored data, such as action record entries or stored terminal output.
- Disk blocks outside the database files, swap, and file system journals (A4).

## Clarifications

- Q: Delete the stored process identities of a normally ended session, or blank their display text and keep the hash? → A: Delete them (A2). Decided under the owner's spec delegation, 2026-11-23.
- Q: When must the freed space and the write log hold no copy of a removed display text? → A: By the next server start (FR-004). A copy may stay while the server runs; only the owner can read the database. Decided under the owner's spec delegation, 2026-11-23. The plan weighs a secure delete for these removals and a write-log checkpoint at the start.

## Deviations

None yet.
