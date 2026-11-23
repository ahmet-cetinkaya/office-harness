---
status: draft
capability: office
plan: specs/changes/0018-idle-sleep/PLAN.md
supersedes: 0001/FR-014
---

<!-- Draft for the spec review gate. Only the owner's approval, given through the review gate, changes the status. -->

# 0018: Put idle agents to sleep

## Intent

With 20 to 30 agents, many of them wait for input for hours. Each idle CLI, and the helper servers that it starts, keeps its memory all that time. This change adds an "Idle sleep" setting, off by default. When the person turns it on, the office puts an agent to sleep after it stays idle longer than a set time: it ends the CLI with the normal stop, keeps the session, and keeps the last screen visible. One key or one button wakes the agent, and the office continues the same session. When the CLI cannot continue it, the office uses the fallback and the hold of 0013.

## Context

- Source of truth: `docs/PRD.md`. References such as `0001/FR-025` point to requirements of the approved changes. Change 0001 is the base change and is not merged into the living spec yet.
- This change depends on 0001 (statuses, the quiet period, the stop, sessions, continuing a session, recent output, the action record), 0002 (the tree, directives, agent-made actions), 0003 (notifications), 0007 (holds and the office as an actor), 0008 (Needs you), 0012 (the queue, pending office text, unsent person text), 0013 (the check period, the fallback to a new session, the hold of a failing agent), and 0015 (reports).
- It changes one requirement of 0001: the list of statuses gets `asleep` (see MODIFIED).
- A sleep is not an exit under 0003/FR-006 (as modified by 0007): the agent shows `asleep`, not `exited`. Like a stop by the person, a sleep notifies nobody (FR-003).
- 0001/FR-004 and 0013 PB-1 stay as they are: the office starts no agent by itself. A sleep only ends a CLI. Only the person's Wake, or another person action, starts it again.
- The idea comes from an MIT-licensed reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, status, quiet period, recent output, action record, desktop app, background server | As defined in 0001. |
| Parent, child, descendant, directive | As defined in 0002. |
| Agent-made action, office hold, agent hold | As defined in 0007. A hold covers an agent under the rules of 0007/FR-004: the office hold, the agent's own hold, or the hold of one of its ancestors. |
| Needs you, item, attention event | As defined in 0008. |
| Queue, person input, pending office text, unsent person text | As defined in 0012. |
| Report | As defined in 0015. A report to a parent waits in the parent's queue. |
| Check period, ends by itself | As defined in 0013: 20 seconds after a start (0013 A2). |
| Idle sleep | The office setting that this change adds. It is off or on. When on, it has an idle time. |
| Idle time | A whole number of minutes from 5 to 1440. |
| Idle | The agent's status is `waiting for input` (0001/FR-014) without a break. |
| Idle since | The latest of three moments: the agent's change to `waiting for input`, the last person input that reached its terminal, and the last time the person turned idle sleep on or changed the idle time. |
| Running child | A child whose status is `starting`, `working`, or `waiting for input`. |
| Blocking condition | One of the six conditions of FR-002 that keep an agent awake. |
| Put to sleep | The office ends the agent's CLI as the person's stop does, keeps its last session for a wake, and shows the agent as `asleep`. |
| Going to sleep | The time from the office's last check before the stop until the CLI has ended. The agent keeps the status `waiting for input` meanwhile. |
| Asleep | The status of an agent that the office put to sleep (MODIFIED 0001/FR-014). |
| Wake | A person action on an asleep agent: continue its last session, with the fallback and the hold of 0013. |

## User stories

### US1: Put idle agents to sleep when the person turns it on (Priority: P1)

The person turns on idle sleep and keeps the idle time of 30 minutes. An agent that waits for input longer than that, and that nobody needs, is put to sleep. Its terminal view still shows its last output.

**Why this priority:** it is the value of the change. It gives back the memory of idle CLIs and their helper servers. It is safe only with the blocking conditions, so they are part of this story.

