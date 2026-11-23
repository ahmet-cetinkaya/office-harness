---
status: shipped
capability: office
plan: specs/changes/archive/0022-waiting-item-sleep/PLAN.md
supersedes: 0003/FR-006 (in part), 0003/SC-004, 0008/FR-001, 0008/FR-005 (as modified by 0015), 0018/FR-002, 0018/FR-009, 0018 Terms (Idle, Idle since)
---

# 0022: Idle agents sleep past their waiting item, and one wait gives one item

<!-- Approved for coding on 2026-11-23 through the review gate (r2 at 45f595f), under the owner's spec delegation; shipped in PR #135. -->

## Intent

A hands-on run of idle sleep (0018) with a real `claude` showed two faults. First, each change from `working` to `waiting for input` adds a Needs you item, and any item keeps the agent awake, so an idle agent sleeps only after the person opens its terminal or dismisses the item. Second, `claude` redraws a hint line now and then while it waits. The office reads that output as `working` for a moment, the item leaves, and the return to `waiting for input` adds a new item and a new notification: the run saw 4 to 8 of them per agent with no input. With 20 to 30 agents, an idle `claude` that nobody looks at never sleeps, and the person learns to ignore the alerts. This change lets an item that only says "waiting for input" stop keeping an agent awake, keeps every other item kind as a blocker, and makes one wait give one item and one notification.

## Context

- Source of truth: `docs/PRD.md`. References such as `0018/FR-002` point to requirements of the approved changes or of the living spec of the office.
- Evidence: the 0018 QA report, section "Hands-on run (2026-11-23)", with a real signed-in `claude`.
- This change depends on 0001 (statuses, the quiet period, sessions), 0003 (notifications), 0007 (the automatic hold), 0008 (Needs you), 0012 (person input, pending office text, the question check), 0013 (the resume hold), 0015 (reports for the person), 0017 (the kept leftover), 0018 (idle sleep), and 0020 (temps, for scope only).
- It changes five requirements, one success criterion, and two Terms of 0018, Idle and Idle since (see MODIFIED). Idle since now counts from the last prompted change to `waiting for input`, not from every change to it. It also changes 0018 A1 in part: while an agent has its waiting item, an open question that the 0012 question check finds on its screen keeps it awake. After the person opens or dismisses the item, 0018 A1 applies as before.
- 0018/SC-003 reads condition (b) of 0018/FR-002 as this change modifies it.
- The office still decides `working` and `waiting for input` from the quiet period of 0001, as before. This change does not change how the office detects a status. It changes what a status change causes.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, status, quiet period | As defined in 0001. |
| Notification | As defined in 0003. |
| Automatic hold | As defined in 0007 (0007/FR-008). |
| Needs you, item, attention event, wait time | As defined in 0008. |
| Person input, pending office text, open question, question check | As defined in 0012. The question check is the check of the bottom lines of the screen in 0012/FR-002: a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval. |
| Resume hold | The hold of 0013/FR-005. The hold after a failed wake (0018/FR-007) is a resume hold. |
| Report for the person | As defined in 0015 (0015/FR-007). |
| Kept leftover | The event of 0017 for a process that the office could not end after an unclean stop. |
| Idle sleep, idle time, blocking condition, put to sleep, asleep, wake | As defined in 0018. |
| Work bound | The quiet period plus 10 seconds: 15 seconds with the default quiet period of 5 seconds (A10). |
| Wait end | One of these events for one agent: (1) a person input reaches its terminal; (2) the office types text into its terminal, such as a directive, a report, a startup prompt, or other office text, with or without Enter; (3) a session of its CLI starts, continues, or ends for any reason (exit, failure, stop, sleep, sign-in check, server stop); (4) the agent has shown `working` without a break for longer than the work bound. |
| Unprompted change | A change of an agent's status from `waiting for input` to `working`, or from `working` back to `waiting for input`, when no wait end occurred since the agent last changed to `waiting for input` in its current session. A redraw of a hint line is one example. A change to `working` is unprompted at first; when that stretch of `working` passes the work bound, the stretch is a wait end (4), and the return to `waiting for input` is prompted. |
| Prompted change | A change of status between `working` and `waiting for input` that is not unprompted. The first change to `waiting for input` in a session is always prompted, because the start of the session is a wait end. |
| Wait | The time from an agent's prompted change to `waiting for input` until its next wait end. Unprompted changes stay inside the same wait. |
| Waiting item | An item in Needs you that recorded only changes to `waiting for input` since it was added and that holds no report for the person. |
| Plain waiting item | A waiting item for which the question check finds no open question on the agent's screen at a check of 0018/FR-002. |

