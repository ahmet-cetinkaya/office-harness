---
status: shipped
capability: hold-and-limits
plan: specs/changes/archive/0007-hold-and-limits/PLAN.md
supersedes: 0002/FR-009, 0003/FR-006 (in part)
---

<!-- Approved by the owner on 2026-11-19. -->

# 0007: Hold switch and agent action limits

## Intent

With 0002, agents create agents, start and stop them, start new tasks, and send directives without the user. One agent that loops can flood a terminal with directives or churn agents faster than the user can read. This change gives the user one action that holds every agent-made action in the office, a hold for one agent and its descendants, and rate limits that refuse an agent's actions beyond a set pace. The terminals keep running through a hold. This change amends the approved 0002.

## Context

- Source of truth: `docs/PRD.md` and the approved specs 0001, 0002, and 0003. `0002/FR-###` points to a requirement in 0002.
- 0002 already fixes the shape of the tree: three levels (D-09, 0002/FR-001), directives go down only, and a member creates no office agents (0002/FR-008, 0002 A4). An agent therefore cannot build an endless chain of agents. The risks left are rate risks: one agent flooding one terminal with directives, an agent creating or restarting agents in a loop, and a CEO or lead that loops over its whole subtree.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, directive, action record, role, descendant | As defined in 0001 and 0002. |
| Agent-made action | An action that a running agent asks the office for from inside its session (0002/FR-008): create an agent, start or stop an agent, start a new task, or send a directive. Actions the user takes in the desktop app are not agent-made. |
| Office hold | A state of the office in which every agent-made action is refused. |
| Agent hold | A state of one agent in which agent-made actions by that agent or its descendants, and agent-made actions on them, are refused. |
| Rate limit | The highest number of one kind of agent-made action that one agent may take in a rolling 60-second window. |

## User stories

### US1 — Hold every agent-made action at once (Priority: P1)

The user sees agents acting in a way he does not want and holds the whole office with one action. No agent can create, start, stop, task, or send a directive until he releases the hold. The terminals keep running and he can still type into them.

**Why this priority:** it is the last line of defense against any loop, including ones that no limit catches, and it works without knowing which agent is at fault.

**Independent test:** with a CEO, one lead, and two members running, hold the office. A directive from the lead to a member is refused with a reason that the lead's terminal shows. The user can still type into every terminal. Release the hold: the same directive goes through. The action record shows the hold, the refusal, and the release, with the user as the actor of the hold and the release.

**Scenarios:**

1. Given agents run, When the user holds the office, Then every agent-made action after that is refused with the reason "the office is on hold", and the refusal is recorded.
2. Given the office is on hold, When the user types into an agent's terminal, Then the input reaches the agent as without a hold.
3. Given the office is on hold, When the user creates, starts, stops, or deletes an agent from the desktop app, Then the action works as without a hold.
4. Given the office is on hold, When the server restarts, Then the office is still on hold.
5. Given the office is on hold, When the user releases the hold, Then agent-made actions work again, and the release is recorded with the user as actor.

### US2 — Hold one agent and its subtree (Priority: P2)

The user holds one agent. That agent and its descendants can no longer take agent-made actions, and no other agent can take agent-made actions on them. The rest of the office keeps working.

**Why this priority:** it stops one team that loops without stopping the other teams, but the office hold (US1) already covers the emergency.

**Independent test:** with a CEO, two leads, and one member under each lead, hold lead A. A directive from lead A to its member is refused. A directive from the CEO to lead A is refused. A directive from lead B to its member goes through. Release lead A: its directive goes through.

**Scenarios:**

1. Given lead A has a running member, When the user holds lead A, Then a directive from lead A to the member is refused with the reason "the agent is on hold", and recorded.
2. Given lead A is on hold, When lead A's member asks for any agent-made action, Then the office refuses it with the same reason.
3. Given lead A is on hold, When the CEO sends a directive to lead A or starts a new task on it, Then the office refuses it with the same reason.
4. Given lead A is on hold, When lead B sends a directive to its own member, Then the directive goes through.
5. Given lead A is on hold, When the user releases lead A, Then agent-made actions by and on lead A and its descendants work again, and the release is recorded.

### US3 — Refuse an agent's actions beyond a set pace (Priority: P2)

An agent that sends directives or creates agents faster than a set limit gets its extra actions refused, sees the reason in its terminal, and, if it keeps hitting the limit, is held automatically so the user is told once instead of reading a flood.

**Why this priority:** it stops the common loop without the user's attention, but the holds (US1, US2) already let the user stop any loop by hand.