**Independent test:** turn on idle sleep with an idle time of 5 minutes. Let 3 agents wait for input with no item in Needs you, no hold, an empty queue, no text at the input line, and no running child. Within 6 minutes and 10 seconds, all 3 show `asleep`. Each terminal view shows the last output of its session. Each agent has one action record entry for the sleep, with the office as actor. No notification and no Needs you item appear.

**Scenarios:**

1. Given idle sleep is off, When agents wait for input for 24 hours, Then the office puts no agent to sleep and starts nothing.
2. Given the person turns idle sleep on, When the setting shows, Then the idle time is 30 minutes, and the person can set any whole number of minutes from 5 to 1440.
3. Given agents have been idle for 3 hours, When the person turns idle sleep on with 30 minutes, Then no agent is put to sleep before 30 minutes after the moment the person turned it on.
4. Given idle sleep is on and an agent has been idle for its idle time, When no blocking condition holds, Then the office begins to put it to sleep within 60 seconds, and the agent shows `asleep` when its CLI has ended.
5. Given an agent has been idle for its idle time, When it has an item in Needs you, a hold covers it, a directive or a report waits in its queue, pending office text or unsent person text waits at its input line, or one of its children runs, Then the office does not put it to sleep.
6. Given the office is about to stop an agent, When the agent changed to `working`, or a person input reached its terminal, since the office's first check, Then the office does not stop it.
7. Given an agent is asleep, When the person opens its terminal view, Then the view shows the recent output of its last session, the lines that the CLI printed while it ended included, with the text that the agent is asleep and a Wake button.
8. Given an agent is asleep, When the background server stops and starts again, Then the agent still shows `asleep`, nothing starts it, and "Resume interrupted" (0013) does not list it.
9. Given agents are asleep, When the person turns idle sleep off, Then the office puts no more agents to sleep, and the asleep agents stay asleep until the person wakes them.

### US2: Wake an asleep agent with one key or one button (Priority: P1)

The person opens an asleep agent and presses Enter or Wake. The office continues the agent's last session.

**Why this priority:** without a wake, a sleep loses the person's place in the work. It ships with US1.

**Independent test:** put an agent with a CLI that can continue a session to sleep. Open its terminal view, give it keyboard focus, and press Enter. Within 10 seconds the agent runs its last session again, the result says "continued", and the agent's action record has one wake entry with the person as actor.

**Scenarios:**

1. Given an agent is asleep and its terminal view has keyboard focus, When the person presses Enter, Then the office wakes the agent.
2. Given an agent is asleep and its terminal view has keyboard focus, When the person presses any other key that is not an app shortcut, Then no CLI gets the key and the agent stays asleep.
3. Given an agent is asleep, When the person presses Wake in its details or in its terminal view, Then the office wakes the agent.
4. Given the agent's CLI can continue its last session, When the office wakes it, Then the CLI continues that session, the result shows "checking" during the check period and then "continued".
5. Given the agent's CLI cannot continue its last session (the CLI does not support it, or the office does not know the CLI's session id), When the office wakes it, Then the office starts a new session at once, and the result and the wake entry give the reason.
6. Given the office or the agent is on hold, When the person wakes the agent, Then the office continues the session as without a hold, and the hold stays.
7. Given two clients press Wake for the same asleep agent at the same time, When the office carries out the wakes, Then it wakes the agent once, and the second client is told that the agent is no longer asleep.

### US3: Fall back and hold when a wake fails (Priority: P2)

When the continued session ends at once, the office starts the agent once in a new session. When that also ends at once, the office stops trying, puts the agent on hold, and tells the person.

**Why this priority:** it keeps a wake from failing in silence or in a loop, as 0013 does for a resume. US1 and US2 deliver their value without it for CLIs that continue their sessions.

**Independent test:** give one agent a CLI that ends 1 second after every start, put it to sleep, and wake it. The office starts it at most 2 times. Then the agent is on hold with the reason, and one notification names it.

**Scenarios:**