## User stories

### US1: An idle agent sleeps although nobody handled its waiting item (Priority: P1)

The person turns on idle sleep with 30 minutes. A `claude` agent answers a prompt and then waits for input. Nobody opens its terminal or dismisses its item. After the idle time, the office puts it to sleep. The item stays in Needs you, so the person still sees that the agent has a result for him.

**Why this priority:** this is the fault that the hands-on run found. Without it, idle sleep gives back memory only for agents that the person has already handled, which defeats the setting with 20 to 30 agents. The other item kinds are part of this story, because the change is safe only when they still keep an agent awake.

**Independent test:** turn on idle sleep with an idle time of 5 minutes. Give one agent one prompt. When it changes to `waiting for input`, it gets one item. Make sure that no other blocking condition holds, and do not open or dismiss the item. Within 6 minutes and 10 seconds of that change, the agent shows `asleep`. Its item is still in Needs you, with the wait time counted from that change. Opening its terminal removes the item and shows the last output and Wake.

**Scenarios:**

1. Given idle sleep is on and an idle agent has a plain waiting item and no other blocking condition, When the idle time has passed since its idle-since moment, Then the office puts it to sleep, and its item stays in Needs you with its wait time and latest event.
2. Given an idle agent whose item recorded a bell, When the idle time passes, Then the office does not put it to sleep.
3. Given an idle agent whose item holds a report for the person, When the idle time passes, Then the office does not put it to sleep.
4. Given an idle agent whose item recorded an automatic hold or a resume hold, When the idle time passes, Then the office does not put it to sleep.
5. Given an idle agent with its waiting item, and its screen shows a permission request as a choice list with a selection mark, When the idle time passes, Then the office does not put it to sleep.
6. Given the agent of scenario 5, When the person dismisses its item, Then the rules of 0018 apply as before, and the office can put it to sleep (0018 A1).
7. Given an asleep agent whose plain waiting item stays in Needs you, When the person presses Wake, Then the item leaves when the agent changes to `starting`.
8. Given an idle agent with its waiting item, When the office cannot read its screen at a check, Then the office does not put it to sleep at that check.

### US2: One wait gives one item and one notification (Priority: P1)

A `claude` agent waits for input and redraws its hint line now and then. The person gets one notification and sees one item for this wait. The item does not leave and come back with each redraw. When the agent gets real input and works again, its next wait gets a new item and a new notification as before.

**Why this priority:** in the run, 4 to 8 notifications per agent came with no input. Repeated alerts train the person to ignore them (0008 A5), and an item that leaves and comes back resets its wait time and its place in Needs you. This story has value on its own, also with idle sleep off.

**Independent test:** run a test CLI that answers one prompt, goes quiet, and then prints one line on its own every 30 seconds for 10 minutes, with no input. The person gets exactly 1 `waiting for input` notification and sees exactly 1 item for it, with the wait time counted from the first change. Then a directive reaches the agent: the item leaves when the agent changes to `working`, and the return to `waiting for input` adds 1 new item and 1 new notification.

**Scenarios:**

