# Scheduled directives

## Intent

The person schedules a directive for an agent to be sent later or on a repeat. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0025-scheduled-directives](../changes/archive/0025-scheduled-directives/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003, 0011: [Agent panel](../agent-panel/SPEC.md)
- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0019: [CLI sign-in check before a start](../cli-sign-in-check/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0018: [Idle sleep](../idle-sleep/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)
- 0020: [Temporary agents](../temporary-agents/SPEC.md)

## Change 0025: Scheduled directives

### Intent

The person gives some directives again and again at the same time: a nightly "run the full test suite and report", a Monday "write the status of the week", a check every four hours. Today the person must be at the desk to type each one, and a job that should start while the person sleeps does not start. This change adds the schedule: the person sets, in the desktop app, a directive text for a chosen agent and when it goes: once, daily at a time, weekly on chosen days at a time, or every N hours. A schedule is armed the moment the person saves it and switches it on, with no restart. Each run obeys the holds and rate limits of 0007, each directive goes through the safe delivery queue of 0012, and the action record names the schedule as the actor. A run that the office could not make at its time is recorded as missed and is not made late, so the office never surprises the person with old work.

### Requirements

- **0025/FR-001** (US1, US4): WHEN the person saves a schedule from the desktop app, THE SYSTEM SHALL keep it only IF its name, after the spaces at its ends are removed and the text is put in Unicode normalization form C, is not blank, has at most 100 characters, has no control characters, has no invisible or direction-changing format characters (the zero-width characters and the bidi controls), and is used by no other schedule of the office, compared without case; a weekday that the repeat names twice counts once; its target is an existing agent; its text follows the text rules of a directive; and its repeat is one of the Terms, a weekly repeat has at least one day, an every-N-hours repeat has a whole N from 1 to 168, and a once repeat of a schedule that is saved on names a future local time; IF any field breaks a rule, THEN THE SYSTEM SHALL keep nothing, name each failing field and its rule, and leave a schedule that the person edited as it was.
- **0025/FR-002** (US4): IF a save of a new schedule would make the number of schedules in the office greater than the schedule cap, THEN THE SYSTEM SHALL keep nothing and refuse the save with a reason that names the cap and its value; THE SYSTEM SHALL let the person set the schedule cap in the office settings as a whole number from 1 to 200, 50 by default, SHALL refuse any other value with that rule and keep the previous value, and SHALL apply a changed value to the next save without deleting or switching off any schedule.
- **0025/FR-003** (US1, US4): WHEN the person saves a schedule that is on, edits one, or switches one on, THE SYSTEM SHALL compute its next due time at once, without a restart, as the first due time that is later than both that moment and the schedule's last handled due time, SHALL show it in the schedules view in local time with the date and the day of the week, and SHALL clear the off reason at a save; and THE SYSTEM SHALL keep every schedule, its switch, its next and last handled due times, its off reason, and its history across server and machine restarts, readable only by the OS user who owns the office (0001/FR-009).
- **0025/FR-004** (US2): THE SYSTEM SHALL compute the due times of a once, daily, or weekly repeat in local time, in the time zone that the machine has at that moment, and recompute the next due time within 60 seconds after the machine's time zone changes; WHERE a set time does not exist on a day because daylight saving time starts, THE SYSTEM SHALL use the first minute after the skipped hour; WHERE a set time occurs twice on a day because daylight saving time ends, THE SYSTEM SHALL use only its first occurrence; and THE SYSTEM SHALL compute the due times of an every-N-hours repeat as the anchor plus N, 2N, 3N, and each further whole multiple of N hours of elapsed time, which no change of time zone or daylight saving time moves.
- **0025/FR-005** (US1, US2): WHILE a schedule is on, THE SYSTEM SHALL start a run within 60 seconds after each due time, and WHILE the sweep runs (0017/FR-007), SHALL keep a due run waiting until the sweep ends; IF the background server did not run at a due time, or the office can start the run only more than 2 minutes (the grace) after its due time, THEN THE SYSTEM SHALL NOT make that run and SHALL handle the due time as missed under FR-006; THE SYSTEM SHALL handle each due time of a schedule at most once, also when the clock is set back; and WHEN the system handles the due time of a once schedule, THE SYSTEM SHALL switch that schedule off.
- **0025/FR-006** (US2): WHEN the system handles due times as missed, THE SYSTEM SHALL add, for each schedule, one missed entry for all its due times that it finds missed at the same moment, with their count and the first and last of them in local time, to the schedule's history and to the action record; SHALL show the last missed time in the schedules view; SHALL set the next due time to the first future one; SHALL switch a once schedule off with the off reason that its time was missed; and SHALL notify the person with one notification for all the missed entries that it adds in the same minute, which says how many schedules missed runs.
- **0025/FR-007** (US3): THE SYSTEM SHALL treat each run as an agent-made action for the holds of 0007 and SHALL check the holds when the run starts and again when its directive enters the target's queue: WHILE the office is on hold, or an agent hold covers the target, THE SYSTEM SHALL refuse the run with the hold as the reason; IF a scheduled directive would make the target's schedule window exceed the value of the rate limit "directives sent to one target agent" (0007 A3), THEN THE SYSTEM SHALL refuse the run with a reason that names the limit and its value; THE SYSTEM SHALL count a scheduled directive only in the target's schedule window and never in the count of directives that agents send to that target, so that no agent's directive is refused because of a schedule; and THE SYSTEM SHALL NOT make a refused run later and SHALL NOT count a refused run toward the automatic hold of any agent (0007/FR-008).
- **0025/FR-008** (US3): IF a schedule's latest run has no result yet (for example while its target wakes), or its directive still waits in a queue, when a new run of that schedule starts, THEN THE SYSTEM SHALL skip the new run with the reason "previous run still waiting".
- **0025/FR-009** (US1, US3): WHEN a run passes FR-007 and FR-008 and its target is running, THE SYSTEM SHALL put the text into the target's queue as a scheduled directive whose sender is the schedule itself, never the target and never an agent, typed under 0012/FR-001 to 0012/FR-005 as modified by this change; SHALL record the run result as 0012 answers it (`submitted`, `waiting` with the place in the queue, or `unsubmitted`), or as `refused` with the reason when the queue is full (0012/FR-004); and SHALL change a `waiting` result to `submitted`, `unsubmitted`, or `withdrawn` with the reason when 0012 settles that directive.
- **0025/FR-010** (US3): WHEN a run passes FR-007 and FR-008 and its target is asleep, THE SYSTEM SHALL wake the target as 0018/FR-007 wakes it for the person, with the sign-in check of 0019/FR-001, the fallback of 0013/FR-004, and the hold of 0013/FR-005, and with the schedule as the actor of the wake entry, and SHALL then go on under FR-009 WHEN the wake result is `continued` or `new session`; WHERE the target is going to sleep, THE SYSTEM SHALL wait until it is asleep and then wake it; IF the wake ends `on hold`, or the session does not start, THEN THE SYSTEM SHALL send no directive and record the run as `failed` with the wake's reason; IF the target is `stopped`, `exited`, `failed`, or `interrupted`, THEN THE SYSTEM SHALL start nothing and record the run as `skipped` with the target's status as the reason; and WHEN the target of a schedule is deleted by the person, or removed because it was a temp that ended (0020/FR-006), THE SYSTEM SHALL switch that schedule off at once and show the off reason.
- **0025/FR-011** (US3): WHEN a run ends `skipped` because its target does not run, `failed`, `refused` for a reason other than a hold, `unsubmitted`, or `withdrawn` for a reason other than a stop of the background server, or WHEN the system switches a schedule off under FR-010, THE SYSTEM SHALL notify the person with the schedule's name, its target, the run result, and the reason, and SHALL add no further such notification for that schedule until a run of it succeeds; THE SYSTEM SHALL add no Needs you item for a schedule notification; and WHEN the person opens a schedule notification, THE SYSTEM SHALL show that schedule in the schedules view, or the schedules view itself for a notification of missed runs (FR-006), and SHALL open no terminal.
- **0025/FR-012** (US1, US3, US4): THE SYSTEM SHALL add each run result and each schedule-made action, refusal, unsubmitted directive, and withdrawal to the action record once, in the target's details, with the schedule as the actor, named by its name and its id with the note that the person made it; SHALL add each save, edit, switch, Run now, and delete of a schedule with the person as the actor, in the details of its target while the target exists; and SHALL keep the last 20 run results of each schedule in its history, newest first, each with its due time, the time the run started, whether it was due or Run now, the result, and the reason.
- **0025/FR-013** (US1, US4): THE SYSTEM SHALL show the person, in one schedules view of the desktop app, every schedule with its name, its target, its repeat in words, its switch as text, its next due time, its last run result with its time, its last missed time, and its off reason; SHALL let the person, with the keyboard alone, make, edit, switch off and on, run now, open the history of, and delete each schedule, a delete only after one confirmation that says that the history goes, that the action record stays, and that a waiting directive stays; SHALL apply an edit and a switch at once under FR-003, where a switch on makes no run for a time the schedule was off and is refused for a once schedule whose time has passed or a schedule with an off reason until the person edits it; and SHALL make a Run now a run under FR-007 to FR-012 that changes neither the next due time nor the switch.
- **0025/FR-014** (US4): THE SYSTEM SHALL let only the person, through the desktop app, make, edit, switch, run, or delete a schedule or change the schedule cap; and IF an agent asks the office for any of these actions, THEN THE SYSTEM SHALL refuse the request, tell the agent the reason, and record the refusal as 0002/FR-009 requires.

This change modified: 0003 Key entities (Notification), 0007/FR-010, 0012/FR-003, 0012/FR-005, 0018/FR-004. The final text is under the change that owns each id.

### Preserved behaviors

- **0025/PB-1:** The office starts no agent at a server start, and no agent without an explicit action (0001/FR-004, 0013/PB-1, 0018/PB-1). An `interrupted` agent stays `interrupted` until the person acts; a run skips it. The only start that a schedule makes is the wake of its own asleep target, at a due time while the server runs or at the person's Run now.
- **0025/PB-2:** The person's own direct actions from the desktop app (create, start, stop, delete, new task, wake, and typing into a terminal) are never held or rate-limited (0007/PB-1, 0018/FR-006). Holds and the per-target limit apply to runs, Run now included.
- **0025/PB-3:** A scheduled directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals other than the subtree one. Person input reaches the terminal at once and unchanged (0012/PB-5).
- **0025/PB-4:** The rate limits of agents count as before (0007 A3): a scheduled directive adds nothing to any count of directives that agents send, and an agent is refused or put on automatic hold only for its own actions (0007/FR-007, 0007/FR-008).
- **0025/PB-5:** The agent's connection to the office gets no new request, and the 0002 tree rules stay as they are: directives from agents go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009).
- **0025/PB-6:** Idle sleep stays as before: a scheduled directive that waits in a queue keeps its target awake as any waiting directive does (0018/FR-002 d), and a directive or report from an agent to an asleep agent is still refused (0018/FR-005).
- **0025/PB-7:** Every desktop app action is available from the keyboard, the actions of this change included, and every status shows as text (0001/FR-014, 0001/FR-028).
- **0025/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Terms

Terms that this change defines. The other terms are as in 0001, 0002, 0003, 0007, 0008, 0012, 0013, 0017, 0018, and 0020.

| Term | Meaning |
|---|---|
| Agent, status, session, stop, action record, desktop app, background server | As defined in 0001. Running means the status `starting`, `working`, or `waiting for input`. |
| Directive, text rules of a directive, agent's connection | As defined in 0002. The text rules: not blank, at most 8,192 characters, and no control characters (0020 A3). |
| Notification | As defined in 0003, as modified by this change: it can refer to a schedule. |
| Agent-made action, office hold, agent hold, rate limit, automatic hold, office settings | As defined in 0007. A hold covers an agent under the rules of 0007/FR-004. |
| Needs you, item | As defined in 0008. |
| Queue | As defined in 0012: the entries that wait to be typed into one agent's terminal. |
| Check period | As defined in 0013: the 20 seconds after a start in which the office watches whether the session ends by itself. |
| Sweep | As defined in 0017. While it runs, starts wait (0017/FR-007). |
| Asleep, going to sleep, wake, wake result | As defined in 0018. |
| Temp | As defined in 0020. A temp can be the target of a schedule. |
| Schedule | A rule that the person makes in the desktop app. It has a name, a target, a directive text, a repeat, and a switch that is on or off. |
| Scheduled directive | The directive that a run of a schedule puts into its target's queue. Its sender is the schedule itself, never the target and never an agent. |
| Target | The agent that a schedule sends its directive to. The schedule follows the agent by its id, not by its name or its place in the tree. |
| Repeat | When a schedule runs: once (a date and a time), daily (a time), weekly (one or more days of the week and a time), or every N hours (N is a whole number from 1 to 168). Times are in hours and minutes. |
| Local time | The wall-clock time of the machine's current time zone, with its daylight saving rules. |
| Due time | A moment at which the repeat says that the schedule runs. |
| Anchor | For an every-N-hours repeat: the moment at which the person last saved a change of the repeat, or switched the schedule on. An edit of other fields does not move it. |
| Run | One attempt of a schedule to send its directive, at a due time or when the person presses Run now. |
| Run result | What one run did: `submitted`, `waiting` (with the place in the queue), `unsubmitted`, `withdrawn`, `refused`, `skipped`, `failed`, or `missed`. Each result other than `submitted` and `waiting` has a reason. |
| Successful run | A run whose directive was submitted. |
| Handled due time | A due time for which the office recorded a run result, `missed` included. |
| Grace | 2 minutes: the longest time after a due time at which the office still starts the run. |
| Missed entry | One entry in a schedule's history for all the due times of that schedule that the office found missed at the same moment: their count, the first one, and the last one. |
| Schedule-made action | An action that a run takes: a wake or a directive. |
| Schedule window | For one target: the scheduled directives that went to it in the last 60 seconds. It is counted apart from the directives of agents (FR-007). |
| Schedule cap | An office setting: the highest number of schedules, on and off, that the office keeps. |
| Schedule history | The last 20 run results of one schedule, newest first. |
| Off reason | Why the office switched a schedule off: its target was deleted or removed, or its one-time due time was missed. |
| Schedules view | The desktop app view that lists every schedule (FR-013). |

### Data flow and trust boundary

- **Boundaries:** the person's desktop app connection, which presents the office token (0001/FR-007), and the agent's connection to the office (0002). Only the first can make, change, or run a schedule.
- **Into the office:** from the person, each schedule's fields, a switch on or off, an edit, a delete, a Run now, and the schedule cap. The office checks every field when the person saves (FR-001).
- **Out of the office, to a target's terminal:** the directive text, typed under the rules of 0012 as a directive's text is (FR-009).
- **Out of the office, to the person:** the schedules view, the schedule history, and notifications (FR-006, FR-011).
- **Into the action record:** each run result, each schedule-made action, each refusal, and each save, switch, edit, delete, and Run now of the person (FR-012).
- **Kept by the background server:** every schedule and its history, readable only by the OS user who owns the office (FR-003).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent makes or changes a schedule, so that it acts later outside the tree rules or outside a hold. | The agent's connection has no schedule request; any such request is refused and recorded (FR-014, PB-5). |
  | A schedule floods a terminal. | The shortest repeat is 1 hour (FR-001); one schedule never has two runs at once (FR-008); the per-target limit in the schedule window and the 0012 queue limit apply (FR-007, FR-009); the schedule cap bounds the count (FR-002). |
  | Schedules use up an agent's rate limit, so a parent's directive is refused or the agent is put on automatic hold. | Scheduled directives count only in the schedule window, never in the window of agents' directives, and a refused run counts toward no automatic hold (FR-007, PB-4). |
  | A schedule acts while the person has held the office or an agent. | A hold refuses each run that it covers, and the office does not make the run later (FR-007). |
  | A directive text carries control characters or escape codes that act on a terminal. | The text rules of a directive refuse them at save (FR-001); the text is typed only under the 0012 rules (PB-3). |
  | A directive presses Enter into a question meant for the person. | The 0012 question check and input-line check apply to every scheduled directive (FR-009, PB-3). |
  | After a restart, a sleep of the machine, or a clock change, old due times all run at once. | A run more than 2 minutes late is missed, not run; missed due times give one entry per schedule (FR-005, FR-006). |
  | A deleted agent's name is reused, and the schedule sends to the new agent. | The schedule follows its target by id; when the target is deleted or removed, the office switches the schedule off (FR-010). |
  | A schedule text holds private words, and other users of the machine read it. | Schedules are kept readable only by the office owner (FR-003, 0001/FR-009). |

- **Residual risk:** an agent CLI runs as the person's OS user and can act outside the office (0002 A8), for example on the office's own files; the office cannot stop that. A schedule that the person switches on keeps acting while the person is away; the person switches it off, holds the office, or deletes it. A run wakes an asleep agent and starts its CLI, which uses the CLI's quota.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Schedule | One rule of the person. | name, target, text, repeat (kind, date, time, days, N, anchor), switch, next due time, last handled due time, last missed time, off reason, whether a schedule notification waits for a successful run | Unique id; name unique in the office. Refers to one agent by id. Kept by the background server. |
| Run result | What one run did. | due time, start time, due or Run now, result, reason, place in the queue, for a missed entry the count and the first and last missed times | Belongs to one schedule; the last 20 are kept in its history and removed with the schedule. Each is also an action record entry. |
| Schedule window | The scheduled directives that went to one target in the last 60 seconds. | target, the times of those directives | One per target. Counted apart from the directives of agents; not shown. |
| Schedule settings | The office bound of schedules. | schedule cap (1 to 200, default 50) | One per office, with the 0007 and 0020 settings. |
| Action record entry (extends 0001, 0002, 0007) | One recorded action. | actor: the user, a named agent, the office, or now a schedule (its name, its id, and the note that the person made it); actions now also the run results and the person's schedule actions | Belongs to the office and shows in the details of the agent it names. An entry of a run is recorded once, in its target's details. Stays after its schedule is deleted. |
| Notification (changes 0003) | An event that the user must see. | can now refer to a schedule and its target, or to the missed runs of one minute | As in MODIFIED, 0003 Key entities. Adds no Needs you item. |

### Success criteria

- **0025/SC-001:** The person saves a schedule that is on, with a once repeat 3 minutes ahead, for a running agent that waits for input: with 0 restarts, the agent's terminal receives the text with Enter within 60 seconds after the due time (1 of 1), and the action record entry names the schedule, its id, and that the person made it.
- **0025/SC-002:** A daily schedule at a set time runs on 10 days in a row that include a start and an end of daylight saving time: 10 of 10 runs start within 60 seconds after the set wall-clock time. On the day the clock skips 02:00 to 03:00, a schedule at 02:30 runs once at 03:00; on the day the clock repeats 01:00 to 02:00, a schedule at 01:30 runs once.
- **0025/SC-003:** An every-2-hours schedule across a start and an end of daylight saving time makes runs whose starts are 2 hours of elapsed time apart, within 60 seconds, in 100% of 12 runs.
- **0025/SC-004:** With an every-1-hour schedule and a once schedule, a server stop that spans 3 due times of the first and the due time of the second leads to 0 runs of those due times, exactly 1 missed entry with count 3 for the first, the second switched off with the off reason "missed", and exactly 1 notification for both, because the office adds both missed entries in the same minute. The first then runs at its next future due time.
- **0025/SC-005:** A due run that waits for the sweep (0017) runs when the sweep ends 90 seconds after the due time (1 of 1), and is missed when the sweep ends 150 seconds after the due time (1 of 1).
- **0025/SC-006:** After a run, the clock is set back 30 minutes: that due time runs 0 more times. The clock is set forward 3 hours over 3 due times of an every-1-hour schedule: 0 of those runs are made, and 1 missed entry with count 3 is added.
- **0025/SC-007:** Under an office hold, 10 of 10 due runs are refused with the hold reason; under an agent hold on the target, 10 of 10 are refused; 100% have an action record entry with the schedule as actor; after the release, 0 of them run later, and 0 agents are on automatic hold.
- **0025/SC-008:** With the limit "directives sent to one target agent" at 3, 4 schedules due in the same minute for one target give 3 directives in the queue and 1 refusal that names the limit and its value; in the same 60 seconds, a directive from the target's parent agent is accepted (1 of 1), and 0 agents are on automatic hold.
- **0025/SC-009:** For an asleep target whose CLI can continue its session, the schedule wakes it and the directive is submitted within 2 minutes after the due time in 10 of 10 runs, each with 1 wake entry with the schedule as actor.
- **0025/SC-010:** For a stopped target, 5 due runs give 5 `skipped` results with the status as reason, 0 starts, and exactly 1 notification; after the person starts the target and 1 run succeeds, the next skip gives exactly 1 new notification.
- **0025/SC-011:** When the person deletes the target of a schedule that is on, the view shows the schedule off with the off reason within 2 seconds, the person gets 1 notification, and 0 runs follow.
- **0025/SC-012:** The person presses Run now twice within 5 seconds while the target is working: the first run waits in the queue and the second is skipped with "previous run still waiting" (1 of 1).
- **0025/SC-013:** With the schedule cap at 3 and 3 schedules, a 4th save is refused with the cap and its value, and 0 schedules are added. 200 schedules with the cap at 200 are accepted.
- **0025/SC-014:** 10 schedules (5 on, 5 off), each with history, survive 5 server restarts and 1 machine restart with 100% of their fields and history unchanged, and the 5 that are on run at their next due time with 0 person actions after the restart.
- **0025/SC-015:** 50 schedules that are on and due in the same minute for 50 running agents that wait for input start 50 of 50 runs within 60 seconds after the due time.
- **0025/SC-016:** Of 10 agent requests to make, edit, switch, run, or delete a schedule, 10 are refused and recorded, and 0 schedules change.
- **0025/SC-017:** With the keyboard alone, the person makes, switches off, edits, switches on, runs now, and deletes a schedule with 0 pointer actions, and each step adds 1 action record entry with the person as actor.
- **0025/SC-018:** The person deletes a schedule whose directive waits in the target's queue: the directive is still typed when the target is ready (1 of 1), the schedule and its history are gone, and 100% of its action record entries stay.
- **0025/SC-019:** A scheduled directive left unsubmitted (the text does not show at the input line) and one withdrawn after 10 minutes in the queue each give exactly 1 action record entry, in the target's action record, with the schedule as actor (2 of 2), and the matching `unsubmitted` and `withdrawn` results in the schedule's history.

### Assumptions

The bare `A1` to `A15` in the text above and in the data flow name these assumptions of 0025.

- **A1:** A run that the office could not make at its time is never made late. A nightly job that fires when the person turns the machine on surprises the person and can collide with the person's work; a burst of catch-up runs is worse; and the office starts nothing by itself at a server start (0001/FR-004, 0013/PB-1, 0018/PB-1). One missed entry per schedule per moment, and one notification per minute, keep a long absence from flooding the history and the person.
- **A2:** The grace of 2 minutes lets a busy server, or the sweep at a start, delay a run without losing it, and is short enough that a run never comes at a time the person did not expect. A due time before the server start is missed even if it is less than 2 minutes before it, because the server did not run then.
- **A3:** A schedule is the person's explicit action, set ahead of time and switched on. So a run may wake the target, as the person's wake does, and 0018/PB-1 stays true. A run never starts an agent that is `stopped`, `exited`, `failed`, or `interrupted`: the person stopped it, or it ended, or the office must not start it after a restart (0001/FR-004).
- **A4:** A run counts as an agent-made action for the holds, because the person is not present when it acts. A hold means "nothing acts without me", so it refuses runs, Run now included. Of the rate limits, only "directives sent to one target agent" applies, with the same value, because one terminal is one input stream whoever sends; the others count actions per sending agent, and a schedule is not an agent. Scheduled directives fill a count of their own per target, so a schedule never makes a parent's directive fail and never brings an agent near an automatic hold. A refused run is not retried, so a hold release does not set off a burst, and it counts toward no automatic hold (0007/FR-008).
- **A5:** The office notifies the person when a run did not deliver for a reason the person may not know: the target does not run, a failure, a limit, an unsubmitted or withdrawn directive, or a switch off by the office. It does not notify for a hold refusal (the person set the hold), an overlap skip (the previous run is still on its way, and its own result notifies when it fails), or a withdrawal at a server stop (the person stopped the server). One notification per schedule until a run succeeds keeps a nightly failure from adding one notification per night.
- **A6:** A schedule notification adds no Needs you item: Needs you lists agents that wait for the person (0008), and a schedule is not an agent. The notification shows as unread when the desktop app opens (0003/FR-007).
- **A7:** Once, daily, and weekly times are wall-clock times, because the person thinks of "02:00 every night". When the time zone changes, the next run follows the new zone, so a daily run can come sooner or later than 24 hours after the last one; at most one run per due time stays true. Every-N-hours counts elapsed time, because the person thinks of "every 4 hours"; it starts N hours after the anchor, so a new or switched-on schedule does not run at once.
- **A8:** The default schedule cap of 50 is far above the number of repeated jobs that one person keeps, and low enough that a mistake cannot make thousands. The range goes to 200. The history keeps the last 20 results per schedule; the action record keeps every entry (0001/FR-003).
- **A9:** A run begins within 60 seconds after its due time. Schedules are for jobs that run hours or days apart; a minute of delay does not change what the person gets.
- **A10:** A directive's text follows the text rules of a directive (0002). The text is checked at save, so a run never fails on its text.
- **A11:** The sender of a scheduled directive is the schedule itself, never the target and never an agent. A schedule has no action record of its own, so each result that 0012 records "in the sender's and the target's action records" is recorded once, in the target's, with the schedule as actor, and also as a run result in the schedule's history. The 0012 withdrawal for a target that left the sender's subtree does not apply, because the person, not an agent, chose the target. The task line (0011) shows a scheduled directive once it is submitted, as for any directive (0012/PB-3).
- **A12:** Run now exists so that the person can try a schedule. It makes a normal run, so it obeys holds, the limit, and the overlap rule, and its result shows in the history. It never changes the next due time, so it does not move an every-N-hours anchor.
- **A13:** A delete removes the schedule and its history, but no action record entry, because the action record is the office's lasting trail (0001/FR-011). It does not withdraw a waiting directive, because it already left the schedule; the person withdraws nothing by deleting a rule.
- **A14:** A schedule follows its target by id. A target that is deleted, or a temp target that ends, switches the schedule off, rather than letting it pick a new agent with the same name, which could be a different job.
- **A15:** Every schedule field can be edited, the target included. An edit never changes a run that already started.

### Out of scope

- Schedules made, changed, or run by an agent, and any agent request about schedules (FR-014).
- A schedule that starts a temp (0020). A follow-up change adds it and reuses the schedule, the repeat and missed-run rules, the schedule as actor, and the schedule notifications of this change.
- Catch-up runs, late runs, and retries of a refused, skipped, or failed run (A1, A4).
- Waking the machine, or running a schedule while the background server is off.
- Other repeats: monthly, cron-style rules, an end date, a run count, skipped dates, or a time zone per schedule.
- One schedule for many targets, or for a team.
- Conditions on a run, such as "only if the target is idle".
- Other actions than a directive, such as a report, a start, a stop, or a new task.
- Starting an agent that is not running; a run only wakes an asleep target (A3).
- Sending schedule results outside the machine (0001/FR-005).
- Import, export, or copying of schedules.
- Split: proposed and done at spec review r1, scheduled temps to a follow-up change. A further split of the 14 ADDED requirements (for example the schedules view apart from the runs) was considered; kept as one, because a schedule without its missed-run, hold, and view rules is not safe to ship alone. The owner can still ask for it at approval.

### Notes from Deviations

- **No requirement is deferred or changed.** 0025 has no Deviations section.
- **Success criteria that the QA report marks PARTIAL (automated).** 0025/SC-002 and 0025/SC-003 prove the daylight saving rules in the domain tests with fixed zones, not over real days. 0025/SC-005 has no wire test of the wait for a running sweep. 0025/SC-007, 0025/SC-009, 0025/SC-015 and 0025/SC-016 run on fewer cases than the criterion names (3 of 10 refused runs, 1 of 10 wakes, 12 agents instead of 50, 5 of 10 agent requests). 0025/SC-014 runs one restart, not 5 restarts and a machine restart. 0025/SC-019 proves the withdraw by a stop of the target, not by the 10-minute queue limit.
- **Edge cases, user stories, the user experience, and the Clarifications** are in the archived spec.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0025-scheduled-directives - 2026-11-26
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