1. Given the office continued the last session of a woken agent, When that session ends by itself within the check period, or does not start, Then the office starts the agent once in a new session with the office as actor and the reason, and the result says "new session" with the reason.
2. Given the office started a new session for a woken agent, When that session ends by itself within the check period, or does not start, Then the office starts nothing more, puts the agent on hold with the office as actor and the reason, notifies the person once naming the agent, and the result says "on hold" with the reason.
3. Given the person stops the agent, starts a new task, or continues a session within the check period, When the session ends, Then the office does not start the agent again (0013 US2 scenario 4).

### US4: Tell senders that an agent is asleep (Priority: P2)

A parent that sends a directive to an asleep child, or a child that sends a report to an asleep parent, learns at once that the receiver is asleep.

**Why this priority:** without it, a sender gets the general "not running" answer and cannot tell a sleep from a crash. The office still refuses the action without this story (0002, 0015/FR-005).

**Independent test:** put a member to sleep. Its lead sends it a directive. The lead's answer says that the member is not running because it is asleep, and the refusal is in the action records of the lead and the member.

**Scenarios:**

1. Given an agent is asleep or going to sleep, When its parent sends it a directive, Then the office refuses the directive, the answer says that the agent is asleep, and the refusal is recorded.
2. Given an agent is asleep or going to sleep, When its child sends it a report, Then the office refuses the report, the answer says that the parent is asleep, and the refusal is recorded.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let the person turn idle sleep on and off in the office settings, with idle sleep off by default and an idle time of 30 minutes when the person first turns it on; SHALL accept an idle time only as a whole number of minutes from 5 to 1440, and refuse any other value with that rule and keep the previous value; SHALL keep the setting in the background server across server and machine restarts; and SHALL apply each change at once, without a restart.
- **FR-002** (US1): WHILE idle sleep is on, THE SYSTEM SHALL begin to put an agent to sleep within 60 seconds of the first moment at which the agent has been idle for at least the idle time since its idle-since moment and none of these blocking conditions holds: (a) the agent is not idle; (b) it has an item in Needs you (0008); (c) a hold covers it (0007/FR-004); (d) a directive or a report waits in its queue (0012, 0015); (e) pending office text or unsent person text waits at its input line (0012); (f) one of its children runs; and THE SYSTEM SHALL check all six conditions again right before it ends the CLI, and SHALL NOT end it when one of them holds.
- **FR-003** (US1): WHEN the system puts an agent to sleep, THE SYSTEM SHALL end the agent's CLI as the person's stop does (0001 A3), SHALL show the agent as `asleep` once the CLI has ended, SHALL keep the last session as the session that a wake continues, SHALL keep the recent output of that session with the lines that the CLI printed while it ended, SHALL add one action record entry for the sleep, and no separate stop entry, with the office as actor and the reason "idle for N minutes", and SHALL NOT notify the person or add a Needs you item for the sleep or for the end of the CLI that the sleep caused.
- **FR-004** (US1, US2): WHILE an agent is asleep, THE SYSTEM SHALL keep it `asleep` across server and machine restarts and after idle sleep is turned off, until the person wakes it, the person or an agent-made action stops it (it then shows `stopped`), deletes it, starts a new task on it (0001/FR-024), or continues one of its sessions (0001/FR-025), or an agent-made action starts it as 0002/FR-008 allows for an agent that is not running; and SHALL NOT list it in "Resume interrupted" (0013/FR-001).
- **FR-005** (US4): WHILE an agent is going to sleep or asleep, THE SYSTEM SHALL refuse a directive to it and a report to it, SHALL answer the sender that the receiver is not running because it is asleep, and SHALL record the refusal as 0002/FR-011 and 0015/FR-009 require; and WHEN a directive or a report still waits in the queue of an agent that is going to sleep, THE SYSTEM SHALL withdraw it with the reason that the receiver is asleep and record the withdrawal as 0012/FR-005 requires.
- **FR-006** (US2): WHILE an agent is asleep, THE SYSTEM SHALL offer the person a Wake button in the agent's details and in its terminal view; WHILE an agent is asleep and its terminal view has keyboard focus, THE SYSTEM SHALL wake the agent when the person presses Enter and SHALL send no key from that view to any CLI; and THE SYSTEM SHALL carry out a wake while the office or the agent is on hold.
- **FR-007** (US2, US3): WHEN the person wakes an agent, THE SYSTEM SHALL continue the agent's last session where its CLI supports it (0001/FR-025) and otherwise start a new session at once with the reason, SHALL apply the fallback of 0013/FR-004 and the hold of 0013/FR-005 to this start as 0013 applies them to a start of its action, and SHALL add one action record entry for the wake with the person as actor, with the reason when the wake started a new session; IF the agent is not asleep when the system carries out the wake, THEN THE SYSTEM SHALL start nothing and tell the person that the agent is no longer asleep.
- **FR-008** (US2, US3): THE SYSTEM SHALL show the result of each wake as text in the agent's details and terminal view as soon as it is known: waking, checking (during the check period), continued, new session (with the reason), on hold (with the reason), or no longer asleep.
- **FR-009** (US1): IF the agent's CLI does not end when the system puts the agent to sleep (the 0001 stop fails), THEN THE SYSTEM SHALL leave the agent's status as the CLI shows it, SHALL record the failure once in the agent's action record with the office as actor and the reason, and SHALL NOT try to put that agent to sleep again until it has left `waiting for input` and has been idle for the idle time again.