1. Given an agent with an item, When it makes an unprompted change to `working` and back to `waiting for input`, Then the item stays with its wait time and its latest event, and the person gets no notification.
2. Given the person dismissed an agent's item, When the agent makes an unprompted change to `working` and back, Then no new item and no notification appear.
3. Given the person opened an agent's terminal and then opened another agent without typing, When the first agent makes an unprompted change to `working` and back, Then no new item and no notification appear.
4. Given an agent with an item, When the office types a directive or a report into its terminal and the agent changes to `working`, Then the item leaves, and When the agent changes back to `waiting for input`, Then it gets a new item and the person gets 1 notification, under the read rule of 0003/FR-006 (b).
5. Given a person typed into an agent's terminal and then left it, When the agent works and changes back to `waiting for input`, Then the change is prompted, and the item and the notification come as before.
6. Given an agent waits for input, When it rings the bell, exits, fails, is put on hold automatically, or sends a report for the person, Then the notification and the item change as before this change.
7. Given an agent with an item makes an unprompted change to `working`, When a person input or office text reaches its terminal while it shows `working`, Then the item leaves.
8. Given the person resizes the window and the CLI redraws its screen, When the agent changes to `working` and back, Then no new item and no notification appear.
9. Given the person opened an agent's item while the agent paused during a task, and no input reached the agent after it, When the agent shows `working` for 2 minutes and then changes to `waiting for input`, Then the stretch passed the work bound, the return is prompted, the agent gets 1 new item and the person gets 1 notification, and with idle sleep on the idle time counts from that return.

## Requirements

### ADDED

- **FR-001** (US1): WHEN the system puts an agent that has a plain waiting item to sleep, THE SYSTEM SHALL keep that item in Needs you with its wait time and its latest event, SHALL record no event on it for the sleep, and SHALL remove it only as 0008/FR-004 and 0008/FR-005 say, for example when the person opens the agent's terminal, dismisses the item, or wakes the agent and it changes to `starting`.
- **FR-002** (US1): IF the system cannot read the screen of an agent that has a waiting item at a check of 0018/FR-002, THEN THE SYSTEM SHALL treat that item as not plain and SHALL NOT put the agent to sleep at that check.

### MODIFIED