**Independent test:** a test agent sends 100 directives to its members in 10 seconds. At most 10 directives reach terminals. The first refused requests name the limit. At the third limit refusal within 60 seconds the agent is put on hold, the user gets one notification that names the agent, and every later request is refused with the reason that the agent is on hold.

**Scenarios:**

1. Given the directive limit per agent is 10 per 60 seconds, When an agent sends its 11th directive within 60 seconds, Then the office refuses it with a reason that names the limit, and records the refusal.
2. Given the limit for directives to one target is 3 per 60 seconds, When an agent sends a 4th directive to the same target within 60 seconds, Then the office refuses it with a reason that names the limit.
3. Given an agent was refused by a limit 3 times within 60 seconds, When the third refusal happens, Then the office holds that agent (as in US2), records the hold with the office as actor, and notifies the user once.
4. Given the user changed a limit in the office settings, When an agent next acts, Then the new limit applies.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let the user hold and release the whole office with one action each from the desktop app.
- **FR-002** (US1): WHILE the office is on hold, THE SYSTEM SHALL refuse every agent-made action with the reason that the office is on hold.
- **FR-003** (US2): THE SYSTEM SHALL let the user hold and release one agent from the desktop app.
- **FR-004** (US2): WHILE an agent is on hold, THE SYSTEM SHALL refuse every agent-made action taken by that agent or one of its descendants, and every agent-made action whose target is that agent or one of its descendants, with the reason that the agent is on hold.
- **FR-005** (US1, US2): WHILE the office or an agent is on hold, THE SYSTEM SHALL keep every agent's terminal and CLI running, deliver the user's typing, and carry out the user's own actions from the desktop app as without a hold.
- **FR-006** (US1, US2): THE SYSTEM SHALL keep the office hold and every agent hold across server and machine restarts until the user releases it.
- **FR-007** (US3): IF an agent's agent-made action would exceed one of the rate limits in the table under Assumption A3, THEN THE SYSTEM SHALL refuse the action with a reason that names the limit and its value.
- **FR-008** (US3): WHEN an agent is refused by a rate limit for the third time within 60 seconds, THE SYSTEM SHALL put that agent on hold and notify the user once, naming the agent.
- **FR-009** (US3): THE SYSTEM SHALL let the user change each rate limit in the office settings, and SHALL apply a changed value to the next agent-made action.
- **FR-010** (US1, US2, US3): THE SYSTEM SHALL add every hold, release, and refusal of this change to the action record, with the user as the actor of a hold or release that the user takes, the office as the actor of an automatic hold, and the refused agent as the actor of a refusal. A refused directive appears in the action record of the sender and in the action record of the target.
- **FR-011** (US1, US2): THE SYSTEM SHALL show in the desktop app, as text, whether the office is on hold and which agents are on hold.

### MODIFIED

- **0003/FR-006** (in part) (US3): WHEN an agent exits, fails, changes from `working` to `waiting for input`, rings the terminal bell, or is put on hold automatically (FR-008), THE SYSTEM SHALL notify the user with the agent's name and the event, EXCEPT that (a) while an unread bell notification of that agent exists, a further bell adds none, and (b) for the agent whose terminal is open in the focused desktop app window, an exit, failure, `waiting for input`, or bell event shows in the notification list as read and sends no OS notification. Exception (b) does not apply to the automatic hold, because the terminal does not show it.
- **0002/FR-009** (US1, US2, US3): IF an agent asks for an action on an agent that is not its descendant, for a role or action that 0002/FR-008 does not allow, for an action while it, its target, or the office is on hold, or for an action beyond a rate limit, THEN THE SYSTEM SHALL refuse the action, tell the agent the reason in the answer to its request, and record the refusal.

### PRESERVED

- **PB-1:** The user's own actions from the desktop app (create, start, stop, delete, new task, typing into a terminal) are never held or rate-limited (0002 A1; 0001/FR-017 as modified by 0003).
- **PB-2:** A hold refuses new agent-made actions only; it stops no running CLI and closes no terminal. Stopping a CLI stays the existing stop action (0001, 0002/FR-007).
- **PB-3:** office-harness creates, changes, or deletes no file in an agent CLI's configuration (0001/FR-021).
- **PB-4:** Every desktop app action is available from the keyboard, including the hold and release actions (0001/FR-028).
- **PB-5:** The tree rules of 0002 stay as they are: three levels, directives down only, a member creates no office agents (0002/FR-001, 0002/FR-008).

## User experience

**Flow:**

