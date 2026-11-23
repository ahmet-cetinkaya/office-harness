---
status: draft
capability: office
plan:
supersedes: 0001/FR-014 (in part), 0001/FR-020, 0008/FR-002 (event names), 0012/FR-002 (in part), 0018/FR-002 (in part), 0001 A1 and 0003 A3 (for `claude` sessions)
---

# 0024: Exact agent status from CLI hooks, `claude` first

## Intent

The office guesses what an agent does from its terminal output: after a quiet period with no output it shows `waiting for input`, and any output makes it `working` again. The guess fails in four ways that the person sees. An agent that finished a task shows `waiting for input` only after 5 seconds. A `claude` that redraws a hint line while it waits flips to `working` and back (0022 had to add a dedupe for that). A finished task and a request for permission look the same, so Needs you (0008) says only "quiet for 5 seconds". And the office cannot know that a permission request is open, so it relies on a screen check (0012) and an item kind (0018) to keep directives and sleep away from it. `claude` can report these events itself: a turn starts, a turn stops, it asks for permission, it compacts its context. This change lets the office give each `claude` session its own settings file, outside every CLI folder, that makes the CLI report these four events to the office on the same machine, with a secret that belongs to that session alone. The office then shows exact status for that session, and falls back to today's guess at once when a report is missing or late. The person's own hooks and settings keep working. Other CLIs do not change. The event model is meant to grow: later changes add more events, such as tool use for spend limits, without changing the four here.

## Context

- Source of truth: `docs/PRD.md`. References such as `0012/FR-002` point to requirements of the approved changes or of the living spec of the office.
- Owner decision B (2026-11-23): the office may start each `claude` session with an office-owned settings file. The file lives outside every CLI folder, the office never edits the CLI's own configuration, and the file only layers over the person's settings for that session.
- This change depends on 0001 (statuses, the quiet period, sessions, the stop), 0003 (notifications), 0008 (Needs you), 0012 (the delivery gate and its screen check), 0018 (idle sleep), 0020 (temps), and 0022 (waits, prompted and unprompted changes).
- This change modifies 0001/FR-014 and 0001/FR-020, adds event names to 0008/FR-002, and adds exact-session rules to 0012/FR-002 and 0018/FR-002. It lifts one out-of-scope line of 0008 ("signals from CLI hooks or CLI configuration") for `claude` sessions. 0001 A1 and 0003 A3 ("the office does not tell finished apart from asks a question") stay true for every session that is not exact.
- The requirements describe behavior only. The plan decides how to build them, including which events of the CLI map to the four events.
- Rows 7, 8, and 10 of the gap list (tool and loop events, spend limits, deny answers, other CLIs) are out of scope. This change keeps the event model open for them (FR-022).

## Terms

| Term | Meaning |
|---|---|
| Agent, session, status, quiet period, recent output, action record, background server, desktop app | As defined in 0001. |
| Notification, Needs you, item, attention event | As defined in 0003 and 0008. |
| Wait end, prompted change, unprompted change, wait | As defined in 0022. |
| Delivery gate, question check, person input | As defined in 0012. |
| Idle sleep, plain waiting item, idle since | As defined in 0018 and 0022. |
| Hook | A command that the CLI runs at a named moment of its work, here to tell the office what just happened. |
| Report | One message from a hook of a session to the office. It names the session and one event kind. |
| Event kind | One of: turn started, turn stopped, turn failed, permission asked, compaction started, compaction ended. This change defines these six. |
| Turn | The work of the CLI on one prompt: from the moment the CLI takes a prompt to the moment it stops with its answer or with an error. Work of the CLI's own helper sub-agents is part of the turn of the main conversation, not a turn of its own. |
| Permission ask | The state from a report of the event kind permission asked until it ends (FR-013). |
| Office settings file | The file that the office makes for one `claude` session and that holds only the status hooks of this change. It exists only while that session runs. |
| Session secret | The random value that the office makes for one session. A report is valid only with the secret of the session it names. |
| Exact session | A session of a CLI that the office starts with the office settings file, from its first accepted report on, while its reports arrive in time. |
| Estimated session | A session that has had no accepted report yet, or that runs without the office settings file. It follows 0001, 0003, 0008, 0012, 0018, and 0022 unchanged. |
| Late session | A session that had an accepted report and now follows the estimated rules because a turn seems stalled (FR-014). The next accepted report makes it exact again. |
| Status source | `exact`, `estimated`, or `late`: which of the three above a session is in. |

