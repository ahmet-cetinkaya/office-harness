# Exact agent status from CLI hooks

## Intent

The status of an agent comes exactly from the hooks of its CLI, `claude` first, and falls back to the estimate from terminal output. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0024-cli-hook-status](../changes/archive/0024-cli-hook-status/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0019: [CLI sign-in check before a start](../cli-sign-in-check/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0018, 0022: [Idle sleep](../idle-sleep/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0005: [Office scene](../office-scene/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)
- 0020: [Temporary agents](../temporary-agents/SPEC.md)

## Change 0024: Exact agent status from CLI hooks, `claude` first

### Intent

The office guesses what an agent does from its terminal output: after a quiet period with no output it shows `waiting for input`, and any output makes it `working` again. The guess fails in four ways that the person sees. An agent that finished a task shows `waiting for input` only after 5 seconds. A `claude` that redraws a hint line while it waits flips to `working` and back (0022 had to add a dedupe for that). A finished task and a request for permission look the same, so Needs you (0008) says only "quiet for 5 seconds". And the office cannot know that a permission request is open, so it relies on a screen check (0012) and an item kind (0018) to keep directives and sleep away from it. `claude` can report these events itself: a turn starts, a turn stops, it asks for permission, it compacts its context. This change lets the office give each `claude` session its own settings file, outside every CLI folder, that makes the CLI report these four events to the office on the same machine, with a secret that belongs to that session alone. The office then shows exact status for that session, and falls back to today's guess at once when a report is missing or late. The person's own hooks and settings keep working. Other CLIs do not change. The event model is meant to grow: later changes add more events, such as tool use for spend limits, without changing the four here.

### Requirements

- **0024/FR-001** (US3): WHEN the system starts a session of an agent whose CLI is `claude`, THE SYSTEM SHALL start the CLI with an office settings file for that session, and SHALL NOT make or pass such a file for a session of any other CLI.
- **0024/FR-002** (US3): THE SYSTEM SHALL keep the office settings file in the office's own storage, outside every CLI's configuration files and folders, readable only by the OS user who owns the office, SHALL put only the status hooks of this change into it, and SHALL NOT create, change, or delete any file in a CLI's configuration files or folders.
- **0024/FR-003** (US3): THE SYSTEM SHALL leave every hook and setting that the CLI would use without the office settings file in effect, and every hook of the person SHALL run as without it.
- **0024/FR-004** (US3): IF the agent's command or its office additions already pass a settings option to the CLI, THEN THE SYSTEM SHALL NOT add the office settings file, SHALL treat the session as estimated, and SHALL say why in the agent's details.
- **0024/FR-005** (US3): THE SYSTEM SHALL make the status hooks report only: they SHALL NOT allow, deny, block, or change any action, prompt, tool, or permission decision of the CLI, and SHALL NOT add text to what the CLI shows or reads.
- **0024/FR-006** (US3): THE SYSTEM SHALL NOT let one report delay the CLI by more than 2 seconds, and a refused or failed report SHALL NOT stop, block, or change the CLI's work.
- **0024/FR-007** (US4): THE SYSTEM SHALL make one session secret per session, SHALL accept a report only with the secret of the session it names, SHALL NOT let that secret do anything but deliver reports of that session, and SHALL NOT show it in any view, notification, action record, or log, store it in the office's database or the agent's stored additions, or put it on the command line of the CLI. The CLI's own environment holds the secret, so the CLI's tools can read it, and a terminal print of it can reach the stored output that the CLI itself produces (A3).
- **0024/FR-008** (US4): THE SYSTEM SHALL accept reports only from the same machine (0001/FR-008), and IF a report names an unknown session, a session that ended, or carries a wrong secret, THEN THE SYSTEM SHALL change nothing and SHALL give the same answer in all three cases.
- **0024/FR-009** (US4): THE SYSTEM SHALL keep from each report only its event kind, the session it names, and whether it comes from one of the CLI's helper sub-agents (FR-023), SHALL discard every other content of it without storing, logging, showing, or sending it, SHALL refuse a report larger than 64 KiB, and SHALL drop reports of one session beyond 50 per second.
- **0024/FR-010** (US4): WHEN a session ends for any reason, THE SYSTEM SHALL revoke its secret and delete its office settings file, and WHEN the background server starts, THE SYSTEM SHALL delete every office settings file that does not belong to a session that runs.
- **0024/FR-011** (US1): WHILE a session is exact, THE SYSTEM SHALL show `working` from a turn started report until the next turn stopped or turn failed report, and while a compaction runs, and `waiting for input` otherwise and while a permission ask is open (FR-013), and output alone SHALL NOT change the status. Every change of status of an exact session is a prompted change (0022 Terms), so the quiet period, the unprompted change, and the work bound do not apply to it.
- **0024/FR-012** (US1): WHEN a turn stopped or turn failed report of an exact session ends the last open turn and no compaction runs, THE SYSTEM SHALL show `waiting for input` within 1 second, with the event `finished its turn` or `its turn ended with an error`.
- **0024/FR-013** (US2): WHEN a permission asked report arrives for an exact session, THE SYSTEM SHALL show `waiting for input` within 1 second, with the event `asks permission`, and THE SYSTEM SHALL end the permission ask at the first of: a person input that reaches the terminal, a turn stopped or turn failed report, a turn started report, a compaction report, or the end of the session. WHEN the ask ends and a turn is still open, THE SYSTEM SHALL show `working`.
- **0024/FR-014** (US3): IF, in an exact session, a turn is open or a compaction runs, no permission ask is open, and the terminal has produced no output for the quiet period, THEN THE SYSTEM SHALL treat the session as late: it SHALL follow the estimated rules, so that the status is `waiting for input` with the event of an estimated session, until the next accepted report makes the session exact again.
- **0024/FR-015** (US3): WHILE a session has had no accepted report, or runs without the office settings file, THE SYSTEM SHALL follow 0001, 0003, 0008, 0012, 0018, and 0022 without change.
- **0024/FR-016** (US2): THE SYSTEM SHALL NOT type a directive into an exact session WHILE a permission ask is open or a compaction runs, in addition to the conditions of 0012/FR-002, and the screen checks of 0012 SHALL stay in force.
- **0024/FR-017** (US2): THE SYSTEM SHALL NOT put an exact session to sleep WHILE a permission ask is open, also when the agent has no item in Needs you, and SHALL treat an item whose latest event is `asks permission` as not plain (0022 Terms).
- **0024/FR-018** (US1, US2): THE SYSTEM SHALL show the event of an item and of a notification for an exact session as `finished its turn`, `its turn ended with an error`, or `asks permission`, and for an estimated or late session as before (`quiet for the quiet period`).
- **0024/FR-019** (US5): THE SYSTEM SHALL show the status source of each session, `exact`, `estimated`, or `late`, as text in the agent's details, with a short reason when it is not `exact`.
- **0024/FR-020** (US5): THE SYSTEM SHALL show `asks permission` or `compacting` as text next to the status of an agent in the agent list, while it applies.
- **0024/FR-021** (US3): THE SYSTEM SHALL NOT change the status, the notifications, the items, the gates, or the sleep of a session of a CLI other than `claude`.
- **0024/FR-022** (US4): THE SYSTEM SHALL treat a report with an event kind that it does not know as no event: no change of status, no error to the CLI, and no effect on other reports.
- **0024/FR-023** (US1): THE SYSTEM SHALL ignore the reports of the CLI's own helper sub-agents for the status of the agent: only the main conversation opens and closes a turn.
- **0024/FR-024** (US3): THE SYSTEM SHALL let the person turn the hook status off for the whole office, with the default on. WHEN it is off, THE SYSTEM SHALL make no office settings file, and the change SHALL apply from the next session start and be said so in the setting, as in 0001/FR-023.
- **0024/FR-025** (US2): THE SYSTEM SHALL NOT end a temp for its idle limit (0020/FR-004) WHILE a permission ask of an exact session of that temp is open, also when its item is gone.

This change modified: 0001/FR-014, 0001/FR-020, 0008/FR-002, 0012/FR-002, 0018/FR-002, and Assumption A1 of 0001 and A3 of 0003 for `claude` sessions. The final text of 0001/FR-014, 0001/FR-020, 0012/FR-002, and 0018/FR-002 is under the change that owns each id. The final text of 0008/FR-002 is under [Change 0008](../needs-you-queue/SPEC.md).

### Preserved behaviors

- **0024/PB-1:** The statuses are the eight of 0001/FR-014. This change adds no status. Every status still shows as text, never color alone (0005 PB-3, 0018 PB-7).
- **0024/PB-2:** An estimated session follows the quiet period exactly as before, including the dedupe of 0022 for unprompted changes (FR-015). A CLI other than `claude` follows 0001 as before (FR-021).
- **0024/PB-3:** The office writes nothing into a CLI's configuration files or folders, the person's `~/.claude` included, and reads none of the CLI's files to find the status (0001/FR-021, 0012 PB-4). The office settings file lives in the office's own storage.
- **0024/PB-4:** The screen checks of 0012 and 0018 stay, for all sessions. A report can only make the office stricter (a permission ask, a compaction) or faster (a turn stop), never skip a screen check.
- **0024/PB-5:** The server and its terminals stay reachable only from the same machine, only with the office token, as before (0001/FR-007 to FR-010). The session secret is a separate and narrower credential (FR-007).
- **0024/PB-6:** Holds, rate limits, and the action record work as before (0007). Reports add no action record entry.
- **0024/PB-7:** A bell, an exit, a failure, an automatic hold, a resume hold, and a report for the person (0015) notify and change items as before (0022 PB-4).
- **0024/PB-8:** Stop, sleep, wake, and resume of sessions work as before (0001/FR-024, 0001/FR-025, 0018). A resumed or woken session gets a new office settings file and a new secret.

### Terms

Terms that this change defines. The other terms are as in 0001, 0003, 0008, 0012, 0018, and 0022.

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

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Hook state of a session | What the office knows about one running session from its reports. | status source, turns open, whether a permission ask is open, whether a compaction runs, time of the last accepted report | One per running session. Lives only while the session runs; a server stop ends it. It is not stored and not shown except as the status source and the words of FR-020. |
| Session secret | The proof that a report comes from the session's own CLI. | value (never shown), session | One per session. Revoked at the end of the session (FR-010). |
| Office settings file | The status hooks for one session. | session, path | One per exact-capable `claude` session. Deleted at the end of the session (FR-010). |
| Report | One event of one session. | session, event kind | Transient. Nothing else of it is kept (FR-009). |
| Hook setting | The person's switch for the hook status. | on or off | One per office; default on. Part of the office's settings (0001). |

### Success criteria

- **0024/SC-001:** In 20 turns of a real signed-in `claude` agent, 20 of 20 changes to `waiting for input` show within 1 second of the end of the answer, and 20 of 20 changes to `working` show within 1 second of the prompt.
- **0024/SC-002:** A real `claude` agent that answers one prompt and then gets no input for 30 minutes shows 0 changes to `working` and causes exactly 1 `waiting for input` notification, with no dependence on the work bound of 0022.
- **0024/SC-003:** In 10 permission asks of a real `claude` agent, 10 of 10 show `waiting for input` with `asks permission` within 1 second, 0 of 50 directives sent during the asks are typed, and with idle sleep on, 0 sleeps occur in 3 times the idle time, also when the person dismissed the item.
- **0024/SC-004:** Of 100 reports with no secret, a wrong secret, the secret of another session, an unknown session, an ended session, or a size above 64 KiB, 0 change any status, 100% of the first four kinds get the same answer, and 0 reports from an address other than the machine's own are accepted.
- **0024/SC-005:** After a session ends by each of exit, failure, stop, sleep, start failure, and server stop, 0 office settings files remain and the old secret is refused 100% of the time. After a server crash and restart, 0 files of earlier runs remain.
- **0024/SC-006:** With a user hook that writes a marker at each turn, 10 of 10 turns write the marker with and without the office settings file, the person's hook answers are not changed, and the person's settings files have the same contents before and after the session.
- **0024/SC-007:** With the office's listener unreachable, 10 of 10 turns finish, no event delays the CLI by more than 2 seconds, and the status of each session follows the quiet period.
- **0024/SC-008:** For agents on `codex`, `gemini`, and `opencode`, 0 office settings files are made, 0 options are added to their command lines, and the 0001 status tests give the same results as before.
- **0024/SC-009:** During a test session of `claude`, office-harness changes 0 files in the CLI's configuration files and folders, `~/.claude` included (0001/SC-005 extended).
- **0024/SC-010:** A test prompt with a unique marker text runs through 10 turns. The marker is in 0 rows of the office's stored data and in 0 lines of its log. The terminal output store holds it as the CLI printed it.
- **0024/SC-011:** In 10 turns that the person interrupts with the Escape key, 10 of 10 sessions show `waiting for input` within the quiet period plus 1 second, with the status source `late`.
- **0024/SC-012:** With the hook status off, 100% of new `claude` sessions are estimated and make 0 office settings files.
- **0024/SC-013:** A temp with an open permission ask is ended 0 times in 3 times its idle limit, also after the person dismissed its item.
- **0024/SC-014:** After 10 sessions, the secrets of all of them are in 0 rows of the office's database, 0 lines of its log, and 0 views, notifications, and action record entries.

### Assumptions

The bare `A1` to `A10` in the text above name these assumptions of 0024.

- **A1:** The facts about `claude` that this change relies on come from the CLI's documentation and are checked by the plan on the installed version, in a hands-on run: an extra settings file layers over the person's settings and keeps all their hooks; a hook that fails does not block the CLI; hooks are held back until the person trusts the folder; the Escape key ends a turn without a stop report; the events of FR-011 to FR-013 exist. Where a fact is wrong, the session is estimated and this spec's fallback holds.
- **A2:** Reports of one session arrive in the order in which the events happened, because the CLI waits for each report before it goes on. The office applies them in the order of arrival.
- **A3:** A process of the same OS user is as trusted as the person (0001). The session secret keeps other OS users, other machines, and web pages out. It does not keep out a process of the same user or the CLI's own tools: they run with the secret in their environment, and an injected model can print it into the terminal, where the stored output keeps it. A forged report can change the displayed status of its own session only. It can end an open permission ask early, and the screen checks of 0012 and 0018 then still guard a directive and a sleep (A5). Nothing in this change tries to hide the secret from the CLI.
- **A4:** A turn of the CLI draws a moving indicator, so an open turn or a running compaction that makes no output for a full quiet period is not working: it was interrupted or its report was lost (FR-014). A CLI setting that turns the indicator off can make a long silent turn look late. The next report makes the session exact again.
- **A5:** The end of a permission ask at the first person input is a simplification. The office has no event for the answer. The screen check of 0012 and the open-question check of 0018 stay, so an ask that is still open after an early end is still guarded.
- **A6:** The hook status is on by default. The person can turn it off for the whole office (FR-024). A per-agent switch is not offered.
- **A7:** The first accepted report makes a session exact. A session whose first report comes late stays estimated until then.
- **A8:** The 2-second delay limit (FR-006), the 64 KiB size (FR-009), and the 50 reports per second (FR-009) are defaults that the plan may tighten, not loosen.
- **A9:** The status of an exact session is not stored. A server stop ends every session, so no hook state outlives the server (0001/FR-004).
- **A10:** A session that the office starts for the sign-in check of 0019 or for any purpose other than an agent's own run gets no office settings file.

### Out of scope

- Tool-use events, loop detection, spend and token limits, and deny answers (gap rows 7, 8, and 10). The event model keeps room for them (FR-022): a later change adds event kinds without changing the meaning of the six here.
- Hooks for `codex`, `gemini`, `opencode`, and custom commands. Each family needs its own change.
- Reading which tool a permission ask is for, or any prompt or answer text (FR-009).
- Answering a permission ask for the person, or any decision by the office through a hook (FR-005).
- Changing the quiet period, the statuses, or the screen checks of 0012 and 0018 (PB-1, PB-4).
- A new Needs you group for permission asks or a different sort order (0008/FR-003 stays).
- Showing a history of reports, and storing them (A9).
- Hooks on a remote machine (0006).

### Notes from Deviations

- **A silent compaction turns the session late, like a silent turn (0024/FR-014).** A running compaction is not excluded from the late rule, because an interrupted compaction sends no end report and the agent would stay `working`. The requirement text already holds this.
- **A late session forgets its open work.** When a session turns late, the server clears its open-turn and compaction flags, so a turn that the person interrupted with Escape does not keep the flag. The requirement says only that the quiet rules apply again.
- **The hook state reaches the desktop inside each agent object.** Every agent answer carries `hook`, and `hook.changed` updates it. No new query exists.
- **The minimum CLI version is 2.1.295.** It comes from the CLI's documentation; only 2.1.322 was run.
- **The scene desk, the agent switcher, and the live-region announcement show only the plain status.** `asks permission` and `compacting` show in the agent list row and the details (0024/FR-020).
- **A command line that the host refuses only for the settings path starts without the hook arguments.** On Windows a `.cmd` command cannot receive some characters of a data folder path. The session starts as an estimated session with a note instead of a refused start; 0024/FR-004 and 0024/FR-019 keep their meaning (no file, no secret). A refusal that the agent's own arguments cause still refuses the start.
- **Success criteria that the QA report marks PARTIAL.** 0024/SC-001, SC-003, SC-004, SC-005, SC-006, SC-007, SC-009, SC-011, and SC-014 were run on fewer cases than their counts (for example 3 turns of a real CLI, 1 real permission ask, 1 of 10 interrupts). 0024/SC-002 passed on the real CLI. The hands-on run is an ignored test that needs an account, so it does not run in CI.
- **User stories, the user experience, edge cases, and the Clarifications** are in the archived spec.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0024-cli-hook-status - 2026-11-26
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