- **0003/FR-006** (as modified by 0007 and 0015) (US2): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, is put on hold automatically (0007/FR-008), or sends a report for the person (0015/FR-007), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and while an unread report notification of that agent exists, a further report adds none; (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification, and exception (b) does not apply to the automatic hold or to a report, because the terminal does not show them; and (c) an unprompted change to `waiting for input` (0022 Terms) adds no notification and sends no OS notification.
- **0003/SC-004** (US2): While the desktop app runs, the user gets a notification within 10 seconds after an agent exits, fails, rings the bell, or makes a prompted change to `waiting for input` (that is, within 10 seconds after the quiet period ends).
- **0008/FR-001** (US2): WHEN an attention event occurs that 0003/FR-006 does not show as read on arrival, THE SYSTEM SHALL add an item for that agent to Needs you, or, if the agent already has an item, SHALL keep that one item and its wait time and record the new event as its latest event. A bell that 0003/FR-006 (a) coalesces still counts as an attention event for Needs you. An unprompted change to `waiting for input` is not an attention event for Needs you: THE SYSTEM SHALL NOT add an item for it and SHALL NOT change an existing item, its wait time, or its latest event because of it.
- **0008/FR-005** (as modified by 0015) (US2): WHEN an agent with an item is deleted or changes to `stopped`, or, for an item that holds no report, changes to `starting`, makes a prompted change to `working`, gets a person input or office text in its terminal while it shows `working`, or has shown `working` without a break for longer than the work bound, or, for an item that holds no report and whose latest event is an automatic hold (the resume hold of 0013 included), has its hold released, THE SYSTEM SHALL remove that agent's item from Needs you; an unprompted change to `working` SHALL leave the item, its wait time, and its latest event unchanged while the stretch of `working` stays within the work bound.
- **0018/FR-002** (US1): WHILE idle sleep is on, THE SYSTEM SHALL begin to put an agent to sleep within 60 seconds of the first moment at which the agent shows `waiting for input`, the idle time has passed since its idle-since moment, and none of these blocking conditions holds: (a) the agent does not show `waiting for input`; (b) it has an item in Needs you that is not a plain waiting item, that is, an item that recorded a bell, a failure, an exit, an automatic hold, a resume hold, or a kept leftover, an item that holds a report for the person, or a waiting item whose agent's screen shows an open question under the question check of 0012/FR-002; (c) a hold covers it (0007/FR-004); (d) a directive or a report waits in its queue (0012, 0015); (e) pending office text or unsent person text waits at its input line (0012); (f) one of its children runs. The idle-since moment is the latest of the agent's last prompted change to `waiting for input`, the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time; an unprompted change does not move it. THE SYSTEM SHALL check all six conditions again right before it ends the CLI, and SHALL NOT end it when one of them holds.
- **0018/FR-009** (US1): IF the agent's CLI does not end when the system puts the agent to sleep (the 0001 stop fails), THEN THE SYSTEM SHALL leave the agent's status as the CLI shows it, SHALL record the failure once in the agent's action record with the office as actor and the reason, and SHALL NOT try to put that agent to sleep again until it has made a prompted change to `waiting for input` after the failure and the idle time has passed since that change; an unprompted change SHALL NOT allow a new try.
- **0018 Terms, Idle** (US1): The agent's status is `waiting for input`. The idle time counts from the idle-since moment, which an unprompted change (0022 Terms) does not move.
- **0018 Terms, Idle since** (US1): The latest of three moments: the agent's last prompted change to `waiting for input` (0022 Terms), the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time. An unprompted change does not move it.

### REMOVED

None.

### PRESERVED

- **PB-1:** An item that recorded a bell, an automatic hold, a resume hold, or a kept leftover, an item that holds a report for the person, and a waiting item whose agent's screen shows an open question under the 0012 question check keep the agent awake (0018/FR-002 b, as modified). Holds, waiting directives and reports, pending office text, unsent person text, and running children keep it awake as before (0018/FR-002 c to f).
- **PB-2:** When the person opens an agent's terminal by any route or dismisses its item, the item leaves Needs you at once (0008/FR-004), and opening the terminal marks the agent's notifications read (0003/FR-009). Dismiss changes no status and is not recorded (0008 A4).
- **PB-3:** An agent that works after a wait end, such as a person input, a directive, a report, or office text, and then changes back to `waiting for input` gets a new item and 1 notification, under the read rule of 0003/FR-006 (b), and its earlier item leaves when it changes to `working` (0008 US1 scenario 5).
- **PB-4:** A bell, an exit, a failure, an automatic hold, a resume hold, a kept leftover, and a report for the person notify and change the item as before, with the coalescing of 0003/FR-006 (a). This change touches only the change between `working` and `waiting for input`.
- **PB-5:** The status that the office shows does not change: an unprompted change still shows `working` for as long as the CLI prints, in the agent list, the office scene, and the subtree read of 0014 (0001/FR-014).
- **PB-6:** While idle sleep is off, the office puts no agent to sleep and starts nothing because of idle time (0018 PB-2). A sleep adds no notification and no item (0018/FR-003).
- **PB-7:** A failed sleep leaves the agent's status as the CLI shows it and adds exactly 1 action record entry, with the office as actor and the reason (0018/FR-009, as modified).
- **PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration, and the question check reads only the screen copy that the office keeps (0001/FR-021, 0012 PB-4).

## User experience

This change adds no new control and no new view. It changes when existing items, notifications, and sleeps appear.

**Flow:**

1. An agent answers a prompt and changes to `waiting for input`. The person gets 1 notification, and Needs you shows 1 item for the agent.
2. The CLI redraws its screen now and then. The agent list can show `working` for a moment. The item stays in place with its wait time, and no new notification comes.
3. With idle sleep on, the idle time passes and no blocking condition holds. The agent shows `asleep`. Its item stays in Needs you.
4. The person opens the item. The terminal view shows the last output and the asleep text with Wake (0018). The item leaves.
5. The person presses Enter or Wake. The agent continues its session (0018).

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Waiting, first change | 1 notification and 1 item with the event and the wait time. | PB-3, 0008/FR-001 (MODIFIED) |
| Waiting, CLI redraws | The same item, unchanged; no new notification; the status can read `working` for a moment. | US2 scenarios 1 to 3 and 8, 0003/FR-006 (MODIFIED), 0008/FR-005 (MODIFIED), PB-5 |
| Waiting after a dismiss, CLI redraws | No item and no notification. | US2 scenario 2, 0008/FR-001 (MODIFIED) |
| New wait after input | The old item has left; 1 new item and 1 notification. | US2 scenarios 4 and 5, PB-3 |
| New wait after long work on its own | After more than the work bound of `working`, the old item has left; at the return, 1 new item and 1 notification. | US2 scenario 9, 0008/FR-005 (MODIFIED), SC-007 |
| Asleep with its item | `asleep` as the status; the item stays in Needs you with its wait time. | US1 scenario 1, FR-001 |
| Awake because of its item | `waiting for input`; the item shows its bell, hold, or report; or the screen shows a question. | US1 scenarios 2 to 5, 0018/FR-002 (MODIFIED), PB-1 |
| Screen not readable at a check | `waiting for input`; no sleep at that check. | US1 scenario 8, FR-002 |
| Empty, loading, server not reachable, unauthorized | As in 0001 and 0008. | 0008 Visible states, 0001/FR-026, 0001/FR-027 |

**Design:** no fixed design exists, and none is needed: no visual changes.

**Accessibility:** no new control. The count change of Needs you is still announced without moving focus (0008), and it now changes less often, because a redraw adds no item. Every status and event still shows as text, never color alone (0008 PB-4, 0018 PB-7).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Wait | The time from a prompted change to `waiting for input` until the next wait end. | agent, start time, whether a wait end has occurred, start of the current stretch of `working` | At most one open wait per agent. It ends at a wait end. It is not shown and lives only while the session runs; a server stop is a wait end. |
| Item (extends 0008) | One agent that waits for the person. | adds: whether it recorded any event other than a change to `waiting for input` | As in 0008: at most one per agent, kept by the background server. Whether it is a waiting item follows from this attribute and its report; whether a waiting item is plain follows from the question check at each sleep check. |

## Success criteria

- **SC-001:** With an idle time of 5 minutes and 10 idle agents that each have a plain waiting item and no other blocking condition, and with no person action, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment, and 100% of their items are still in Needs you afterwards.
- **SC-002:** For each of these item kinds alone, a recorded bell, a recorded automatic hold, a recorded resume hold, a report for the person, and an open question on the screen (a yes-or-no choice, a choice list with a selection mark, and a request for permission), an idle agent whose only blocking condition is that item is put to sleep 0 times in 3 times the idle time.
- **SC-003:** A test CLI that answers one prompt and then prints one line on its own every 30 seconds for 20 minutes, with no input, causes exactly 1 `waiting for input` notification and exactly 1 item, and the item's wait time at the end is at least 20 minutes.
- **SC-004:** With a real `claude` that answers one prompt and then gets no input for 30 minutes, the person gets exactly 1 `waiting for input` notification for that agent (the 0018 hands-on run saw 4 to 8), and with an idle time of 5 minutes the agent shows `asleep` within 6 minutes and 10 seconds of its prompted change, with 0 dismisses and 0 terminal opens.
- **SC-005:** In 10 rounds in which a directive reaches an agent that has an item, 10 of 10 items leave when the agent changes to `working`, and 10 of 10 returns to `waiting for input` each add exactly 1 new item and 1 notification.
- **SC-006:** In 10 sleep checks of an agent that has a waiting item and whose screen cannot be read, the agent is put to sleep 0 times.
- **SC-007:** In 10 rounds in which the person opens an agent's item and the agent then works for 2 minutes on its own, with no input, and returns to `waiting for input`, 10 of 10 returns each add exactly 1 new item and 1 notification, and with an idle time of 5 minutes the agent is put to sleep 0 times before 5 minutes have passed since that return.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The agent has not changed to `waiting for input` in its session yet. | Empty | No wait exists. Its first change to `waiting for input` is prompted and adds the item and the notification as before (Terms: Prompted change). |
| The agent has no item, because the event showed as read in its focused terminal (0003/FR-006 b). | Empty | A later unprompted change adds no item and no notification (0008/FR-001, MODIFIED). |
| The CLI prints output while it shows a question, for example a permission request that redraws. | Boundary | The change is unprompted: no new item, no notification. The question keeps the agent awake while the item stays (0018/FR-002 b). |
| The CLI works on its own after a wait, with no input, for example when a background task of the CLI ends and it goes on, or it ends a long tool run after the person opened its item. | Boundary | When the stretch of `working` passes the work bound, that is a wait end: the item leaves (0008/FR-005, MODIFIED). The return is prompted: 1 new item, 1 notification, and a new idle-since moment, so no sleep comes before the idle time from the return (US2 scenario 9, SC-007, A2). |
| The CLI works on its own for less than the work bound, for example a short burst of output. | Boundary | The office treats it as a redraw: the item stays, and the return adds no notification (A2, A10). |
| The person sets a longer quiet period. | Boundary | The work bound grows with it, so a redraw, whose `working` stretch lasts about one quiet period, stays under the bound (A10). |
| The person resizes the window, and the CLI redraws its screen. | Boundary | A resize is not person input. The change is unprompted (US2 scenario 8). |
| The item recorded a bell, and later a change to `waiting for input` became its latest event. | Boundary | The item is not plain. The agent stays awake until the person handles the item (0018/FR-002 b, A5). |
| The question check gives a false match on a screen with no real question. | Invalid | The agent stays awake while its item stays. The person opens or dismisses the item, and then 0018 applies (A4). |
| The screen shows a question of a shape that the question check does not know. | Invalid | The item counts as plain, and the agent can be put to sleep. A wake continues the session, and the CLI decides whether it shows the question again (A4, 0018 A1). |
| A directive is typed into the terminal while the agent shows `working` from an unprompted change. | Concurrent | 0012/FR-002 types only while the agent waits for input, so this does not occur for a directive. A person input or other office text at that time removes the item (0008/FR-005, MODIFIED). |
| A person input and an unprompted change reach the office at the same moment. | Concurrent | The input is a wait end. The next change to `working` is prompted, and the item leaves. |
| The office makes its last sleep check while an unprompted change shows `working`. | Concurrent | Condition (a) holds and the office does not stop the CLI. After a return within the work bound, the agent can be put to sleep at once, because the idle-since moment did not move (0018/FR-002, MODIFIED). |
| Two desktop clients: one dismisses the item, and the CLI then redraws. | Concurrent | No new item in either client (0008 A6). |
| The office cannot read the agent's screen at a sleep check. | Failure | The item counts as not plain, and the agent stays awake at that check (FR-002). |
| The server stops while an agent has a plain waiting item. | Failure | The session ends, which is a wait end. The item survives as in 0008. The agent shows `interrupted` at the next start, and a later session starts a new wait. |
| The CLI exits or fails during a wait. | Failure | The exit or the failure notifies and changes the item as before (PB-4). |
| The desktop app cannot reach the background server, or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). The server keeps the items and the waits. |