## User stories

### US1: The status of a `claude` agent follows its turns (Priority: P1)

The person gives a `claude` agent a prompt. When the answer is complete, the agent shows `waiting for input` within a second, not after the quiet period. While the agent waits and redraws a hint line, the status does not move to `working` at all.

**Why this priority:** this is the core of the gap. The quiet period is the root of the late status, the flip-flop, and the 0022 workaround. Every other story builds on an exact status.

**Independent test:** start a `claude` agent. Send it a prompt. Measure the time from the end of its answer to the status `waiting for input`. Then leave it for 10 minutes with no input and count the status changes.

**Scenarios:**

1. Given a `claude` agent in an exact session, When a turn starts, Then the status is `working` within 1 second, and it stays `working` until the turn stops.
2. Given an open turn, When the turn stops, Then the status is `waiting for input` within 1 second, with 1 item and 1 notification as for any prompted change (0022).
3. Given an exact session that waits, When the CLI redraws its hint line, Then the status stays `waiting for input`, and no item or notification appears (output alone does not change the status).
4. Given an open turn, When the CLI auto-compacts its context, Then the status stays `working`, and the person can see that it compacts (US5).
5. Given the person runs the compaction by hand while no turn is open, When it ends, Then the status changes from `working` back to `waiting for input`.
6. Given a turn that ended with an error such as a rate limit, When the report arrives, Then the status is `waiting for input` and the item says that the turn ended with an error.

### US2: Needs you and the gates know a permission ask (Priority: P1)

A `claude` agent asks for permission to run a tool. The person sees `waiting for input`, a notification "asks permission", and an item that says so, instead of "quiet for 5 seconds". A directive from a parent is not typed into the open ask, and the agent does not go to sleep while the ask is open, even after the person dismissed its item.

**Why this priority:** a permission ask is the case where a wrong guess costs most: a typed directive can answer it, and a sleep ends the CLI with a question open. Today the office detects it from the screen and may miss it.

**Independent test:** make a `claude` agent ask permission for a tool. Check the status, the item text, and the notification. Send 5 directives and wait 3 times the idle time with idle sleep on. Answer the ask by keyboard and check that the status becomes `working`.

**Scenarios:**

1. Given a `claude` agent working, When it asks for permission, Then within 1 second the status is `waiting for input`, the item and the notification say that it asks permission, and the item sorts with `waiting for input` (0008/FR-003).
2. Given an open permission ask, When a parent sends a directive, Then the directive waits in its queue and nothing is typed (0012/FR-002, as modified).
3. Given an open permission ask and idle sleep on, When the idle time has passed, Then the office does not put the agent to sleep, also when the person dismissed its item or opened its terminal earlier.
4. Given an open permission ask, When a person input reaches the terminal and the turn is still open, Then the ask ends and the status is `working`.
5. Given an open permission ask, When the turn stops, Then the ask ends and the status is `waiting for input` with the item event "finished its turn".
6. Given an agent that finished a turn, When it waits, Then its item says that it finished its turn, not that it was quiet.
7. Given a temp with an open permission ask whose item the person dismissed, When its idle limit passes, Then the office does not end the temp.

### US3: Hooks never get in the way, and a missing hook costs nothing (Priority: P1)

The person already has his own hooks and settings for `claude`. They all keep running. If the office hooks cannot report, because the CLI never starts them, the office is not reachable, or the CLI is too old, the agent works as before and the status follows the quiet period as it does today.

**Why this priority:** the office adds something to every `claude` session. If it could break the CLI, slow it down, change its answers, or lock the person out of his own setup, the feature would cost more than it gives. The owner's decision B is conditional on this.

**Independent test:** configure a user hook that writes a marker at each turn. Run 10 turns with the office hooks and 10 without: the marker count is the same. Stop the office's listener and run 10 turns: they finish, each event delayed by at most 2 seconds, and the status follows the quiet period.

**Scenarios:**