1. The user sees agents acting in a way he does not want. He presses the office hold action, from the keyboard or the panel.
2. The panel shows "Office on hold" as text until he releases it. Agents that try to act get a refusal in the answer to their request, which their CLI shows in their terminal.
3. To stop one team instead, he selects its lead and holds that agent. The agent's row shows "on hold" as text.
4. When an agent hits a rate limit three times, the office holds it and the user gets one notification (as in 0003) that names the agent and the limit.
5. He releases a hold with one action. The release shows in the action record.
6. He changes a limit in the office settings, where each limit shows its current value and its default.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| No hold | No hold text; the hold actions are available. | FR-001, FR-003 |
| Office on hold | "Office on hold" as text in the panel, with the release action. | FR-011, US1 |
| Agent on hold | "on hold" as text in the agent's row and details, with the release action. | FR-011, US2 |
| Automatic hold | The agent shows "on hold"; one notification names the agent and the limit it hit. | FR-008, US3 scenario 3 |
| Refused agent action | The action record shows the actor, the action, and the reason. | FR-010 |
| Office record | The office hold and release entries, which have no target agent, in a list beside the office hold control. | FR-010, A11 |
| Settings | Each limit with its current value and default; an invalid value shows why it is refused. | FR-009, Edge cases |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The office hold, the agent hold, and both releases work from the keyboard (PB-4); the office hold has its own shortcut.
- Hold state is text, never color alone, and is part of the accessible name of the agent row.
- A change of the office hold state is announced to assistive technology.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Office hold | Whether the office holds agent-made actions. | on or off, since when, who set it | One per office; stored. |
| Agent hold | Whether one agent and its subtree are held. | on or off, since when, who set it (user or office), the reason | At most one per agent; stored with the agent; removed with the agent. |
| Rate limit setting | One limit of Assumption A3. | kind, value, default | One per kind in the office settings. |
| Action record entry (extends 0002) | One recorded action. | actions now also `hold`, `release`; actor can be the office for an automatic hold | Belongs to the office. |

## Success criteria

- **SC-001:** In a test where one agent asks to send 100 directives within 10 seconds, at most 10 directives reach terminals, and 100% of the refused requests show in the action record with the agent as actor and a reason: the limit for the refusals before the automatic hold, and the hold for the refusals after it.
- **SC-002:** After the user presses the office hold action, 100% of agent-made actions are refused, measured from the first request after the action, while 100% of the user's typed keys still reach the terminals.
- **SC-003:** After a server restart with the office on hold, 0 agent-made actions succeed until the user releases the hold.
- **SC-004:** A looping agent that hits a limit 3 times within 60 seconds causes exactly 1 notification to the user and is on hold after the third refusal.
- **SC-005:** The user holds the office from the keyboard in 1 key press from any panel view, a focused terminal included (A9, as changed by Deviations, D9 = b). The first approved text said "whose keyboard focus is outside a terminal".

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The user holds the office when no agent runs. | Empty | The office goes on hold; the next agent-made action after a start is refused. |
| The user holds an agent that is already on hold, or releases one that is not. | Invalid | Nothing changes; the app says that the agent is already held or not held. |
| The user sets a limit to 0, a negative number, or text. | Invalid | The settings refuse the value and say that a limit is a whole number of at least 1. |
| An agent sends exactly as many directives as the limit allows within 60 seconds. | Boundary | All of them go through; only the next one is refused. |
| A directive arrives at a target that the user holds at the same moment. | Concurrent | The directive is either delivered whole or refused and recorded; it is never partly delivered (0002/FR-010). |
| The user releases the office hold while an agent is held automatically. | Concurrent | The agent stays on hold; an office release does not release agent holds. |
| The user deletes an agent that is on hold. | Concurrent | The agent and its hold are deleted together after the usual confirmation (0002/FR-006). |
| A held agent's parent is deleted with its subtree. | Concurrent | The held agent is deleted with the subtree; no hold remains for a missing agent. |
| An agent outside the hierarchy (no role) acts. | Unauthorized | It has no agent-made actions in 0002; nothing in this change applies to it. |
| The hold state cannot be stored. | Failure | The hold or release is refused, the app says it could not be stored, and the previous state stays in force. |
| The server restarts within a 60-second window. | Failure | Rate counts start again at 0; holds stay (FR-006). |

## Assumptions