## Assumptions

- **A1:** An unprompted change to `waiting for input` adds no 0003 notification and no OS notification, as well as no item. One wait gives at most one `waiting for input` notification. The person still gets the notification of the prompted change that began the wait.
- **A2:** The office decides whether a change is unprompted only from the wait ends that it sees itself: person input, text that it types, session starts and ends, and the length of a stretch of `working`. It never reads the screen for this. A CLI that works by itself for longer than the work bound ends the wait, so the end of that work gives a new item and a notification, also after the person opened or dismissed the earlier item. A CLI that prints by itself for a shorter time stays in the same wait.
- **A3:** An item can show next to the status `working` during an unprompted change, for at most the work bound. The status detection of 0001 does not change.
- **A4:** The question check at a sleep check is the 0012 check, with its known limits. A false match only keeps the agent awake until the person handles its item. A question shape that the check does not know reads as no question, and the agent can be put to sleep with its item; this is the 0018 A1 risk, now limited to unknown shapes.
- **A5:** An item that recorded a bell in its life keeps the agent awake also when a later change to `waiting for input` became its latest event, because a bell is the CLI's explicit call for the person. A CLI that rings the bell at each wait keeps its agent awake until the person handles the item.
- **A6:** A plain waiting item stays in Needs you when its agent goes to sleep, because 0008/FR-005 removes no item for `asleep` and the person has not seen the result yet. The wake changes the agent to `starting`, which removes the item.
- **A7:** The idle-since moment counts from the last prompted change to `waiting for input`, so a CLI that redraws more often than the idle time can still be put to sleep. The sleep still needs `waiting for input` at both checks. Decided under the owner's spec delegation (Clarifications).
- **A8:** Failed, exited, and kept-leftover items belong to agents whose CLI does not run, so 0018 never considers them for a sleep (0018 A14). They are named in 0018/FR-002 (b) only so that the rule for each item kind is explicit.
- **A9:** The temps of 0020 get the dedupe of US2, because they are agents. The idle limit of a temp still stops while it has any item in Needs you (0020/FR-004, 0020 A12). This change does not modify that rule.
- **A10:** The work bound is the quiet period plus 10 seconds. The redraw output of a hint line lasts well under 1 second, but the status stays `working` until one quiet period passes with no output, so a redraw shows `working` for about one quiet period. A fixed 10 seconds would make every redraw count as work once the person sets a quiet period of 10 seconds or more. 10 more seconds keep redraws inside the bound and still end the wait for real work within seconds. A CLI that prints by itself without a break for longer than the bound while it waits, such as a long animation, gives 1 new item and 1 notification at its return.