### MODIFIED

- **0001/FR-014** (US1): THE SYSTEM SHALL show every agent of the office with its name and its status as text, where the status is one of `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, `interrupted`, or `asleep`, `waiting for input` means that the CLI runs and produced no output during the quiet period, and `asleep` means that the office ended the CLI after an idle time and keeps its last session for a wake (0018/FR-003).

### PRESERVED

- **PB-1:** No agent starts without an explicit action. The office starts no agent by itself, at a server start or at any other time (0001/FR-004, 0013 PB-1). The fallback of FR-007 follows only the person's wake.
- **PB-2:** While idle sleep is off, the office behaves as before this change: it ends no CLI and starts nothing because of idle time.
- **PB-3:** A hold refuses only agent-made actions. The office carries out the person's own actions, Wake included, as before (0007 PB-1, PB-2). A hold stops no CLI by itself; a hold keeps an agent awake (FR-002 c).
- **PB-4:** The person's stop, new task, continue, and delete do what they did before (0001/FR-012, FR-024, FR-025). Each start, continue, stop, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).
- **PB-5:** Person input reaches the terminal of every agent whose CLI runs at once and unchanged, and the app shortcuts still leave a focused terminal (0001/FR-017 as modified by 0003, 0012 PB-5).
- **PB-6:** The notification of a wake hold is a notification of an automatic hold under 0003/FR-006 as modified by 0007: it names the agent and the event, and the focused-terminal exception does not apply to it (0013 PB-6).
- **PB-7:** Every status, `asleep` included, shows as text, never color alone, in the agent list, the office scene, and the subtree read of 0014/FR-001 (0001/FR-014, 0005 PB-3, 0014/FR-001).
- **PB-8:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

## User experience

**Flow (sleep):**

1. In the office settings, the person turns on "Idle sleep". The idle time field shows 30 minutes. A help text says that the office ends the CLI of an agent that stays idle this long, keeps its session, and continues it on Wake; that a CLI that cannot continue a session starts a new one; and that the sleep also ends every program that the CLI started and that still runs, such as a background task or a development server.
2. An agent stays idle for the idle time, and no blocking condition holds.
3. While its CLI ends, the agent keeps the status `waiting for input`.
4. The agent list, the office scene, and the subtree read show `asleep`. The terminal view shows the last output and the text "Asleep. Press Enter or Wake to continue the last session." with a Wake button.
5. The agent's details show the sleep entry in the action record: actor office, reason "idle for N minutes".

**Flow (wake):**

1. The person opens the asleep agent's terminal view and presses Enter, or presses Wake in the terminal view or in the details.
2. The result text shows "Waking", then "Checking" during the check period.
3. The result text shows "Continued", "New session" with the reason, or "On hold" with the reason. On a hold, one notification names the agent.
4. The action record shows the wake entry with the person as actor, and an entry for each new session and hold that the office made, with the office as actor and the reason.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Idle sleep off (default) | The setting is off; the idle time field is not available. | FR-001, US1 scenario 1 |
| Idle sleep on | The setting is on, with the idle time in minutes and the allowed range. | FR-001, US1 scenario 2 |
| Invalid idle time | The setting refuses the value, says that the idle time is a whole number of minutes from 5 to 1440, and keeps the previous value. | FR-001, Edge cases |
| Setting not saved | The reason; the setting keeps its previous value. | FR-001, Edge cases |
| Asleep | `asleep` as the status everywhere; the terminal view shows the last output, the asleep text, and Wake. | 0001/FR-014 (MODIFIED), FR-003, FR-006 |
| Waking | "Waking", then "Checking" during the check period; Wake is not available. | FR-008 |
| Continued | "Continued"; the terminal shows the live output. | FR-008, US2 scenario 4 |
| New session | "New session" with the reason. | FR-007, FR-008, US2 scenario 5, US3 scenario 1 |
| On hold | "On hold" with the reason; one notification. | FR-007, FR-008, US3 scenario 2 |
| No longer asleep | "This agent is no longer asleep"; nothing started. | FR-007, US2 scenario 7 |
| Sleep failed | An action record entry with the office as actor and the reason; the agent keeps its status. | FR-009 |
| Directive or report refused | The sender's answer says that the receiver is asleep; the refusal is in the records. | FR-005, US4 |
| Server not reachable or token rejected | The 0001 states; Wake and the setting are not available. | 0001/FR-026, 0001/FR-027 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The person can use the idle sleep switch, the idle time field, and Wake from the keyboard (0001/FR-028), and focus is always visible. Enter in the focused terminal view of an asleep agent is the one-key wake.
- The idle time field has a label that names the unit and the range.
- The status `asleep` and each wake result are text, never color alone.
- A sleep or a wake result of the agent whose terminal view or details are open is announced to assistive technology (A12).
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Idle sleep setting | Whether the office puts idle agents to sleep. | on or off, idle time in minutes, when it was last turned on or changed | One per office; kept by the background server. |
| Agent (extends 0001) | One agent. | status: also `asleep`; idle since (kept only while the background server runs) | As in 0001. |
| Session (extends 0001) | One run of an agent CLI. | the last session of an asleep agent is the one a wake continues | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | actions now also sleep, failed sleep, and wake; reason (optional) | Belongs to the agent it names (0001/FR-011). |
| Wake result | What a wake did. | agent, result (waking, checking, continued, new session, on hold, no longer asleep), reason | Shown in the agent's details and terminal view; not stored (A13). |

## Success criteria

- **SC-001:** With idle sleep never turned on, 10 agents that wait for input for 24 hours cause 0 sleeps, 0 starts by the office, and 0 sleep entries in the action record.
- **SC-002:** With an idle time of 5 minutes and 10 idle agents for which no blocking condition holds, 100% of them show `asleep` within 6 minutes and 10 seconds of their idle-since moment (5 minutes of idle time, at most 60 seconds to begin, at most 10 seconds of stop grace time).
- **SC-003:** For each of the blocking conditions (b) to (f) of FR-002, an idle agent for which only that condition holds is put to sleep 0 times in 3 times the idle time; for condition (a), an agent that changes to `working` between the first and the last check is stopped 0 times in 10 tries.
- **SC-004:** After the person turns idle sleep on with 30 minutes, 10 agents that were idle for 3 hours cause 0 sleeps in the first 30 minutes.
- **SC-005:** For an asleep agent whose CLI can continue its session, 1 key press (Enter) or 1 button press wakes it, and in 100% of 20 wakes the agent runs its last session again within 10 seconds of the press.
- **SC-006:** Over 20 sleep and wake cycles, each sleep adds exactly 1 action record entry (office as actor) and each successful wake adds exactly 1 (person as actor), and the sleeps cause 0 notifications and 0 Needs you items.
- **SC-007:** A wake of an agent whose CLI ends within 1 second of every start starts the agent at most 2 times; then the agent is on hold and exactly 1 notification names it.
- **SC-008:** After 10 server restarts, 5 of them after a SIGKILL of the server, 100% of the agents that were asleep still show `asleep`, 0 of them start without a person action, and "Resume interrupted" lists 0 of them.
- **SC-009:** 100% of the directives and reports sent to an asleep agent are refused, and each answer says that the receiver is asleep.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| Idle sleep is off (the default). | Empty | The office puts no agent to sleep and starts nothing (FR-001, PB-2). |
| The person enters 4, 1441, 0, 2.5, or text as the idle time. | Invalid | The setting refuses it, says the rule, and keeps the previous value (FR-001). |
| The person turns idle sleep on while agents have been idle for hours. | Boundary | The idle time counts from the moment of turning on; no agent sleeps before it passes (Terms: Idle since, SC-004). |
| The setting cannot be saved (the server is not reachable, or its storage fails). | Failure | The reason shows; the setting keeps its previous value, and the office goes on applying that value (FR-001). |
| The person changes the idle time while agents are idle. | Boundary | The idle time counts again from the change, for every agent (Terms: Idle since). |
| The agent changes to `working`, or a person input reaches it, before the last check. | Concurrent | The last check finds it, and the office does not stop the CLI (FR-002, US1 scenario 6). |
| The agent prints output, rings the bell, or a person types, after the last check while the CLI ends. | Concurrent | The sleep completes. The output stays in the recent output; a bell notifies as any bell does (0003/FR-006). Keys reach the CLI while it runs and are lost when it ends. The person can wake the agent (A5). |
| A directive or a report arrives while the agent is going to sleep. | Concurrent | It is refused with the reason that the agent is asleep, and recorded (FR-005). |
| A directive or a report entered the queue after the office's last check, just before the stop began. | Concurrent | It is withdrawn with the reason that the agent is asleep, and the withdrawal is in the sender's and the receiver's records (FR-005, 0012/FR-005). |
| The agent has a running child. | Boundary | It is not put to sleep (FR-002 f). When no child runs any more, the normal rule applies. |
| The person starts a child of an asleep parent. | Boundary | The parent stays asleep. A report of the child to it is refused with the reason that the parent is asleep (FR-005, A9). |
| The agent waits on a question that the person has already seen. | Boundary | It can be put to sleep; the office does not read the screen for questions. A wake continues the session (A1). |
| The CLI shows `waiting for input` while a program that it started still runs, for example a background task of the CLI, a development server, or a build started in the background. | Boundary | No blocking condition sees it. The sleep ends that program with the CLI, as the person's stop does, and its work is lost; the help text of the setting says so (A15). |
| The person stops an asleep agent. | Boundary | It shows `stopped`; the stop is recorded with the person as actor (FR-004). |
| The person deletes an asleep agent. | Boundary | It is deleted as a stopped agent is (FR-004, 0001/FR-012). |
| The office is put on hold while an agent is asleep, and then the person wakes it. | Boundary | The office carries out the wake; the agent's agent-made actions stay refused (FR-006, 0007/FR-002). |
| An agent-made action starts or stops an asleep agent. | Boundary | A start acts as on a stopped agent, as 0002 allows; it is not a wake, so no fallback and no hold of FR-007 apply. A stop makes it `stopped`, as the person's stop does (A10). |
| The agent's working folder or worktree is missing at a wake (0010/FR-008). | Failure | The continue does not start, the new session does not start either, and the agent is put on hold with the reason and one notification (FR-007). |
| The CLI does not support continuing, or the office does not know its session id. | Boundary | The wake starts a new session at once, with the reason (US2 scenario 5). |
| The agent is already on hold when a wake fails twice. | Boundary | No second hold and no second notification are made (0013 A9). |
| The server stops during a wake's check period. | Failure | The remaining checks end; the agent follows 0001/FR-004 and shows `interrupted` at the next start. |
| The server stops while an agent is going to sleep. | Failure | When the CLI ended for the sleep first, the agent shows `asleep`; otherwise it shows `interrupted` at the next start (0001/FR-004) (A6). |
| The server stops while an agent is asleep. | Boundary | The agent shows `asleep` at the next start (FR-004). |
| Two clients press Wake at the same time. | Concurrent | The office wakes the agent once; the second client is told that the agent is no longer asleep (FR-007). |
| The person stops, deletes, starts a new task on, or continues a session of an agent that is going to sleep. | Concurrent | The office carries out the person's action after the CLI has ended, as for an asleep agent (FR-004). |
| The person pastes text, with or without a line break, into the focused terminal view of an asleep agent. | Invalid | No CLI gets the text and the agent stays asleep; only a press of the Enter key wakes it (FR-006). |
| The CLI does not end at the sleep stop. | Failure | The agent keeps its status, the failure is recorded once, and the office does not try again until the agent was active and then idle for the idle time again (FR-009). |
| The person turns idle sleep off while agents are asleep. | Boundary | They stay asleep until woken (FR-004). An agent that is going to sleep completes its sleep. |
| The asleep status cannot be stored (for example, the disk is full). | Failure | The 0001 storage alert shows. After a restart the agent can show `interrupted` instead of `asleep`; nothing starts it (0001/FR-004). |
| The desktop app cannot reach the background server, or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). The server keeps applying the stored setting. |

## Assumptions

- **A1:** Idle means `waiting for input` under the quiet period of 0001 (0001 A1). The office does not read the screen for an open question before a sleep. The 0012 question check is not a blocking condition. An agent that waits on a question can be put to sleep when no Needs you item holds it, for example after the person opened its terminal. A wake continues the session; the CLI decides whether it shows the question again.
- **A2:** Opening or watching a terminal view is not person input. An agent can be put to sleep while its terminal view is open; the view then shows the asleep text and Wake.
- **A3:** The office checks the agents at least once a minute. The check period is not a setting in this change.
- **A4:** The recent output of an asleep agent includes the lines that the CLI printed while it ended, because the office keeps recording output until the CLI ends (0001/FR-019).
- **A5:** After the last check, the stop has begun and cannot be taken back. The sleep completes even when the agent prints output or a person types while the CLI ends. During that time the agent keeps the status `waiting for input` and Wake is not offered. The stop takes at most the 10-second stop grace time (0001 A3).
- **A6:** A server stop while an agent is going to sleep ends the CLI as for any running agent. The agent shows `asleep` only when its CLI ended for the sleep before the server stop.
- **A7:** An agent whose CLI cannot continue a session is put to sleep like any other. A wake then starts a new session with the reason, and the help text of the setting says so. The office does not keep such agents awake.
- **A8:** A wake follows the 0013 rules for the start: the 20-second check period (0013 A2), any end within it counts as ending by itself (0013 A3), at most one fallback (0013 A8), and no second hold for an agent already on hold (0013 A9).
- **A9:** The office keeps a parent awake while one of its children runs, because a report to a parent that is not running is refused (0015/FR-005). The person can still start a child of an asleep parent; the parent then stays asleep.
- **A10:** Agent-made actions treat an asleep agent as an agent that is not running, as they do today. An agent-made start does what 0002 does for a stopped agent and is not a wake. An agent-made stop of an asleep agent makes it `stopped`, as the person's stop does, because both mean that the agent is not to be woken. Waking from an agent request, a directive, or a report is out of scope.
- **A11:** A change of the idle sleep setting is a settings change and is not recorded in the action record (as 0007 A12). Each sleep, failed sleep, and wake is recorded.
- **A12:** Only the sleep and the wake results of the agent whose terminal view or details are open are announced to assistive technology. The agent list shows the status of every agent as text. This keeps 30 agents from flooding a screen reader.
- **A13:** The wake result is not stored. It shows until the agent's next sleep, the person's next action on the agent, or a server restart. The action record, the status, and the hold list keep what happened.
- **A14:** Only agents with a running CLI can be idle. Agents that are `exited`, `failed`, `stopped`, `interrupted`, or `asleep` are never put to sleep.
- **A15:** A CLI can show `waiting for input` while a program that it started still runs, for example a background task of the CLI, a development server, or a build started in the background. The office does not detect such programs. A sleep ends the CLI as the person's stop does (0001 A3), and such a program ends with it, as it does at the person's stop, so its work ends too. A program that detached itself from the CLI can keep running, as after the person's stop. The help text of the setting says this, so that the person turns idle sleep on knowing it.

## Out of scope

- Waking an agent on a directive, a report, or an agent request (A10).
- Putting agents to sleep for other reasons, such as memory pressure or a schedule.
- An idle sleep setting per agent.
- A "Wake all" action.
- A setting for the check period (A3, A8).
- Reading CLI output to find a refusal or a question (A1, 0013 A4).
- Finding programs that a CLI started and that still run, to keep such an agent awake (A15).

## Clarifications

### Session 2026-11-23

- Q: Is idle sleep on or off by default? → A: Off. While it is off, the office puts no agent to sleep and starts nothing (FR-001, PB-2). Decided under the owner's spec delegation, 2026-11-23.
- Q: Which idle times are allowed, and which value does the setting start with? → A: Whole minutes from 5 to 1440; 30 when the person first turns it on (FR-001). Decided under the owner's spec delegation, 2026-11-23.
- Q: Where is the setting kept, and when does a change apply? → A: In the background server, so the office applies it while the desktop app is closed; a change applies at once (FR-001). Decided under the owner's spec delegation, 2026-11-23.
- Q: When does the idle time start to count? → A: From the latest of the change to `waiting for input`, the last person input, and the last time the person turned the setting on or changed the idle time, so turning it on never puts long-idle agents to sleep at once (Terms: Idle since). Decided under the owner's spec delegation, 2026-11-23.
- Q: What keeps an idle agent awake? → A: An item in Needs you, a hold that covers it, a waiting directive or report, pending office text or unsent person text, or a running child. The office checks again right before the stop (FR-002). Decided under the owner's spec delegation, 2026-11-23.
- Q: Why does a running child keep its parent awake? → A: So that the child's reports reach the parent, because 0015/FR-005 refuses a report to a parent that is not running (A9). Decided under the owner's spec delegation, 2026-11-23.
- Q: Is a sleep an attention event? → A: No. It causes no notification and no Needs you item (FR-003). Decided under the owner's spec delegation, 2026-11-23.
- Q: What does an asleep agent show after a server restart? → A: `asleep`, not `interrupted`, and "Resume interrupted" does not list it (FR-004). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens to a directive or a report to an agent that is asleep or going to sleep? → A: It is refused with the reason that the receiver is asleep, and recorded (FR-005). Decided under the owner's spec delegation, 2026-11-23.
- Q: What is the one key that wakes an agent, and what do other keys do? → A: Enter in the focused terminal view of an asleep agent; other keys reach no CLI (FR-006). Decided under the owner's spec delegation, 2026-11-23.
- Q: Can the person press Wake while the stop is still running? → A: No. Wake is offered only once the agent is asleep; during the stop, at most the stop grace time, the agent keeps the status `waiting for input` (FR-006, A5). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does a wake work under a hold? → A: Yes. A wake is a person action, so the office carries it out under a hold; the hold stays (FR-006, PB-3). Decided under the owner's spec delegation, 2026-11-23.
- Q: What does a wake do when the continue fails? → A: The 0013 fallback: one new session, then a hold with one notification (FR-007, A8). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens when the CLI does not end at the sleep stop? → A: The agent stays as it was, the failure is recorded once, and the office tries again only after the agent was active and then idle again (FR-009). Decided under the owner's spec delegation, 2026-11-23.
- Q: Does an agent-made action wake an asleep agent? → A: No. Agent-made actions treat it as not running, as today (A10). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens to programs that the CLI started and that still run when it shows `waiting for input`? → A: The sleep ends them with the CLI, as the person's stop does; the office does not detect them, and the help text of the setting warns about it (A15, Out of scope). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens to a directive that entered the queue just before the stop began? → A: It is withdrawn with the reason that the receiver is asleep and recorded (FR-005). Decided under the owner's spec delegation, 2026-11-23.
- Q: Which sleeps and wakes are announced to assistive technology? → A: Those of the agent whose terminal view or details are open (A12). Decided under the owner's spec delegation, 2026-11-23.

## Deviations
