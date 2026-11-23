---
status: draft
capability: office
plan:
supersedes: 0018/FR-004
---

# 0025: Scheduled directives

## Intent

The person gives some directives again and again at the same time: a nightly "run the full test suite and report", a Monday "write the status of the week", a check every four hours. Today the person must be at the desk to type each one, and a job that should start while the person sleeps does not start. This change adds the schedule: the person sets, in the desktop app, a directive text for a chosen agent, or a temp (0020) with an objective, and when it runs: once, daily at a time, weekly on chosen days at a time, or every N hours. A schedule is armed the moment the person saves it and switches it on, with no restart. Each run obeys the holds and rate limits of 0007, each directive goes through the safe delivery queue of 0012, and the action record names the schedule as the actor. A run that the office could not make at its time is recorded as missed and is not made late, so the office never surprises the person with old work.

## Context

- Source of truth: `docs/PRD.md` names no release item for schedules. The owner approved this change as a gap beyond R1 to R9 on 2026-11-23.
- This change depends on 0001 (agents, statuses, the action record, durability, keyboard access), 0002 (the tree, directives, the text rules of a directive, the agent's connection), 0003 (notifications), 0007 (holds, rate limits, office settings), 0008 (Needs you), 0012 (the queue and safe typing), 0013 (the fallback and the hold of a start that fails), 0017 (the sweep gate at a server start), 0018 (asleep, wake), 0019 (the sign-in check), and 0020 (temps, the temp cap). References such as `0012/FR-005` point to requirements in those changes.
- 0020 put "Starting temps on a schedule" out of scope. This change adds it for the person only.
- 0018/PB-1 says that the office starts no agent by itself. This stays true: a run acts only because the person made the schedule and switched it on, which is the person's explicit action set ahead of time (A3). At a server start the office starts nothing and makes no run late (FR-005).
- The office does not read a CLI's prompts (0001 A1, 0003 A3). A submitted directive means that the office typed it and pressed Enter under 0012, not that the agent did the work.
- The idea comes from a reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, status, session, stop, action record, desktop app, background server | As defined in 0001. Running means the status `starting`, `working`, or `waiting for input`. |
| Directive, text rules of a directive, agent's connection | As defined in 0002. The text rules: not blank, at most 8,192 characters, and no control characters (0020 A3). |
| Notification | As defined in 0003. |
| Agent-made action, office hold, agent hold, rate limit, automatic hold, office settings | As defined in 0007. A hold covers an agent under the rules of 0007/FR-004. |
| Needs you, item | As defined in 0008. |
| Queue | As defined in 0012: the entries that wait to be typed into one agent's terminal. |
| Check period | As defined in 0013: the 20 seconds after a start in which the office watches whether the session ends by itself. |
| Sweep | As defined in 0017. While it runs, starts wait (0017/FR-007). |
| Asleep, going to sleep, wake, wake result | As defined in 0018. |
| Temp, objective, creator, temp cap, temps view | As defined in 0020. |
| Schedule | A rule that the person makes in the desktop app. It has a name, one action (a scheduled directive or a scheduled temp), a repeat, and a switch that is on or off. |
| Scheduled directive | A schedule action that puts one directive text into the queue of one chosen agent, the target. |
| Scheduled temp | A schedule action that creates and starts one temp with an objective, with the fields that the person's temp create takes: the place in the tree (role and parent), the CLI command, the working folder, the project, and the own worktree choice (0020/FR-001, 0020/FR-002). |
| Target | The agent of a scheduled directive. The schedule follows the agent by its id, not by its name or its place in the tree. |
| Temp parent | The parent that a scheduled temp names, when its place has one. |
| Repeat | When a schedule runs: once (a date and a time), daily (a time), weekly (one or more days of the week and a time), or every N hours (N is a whole number from 1 to 168). Times are in hours and minutes. |
| Local time | The wall-clock time of the machine's current time zone, with its daylight saving rules. |
| Due time | A moment at which the repeat says that the schedule runs. |
| Anchor | For an every-N-hours repeat: the moment at which the person last saved a change of the repeat, or switched the schedule on. An edit of other fields does not move it. |
| Run | One attempt of a schedule to do its action, at a due time or when the person presses Run now. |
| Run result | What one run did: `submitted`, `waiting` (with the place in the queue), `unsubmitted`, `withdrawn`, `refused`, `skipped`, `failed`, `temp started` (with the temp's name and id), or `missed`. Each result other than `submitted`, `waiting`, and `temp started` has a reason. |
| Successful run | A run whose directive was submitted, or that started a temp. |
| Handled due time | A due time for which the office recorded a run result, `missed` included. |
| Grace | 2 minutes: the longest time after a due time at which the office still starts the run. |
| Missed entry | One entry in a schedule's history for all the due times of that schedule that the office found missed at the same moment: their count, the first one, and the last one. |
| Schedule-made action | An action that a run takes: a wake, a directive, or a temp create. |
| Schedule cap | An office setting: the highest number of schedules, on and off, that the office keeps. |
| Schedule history | The last 20 run results of one schedule, newest first. |
| Off reason | Why the office switched a schedule off: its target was deleted, its temp parent was deleted, or its one-time due time was missed. |
| Schedules view | The desktop app view that lists every schedule (FR-014). |

## Data flow and trust boundary

- **Boundaries:** the person's desktop app connection, which presents the office token (0001/FR-007), and the agent's connection to the office (0002). Only the first can make, change, or run a schedule.
- **Into the office:** from the person, each schedule's fields, a switch on or off, an edit, a delete, a Run now, and the schedule cap. The office checks every field when the person saves (FR-001).
- **Out of the office, to a target's terminal:** the directive text, typed under the rules of 0012 as a directive's text is (FR-009).
- **Out of the office, to a new temp's terminal:** the objective, typed as 0020/FR-001 types it (FR-011).
- **Out of the office, to the person:** the schedules view, the schedule history, and notifications (FR-006, FR-012).
- **Into the action record:** each run result, each schedule-made action, each refusal, and each save, switch, edit, delete, and Run now of the person (FR-013).
- **Kept by the background server:** every schedule and its history, readable only by the OS user who owns the office (FR-003).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent makes or changes a schedule, so that it acts later outside the tree rules or outside a hold. | The agent's connection has no schedule request; any such request is refused and recorded (FR-015, PB-5). |
  | A schedule floods a terminal or fills the machine with temps. | The shortest repeat is 1 hour (FR-001); one schedule never has two runs at once (FR-008); the rate limit "directives sent to one target agent", the 0012 queue limit, and the temp cap apply (FR-007, FR-009); the schedule cap bounds the count (FR-002). |
  | A schedule acts while the person has held the office or an agent. | A hold refuses each run that it covers, and the office does not make the run later (FR-007). |
  | A directive text or an objective carries control characters or escape codes that act on a terminal. | The text rules of a directive refuse them at save (FR-001); the text is typed only under the 0012 rules (PB-3). |
  | A directive presses Enter into a question meant for the person. | The 0012 question check and input-line check apply to every scheduled directive (FR-009, PB-3). |
  | After a restart, a sleep of the machine, or a clock change, old due times all run at once. | A run more than 2 minutes late is missed, not run; missed due times give one entry per schedule (FR-005, FR-006). |
  | A deleted agent's name is reused, and the schedule sends to the new agent. | The schedule follows its target by id; when the target is deleted, the office switches the schedule off (FR-010). |
  | A schedule text holds private words, and other users of the machine read it. | Schedules are kept readable only by the office owner (FR-003, 0001/FR-009). |

- **Residual risk:** an agent CLI runs as the person's OS user and can act outside the office (0002 A8), for example on the office's own files; the office cannot stop that. A schedule that the person switches on keeps acting while the person is away; the person switches it off, holds the office, or deletes it. A run wakes an asleep agent and starts its CLI, which uses the CLI's quota.

## User stories

### US1 — The person schedules a directive for an agent (Priority: P1)

The person makes a schedule that sends "run the full test suite and report" to lead L1 every day at 02:00. The person saves it and switches it on. With no restart, the schedules view shows the next run at 02:00. At 02:00 the office puts the text into L1's queue, and L1's terminal receives it with Enter when L1 waits for input. The schedule's history shows `submitted`, and L1's action record names the schedule, its id, and that the person made it.

**Why this priority:** this is the gap the change closes. Without it the person must be present to give each repeated directive.

**Independent test:** agent A1 runs and waits for input. The person makes a schedule "smoke" with the text "print hello" for A1, once, 3 minutes ahead, and switches it on. With no restart, the schedules view shows the next run time in local time. Within 60 seconds after that time, A1's terminal receives "print hello" with Enter. The history shows 1 result `submitted`, and A1's action record has 1 directive entry whose actor is the schedule "smoke" with its id and the note that the person made it. The schedule is then off, because a once repeat has no further due time.

**Scenarios:**

1. Given the person fills the schedule form with a name, the directive action, a target, a text, and a repeat, When the person saves it with the switch on, Then the office keeps it, computes the next due time at once, and the schedules view shows it.
2. Given a schedule is on, When its due time comes and the target waits for input, Then the office puts the text into the target's queue within 60 seconds, and the text is typed and submitted under the 0012 rules.
3. Given a schedule is on, When its due time comes and the target is working, Then the text waits in the target's queue and the history shows `waiting` with its place; When the office later submits it, Then the history shows `submitted`.
4. Given the person enters a text with a control character, a blank text, or a text over 8,192 characters, When the person saves, Then the office saves nothing and names the field and the rule.
5. Given a schedule with a daily repeat at 02:00, When a run is handled, Then the next due time is 02:00 of the next day in local time.
6. Given a schedule is on, When the person switches it off, Then no run starts at its next due time, and the view shows it as off.

### US2 — Runs that the office could not make at their time are not made late (Priority: P1)

The machine is off from 23:00 to 08:00. A schedule runs every hour, and another one runs once at 03:00. At the server start the office makes no late run. It records one missed entry for the hourly schedule with the count and the first and last missed times, switches the one-time schedule off with the reason that its time was missed, and notifies the person once. The hourly schedule runs again at its next future due time.

**Why this priority:** a nightly directive that fires at 08:00 while the person starts work is a surprise that can collide with the person's own work, and a burst of nine late runs is worse. The office starts nothing by itself at a server start (0001/FR-004, 0013/PB-1, 0018/PB-1).

**Independent test:** a schedule S1 every 1 hour, anchor 10:00, target A1, and a schedule S2 once at 12:30. Stop the server at 10:30 and start it at 13:40. A1's terminal receives 0 directives in the first 3 minutes. S1's history has exactly 1 missed entry with count 3, first 11:00, and last 13:00; its next due time is 14:00. S2 is off with the off reason "missed", and its history has 1 missed entry. The person gets exactly 1 notification that says 2 schedules missed runs.

**Scenarios:**

1. Given the server did not run at a due time, When the server starts, Then the office does not make that run and records it in a missed entry.
2. Given the server runs and the sweep (0017) or other work delays a run, When the office can start the run at most 2 minutes after its due time, Then it starts the run.
3. Given the office can start a run only more than 2 minutes after its due time, for example after a sleep of the machine, Then it does not make the run and records it as missed.
4. Given the person sets the clock 3 hours forward past 3 due times of one schedule, Then the office records 1 missed entry with count 3 and makes 0 of those runs.
5. Given the person sets the clock back 30 minutes after a run, Then the office does not make that due time's run again.
6. Given a daily schedule at 02:30 and the night when daylight saving time starts skips 02:00 to 03:00, Then the run starts at 03:00 that night; given the night when daylight saving time ends repeats 01:00 to 02:00, a daily schedule at 01:30 runs once, at the first 01:30.
7. Given a schedule every 2 hours, When daylight saving time starts or ends, Then the runs stay 2 hours of elapsed time apart.
8. Given the machine's time zone changes, Then the next due time of a once, daily, or weekly schedule follows the new zone, and an every-N-hours schedule does not change.

### US3 — A run respects the holds, the limits, and the target's state (Priority: P1)

A run is a schedule-made action. The office hold and an agent hold that covers the target refuse it, the rate limit per target agent applies, a run never overlaps the previous run of its schedule, an asleep target is woken first, and a target that does not run is skipped. The person learns of a run that did not deliver from one notification per schedule, not from a flood.

**Why this priority:** a schedule acts while the person is away. Without these guards a schedule is a way around the holds of 0007, can flood a terminal, and can fail every night with nobody told.

**Independent test:** with the office on hold, 3 due runs of a schedule are refused with the reason that the office is on hold, each in the history and in the target's action record with the schedule as actor; after the release, 0 of them run later. A schedule whose target is asleep wakes it and submits the directive within 2 minutes after the due time. A schedule whose target is stopped records 3 skipped runs and gives exactly 1 notification.

**Scenarios:**

1. Given the office is on hold, or an agent hold covers the target, When a due time comes, Then the office refuses the run with the hold as the reason, records it, does not make it later, and puts no agent on automatic hold.
2. Given 3 directives went to agent A1 in the last 60 seconds and the limit "directives sent to one target agent" is 3, When a schedule's run for A1 starts, Then the office refuses it with the limit and its value.
3. Given the directive of a schedule's latest run still waits in a queue, When the next run starts, Then the office skips it with the reason "previous run still waiting".
4. Given the temp that a schedule's latest run started still exists, When the next run starts, Then the office skips it with the reason "previous temp still running".
5. Given the target is asleep and no hold covers it, When a run starts, Then the office wakes it as the person's wake does, with the schedule as the actor of the wake, and puts the directive into its queue when the wake result is `continued` or `new session`.
6. Given the target is asleep and its CLI is not signed in (0019), or the wake ends on hold (0013/FR-005), When a run starts, Then the office sends no directive and records the run as `failed` with the wake's reason.
7. Given the target is `stopped`, `exited`, `failed`, or `interrupted`, When a run starts, Then the office starts nothing, skips the run with the target's status as the reason, and notifies the person once for that schedule until a run of it succeeds.
8. Given the person deletes the target, Then the office switches the schedule off with the off reason that its target was deleted, and notifies the person once.
9. Given the directive of a run waits in the queue for 10 minutes, Then 0012/FR-005 withdraws it, and the history shows `withdrawn` with the reason.

### US4 — The person manages schedules (Priority: P2)

The person sees every schedule in one view, with its next run and its last result as text, and switches each one off and on, edits it, runs it now, reads its history, and deletes it, all with the keyboard alone.

**Why this priority:** US1 lets the person make a schedule, see it, and switch it off. This story adds the edit, the delete, the history, and Run now, which the person needs when a schedule exists for weeks. It is P2 because the P1 view already gives the safety control.

**Independent test:** with the keyboard only, the person makes a schedule, switches it off, edits its time, switches it on, presses Run now, opens its history, and deletes it after one confirmation. Each step shows its result as text, and each one adds 1 action record entry with the person as actor.

**Scenarios:**

1. Given a schedule is on, When the person edits its repeat and saves, Then the office computes the next due time from the new repeat at once and shows it.
2. Given a schedule was off for 3 days, When the person switches it on, Then the next due time is the first future one, and the office makes no run for the days it was off.
3. Given a once schedule whose time has passed, When the person switches it on, Then the office refuses it and says that the time has passed.
4. Given a schedule, When the person presses Run now, Then the office makes one run at once under every rule of US3, and the next due time and the switch do not change.
5. Given a schedule whose directive waits in a queue, When the person deletes the schedule and confirms, Then the schedule and its history leave, the waiting directive stays and is typed under 0012, and the action record keeps every entry of the schedule.
6. Given there are no schedules, When the person opens the schedules view, Then it says that there are no schedules and offers New schedule.
7. Given the office holds as many schedules as the cap, When the person saves a new one, Then the office saves nothing and names the cap and its value.
8. Given an agent asks the office to make, change, switch, run, or delete a schedule, Then the office refuses and records the request.

### US5 — A schedule starts a temp (Priority: P2)

The person makes a schedule that, every Monday at 09:00, starts a member temp under lead L1 in the project folder, with the objective "write the status of the week and report done". At each due time the office creates the temp as the person's temp create does, and the temps view names the schedule as its creator.

**Why this priority:** a scheduled temp cleans up after itself (0020), so a repeated job does not leave agents behind. It is P2 because a scheduled directive to a lasting agent already covers many repeated jobs.

**Independent test:** in a git repository, a schedule "weekly status" makes a member temp under lead L1 with an objective, once, 3 minutes ahead. Within 60 seconds after the due time a temp exists under L1 in its own worktree, its name starts with "weekly status", its terminal receives the objective, the temps view shows the schedule as its creator, and the history shows `temp started` with the temp's name and id.

**Scenarios:**

1. Given a scheduled temp is on, When its due time comes, Then the office creates and starts the temp at the place, with the CLI command, the folder, the project, and the own worktree choice that the schedule names, and types the objective under 0020/FR-001.
2. Given the temp cap is reached, When a run starts, Then the office creates nothing and refuses the run with the cap and its value.
3. Given an agent hold covers the temp parent, When a run starts, Then the office refuses it with the hold.
4. Given the person deletes the temp parent, Then the office switches the schedule off with the off reason that its temp parent was deleted.
5. Given the schedule's working folder no longer exists, When a run starts, Then the office creates nothing and records the run as `failed` with the reason.

## Requirements

### ADDED

- **FR-001** (US1, US4, US5): WHEN the person saves a schedule from the desktop app, THE SYSTEM SHALL keep it only IF its name is not blank, has at most 100 characters, has no control characters, and is used by no other schedule of the office, compared without case; its action is a scheduled directive with an existing target and a text that follows the text rules of a directive, or a scheduled temp whose fields a person's temp create accepts at that moment (0020/FR-001, 0020/FR-002, 0002/FR-003) and whose objective follows the text rules of a directive; and its repeat is one of the Terms, a weekly repeat has at least one day, an every-N-hours repeat has a whole N from 1 to 168, and a once repeat of a schedule that is saved on names a future local time; IF any field breaks a rule, THEN THE SYSTEM SHALL keep nothing, name each failing field and its rule, and leave a schedule that the person edited as it was.
- **FR-002** (US4): IF a save of a new schedule would make the number of schedules in the office greater than the schedule cap, THEN THE SYSTEM SHALL keep nothing and refuse the save with a reason that names the cap and its value; THE SYSTEM SHALL let the person set the schedule cap in the office settings as a whole number from 1 to 200, 50 by default, SHALL refuse any other value with that rule and keep the previous value, and SHALL apply a changed value to the next save without deleting or switching off any schedule.
- **FR-003** (US1, US4): WHEN the person saves a schedule that is on, edits one, or switches one on, THE SYSTEM SHALL compute its next due time at once, without a restart, as the first due time that is later than both that moment and the schedule's last handled due time, and SHALL show it in the schedules view in local time with the date and the day of the week, and SHALL clear the off reason at a save; and THE SYSTEM SHALL keep every schedule, its switch, its next and last handled due times, its off reason, and its history across server and machine restarts, readable only by the OS user who owns the office (0001/FR-009).
- **FR-004** (US2): THE SYSTEM SHALL compute the due times of a once, daily, or weekly repeat in local time, in the time zone that the machine has at that moment, and recompute the next due time within 60 seconds after the machine's time zone changes; WHERE a set time does not exist on a day because daylight saving time starts, THE SYSTEM SHALL use the first minute after the skipped hour; WHERE a set time occurs twice on a day because daylight saving time ends, THE SYSTEM SHALL use only its first occurrence; and THE SYSTEM SHALL compute the due times of an every-N-hours repeat as the anchor plus N, 2N, 3N, and each further whole multiple of N hours of elapsed time, which no change of time zone or daylight saving time moves.
- **FR-005** (US1, US2): WHILE a schedule is on, THE SYSTEM SHALL start a run within 60 seconds after each due time, and WHILE the sweep runs (0017/FR-007), SHALL keep a due run waiting until the sweep ends; IF the background server did not run at a due time, or the office can start the run only more than 2 minutes (the grace) after its due time, THEN THE SYSTEM SHALL NOT make that run and SHALL handle the due time as missed under FR-006; THE SYSTEM SHALL handle each due time of a schedule at most once, also when the clock is set back; and WHEN the system handles the due time of a once schedule, THE SYSTEM SHALL switch that schedule off.
- **FR-006** (US2): WHEN the system handles due times as missed, THE SYSTEM SHALL add, for each schedule, one missed entry for all its due times that it finds missed at the same moment, with their count and the first and last of them in local time, to the schedule's history and to the action record; SHALL show the last missed time in the schedules view; SHALL set the next due time to the first future one; SHALL switch a once schedule off with the off reason that its time was missed; and SHALL notify the person with one notification for all the missed entries that it adds in the same minute, which says how many schedules missed runs.
- **FR-007** (US3, US5): THE SYSTEM SHALL treat each run as an agent-made action for the holds of 0007 and SHALL check the holds when the run starts and again when its directive enters the target's queue or its temp is created: WHILE the office is on hold, or an agent hold covers the target or the temp parent, THE SYSTEM SHALL refuse the run with the hold as the reason; IF a scheduled directive would exceed the rate limit "directives sent to one target agent" (0007 A3), or a scheduled temp would exceed the temp cap (0020/FR-003), THEN THE SYSTEM SHALL refuse the run with a reason that names the limit or the cap and its value; and THE SYSTEM SHALL NOT make a refused run later and SHALL NOT count a refused run toward the automatic hold of any agent (0007/FR-008).
- **FR-008** (US3, US5): IF a schedule's latest run has no result yet (for example while its target wakes), or its directive still waits in a queue, when a new run of that schedule starts, THEN THE SYSTEM SHALL skip the new run with the reason "previous run still waiting"; and IF the temp that a schedule's latest run started still exists, in any status, when a new run of that schedule starts, THEN THE SYSTEM SHALL skip the new run with the reason "previous temp still running".
- **FR-009** (US1, US3): WHEN a run of a scheduled directive passes FR-007 and FR-008 and its target is running, THE SYSTEM SHALL put the text into the target's queue as a directive whose sender is the schedule, typed under 0012/FR-001 to 0012/FR-005, where the withdrawal for a target that left the sender's subtree does not apply; SHALL record the run result as 0012 answers it (`submitted`, `waiting` with the place in the queue, or `unsubmitted`), or as `refused` with the reason when the queue is full (0012/FR-004); and SHALL change a `waiting` result to `submitted`, `unsubmitted`, or `withdrawn` with the reason when 0012 settles that directive.
- **FR-010** (US3): WHEN a run of a scheduled directive passes FR-007 and FR-008 and its target is asleep, THE SYSTEM SHALL wake the target as 0018/FR-007 wakes it for the person, with the sign-in check of 0019/FR-001, the fallback of 0013/FR-004, and the hold of 0013/FR-005, and with the schedule as the actor of the wake entry, and SHALL then go on under FR-009 WHEN the wake result is `continued` or `new session`; WHERE the target is going to sleep, THE SYSTEM SHALL wait until it is asleep and then wake it; IF the wake ends `on hold`, or the session does not start, THEN THE SYSTEM SHALL send no directive and record the run as `failed` with the wake's reason; IF the target is `stopped`, `exited`, `failed`, or `interrupted`, THEN THE SYSTEM SHALL start nothing and record the run as `skipped` with the target's status as the reason; and WHEN the target of a schedule, or the temp parent of a scheduled temp, is deleted by the person or removed because it was a temp that ended (0020/FR-006), THE SYSTEM SHALL switch that schedule off at once and show the off reason.
- **FR-011** (US5): WHEN a run of a scheduled temp passes FR-007 and FR-008, THE SYSTEM SHALL create and start a temp with the schedule's fields as 0020/FR-001 and 0020/FR-002 do for the person's temp create, with the schedule as the creator and the sender of the objective, and with a name that begins with the schedule's name and is unique in the office; SHALL record the run as `temp started` with the temp's name and id; and IF the create cannot be made (a 0002 place rule, a missing folder or CLI, or a worktree failure of 0010), THEN THE SYSTEM SHALL create nothing and record the run as `failed` with the reason.
- **FR-012** (US3): WHEN a run ends `skipped` because its target does not run, `failed`, `refused` for a reason other than a hold, `unsubmitted`, or `withdrawn` for a reason other than a stop of the background server, or WHEN the system switches a schedule off under FR-010, THE SYSTEM SHALL notify the person with the schedule's name, its target or temp parent, the run result, and the reason, and SHALL add no further such notification for that schedule until a run of it succeeds; and THE SYSTEM SHALL add no Needs you item for a schedule notification.
- **FR-013** (US1, US3, US4): THE SYSTEM SHALL add each run result and each schedule-made action, refusal, and withdrawal to the action record with the schedule as the actor, named by its name and its id with the note that the person made it, in the details of the agent that the run acted on (the target, the new temp, or the temp parent for a run that made no temp) when there is one; SHALL add each save, edit, switch, Run now, and delete of a schedule with the person as the actor, in the details of its target or temp parent when there is one; and SHALL keep the last 20 run results of each schedule in its history, newest first, each with its due time, the time the run started, whether it was due or Run now, the result, and the reason.
- **FR-014** (US1, US4): THE SYSTEM SHALL show the person, in one schedules view of the desktop app, every schedule with its name, its action (the target, or the temp's place and objective), its repeat in words, its switch as text, its next due time, its last run result with its time, its last missed time, and its off reason; SHALL let the person, with the keyboard alone, make, edit, switch off and on, run now, open the history of, and delete each schedule, a delete only after one confirmation that says that the history goes, that the action record stays, and that a waiting directive or a started temp stays; SHALL apply an edit and a switch at once under FR-003, where a switch on makes no run for a time the schedule was off and is refused for a once schedule whose time has passed or a schedule with an off reason until the person edits it; and SHALL make a Run now a run under FR-007 to FR-013 that changes neither the next due time nor the switch.
- **FR-015** (US4): THE SYSTEM SHALL let only the person, through the desktop app, make, edit, switch, run, or delete a schedule or change the schedule cap; and IF an agent asks the office for any of these actions, THEN THE SYSTEM SHALL refuse the request, tell the agent the reason, and record the refusal as 0002/FR-009 requires.

### MODIFIED

- **0018/FR-004** (US3): WHILE an agent is asleep, THE SYSTEM SHALL keep it `asleep` across server and machine restarts and after idle sleep is turned off, until the person wakes it, a run of a schedule wakes it (0025/FR-010), the person or an agent-made action stops it (it then shows `stopped`), deletes it, starts a new task on it (0001/FR-024), or continues one of its sessions (0001/FR-025), or an agent-made action starts it as 0002/FR-008 allows for an agent that is not running; and SHALL NOT list it in "Resume interrupted" (0013/FR-001).

### PRESERVED

- **PB-1:** The office starts no agent at a server start, and no agent without an explicit action (0001/FR-004, 0013/PB-1, 0018/PB-1). An `interrupted` agent stays `interrupted` until the person acts; a run skips it. The only starts that a schedule makes are the wake of its own asleep target and the create of its own temp, at a due time while the server runs or at the person's Run now.
- **PB-2:** The person's own direct actions from the desktop app (create, start, stop, delete, new task, wake, a temp create from the create form, and typing into a terminal) are never held or rate-limited (0007/PB-1, 0018/FR-006). Holds and the per-target rate limit apply to runs, Run now included.
- **PB-3:** A scheduled directive keeps every 0012 behavior: one queue per target, the readiness and question checks, the input-line check before Enter, the 20-entry queue, the 10-minute limit, and the withdrawals. Person input reaches the terminal at once and unchanged (0012/PB-5).
- **PB-4:** A temp that a schedule started is a temp under every 0020 rule: the cap, the idle limit, the ends and their reasons, the keep rule, the end notice, and the temps view (0020/FR-003 to 0020/FR-009).
- **PB-5:** The agent's connection to the office gets no new request, and the 0002 tree rules stay as they are: directives go down to descendants only, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009).
- **PB-6:** Idle sleep stays as before: a scheduled directive that waits in a queue keeps its target awake as any waiting directive does (0018/FR-002 d), and a directive or report from an agent to an asleep agent is still refused (0018/FR-005).
- **PB-7:** Every desktop app action is available from the keyboard, the actions of this change included, and every status shows as text (0001/FR-014, 0001/FR-028).
- **PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

## User experience

**Flow (the person schedules a directive):**

1. The person opens the schedules view and presses New schedule.
2. The person gives a name, picks "Directive", picks the target from the agent list, types the text, picks a repeat (once, daily, weekly, or every N hours) and its time or days, and leaves the switch on.
3. The person saves. The view shows the schedule with its next run in local time.
4. At the due time the office puts the text into the target's queue; the history shows the result.

**Flow (the person schedules a temp):**

1. As steps 1 and 2 above, but the person picks "Temp" and fills the fields of the person's temp create: place in the tree, CLI command, folder, project, own worktree, and objective.
2. At the due time a temp starts; the temps view shows it with the schedule as its creator.

**Flow (the person manages a schedule):**

1. The person selects a schedule and switches it off, edits it, presses Run now, opens its history, or presses Delete.
2. A delete asks one confirmation; Escape cancels it.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| No schedules | The view says that there are no schedules and offers New schedule. | US4 scenario 6, FR-014 |
| Schedule on | The row: name, action, repeat in words, "on", next run in local time with date and day. | FR-003, FR-014 |
| Schedule off | The row shows "off" and no next run. | FR-014 |
| Switched off by the office | The row shows "off" and the off reason: target deleted, temp parent deleted, or time missed. One notification. | FR-006, FR-010, FR-012 |
| Save refused | The form names each failing field and its rule, or the cap and its value; nothing is kept. | FR-001, FR-002 |
| Switch on refused | The row says that the time has passed, or that the target or temp parent is gone, and that the person must edit the schedule. | FR-014 |
| Run waiting | The last result is `waiting` with the place in the queue. | FR-009 |
| Run succeeded | The last result is `submitted` or `temp started` with the temp's name. | FR-009, FR-011 |
| Run refused, skipped, failed, unsubmitted, or withdrawn | The last result with its reason, as text; one notification per schedule for the kinds of FR-012. | FR-007 to FR-012 |
| Missed | The last missed time, and a missed entry with its count and its first and last times in the history; one notification for all schedules that missed runs in the same minute. | FR-006 |
| Waking target | The target's wake result (0018/FR-008) in its details; the schedule's last result changes when the wake ends. | FR-010 |
| History | The last 20 results, newest first, each with its due time, start time, due or Run now, result, and reason. | FR-013 |
| Server not reachable or not authorized | The 0001 states show before the view. | PB-7, 0001/FR-026, 0001/FR-027 |

**Design:** the schedules view is one new view of the desktop app, built from the existing list, details, and form elements, beside the temps view; it adds no new visual style. The office settings get the schedule cap beside the 0007 and 0020 settings. The plan must not add other visuals without a design that the owner approves.

**Accessibility:**

- The switch, the next run, the last result, the reasons, and the off reason are text, never color or an icon alone (0001/FR-014).
- Every control of the view, the form, the history, and the delete confirmation works from the keyboard with a visible focus; Escape closes the form and the confirmation and changes nothing.
- A row's accessible name holds the schedule's name, its switch, and its next run; the action, the repeat, and the last result are its accessible description.
- The repeat is chosen with standard keyboard controls; the days of a weekly repeat are separate check boxes with the day's name as the label.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text). At 360 px width and at 200% zoom the view and the form reflow without horizontal scrolling.
- A run result is announced only through its notification (FR-012) and the change of the row; the view does not move focus on its own.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Schedule | One rule of the person. | name, action kind (directive or temp), target (directive) or temp fields and objective (temp), text, repeat (kind, date, time, days, N, anchor), switch, next due time, last handled due time, last missed time, off reason, whether a schedule notification waits for a successful run | Unique id; name unique in the office. A directive schedule refers to one agent by id; a temp schedule refers to its temp parent by id when it has one. Kept by the background server. |
| Run result | What one run did. | due time, start time, due or Run now, result, reason, place in the queue, temp name and id, for a missed entry the count and the first and last missed times | Belongs to one schedule; the last 20 are kept in its history and removed with the schedule. Each is also an action record entry. |
| Schedule settings | The office bound of schedules. | schedule cap (1 to 200, default 50) | One per office, with the 0007 and 0020 settings. |
| Action record entry (extends 0001, 0007) | One recorded action. | actor can now be a schedule (its name, its id, and the note that the person made it); actions now also the run results and the person's schedule actions | Belongs to the office and shows in the details of the agent it names. Stays after its schedule is deleted. |
| Notification (extends 0003) | An event that the person must see. | can now refer to a schedule (its name and its target or temp parent) or to the missed runs of a moment (the number of schedules) | Belongs to the office. Adds no Needs you item. |
| Temp (extends 0020) | An agent for one job. | creator can now be a schedule | As in 0020. |

## Success criteria

- **SC-001:** The person saves a schedule that is on, with a once repeat 3 minutes ahead, for a running agent that waits for input: with 0 restarts, the agent's terminal receives the text with Enter within 60 seconds after the due time (1 of 1), and the action record entry names the schedule, its id, and that the person made it.
- **SC-002:** A daily schedule at a set time runs on 10 days in a row that include a start and an end of daylight saving time: 10 of 10 runs start within 60 seconds after the set wall-clock time. On the day the clock skips 02:00 to 03:00, a schedule at 02:30 runs once at 03:00; on the day the clock repeats 01:00 to 02:00, a schedule at 01:30 runs once.
- **SC-003:** An every-2-hours schedule across a start and an end of daylight saving time makes runs whose starts are 2 hours of elapsed time apart, within 60 seconds, in 100% of 12 runs.
- **SC-004:** With an every-1-hour schedule and a once schedule, a server stop that spans 3 due times of the first and the due time of the second leads to 0 runs of those due times, exactly 1 missed entry with count 3 for the first, the second switched off with the off reason "missed", and exactly 1 notification. The first then runs at its next future due time.
- **SC-005:** A due run that waits for the sweep (0017) runs when the sweep ends 90 seconds after the due time (1 of 1), and is missed when the sweep ends 150 seconds after the due time (1 of 1).
- **SC-006:** After a run, the clock is set back 30 minutes: that due time runs 0 more times. The clock is set forward 3 hours over 3 due times of an every-1-hour schedule: 0 of those runs are made, and 1 missed entry with count 3 is added.
- **SC-007:** Under an office hold, 10 of 10 due runs are refused with the hold reason; under an agent hold on the target, 10 of 10 are refused; 100% have an action record entry with the schedule as actor; after the release, 0 of them run later, and 0 agents are on automatic hold.
- **SC-008:** With the limit "directives sent to one target agent" at 3, 4 schedules due in the same minute for one target give 3 directives in the queue and 1 refusal that names the limit and its value; 0 agents are on automatic hold.
- **SC-009:** For an asleep target whose CLI can continue its session, the schedule wakes it and the directive is submitted within 2 minutes after the due time in 10 of 10 runs, each with 1 wake entry with the schedule as actor.
- **SC-010:** For a stopped target, 5 due runs give 5 `skipped` results with the status as reason, 0 starts, and exactly 1 notification; after the person starts the target and 1 run succeeds, the next skip gives exactly 1 new notification.
- **SC-011:** When the person deletes the target of a schedule that is on, the view shows the schedule off with the off reason within 2 seconds, the person gets 1 notification, and 0 runs follow.
- **SC-012:** Run now twice within 5 seconds, while the target is working: the first run waits in the queue and the second is skipped with "previous run still waiting" (1 of 1). An every-1-hour scheduled temp whose temp runs for 3 hours gives 2 skips with "previous temp still running" and 0 second temps.
- **SC-013:** With the schedule cap at 3 and 3 schedules, a 4th save is refused with the cap and its value, and 0 schedules are added. 200 schedules with the cap at 200 are accepted.
- **SC-014:** 10 schedules (5 on, 5 off), each with history, survive 5 server restarts and 1 machine restart with 100% of their fields and history unchanged, and the 5 that are on run at their next due time with 0 person actions after the restart.
- **SC-015:** 50 schedules that are on and due in the same minute for 50 running agents that wait for input start 50 of 50 runs within 60 seconds after the due time.
- **SC-016:** A scheduled temp in a git repository starts a temp under the named parent in its own worktree within 60 seconds after its due time (1 of 1); its name begins with the schedule's name, and the temps view shows the schedule as its creator.
- **SC-017:** Of 10 agent requests to make, edit, switch, run, or delete a schedule, 10 are refused and recorded, and 0 schedules change.
- **SC-018:** With the keyboard alone, the person makes, switches off, edits, switches on, runs now, and deletes a schedule with 0 pointer actions, and each step adds 1 action record entry with the person as actor.
- **SC-019:** The person deletes a schedule whose directive waits in the target's queue: the directive is still typed when the target is ready (1 of 1), the schedule and its history are gone, and 100% of its action record entries stay.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The name is blank, over 100 characters, holds a control character, or is the name of another schedule. | Invalid | Nothing is kept; the field and the rule are named (FR-001). |
| The text or the objective is blank, over 8,192 characters, or holds a control character, such as an escape code. | Invalid | Nothing is kept; the field and the rule are named (FR-001). |
| A weekly repeat with no day; N of 0, 169, or not a whole number; a once time in the past. | Invalid | Nothing is kept; the rule is named (FR-001). |
| The person picks no target, or a temp place that the 0002 rules refuse. | Invalid | Nothing is kept; the reason names the allowed roles, as 0002/FR-003 does (FR-001). |
| The save would pass the schedule cap. | Boundary | Refused with the cap and its value (FR-002). |
| The person lowers the cap below the number of schedules. | Boundary | No schedule is deleted or switched off; new saves are refused until the count is below the cap (FR-002). |
| A cap value outside 1 to 200, or not a whole number. | Invalid | Refused with the range; the old value stays (FR-002). |
| The server is off at a due time, the machine sleeps over it, or the clock is set forward past it. | Failure | Not made; one missed entry per schedule; one notification (FR-005, FR-006). |
| A due time falls while the sweep runs. | Boundary | The run waits; it runs when the sweep ends within the grace, otherwise it is missed (FR-005). |
| The clock is set back after a run. | Boundary | The handled due time does not run again (FR-005). |
| The time zone changes. | Boundary | Once, daily, and weekly schedules follow the new zone within 60 seconds; a daily run can then come sooner or later than 24 hours after the last one (FR-004, A7). Every-N-hours schedules do not move. |
| A set time that daylight saving time skips, or repeats. | Boundary | The first minute after the skipped hour; the first of the two times (FR-004). |
| Many schedules are due in the same minute. | Concurrent | Each run starts within 60 seconds (SC-015); runs to one target meet the per-target rate limit and the queue limit (FR-007, FR-009). |
| The office or an agent hold covers the run, or a hold starts while the target wakes. | Unauthorized | Refused with the hold, also at the second check before the queue; not made later; no automatic hold (FR-007). |
| An agent asks the office to make, change, switch, run, or delete a schedule. | Unauthorized | Refused and recorded (FR-015). |
| The target is working, or the person types in its terminal. | Concurrent | The directive waits in the queue under 0012 for at most 10 minutes; then it is withdrawn and recorded (FR-009). |
| The target's queue is full. | Boundary | Refused with the reason that the queue is full (FR-009). |
| The previous run has no result yet (its target still wakes), its directive still waits, or its temp still exists, `interrupted` included. | Concurrent | The new run is skipped with its reason (FR-008). |
| The target is asleep. | Boundary | Woken with the schedule as actor; the directive follows the wake (FR-010). |
| The target is going to sleep. | Concurrent | The run waits until it is asleep, then wakes it (FR-010). |
| The target's wake ends on hold, or its CLI is not signed in. | Failure | No directive; `failed` with the reason; one notification (FR-010, FR-012). |
| The target is `stopped`, `exited`, `failed`, or `interrupted`. | Boundary | Skipped with the status; the office starts nothing; one notification until a run succeeds (FR-010, FR-012). |
| The target is deleted, also when it is a temp that ended. | Boundary | The schedule is switched off at once with the off reason; one notification; a switch on is refused until the person picks another target (FR-010, FR-012, FR-014). |
| A once schedule ran, or its time was missed. | Boundary | It is switched off; a switch on is refused until the person gives a future time (FR-005, FR-006, FR-014). |
| The target moves in the tree, for example a lead taken under a CEO that the person creates later. | Boundary | The schedule follows the agent by id; holds are checked with the tree as it is at the run (FR-007). |
| The temp parent is deleted. | Boundary | The schedule is switched off with the off reason (FR-010). |
| The temp parent is not running. | Boundary | The temp is created as the person's temp create allows; its reports to that parent are refused under 0015/FR-005. |
| The temp cap is reached, the folder is gone, the CLI is missing, or the worktree cannot be made. | Failure | Refused or `failed` with the reason; nothing stays behind (FR-007, FR-011). |
| The person edits a schedule while its directive waits, or while its target wakes. | Concurrent | The run goes on with the values it started with; the waiting directive stays; the next due time is computed from the new values (FR-003, FR-014). |
| The person switches a schedule off or deletes it while its run waits. | Concurrent | The waiting directive stays and is typed under 0012; a started temp stays; no new run starts (FR-014). |
| The person presses Run now on a schedule that is off. | Boundary | One run under every rule; the schedule stays off and its next due time does not change (FR-014). |
| Two desktop app windows edit one schedule. | Concurrent | Each save is checked and recorded; the later save is the one kept, and both windows show it. |
| The background server stops normally while a run's directive waits, or while the target wakes. | Failure | The directive is withdrawn and recorded as 0012/FR-005 requires (no notification); a wake that did not end leaves the target `interrupted`, and the run is `failed` with the reason that the server stopped. |
| The background server stops without a clean stop during a run. | Failure | At the next start, a run with no final result is recorded as `failed` with the reason that the server stopped; the office starts nothing for it (PB-1). |
| A schedule targets an agent with the same name as a deleted one. | Boundary | Not possible: the old schedule was switched off when its target was deleted, and the person picks the new target by id. |
| Many schedules; a schedule with many runs. | Boundary | The view lists every schedule; each history keeps the last 20 results; the action record keeps every entry (FR-013). |

## Assumptions

- **A1:** A run that the office could not make at its time is never made late. A nightly job that fires when the person turns the machine on surprises the person and can collide with the person's work; a burst of catch-up runs is worse; and the office starts nothing by itself at a server start (0001/FR-004, 0013/PB-1, 0018/PB-1). One missed entry per schedule per moment, and one notification per moment, keep a long absence from flooding the history and the person.
- **A2:** The grace of 2 minutes lets a busy server, or the sweep at a start, delay a run without losing it, and is short enough that a run never comes at a time the person did not expect. A due time before the server start is missed even if it is less than 2 minutes before it, because the server did not run then.
- **A3:** A schedule is the person's explicit action, set ahead of time and switched on. So a run may wake the target, as the person's wake does, and 0018/PB-1 stays true. A run never starts an agent that is `stopped`, `exited`, `failed`, or `interrupted`: the person stopped it, or it ended, or the office must not start it after a restart (0001/FR-004).
- **A4:** A run counts as an agent-made action for the holds, because the person is not present when it acts. A hold means "nothing acts without me", so it refuses runs, Run now included. Of the rate limits, only "directives sent to one target agent" applies, because the others count actions per sending agent and a schedule is not an agent. A refused run is not retried, so a hold release does not set off a burst; and a schedule's refusal puts no agent on automatic hold, because no agent made it (0007/FR-008).
- **A5:** The office notifies the person when a run did not deliver for a reason the person may not know: the target does not run, a failure, a limit, an unsubmitted or withdrawn directive, or a switch off by the office. It does not notify for a hold refusal (the person set the hold), an overlap skip (the previous run is still on its way, and its own result notifies when it fails), or a withdrawal at a server stop (the person stopped the server). One notification per schedule until a run succeeds keeps a nightly failure from adding one notification per night.
- **A6:** A schedule notification adds no Needs you item: Needs you lists agents that wait for the person (0008), and a schedule is not an agent. The notification shows as unread when the desktop app opens (0003/FR-007).
- **A7:** Once, daily, and weekly times are wall-clock times, because the person thinks of "02:00 every night". When the time zone changes, the next run follows the new zone, so a daily run can come sooner or later than 24 hours after the last one; at most one run per due time stays true. Every-N-hours counts elapsed time, because the person thinks of "every 4 hours"; it starts N hours after the anchor, so a new or switched-on schedule does not run at once.
- **A8:** The default schedule cap of 50 is far above the number of repeated jobs that one person keeps, and low enough that a mistake cannot make thousands. The range goes to 200. The history keeps the last 20 results per schedule; the action record keeps every entry (0001/FR-003).
- **A9:** A run begins within 60 seconds after its due time. Schedules are for jobs that run hours or days apart; a minute of delay does not change what the person gets.
- **A10:** A directive's text follows the text rules of a directive (0002). The text is checked at save, so a run never fails on its text. The objective of a scheduled temp follows the same rules (0020 A3).
- **A11:** A schedule's sender is the schedule itself. The 0012 withdrawal for a target that left the sender's subtree does not apply, because the person, not an agent, chose the target. The task line (0011) shows a scheduled directive once it is submitted, as for any directive (0012/PB-3).
- **A12:** The office names each temp of a scheduled temp with the schedule's name and a part that makes the name unique in the office, such as the run's local date and time. The plan fixes the form.
- **A13:** Run now exists so that the person can try a schedule. It makes a normal run, so it obeys holds, the limit, and the overlap rule, and its result shows in the history. It never changes the next due time, so it does not move an every-N-hours anchor.
- **A14:** A delete removes the schedule and its history, but no action record entry, because the action record is the office's lasting trail (0001/FR-011). It does not withdraw a waiting directive or end a temp, because those already left the schedule; the person withdraws nothing by deleting a rule.
- **A15:** A schedule follows its target by id. A target that is deleted switches the schedule off, rather than letting it pick a new agent with the same name, which could be a different job.
- **A16:** Every schedule field can be edited, the action kind included. An edit never changes a run that already started.

## Clarifications

### Session 2026-11-23 (drafted under the owner's delegation)

- Q: Does a run missed while the server was off, or while the machine slept, run late? → A: No; it is recorded once as missed, and the next run is the next future due time (FR-005, FR-006, A1). A catch-up run, and a catch-up of only the last run, were considered and not chosen.
- Q: What does a run do with an asleep target? → A: It wakes it as the person's wake does and then queues the directive (FR-010, A3). Skipping asleep targets was not chosen: idle sleep would then silently stop every nightly schedule.
- Q: Do holds and rate limits apply to runs? → A: Yes, as for an agent-made action; a refused run is not retried, and it puts no agent on automatic hold (FR-007, A4).
- Q: Can two runs of one schedule overlap? → A: No; a new run is skipped while the previous directive waits or the previous temp exists (FR-008).
- Q: Can an agent make a schedule? → A: No; only the person, from the desktop app (FR-015).

## Out of scope

- Schedules made, changed, or run by an agent, and any agent request about schedules (FR-015).
- Catch-up runs, late runs, and retries of a refused, skipped, or failed run (A1, A4).
- Waking the machine, or running a schedule while the background server is off.
- Other repeats: monthly, cron-style rules, an end date, a run count, skipped dates, or a time zone per schedule.
- One schedule for many targets, or for a team.
- Conditions on a run, such as "only if the target is idle".
- Other actions than a directive and a temp, such as a report, a start, a stop, or a new task.
- Starting an agent that is not running; a run only wakes an asleep target (A3).
- Sending schedule results outside the machine (0001/FR-005).
- Import, export, or copying of schedules.
- Split: proposed `0025-scheduled-directives` (scheduled directives, FR-001 to FR-010 and FR-012 to FR-015) and a later `scheduled-temps` change (FR-011 and the temp parts of FR-001, FR-007, FR-008, and FR-010); kept as one in this draft, because the owner approved one gap that names both actions, and the two share every schedule rule (save, time, missed runs, holds, overlap, history, view); the temp action adds one requirement. The owner confirms or splits at approval.