## Out of scope

- Changing how the office detects `working` and `waiting for input`, or the quiet period of 0001 (PB-5, A3).
- New item kinds in Needs you, such as a separate "question" or "permission" item, and new text on an item (A4).
- Reading the screen to tell a redraw from real work (A2).
- The idle limit of temps in 0020: a plain waiting item still stops it (A9).
- Reminders or repeated alerts for a wait that stays long (0008 A5).

## Clarifications

### Session 2026-11-23

- Q: Does a plain `waiting for input` item still keep an idle agent awake? → A: No. Once the idle time has passed, it stops blocking idle sleep. Items for a bell, a failure, an exit, an automatic hold, a resume hold, a kept leftover, a report for the person, and an open question that the 0012 question check finds keep blocking (0018/FR-002 b, MODIFIED). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does output that the CLI prints by itself while it waits add a new item or end the existing one? → A: No. A change with no wait end since the last change to `waiting for input` is unprompted: it adds no item and does not end or change the existing one (0008/FR-001 and 0008/FR-005, MODIFIED). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does an unprompted return to `waiting for input` add a 0003 notification? → A: No. One wait gives at most one `waiting for input` notification (0003/FR-006 c, A1). Decided under the owner's spec delegation, 2026-11-23.
- Q: How is "the same wait" defined? → A: From wait ends that the office observes itself, without the screen: a person input, text that the office types, or a session start or end (Terms: Wait end, Unprompted change). Review r1 added a fourth wait end, a stretch of `working` longer than the work bound (see below). Decided under the owner's spec delegation, 2026-11-23.
- Q: There is no "question" or "permission" item kind in Needs you. How does such an item keep blocking? → A: A waiting item counts as not plain while the 0012 question check finds an open question on the agent's screen at the sleep check, and the agent stays awake when the office cannot read the screen (0018/FR-002 b, FR-002, A4). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does an unprompted change restart the idle-since moment of 0018? → A: No. It keeps the moment of the last prompted change to `waiting for input`, so a CLI that redraws more often than the idle time can still be put to sleep (0018/FR-002, A7). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does a plain waiting item also stop blocking the idle limit of a temp (0020/FR-004)? → A: No, out of scope. Ending a temp is final, and a sleep can be undone with a wake (A9). Decided under the owner's spec delegation, 2026-11-23.
- Q: How does the office keep the end of real work from looking like a redraw, without reading the screen? → A: A stretch of `working` longer than the work bound, the quiet period plus 10 seconds, is a wait end. The return after it is prompted, gives 1 item and 1 notification, and moves the idle-since moment (Terms, 0008/FR-005, A10). Decided in spec review r1 under the owner's spec delegation, 2026-11-23.
- Q: How does a failed sleep get a new try now that an unprompted change does not move the idle-since moment? → A: Only after a prompted change to `waiting for input` and a full idle time from it (0018/FR-009, MODIFIED). Decided in spec review r1 under the owner's spec delegation, 2026-11-23.

## Deviations