- **A1:** A hold refuses new requests only. An action that the office has already accepted when the hold starts finishes.
- **A2:** An agent hold covers the agent and its descendants as the tree is at the moment of each request. An agent that joins the subtree later is covered at once.
- **A3:** The rate limits and their defaults, each counted over a rolling 60 seconds:

  | Limit | Default | Reason |
  |---|---|---|
  | Directives sent by one agent | 10 | A lead that briefs 8 members in one burst fits; a loop does not. |
  | Directives sent to one target agent | 3 | One terminal is one input stream; more is flooding. |
  | Agents created by one agent | 5 | A lead that creates a 5-member team fits. |
  | Starts, stops, and new tasks by one agent | 10 | One restart round of a 5-member team fits twice. |

- **A4:** A refused action is not queued or retried by the office. The agent decides whether to try again; a queue would hide a loop.
- **A5:** The office counts refusals for the automatic hold (FR-008) over the same rolling 60 seconds, across all limit kinds. Hold refusals do not count.
- **A6:** The user releases an automatic hold the same way as a hold he set. A hold never ends on a timer.
- **A7:** The notification of FR-008 uses the notification channel of 0003 and follows its settings.
- **A8:** No loop detector is promised. The office promises the limits and the holds, not that it recognizes every loop.
- **A9:** While a terminal has keyboard focus, every key goes to the CLI except the documented app shortcuts (0001/FR-017 as modified by 0003). The hold shortcut `Ctrl+Shift+H` is one of them: the owner decided D9 = b on 2026-11-19, so the hold key works while a terminal has focus (see Deviations, "Hold shortcut from a terminal"). The first approved text said the user presses the focus shortcut first.
- **A10:** When more than one reason applies to one request, the office names the first in this order: the rules of 0002, the office hold, an agent hold, a rate limit. Only a refusal whose reason is a rate limit counts toward the automatic hold (A5).
- **A11:** An office hold or release has no target agent. Its action record entry shows in an office record beside the office hold control, not in an agent's details.
- **A12:** The holds and the limits are a brake against mistakes and loops, not a security boundary: an agent CLI runs as the user's OS user and can act outside the office (0002 A8, 0001 A7). A change of a limit is a settings change and is not recorded in the action record.

## Clarifications

### Session 2026-11-19 (owner-decided)

- Q: Should this change add a size cap on agent-created agents (for example 50 per office, 12 per parent), modifying 0002/FR-005? → A: No. 0002/FR-005 stays unchanged; the creation rate limit (A3) and the automatic hold (FR-008) cover runaway creation.
- Q: Does an agent hold also refuse actions on the held agent by agents outside its subtree (its parent and the CEO)? → A: Yes, as FR-004 says. The user's own actions are never refused (PB-1).
- Q: Where is a refused directive recorded, in the sender's action record or the target's? → A: In both, one entry each, one entry when they are the same agent (owner decision, 2026-11-20).

## Out of scope

- Token, cost, or turn limits on an agent's CLI (post-MVP).
- Stopping or killing a CLI process on hold; the existing stop does that (PB-2).
- Detecting loops by reading terminal output (A8).
- Limits on the user's own actions (PB-1).
- Queueing refused actions (A4).
- A size cap on how many agents an agent may create; the owner declined it on 2026-11-19 (Clarifications).
- Changing the tree rules of 0002 (PB-5).

## Deviations

Implementation notes. The requirements above did not change.

- **One service.** The plan (D1) names four application items: `SetHold`, `ReleaseHold`, `ListHolds`, and `GuardAgentAction`. In the code they are methods of one `HoldService` (`set_hold`, `release_hold`, `list`, `guard`). It lives in `src/modules/hierarchy/application/src/commands/hold/hold_service.rs`.
- **Steady clock.** The rate windows and the automatic-hold tracker read wall time once at start and then a monotonic clock. A change of the system clock does not free or add a slot. A hold's `since` and its record entry use the wall clock, so they line up with the other entries of the action record.
- **Unknown fields.** A hold request refuses a field it does not name. The answer is `invalid` with the message `<field> is not a field of <request>`. A misspelt `agent_id` therefore never acts on the office.
- **Office record request.** The office-level entries come from a new `office.actions` request (no fields), not from `action.list` without an agent id as D6 first said. The hold-protocol contract documents it.
- **Hold shortcut from a terminal (D9 = b, owner-decided 2026-11-19).** This changes A9 and SC-005. `Ctrl+Shift+H` is one of the app shortcuts that a focused terminal lets out (0003's MODIFIED 0001/FR-017 gets the office hold shortcut beside the focus, agent, and switcher shortcuts). The user holds or releases the office in 1 key press from any panel view, a focused terminal included. The CLI no longer gets `Ctrl+Shift+H`.