1. Given a person with his own hooks and settings for `claude`, When the office starts a session, Then every one of his hooks still runs, in the same way, and none of his settings changes.
2. Given the CLI never starts the office hooks (hooks turned off in the person's or a managed policy's settings, or a CLI without hooks), When the session runs, Then it stays an estimated session and behaves as today.
3. Given an exact session, When the office cannot be reached for a report, Then the CLI is not stopped, blocked, or changed, a report delays it by at most 2 seconds, and the session becomes late when a turn stalls (FR-014).
4. Given a turn that the person interrupts with the Escape key, so that no stop report comes, When the screen has no output for the quiet period, Then the session becomes late and the status is `waiting for input`.
5. Given an agent whose command or office additions already pass a settings option to the CLI, When the office starts a session, Then it adds no file, the session is estimated, and the agent's details say so.
6. Given agents on `codex`, `gemini`, `opencode`, or a custom command, When the office starts a session, Then no file is made, no option is added, and the status follows 0001 as before.
7. Given the person turned the hook status off (FR-024), When a `claude` session starts, Then no file is made and the session is estimated.

### US4: Only the session's own CLI can report (Priority: P1)

A report can change the displayed status of one session and nothing else. It is accepted only from the same machine, only with that session's secret, and only while the session runs. Nobody can use it to read a terminal, type into one, or control an agent. The report's content, such as the prompt text, is dropped.

**Why this priority:** the office opens a new way in. The person's terminals, his prompts, and his agents are valuable, and the office promises that data stays on his machine (0001/FR-005 to FR-009).

**Independent test:** send reports with no secret, a wrong secret, the secret of another session, the secret of an ended session, a size of 1 MB, and from a non-loopback address. Count the status changes (expected: 0). Run a turn with a prompt that holds a unique marker text and search the office's stored data and logs for the marker.

**Scenarios:**

1. Given a valid report of a running session, When it arrives, Then the office applies its event kind to that session and to no other.
2. Given a report with an unknown session, a wrong secret, or a session that ended, When it arrives, Then the office changes nothing and gives one same answer for all three, so that the sender cannot tell them apart.
3. Given a report from another machine, When it arrives, Then it is not accepted (0001/FR-008).
4. Given a session that ended for any reason, When its secret is used, Then the report is refused.
5. Given a report that carries the prompt, the answer, a tool input, or a file path, When the office handles it, Then it keeps none of that content anywhere.
6. Given a session that ended or a server that stops, When the office has cleaned up, Then no office settings file of that session remains.

### US5: The person sees where the status comes from (Priority: P2)

The agent's details say whether its status is exact or estimated, and the agent list says in text when an agent asks permission or compacts.

**Why this priority:** a status that the person cannot place leaves him unsure whether to trust it. This is polish on top of US1 and US2, so it can ship after them.

**Independent test:** open the details of a `claude` agent and of a `codex` agent, and check the status source text. Make the `claude` agent ask permission and compact, and check the list.

**Scenarios:**

1. Given an exact session, When the person opens the agent's details, Then they show `exact` as the status source.
2. Given an estimated or late session, When the person opens the details, Then they show `estimated` or `late`, with a one-line reason in text.
3. Given a permission ask or a compaction, When the person looks at the agent list, Then the status text carries `asks permission` or `compacting` next to the status.

## Requirements

### ADDED

