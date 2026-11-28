# Idle sleep

## Intent

Idle agents go to sleep and wake on demand, also when they have a waiting item. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0018-idle-sleep](../changes/archive/0018-idle-sleep/SPEC.md)
- [0022-waiting-item-sleep](../changes/archive/0022-waiting-item-sleep/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0018/FR-002 | 0022, 0024 | Change 0018, Requirements |
| 0018/FR-004 | 0025 | Change 0018, Requirements |
| 0018/FR-009 | 0022 | Change 0018, Requirements |
| 0018 Terms, Idle and Idle since | 0022 | Change 0022, Terms |

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0014, 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0024: [Exact agent status from CLI hooks](../agent-status/SPEC.md)
- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0019: [CLI sign-in check before a start](../cli-sign-in-check/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0005: [Office scene](../office-scene/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)
- 0025: [Scheduled directives](../scheduled-directives/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0018: Put idle agents to sleep

### Intent

With 20 to 30 agents, many of them wait for input for hours. Each idle CLI, and the helper servers that it starts, keeps its memory all that time. This change adds an "Idle sleep" setting, off by default. When the person turns it on, the office puts an agent to sleep after it stays idle longer than a set time: it ends the CLI with the normal stop, keeps the session, and keeps the last screen visible. One key or one button wakes the agent, and the office continues the same session. When the CLI cannot continue it, the office uses the fallback and the hold of 0013.

### Requirements

- **0018/FR-001** (US1): THE SYSTEM SHALL let the person turn idle sleep on and off in the office settings, with idle sleep off by default and an idle time of 30 minutes when the person first turns it on; SHALL accept an idle time only as a whole number of minutes from 5 to 1440, and refuse any other value with that rule and keep the previous value; SHALL keep the setting in the background server across server and machine restarts; and SHALL apply each change at once, without a restart.
- **0018/FR-002** (US1): WHILE idle sleep is on, THE SYSTEM SHALL begin to put an agent to sleep within 60 seconds of the first moment at which the agent shows `waiting for input`, the idle time has passed since its idle-since moment, and none of these blocking conditions holds: (a) the agent does not show `waiting for input`; (b) it has an item in Needs you that is not a plain waiting item, that is, an item that recorded a bell, a failure, an exit, an automatic hold, a resume hold, a kept leftover, or a permission ask (0024), an item that holds a report for the person, or a waiting item whose agent's screen shows an open question under the question check of 0012/FR-002; (c) a hold covers it (0007/FR-004); (d) a directive or a report waits in its queue (0012, 0015); (e) pending office text or unsent person text waits at its input line (0012); (f) one of its children runs; (g) the agent's session is exact and a permission ask is open, also when the agent has no item (0024/FR-017). The idle-since moment is the latest of the agent's last prompted change to `waiting for input`, the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time; an unprompted change does not move it. THE SYSTEM SHALL check all seven conditions again right before it ends the CLI, and SHALL NOT end it when one of them holds. (modified by 0022, 0024)
- **0018/FR-003** (US1): WHEN the system puts an agent to sleep, THE SYSTEM SHALL end the agent's CLI as the person's stop does (0001 A3), SHALL show the agent as `asleep` once the CLI has ended, SHALL keep the last session as the session that a wake continues, SHALL keep the recent output of that session with the lines that the CLI printed while it ended, SHALL add one action record entry for the sleep, and no separate stop entry, with the office as actor and the reason "idle for N minutes", and SHALL NOT notify the person or add a Needs you item for the sleep or for the end of the CLI that the sleep caused.
- **0018/FR-004** (US1, US2): WHILE an agent is asleep, THE SYSTEM SHALL keep it `asleep` across server and machine restarts and after idle sleep is turned off, until the person wakes it, a run of a schedule wakes it (0025/FR-010), the person or an agent-made action stops it (it then shows `stopped`), deletes it, starts a new task on it (0001/FR-024), or continues one of its sessions (0001/FR-025), or an agent-made action starts it as 0002/FR-008 allows for an agent that is not running; and SHALL NOT list it in "Resume interrupted" (0013/FR-001). (modified by 0025)
- **0018/FR-005** (US4): WHILE an agent is going to sleep or asleep, THE SYSTEM SHALL refuse a directive to it and a report to it, SHALL answer the sender that the receiver is not running because it is asleep, and SHALL record the refusal as 0002/FR-011 and 0015/FR-009 require; and WHEN a directive or a report still waits in the queue of an agent that is going to sleep, THE SYSTEM SHALL withdraw it with the reason that the receiver is asleep and record the withdrawal as 0012/FR-005 requires.
- **0018/FR-006** (US2): WHILE an agent is asleep, THE SYSTEM SHALL offer the person a Wake button in the agent's details and in its terminal view; WHILE an agent is asleep and its terminal view has keyboard focus, THE SYSTEM SHALL wake the agent when the person presses Enter and SHALL send no key from that view to any CLI; and THE SYSTEM SHALL carry out a wake while the office or the agent is on hold.
- **0018/FR-007** (US2, US3): WHEN the person wakes an agent, THE SYSTEM SHALL continue the agent's last session where its CLI supports it (0001/FR-025) and otherwise start a new session at once with the reason, SHALL apply the fallback of 0013/FR-004 and the hold of 0013/FR-005 to this start as 0013 applies them to a start of its action, and SHALL add one action record entry for the wake with the person as actor, with the reason when the wake started a new session; IF the agent is not asleep when the system carries out the wake, THEN THE SYSTEM SHALL start nothing and tell the person that the agent is no longer asleep.
- **0018/FR-008** (US2, US3): THE SYSTEM SHALL show the result of each wake as text in the agent's details and terminal view as soon as it is known: waking, checking (during the check period), continued, new session (with the reason), on hold (with the reason), or no longer asleep.
- **0018/FR-009** (US1): IF the agent's CLI does not end when the system puts the agent to sleep (the 0001 stop fails), THEN THE SYSTEM SHALL leave the agent's status as the CLI shows it, SHALL record the failure once in the agent's action record with the office as actor and the reason, and SHALL NOT try to put that agent to sleep again until it has made a prompted change to `waiting for input` after the failure and the idle time has passed since that change; an unprompted change SHALL NOT allow a new try. (modified by 0022)

This change modified: 0001/FR-014. The final text is under the change that owns that id.

Change 0022 modified 0018/FR-002, 0018/FR-009, and the Terms Idle and Idle since. The text above is final. The Terms are under Change 0022, Terms.

### Preserved behaviors

- **0018/PB-1:** No agent starts without an explicit action. The office starts no agent by itself, at a server start or at any other time (0001/FR-004, 0013 PB-1). The fallback of FR-007 follows only the person's wake.
- **0018/PB-2:** While idle sleep is off, the office behaves as before this change: it ends no CLI and starts nothing because of idle time.
- **0018/PB-3:** A hold refuses only agent-made actions. The office carries out the person's own actions, Wake included, as before (0007 PB-1, PB-2). A hold stops no CLI by itself; a hold keeps an agent awake (FR-002 c).
- **0018/PB-4:** The person's stop, new task, continue, and delete do what they did before (0001/FR-012, FR-024, FR-025). Each start, continue, stop, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).
- **0018/PB-5:** Person input reaches the terminal of every agent whose CLI runs at once and unchanged, and the app shortcuts still leave a focused terminal (0001/FR-017 as modified by 0003, 0012 PB-5).
- **0018/PB-6:** The notification of a wake hold is a notification of an automatic hold under 0003/FR-006 as modified by 0007: it names the agent and the event, and the focused-terminal exception does not apply to it (0013 PB-6).
- **0018/PB-7:** Every status, `asleep` included, shows as text, never color alone, in the agent list, the office scene, and the subtree read of 0014/FR-001 (0001/FR-014, 0005 PB-3, 0014/FR-001).
- **0018/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Idle sleep setting | Whether the office puts idle agents to sleep. | on or off, idle time in minutes, when it was last turned on or changed | One per office; kept by the background server. |
| Agent (extends 0001) | One agent. | status: also `asleep`; idle since (kept only while the background server runs) | As in 0001. |
| Session (extends 0001) | One run of an agent CLI. | the last session of an asleep agent is the one a wake continues | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | actions now also sleep, failed sleep, and wake; reason (optional) | Belongs to the agent it names (0001/FR-011). |
| Wake result | What a wake did. | agent, result (waking, checking, continued, new session, on hold, no longer asleep), reason | Shown in the agent's details and terminal view; not stored; it shows until the agent's next sleep, the person's next action on the agent, or a server restart. |

### Success criteria

- **0018/SC-001:** With idle sleep never turned on, 10 agents that wait for input for 24 hours cause 0 sleeps, 0 starts by the office, and 0 sleep entries in the action record.
- **0018/SC-002:** With an idle time of 5 minutes and 10 idle agents for which no blocking condition holds, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment (5 minutes of idle time, at most 60 seconds to begin, at most 10 seconds of stop grace time).
- **0018/SC-003:** For each of the blocking conditions (b) to (f) of FR-002, an idle agent for which only that condition holds is put to sleep 0 times in 3 times the idle time; for condition (a), an agent that changes to `working` between the first and the last check is stopped 0 times in 10 tries.
- **0018/SC-004:** After the person turns idle sleep on with 30 minutes, 10 agents that were idle for 3 hours cause 0 sleeps in the first 30 minutes.
- **0018/SC-005:** For an asleep agent whose CLI can continue its session, 1 key press (Enter) or 1 button press wakes it, and in 100% of 20 wakes the agent runs its last session again within 10 seconds of the press.
- **0018/SC-006:** Over 20 sleep and wake cycles, each sleep adds exactly 1 action record entry (office as actor) and each successful wake adds exactly 1 (person as actor), and the sleeps cause 0 notifications and 0 Needs you items.
- **0018/SC-007:** A wake of an agent whose CLI ends within 1 second of every start starts the agent at most 2 times; then the agent is on hold and exactly 1 notification names it.
- **0018/SC-008:** After 10 server restarts, 5 of them after a SIGKILL of the server, 100% of the agents that were asleep still show `asleep`, 0 of them start without a person action, and "Resume interrupted" lists 0 of them.
- **0018/SC-009:** 100% of the directives and reports sent to an asleep agent are refused, and each answer says that the receiver is asleep.

### Notes from Deviations

- **A sleep whose CLI ended on its own (0018/FR-009).** A sleep whose CLI ended on its own just before the stop records one failed `sleep` entry with the reason "the CLI had already ended", answers `conflict` with that text, and changes no status: the exit event of the CLI sets it as for any exit. The agent is not asleep then. The office does not bar the agent, because the stop did not fail, and it has left `waiting for input` anyway.
- **A client cannot wake an agent past the watch.** A user client cannot send `agent.wake`. It gets `invalid` "agent.wake is made by wake.start only", as for `agent.sleep`, because a bare wake skips the 0013 watch. `wake.start` and the bulk resume run it inside the server. The desktop app sends only `wake.start`.
- **Refused directive to an asleep agent (0018/FR-005).** The action record shows the refusal with the same `invalid: ` prefix that the not-running refusal has. The wire message is the contract text "the agent is not running, it is asleep: <name>".
- **Idle time check in two places.** The desktop app checks the idle time (0, or 5 to 1440) before it sends `settings.set`, and the server checks it again. The server decides.
- **Sign-in check before a wake.** `agent.wake` is a starting request, so the sign-in check of 0019 also runs before a wake, and a stopping server refuses a wake as it refuses any start.
- **The wake result is not stored (assumption A13 of 0018).** It shows until the agent's next sleep, the person's next action on the agent, or a server restart. The action record, the status, and the hold list keep what happened.

## Change 0022: Idle agents sleep past their waiting item, and one wait gives one item

### Intent

A hands-on run of idle sleep (0018) with a real `claude` showed two faults. First, each change from `working` to `waiting for input` adds a Needs you item, and any item keeps the agent awake, so an idle agent sleeps only after the person opens its terminal or dismisses the item. Second, `claude` redraws a hint line now and then while it waits. The office reads that output as `working` for a moment, the item leaves, and the return to `waiting for input` adds a new item and a new notification: the run saw 4 to 8 of them per agent with no input. With 20 to 30 agents, an idle `claude` that nobody looks at never sleeps, and the person learns to ignore the alerts. This change lets an item that only says "waiting for input" stop keeping an agent awake, keeps every other item kind as a blocker, and makes one wait give one item and one notification.

### Requirements

- **0022/FR-001** (US1): WHEN the system puts an agent that has a plain waiting item to sleep, THE SYSTEM SHALL keep that item in Needs you with its wait time and its latest event, SHALL record no event on it for the sleep, and SHALL remove it only as 0008/FR-004 and 0008/FR-005 say, for example when the person opens the agent's terminal, dismisses the item, or wakes the agent and it changes to `starting`.
- **0022/FR-002** (US1): IF the system cannot read the screen of an agent that has a waiting item at a check of 0018/FR-002, THEN THE SYSTEM SHALL treat that item as not plain and SHALL NOT put the agent to sleep at that check.

This change modified: 0003/FR-006, 0003/SC-004, 0008/FR-001, 0008/FR-005, 0018/FR-002, 0018/FR-009, and the 0018 Terms Idle and Idle since. The final text of 0003/FR-006, 0003/SC-004, 0018/FR-002, 0018/FR-009, and the Terms is under the change that owns each id, or under Terms below. The final text of 0008/FR-001 and 0008/FR-005 is under [Change 0008](../needs-you-queue/SPEC.md).

### Preserved behaviors

- **0022/PB-1:** An item that recorded a bell, an automatic hold, a resume hold, or a kept leftover, an item that holds a report for the person, and a waiting item whose agent's screen shows an open question under the 0012 question check keep the agent awake (0018/FR-002 b, as modified). Holds, waiting directives and reports, pending office text, unsent person text, and running children keep it awake as before (0018/FR-002 c to f).
- **0022/PB-2:** When the person opens an agent's terminal by any route or dismisses its item, the item leaves Needs you at once (0008/FR-004), and opening the terminal marks the agent's notifications read (0003/FR-009). Dismiss changes no status and is not recorded (0008 A4).
- **0022/PB-3:** An agent that works after a wait end, such as a person input, a directive, a report, or office text, and then changes back to `waiting for input` gets a new item and 1 notification, under the read rule of 0003/FR-006 (b), and its earlier item leaves when it changes to `working` (0008 US1 scenario 5).
- **0022/PB-4:** A bell, an exit, a failure, an automatic hold, a resume hold, a kept leftover, and a report for the person notify and change the item as before, with the coalescing of 0003/FR-006 (a). This change touches only the change between `working` and `waiting for input`.
- **0022/PB-5:** The status that the office shows does not change: an unprompted change still shows `working` for as long as the CLI prints, in the agent list, the office scene, and the subtree read of 0014 (0001/FR-014).
- **0022/PB-6:** While idle sleep is off, the office puts no agent to sleep and starts nothing because of idle time (0018 PB-2). A sleep adds no notification and no item (0018/FR-003).
- **0022/PB-7:** A failed sleep leaves the agent's status as the CLI shows it and adds exactly 1 action record entry, with the office as actor and the reason (0018/FR-009, as modified).
- **0022/PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration, and the question check reads only the screen copy that the office keeps (0001/FR-021, 0012 PB-4).

### Terms

Terms that this change defines. The other terms are as in 0001, 0003, 0007, 0008, 0012, 0013, 0015, 0017, and 0018.

| Term | Meaning |
|---|---|
| Work bound | The quiet period plus 10 seconds: 15 seconds with the default quiet period of 5 seconds (A10). |
| Wait end | One of these events for one agent: (1) a person input reaches its terminal; (2) the office types text into its terminal, such as a directive, a report, a startup prompt, or other office text, with or without Enter; (3) a session of its CLI starts, continues, or ends for any reason (exit, failure, stop, sleep, sign-in check, server stop); (4) the agent has shown `working` without a break for longer than the work bound. |
| Unprompted change | A change of an agent's status from `waiting for input` to `working`, or from `working` back to `waiting for input`, when no wait end occurred since the agent last changed to `waiting for input` in its current session. A redraw of a hint line is one example. A change to `working` is unprompted at first; when that stretch of `working` passes the work bound, the stretch is a wait end (4), and the return to `waiting for input` is prompted. |
| Prompted change | A change of status between `working` and `waiting for input` that is not unprompted. The first change to `waiting for input` in a session is always prompted, because the start of the session is a wait end. |
| Wait | The time from an agent's prompted change to `waiting for input` until its next wait end. Unprompted changes stay inside the same wait. |
| Waiting item | An item in Needs you that recorded only changes to `waiting for input` since it was added and that holds no report for the person. |
| Plain waiting item | A waiting item for which the question check finds no open question on the agent's screen at a check of 0018/FR-002. |

Modified terms of 0018 (final text):

- **Idle:** The agent's status is `waiting for input`. The idle time counts from the idle-since moment, which an unprompted change (0022 Terms) does not move.
- **Idle since:** The latest of three moments: the agent's last prompted change to `waiting for input` (0022 Terms), the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time. An unprompted change does not move it.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Wait | The time from a prompted change to `waiting for input` until the next wait end. | agent, start time, whether a wait end has occurred, start of the current stretch of `working` | At most one open wait per agent. It ends at a wait end. It is not shown and lives only while the session runs; a server stop is a wait end. |
| Item (extends 0008) | One agent that waits for the person. | adds: whether it recorded any event other than a change to `waiting for input` | As in 0008: at most one per agent, kept by the background server. Whether it is a waiting item follows from this attribute and its report; whether a waiting item is plain follows from the question check at each sleep check. |

### Success criteria

- **0022/SC-001:** With an idle time of 5 minutes and 10 idle agents that each have a plain waiting item and no other blocking condition, and with no person action, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment, and 100% of their items are still in Needs you afterwards.
- **0022/SC-002:** For each of these item kinds alone, a recorded bell, a recorded automatic hold, a recorded resume hold, a report for the person, and an open question on the screen (a yes-or-no choice, a choice list with a selection mark, and a request for permission), an idle agent whose only blocking condition is that item is put to sleep 0 times in 3 times the idle time.
- **0022/SC-003:** A test CLI that answers one prompt and then prints one line on its own every 30 seconds for 20 minutes, with no input, causes exactly 1 `waiting for input` notification and exactly 1 item, and the item's wait time at the end is at least 20 minutes.
- **0022/SC-004:** With a real `claude` that answers one prompt and then gets no input for 30 minutes, the person gets exactly 1 `waiting for input` notification for that agent (the 0018 hands-on run saw 4 to 8), and with an idle time of 5 minutes the agent shows `asleep` within 6 minutes and 10 seconds of its prompted change, with 0 dismisses and 0 terminal opens.
- **0022/SC-005:** In 10 rounds in which a directive reaches an agent that has an item, 10 of 10 items leave when the agent changes to `working`, and 10 of 10 returns to `waiting for input` each add exactly 1 new item and 1 notification.
- **0022/SC-006:** In 10 sleep checks of an agent that has a waiting item and whose screen cannot be read, the agent is put to sleep 0 times.
- **0022/SC-007:** In 10 rounds in which the person opens an agent's item and the agent then works for 2 minutes on its own, with no input, and returns to `waiting for input`, 10 of 10 returns each add exactly 1 new item and 1 notification, and with an idle time of 5 minutes the agent is put to sleep 0 times before 5 minutes have passed since that return.

### Notes from Deviations

- **No requirement is deferred or changed.** The Deviations section of 0022 is empty.
- **Criteria that tests ran with fewer rounds or shorter times (QA report).** 0022/SC-001, 0022/SC-003, 0022/SC-005, and 0022/SC-007 ran with fewer agents or rounds, a shortened quiet period, a shortened idle time, or a shorter run than the criterion states. 0022/SC-004 passed in a 30-minute run with a real signed-in `claude`: 1 notification, and a sleep 311.4 seconds after the prompted wait, with no dismiss and no terminal view open.
- **A failing stop (0018/FR-009, as modified).** The fake CLIs cannot make a stop fail, so a unit test covers the rule that a failed sleep bars the agent until a prompted wait, and no integration test covers it.
- **Assumption that the text names.** The bare `A10` in the Terms above names this assumption of 0022:
  **A10:** The work bound is the quiet period plus 10 seconds. The redraw output of a hint line lasts well under 1 second, but the status stays `working` until one quiet period passes with no output, so a redraw shows `working` for about one quiet period. A fixed 10 seconds would make every redraw count as work once the person sets a quiet period of 10 seconds or more. 10 more seconds keep redraws inside the bound and still end the wait for real work within seconds. A CLI that prints by itself without a break for longer than the bound while it waits, such as a long animation, gives 1 new item and 1 notification at its return.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0018-idle-sleep - 2026-11-23
- 0022-waiting-item-sleep - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