- **FR-001** (US3): WHEN the system starts a session of an agent whose CLI is `claude`, THE SYSTEM SHALL start the CLI with an office settings file for that session, and SHALL NOT make or pass such a file for a session of any other CLI.
- **FR-002** (US3): THE SYSTEM SHALL keep the office settings file in the office's own storage, outside every CLI's configuration files and folders, readable only by the OS user who owns the office, SHALL put only the status hooks of this change into it, and SHALL NOT create, change, or delete any file in a CLI's configuration files or folders.
- **FR-003** (US3): THE SYSTEM SHALL leave every hook and setting that the CLI would use without the office settings file in effect, and every hook of the person SHALL run as without it.
- **FR-004** (US3): IF the agent's command or its office additions already pass a settings option to the CLI, THEN THE SYSTEM SHALL NOT add the office settings file, SHALL treat the session as estimated, and SHALL say why in the agent's details.
- **FR-005** (US3): THE SYSTEM SHALL make the status hooks report only: they SHALL NOT allow, deny, block, or change any action, prompt, tool, or permission decision of the CLI, and SHALL NOT add text to what the CLI shows or reads.
- **FR-006** (US3): THE SYSTEM SHALL NOT let one report delay the CLI by more than 2 seconds, and a refused or failed report SHALL NOT stop, block, or change the CLI's work.
- **FR-007** (US4): THE SYSTEM SHALL make one session secret per session, SHALL accept a report only with the secret of the session it names, SHALL NOT let that secret do anything but deliver reports of that session, and SHALL NOT show it in any view, notification, action record, or log, store it in the office's database or the agent's stored additions, or put it on the command line of the CLI. The CLI's own environment holds the secret, so the CLI's tools can read it, and a terminal print of it can reach the stored output that the CLI itself produces (A3).
- **FR-008** (US4): THE SYSTEM SHALL accept reports only from the same machine (0001/FR-008), and IF a report names an unknown session, a session that ended, or carries a wrong secret, THEN THE SYSTEM SHALL change nothing and SHALL give the same answer in all three cases.
- **FR-009** (US4): THE SYSTEM SHALL keep from each report only its event kind, the session it names, and whether it comes from one of the CLI's helper sub-agents (FR-023), SHALL discard every other content of it without storing, logging, showing, or sending it, SHALL refuse a report larger than 64 KiB, and SHALL drop reports of one session beyond 50 per second.
- **FR-010** (US4): WHEN a session ends for any reason, THE SYSTEM SHALL revoke its secret and delete its office settings file, and WHEN the background server starts, THE SYSTEM SHALL delete every office settings file that does not belong to a session that runs.
- **FR-011** (US1): WHILE a session is exact, THE SYSTEM SHALL show `working` from a turn started report until the next turn stopped or turn failed report, and while a compaction runs, and `waiting for input` otherwise and while a permission ask is open (FR-013), and output alone SHALL NOT change the status. Every change of status of an exact session is a prompted change (0022 Terms), so the quiet period, the unprompted change, and the work bound do not apply to it.
- **FR-012** (US1): WHEN a turn stopped or turn failed report of an exact session ends the last open turn and no compaction runs, THE SYSTEM SHALL show `waiting for input` within 1 second, with the event `finished its turn` or `its turn ended with an error`.
- **FR-013** (US2): WHEN a permission asked report arrives for an exact session, THE SYSTEM SHALL show `waiting for input` within 1 second, with the event `asks permission`, and THE SYSTEM SHALL end the permission ask at the first of: a person input that reaches the terminal, a turn stopped or turn failed report, a turn started report, a compaction report, or the end of the session. WHEN the ask ends and a turn is still open, THE SYSTEM SHALL show `working`.
- **FR-014** (US3): IF, in an exact session, a turn is open, no permission ask is open, no compaction runs, and the terminal has produced no output for the quiet period, THEN THE SYSTEM SHALL treat the session as late: it SHALL follow the estimated rules, so that the status is `waiting for input` with the event of an estimated session, until the next accepted report makes the session exact again.
- **FR-015** (US3): WHILE a session has had no accepted report, or runs without the office settings file, THE SYSTEM SHALL follow 0001, 0003, 0008, 0012, 0018, and 0022 without change.
- **FR-016** (US2): THE SYSTEM SHALL NOT type a directive into an exact session WHILE a permission ask is open or a compaction runs, in addition to the conditions of 0012/FR-002, and the screen checks of 0012 SHALL stay in force.
- **FR-017** (US2): THE SYSTEM SHALL NOT put an exact session to sleep WHILE a permission ask is open, also when the agent has no item in Needs you, and SHALL treat an item whose latest event is `asks permission` as not plain (0022 Terms).
- **FR-018** (US1, US2): THE SYSTEM SHALL show the event of an item and of a notification for an exact session as `finished its turn`, `its turn ended with an error`, or `asks permission`, and for an estimated or late session as before (`quiet for the quiet period`).
- **FR-019** (US5): THE SYSTEM SHALL show the status source of each session, `exact`, `estimated`, or `late`, as text in the agent's details, with a short reason when it is not `exact`.
- **FR-020** (US5): THE SYSTEM SHALL show `asks permission` or `compacting` as text next to the status of an agent in the agent list, while it applies.
- **FR-021** (US3): THE SYSTEM SHALL NOT change the status, the notifications, the items, the gates, or the sleep of a session of a CLI other than `claude`.
- **FR-022** (US4): THE SYSTEM SHALL treat a report with an event kind that it does not know as no event: no change of status, no error to the CLI, and no effect on other reports.
- **FR-023** (US1): THE SYSTEM SHALL ignore the reports of the CLI's own helper sub-agents for the status of the agent: only the main conversation opens and closes a turn.
- **FR-024** (US3): THE SYSTEM SHALL let the person turn the hook status off for the whole office, with the default on. WHEN it is off, THE SYSTEM SHALL make no office settings file, and the change SHALL apply from the next session start and be said so in the setting, as in 0001/FR-023.
- **FR-025** (US2): THE SYSTEM SHALL NOT end a temp for its idle limit (0020/FR-004) WHILE a permission ask of an exact session of that temp is open, also when its item is gone.

### MODIFIED

- **0001/FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`, or `asleep`; for an estimated or late session, `waiting for input` means that the CLI runs and produced no output during the quiet period; for an exact session it means that no turn runs and no compaction runs, or that a permission ask is open (0024/FR-011, 0024/FR-013); and `asleep` means that the office ended the CLI after an idle time and keeps its last session for a wake (0018/FR-003).
- **0001/FR-020** (US3): THE SYSTEM SHALL start each agent CLI with the user's own home folder, login environment, and CLI configuration, changed only by that agent's office additions and, for `claude`, by the office settings file of 0024/FR-001, which only adds the status hooks.
- **0008/FR-002** (US2): THE SYSTEM SHALL show each item with the agent's name, project, latest event as the office saw it (for an exact session `finished its turn`, `its turn ended with an error`, or `asks permission`; otherwise quiet for the quiet period, bell, exit with its exit code, put on hold by the office with the limit it hit, or a report with its kind), and wait time, as text, and, while the item holds a report, the report's kind and its whole text in visible form.
- **0012/FR-002** (US2): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, no unsent person text waits at its input line, no pending office text (a startup prompt or an unsubmitted directive) waits in the target's current session, the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval), and, WHILE the target's session is exact (0024), no permission ask is open and no compaction runs (0024/FR-016); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **0018/FR-002** (US2): WHILE idle sleep is on, THE SYSTEM SHALL begin to put an agent to sleep within 60 seconds of the first moment at which the agent shows `waiting for input`, the idle time has passed since its idle-since moment, and none of these blocking conditions holds: (a) the agent does not show `waiting for input`; (b) it has an item in Needs you that is not a plain waiting item, that is, an item that recorded a bell, a failure, an exit, an automatic hold, a resume hold, a kept leftover, or a permission ask (0024), an item that holds a report for the person, or a waiting item whose agent's screen shows an open question under the question check of 0012/FR-002; (c) a hold covers it (0007/FR-004); (d) a directive or a report waits in its queue (0012, 0015); (e) pending office text or unsent person text waits at its input line (0012); (f) one of its children runs; (g) the agent's session is exact and a permission ask is open, also when the agent has no item (0024/FR-017). The idle-since moment is the latest of the agent's last prompted change to `waiting for input`, the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time; an unprompted change does not move it. THE SYSTEM SHALL check all seven conditions again right before it ends the CLI, and SHALL NOT end it when one of them holds.

### REMOVED

None.

### PRESERVED

- **PB-1:** The statuses are the eight of 0001/FR-014. This change adds no status. Every status still shows as text, never color alone (0005 PB-3, 0018 PB-7).
- **PB-2:** An estimated session follows the quiet period exactly as before, including the dedupe of 0022 for unprompted changes (FR-015). A CLI other than `claude` follows 0001 as before (FR-021).
- **PB-3:** The office writes nothing into a CLI's configuration files or folders, the person's `~/.claude` included, and reads none of the CLI's files to find the status (0001/FR-021, 0012 PB-4). The office settings file lives in the office's own storage.
- **PB-4:** The screen checks of 0012 and 0018 stay, for all sessions. A report can only make the office stricter (a permission ask, a compaction) or faster (a turn stop), never skip a screen check.
- **PB-5:** The server and its terminals stay reachable only from the same machine, only with the office token, as before (0001/FR-007 to FR-010). The session secret is a separate and narrower credential (FR-007).
- **PB-6:** Holds, rate limits, and the action record work as before (0007). Reports add no action record entry.
- **PB-7:** A bell, an exit, a failure, an automatic hold, a resume hold, and a report for the person (0015) notify and change items as before (0022 PB-4).
- **PB-8:** Stop, sleep, wake, and resume of sessions work as before (0001/FR-024, 0001/FR-025, 0018). A resumed or woken session gets a new office settings file and a new secret.

## User experience

The desktop app gets one setting and a few words of text. No new view.

**Flow:**

1. The person starts or wakes a `claude` agent. It runs as before.
2. The person sends it a prompt. The status shows `working`. When the answer is complete, the status shows `waiting for input` within a second. Needs you shows 1 item: `finished its turn`, with the wait time.
3. The agent asks permission for a tool. The agent list shows `waiting for input` with `asks permission`. Needs you shows 1 item, and 1 notification says `asks permission`.
4. The person answers by keyboard. The status shows `working`; the item leaves.
5. The agent compacts. The list shows `working` with `compacting`.
6. In the details of the agent, the status source reads `exact`. For a `codex` agent it reads `estimated`.
7. In the settings, the person can turn the hook status off. The setting says that the change applies from the next session start.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Exact, working | `working`; status source `exact`. | FR-011, FR-019 |
| Exact, turn done | `waiting for input` within 1 s; item `finished its turn`. | FR-012, FR-018 |
| Exact, turn failed | `waiting for input`; item `its turn ended with an error`. | FR-012, FR-018 |
| Exact, permission ask | `waiting for input` with `asks permission`; item and notification say so; no directive typed; no sleep. | FR-013, FR-016, FR-017, FR-020 |
| Exact, compacting | `working` with `compacting`. | FR-011, FR-020 |
| Estimated | As today; status source `estimated` with the reason: no report yet, hooks off, a settings option in the command or the office additions, or another CLI. | FR-004, FR-015, FR-019 |
| Late | The quiet-period status; status source `late` with the reason `no report for a stalled turn`. | FR-014, FR-019 |
| Hook status off | Setting off; every session `estimated`; reason `hook status is off`. | FR-024 |
| Empty, loading, server not reachable, unauthorized | As in 0001. | 0001/FR-026, 0001/FR-027 |

**Design:** no fixed design exists, and none is needed beyond one setting row and text in the details and the list.

**Accessibility:** the setting is reachable and operable by keyboard (0001/FR-028). Status source, `asks permission`, and `compacting` are text, never color alone, and part of the text that a screen reader reads for the row. The count of Needs you is announced as before without moving focus (0008).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Hook state of a session | What the office knows about one running session from its reports. | status source, turns open, whether a permission ask is open, whether a compaction runs, time of the last accepted report | One per running session. Lives only while the session runs; a server stop ends it. It is not stored and not shown except as the status source and the words of FR-020. |
| Session secret | The proof that a report comes from the session's own CLI. | value (never shown), session | One per session. Revoked at the end of the session (FR-010). |
| Office settings file | The status hooks for one session. | session, path | One per exact-capable `claude` session. Deleted at the end of the session (FR-010). |
| Report | One event of one session. | session, event kind | Transient. Nothing else of it is kept (FR-009). |
| Hook setting | The person's switch for the hook status. | on or off | One per office; default on. Part of the office's settings (0001). |

## Success criteria

- **SC-001:** In 20 turns of a real signed-in `claude` agent, 20 of 20 changes to `waiting for input` show within 1 second of the end of the answer, and 20 of 20 changes to `working` show within 1 second of the prompt.
- **SC-002:** A real `claude` agent that answers one prompt and then gets no input for 30 minutes shows 0 changes to `working` and causes exactly 1 `waiting for input` notification, with no dependence on the work bound of 0022.
- **SC-003:** In 10 permission asks of a real `claude` agent, 10 of 10 show `waiting for input` with `asks permission` within 1 second, 0 of 50 directives sent during the asks are typed, and with idle sleep on, 0 sleeps occur in 3 times the idle time, also when the person dismissed the item.
- **SC-004:** Of 100 reports with no secret, a wrong secret, the secret of another session, an unknown session, an ended session, or a size above 64 KiB, 0 change any status, 100% of the first four kinds get the same answer, and 0 reports from an address other than the machine's own are accepted.
- **SC-005:** After a session ends by each of exit, failure, stop, sleep, start failure, and server stop, 0 office settings files remain and the old secret is refused 100% of the time. After a server crash and restart, 0 files of earlier runs remain.
- **SC-006:** With a user hook that writes a marker at each turn, 10 of 10 turns write the marker with and without the office settings file, the person's hook answers are not changed, and the person's settings files have the same contents before and after the session.
- **SC-007:** With the office's listener unreachable, 10 of 10 turns finish, no event delays the CLI by more than 2 seconds, and the status of each session follows the quiet period.
- **SC-008:** For agents on `codex`, `gemini`, and `opencode`, 0 office settings files are made, 0 options are added to their command lines, and the 0001 status tests give the same results as before.
- **SC-009:** During a test session of `claude`, office-harness changes 0 files in the CLI's configuration files and folders, `~/.claude` included (0001/SC-005 extended).
- **SC-010:** A test prompt with a unique marker text runs through 10 turns. The marker is in 0 rows of the office's stored data and in 0 lines of its log. The terminal output store holds it as the CLI printed it.
- **SC-011:** In 10 turns that the person interrupts with the Escape key, 10 of 10 sessions show `waiting for input` within the quiet period plus 1 second, with the status source `late`.
- **SC-012:** With the hook status off, 100% of new `claude` sessions are estimated and make 0 office settings files.
- **SC-013:** A temp with an open permission ask is ended 0 times in 3 times its idle limit, also after the person dismissed its item.
- **SC-014:** After 10 sessions, the secrets of all of them are in 0 rows of the office's database, 0 lines of its log, and 0 views, notifications, and action record entries.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The session has had no report yet, for example at the start, or while the CLI waits for the person to trust the folder. | Empty | Estimated session: today's rules (FR-015). |
| A turn started report arrives while a turn is open. | Boundary | The turn stays open; one turn stopped report ends it. |
| A turn stopped report arrives with no open turn. | Boundary | The office shows `waiting for input` if it does not already; no item or notification is added for a status that did not change. |
| A compaction runs inside a turn and the CLI prints nothing for longer than the quiet period. | Boundary | The session is not late: a compaction counts as work (FR-014). |
| The person answers a permission ask with the keyboard, and the ask ends at the first key. | Boundary | The status shows `working` while the CLI may still wait. The screen check of 0012 still guards the directive (PB-4). Accepted as A5. |
| A report names a session that was woken or resumed, with the secret of its earlier run. | Invalid | Refused: a new run has a new secret (FR-007, PB-8). |
| A report has a body that is not valid, or an event kind that is not known. | Invalid | No effect on the status; the CLI is not blocked (FR-022, FR-006). |
| Two reports arrive at the same moment from one session. | Concurrent | The office applies them in the order of arrival, one at a time. |
| A person input and a permission asked report arrive at the same moment. | Concurrent | The person input is a wait end; the ask opens after it and ends at the next input or event. |
| The CLI's helper sub-agent stops or asks. | Concurrent | No change of the agent's status (FR-023). |
| The installed `claude` does not know the settings option, or rejects the office settings file at start. | Failure | The session still starts, without the file, as an estimated session with that reason. The person is never worse off than today (US3). |
| The person's or a managed policy turns hooks off for `claude`. | Failure | No report arrives; the session stays estimated (US3 scenario 2). |
| The office's listener is not reachable when a hook runs. | Failure | The hook fails without blocking the CLI, after at most 2 seconds; the session becomes late when a turn stalls (FR-006, FR-014). |
| The background server restarts while a session of the CLI runs. | Failure | The session ends with the server and shows `interrupted` (0001/FR-004); its file is deleted (FR-010). |
| The settings file cannot be written. | Failure | The session starts without it, as an estimated session with that reason. |
| A process of the same OS user, or the CLI's own tool or an injected model, reads the secret from the CLI's environment and forges reports for that session. | Unauthorized | It can change the displayed status of that one session, for example end an open permission ask early, and nothing else: the screen checks of 0012 and 0018 stay in force, and the secret reads no terminal, types nothing, and controls no agent (A3). |
| The desktop app cannot reach the server, or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). The server keeps the hook state. |

## Assumptions

- **A1:** The facts about `claude` that this change relies on come from the CLI's documentation and are checked by the plan on the installed version, in a hands-on run: an extra settings file layers over the person's settings and keeps all their hooks; a hook that fails does not block the CLI; hooks are held back until the person trusts the folder; the Escape key ends a turn without a stop report; the events of FR-011 to FR-013 exist. Where a fact is wrong, the session is estimated and this spec's fallback holds.
- **A2:** Reports of one session arrive in the order in which the events happened, because the CLI waits for each report before it goes on. The office applies them in the order of arrival.
- **A3:** A process of the same OS user is as trusted as the person (0001). The session secret keeps other OS users, other machines, and web pages out. It does not keep out a process of the same user or the CLI's own tools: they run with the secret in their environment, and an injected model can print it into the terminal, where the stored output keeps it. A forged report can change the displayed status of its own session only. It can end an open permission ask early, and the screen checks of 0012 and 0018 then still guard a directive and a sleep (A5). Nothing in this change tries to hide the secret from the CLI.
- **A4:** A turn of the CLI draws a moving indicator, so an open turn that makes no output for a full quiet period is not working: it was interrupted or its report was lost (FR-014). A CLI setting that turns the indicator off can make a long silent turn look late. The next report makes the session exact again.
- **A5:** The end of a permission ask at the first person input is a simplification. The office has no event for the answer. The screen check of 0012 and the open-question check of 0018 stay, so an ask that is still open after an early end is still guarded.
- **A6:** The hook status is on by default. The person can turn it off for the whole office (FR-024). A per-agent switch is not offered.
- **A7:** The first accepted report makes a session exact. A session whose first report comes late stays estimated until then.
- **A8:** The 2-second delay limit (FR-006), the 64 KiB size (FR-009), and the 50 reports per second (FR-009) are defaults that the plan may tighten, not loosen.
- **A9:** The status of an exact session is not stored. A server stop ends every session, so no hook state outlives the server (0001/FR-004).
- **A10:** A session that the office starts for the sign-in check of 0019 or for any purpose other than an agent's own run gets no office settings file.

## Out of scope

- Tool-use events, loop detection, spend and token limits, and deny answers (gap rows 7, 8, and 10). The event model keeps room for them (FR-022): a later change adds event kinds without changing the meaning of the six here.
- Hooks for `codex`, `gemini`, `opencode`, and custom commands. Each family needs its own change.
- Reading which tool a permission ask is for, or any prompt or answer text (FR-009).
- Answering a permission ask for the person, or any decision by the office through a hook (FR-005).
- Changing the quiet period, the statuses, or the screen checks of 0012 and 0018 (PB-1, PB-4).
- A new Needs you group for permission asks or a different sort order (0008/FR-003 stays).
- Showing a history of reports, and storing them (A9).
- Hooks on a remote machine (0006).

## Clarifications

### Session 2026-11-23

- Q: May the office give each `claude` session its own settings file? → A: Yes, owner decision B, 2026-11-23: the file lives outside every CLI folder, never edits `~/.claude`, and layers over the person's settings for that session (FR-001, FR-002, FR-003).
- Q: What happens when hooks are missing or late? → A: The office falls back to today's behavior: estimated until the first report, late when a turn stalls (FR-014, FR-015). Owner decision B, 2026-11-23.
- Q: Can the person turn the hook status off? → A: Yes, one office-wide switch, default on (FR-024, A6). Default adopted by the spec writer; review may change it.
- Q: Is a permission ask a status of its own? → A: No. It is `waiting for input` with the event `asks permission`, so the eight statuses stay (PB-1, FR-013). Default adopted by the spec writer; review may change it.
- Q: Does the first session start count as a report? → A: No. Only the four event groups of this change count, so a freshly started agent follows the quiet period until its first turn (A7). Default adopted by the spec writer.

## Deviations
