---
status: shipped
capability: office
plan: specs/changes/archive/0008-needs-you-queue/PLAN.md
supersedes: 0001/FR-017 (as modified by 0003)
---

<!-- Approved on 2026-11-19 through the review gate; shipped in PR #93. -->

# 0008: Needs-you queue

## Intent

With 12 to 30 agents, the owner must find the agents that wait for him. Today 0003 gives him one notification per event, but a notification list is ordered by arrival, keeps items after the agent moved on, and has no key that takes him to the next agent that waits. This change adds one standing list in the panel, "Needs you", with one item per agent that waits for him, grouped by what happened and ordered by how long it has waited, and one key that opens the terminal of the agent at the top. Each item says what the office saw, not what the agent meant. This change amends the approved change 0003.

## Context

- Source of truth: `docs/PRD.md`. This change serves R9 (find the agent that needs the user) and the PRD question "which agent waits for me?".
- This change depends on 0001 (agents, statuses, quiet period, terminals) and 0003 (the panel, the notification events of 0003/FR-006, and the read rule of 0003/FR-009) and 0007 (the automatic hold, which 0007 added to 0003/FR-006). References such as `0003/FR-006` point to requirements in change 0003.
- The queue is a view over the events that 0003 already detects. It adds no new detector: it does not read the CLI's prompts and it writes nothing to the CLI's configuration (0001/FR-021).
- 0001 A1 and 0003 A3 hold: the office does not tell "finished" apart from "asks a question". An item therefore names the event the office saw (quiet for the quiet period, bell, exit code), never a guess about the agent's intent.
- Stack decision, made by the owner on 2026-11-15, as in 0003. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, status, quiet period, desktop app | As defined in 0001. |
| Panel, agent list, notification | As defined in 0003. |
| Attention event | One of the events of 0003/FR-006 as modified by 0007: an agent changes from `working` to `waiting for input`, exits, fails, rings the terminal bell, or is put on hold automatically (0007/FR-008). |
| Needs you | The list in the panel that this change adds. It holds at most one item per agent. |
| Item | The entry of one agent in Needs you. |
| Wait time | The time since the oldest attention event of the item that the user has not handled. |
| Jump shortcut | The app shortcut that opens the terminal of the agent at the top of Needs you. |

## User stories

### US1 — See which agents wait for me, most urgent first (Priority: P1)

The user sees one list of the agents that wait for him, with what the office saw and how long each has waited: failed agents first, then agents that wait for input or rang the bell, then exited agents, and the longest wait first inside each group.

**Why this priority:** it answers "which agent waits for me, and which one waited longest" without reading every terminal. It is the smallest slice with value on its own: the user can open an item with the existing 0003 routes.

**Independent test:** run 12 agents. Let 3 of them become `waiting for input` at different times and let 1 exit with a nonzero code. Needs you shows exactly those 4, the failed one first and then the 3 waiting ones longest wait first, each with the event and its wait time. Open the terminal of one of them: its item leaves the list.

**Scenarios:**

1. Given 12 agents run and none waits, When the user looks at Needs you, Then it says that no agent needs him.
2. Given agent A became `waiting for input` 3 minutes ago and agent B 1 minute ago, When the user looks at Needs you, Then A is above B, and each item shows the agent's name, project, the event (for example "quiet for 5 s"), and the wait time.
3. Given agent C failed with exit code 2, When the user looks at Needs you, Then C's item says "failed, exit code 2".
4. Given agent A has an item, When the user opens A's terminal by any route, Then A's item leaves Needs you.
5. Given agent A has an item for `waiting for input`, When A changes back to `working` without the user (for example after a directive from its lead under 0002), Then A's item leaves Needs you.
6. Given agent A has an item, When A rings the bell, Then A still has one item; it names the bell as the latest event and keeps its wait time.
7. Given the desktop app was closed while 2 agents became `waiting for input`, When the user opens the app, Then Needs you shows both items with their wait times.

### US2 — Go to the next agent that waits with one key (Priority: P1)

The user presses one key, from anywhere in the app, and the terminal of the agent at the top of Needs you opens with keyboard focus. Pressing it again goes to the next one.

**Why this priority:** this is the owner's success measure: with 30 agents, the terminal of the agent at the top of Needs you opens in one key. It is P1 because without it the list is one more place to read.

**Independent test:** with 30 agents and 5 items in Needs you, press the jump shortcut 5 times while a terminal has focus. Each press opens the terminal of the item at the top with focus, that item leaves, and after the fifth press Needs you is empty.

**Scenarios:**

1. Given Needs you has items, When the user presses the jump shortcut, Then the terminal of the agent at the top opens with keyboard focus and its item leaves Needs you.
2. Given the terminal of agent A has keyboard focus and Needs you has items, When the user presses the jump shortcut, Then the shortcut does not reach A's CLI and the top item's terminal opens.
3. Given Needs you is empty, When the user presses the jump shortcut, Then the open terminal stays open and the app says that no agent needs him.
4. Given the project switcher shows project P and the top item belongs to project Q, When the user presses the jump shortcut, Then the agent's terminal opens as it does through the 0003 agent switcher.

### US3 — See the count when the app is in the background (Priority: P2)

The user sees how many agents wait for him in the window title, and can remove an item that he does not want to act on.

**Why this priority:** the count helps when the app is minimized or behind another window, and dismiss keeps the list honest for exited agents he will not restart. Both are useful after US1 and US2 exist.

**Independent test:** with 3 items, minimize the app: the window title shows 3. Dismiss one item without opening its terminal: the title shows 2 and the item is gone.

**Scenarios:**

1. Given Needs you has 3 items, When the user looks at the window title, Then it shows the number 3.
2. Given Needs you has an item for an exited agent, When the user dismisses it, Then the item leaves Needs you and the agent's status stays `exited`.
3. Given Needs you becomes empty, When the user looks at the window title, Then it shows no count.

## Requirements

### ADDED

- **FR-001** (US1): WHEN an attention event occurs that 0003/FR-006 does not show as read on arrival, THE SYSTEM SHALL add an item for that agent to Needs you, or, if the agent already has an item, SHALL keep that one item and its wait time and record the new event as its latest event. A bell that 0003/FR-006 (a) coalesces still counts as an attention event for Needs you.
- **FR-002** (US1): THE SYSTEM SHALL show each item with the agent's name, project, latest event as the office saw it (quiet for the quiet period, bell, exit with its exit code, or put on hold by the office with the limit it hit), and wait time, as text.
  - Amended by 0015 (`specs/changes/archive/0015-upward-reports/SPEC.md`): a report is an event too, shown with its kind; an item that holds a report shows the report's kind and whole text.
- **FR-003** (US1): THE SYSTEM SHALL order the items in groups by their latest event, `failed` first, then `waiting for input`, bell and automatic hold, then `exited`, and inside each group by wait time, longest first, and items with the same wait time by agent name.
  - Amended by 0015: a report sorts with `waiting for input`, bell, and the automatic hold (the 0013 resume hold included).
- **FR-004** (US1, US3): WHEN the user opens an agent's terminal by any route, or dismisses the agent's item, THE SYSTEM SHALL remove that agent's item from Needs you.
- **FR-005** (US1): WHEN an agent with an item changes to `starting`, `working`, or `stopped`, is deleted, or, for an item whose latest event is an automatic hold, has its hold released, THE SYSTEM SHALL remove that agent's item from Needs you.
  - Amended by 0015: an item that holds a report stays when the agent changes to `starting` or `working` and when a hold is released; it leaves when the agent stops or is deleted, its terminal opens, or it is dismissed.
- **FR-006** (US1): IF an attention event occurs while the desktop app is closed, THEN THE SYSTEM SHALL show its item in Needs you, with the wait time counted from the event, when the app opens.
- **FR-007** (US2): WHEN the user presses the jump shortcut and Needs you has items, THE SYSTEM SHALL open the terminal of the agent at the top of Needs you with keyboard focus.
- **FR-008** (US2): IF the user presses the jump shortcut while Needs you is empty, THEN THE SYSTEM SHALL keep the current view and say that no agent needs the user.
- **FR-009** (US3): WHILE Needs you has one or more items, THE SYSTEM SHALL show the number of items as text next to Needs you in the panel and in the desktop app's window title in either view.
- **FR-010** (US2): WHILE the office scene (0005) is open, WHEN the user presses the jump shortcut and Needs you has items, THE SYSTEM SHALL open the top agent's desk in the scene with keyboard focus, as 0005/FR-005 does, and SHALL leave the panel's project choice and selected agent unchanged (0005/FR-006).

### MODIFIED

- **0001/FR-017** (as modified by 0003) (US2): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts that approved changes let out of a terminal (the focus, agent, and switcher shortcuts of 0003 A1, and any that another approved change adds, such as the 0007 office hold shortcut) and the jump shortcut.

### PRESERVED

- **PB-1:** Notifications keep their 0003 behavior: each attention event notifies once (0003/FR-006), unread notifications survive app closure (0003/FR-007), and opening a terminal marks them read (0003/FR-009). Needs you adds no OS notification and no repeated alert.
- **PB-2:** The office does not read CLI prompts and does not create, change, or delete anything in a CLI's configuration (0001/FR-021).
- **PB-3:** `stopped` and `interrupted` cause no attention event (0003/FR-006).
- **PB-4:** Every desktop app action, including the jump shortcut and dismiss, is available from the keyboard (0001/FR-028), and every status and event shows as text, never color alone (0001/FR-014).
- **PB-5:** No office data leaves the user's machine (0001/FR-005).

## User experience

**Flow:**

1. The user works in one agent's terminal. Another agent goes quiet for the quiet period; 0003 notifies him, and the agent gets an item in Needs you.
2. Needs you shows in the panel, failed agents first, then waiting ones, then exited ones, longest wait first inside each group, with what the office saw and the wait time. The window title shows the count.
3. He presses the jump shortcut. The top agent's terminal opens with focus, and its item leaves the list.
4. He answers the agent, then presses the jump shortcut again for the next one.
5. For an exited agent he will not restart, he dismisses its item.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading, server not reachable, unauthorized | As in 0001 (0003 PB-5). | 0003 PB-5 |
| Empty | "No agent needs you" as text; no count in the title. | US1 scenario 1, FR-008, FR-009 |
| Items | One row per agent: name, project, latest event, wait time; the count in the title. | FR-002, FR-003, FR-009 |
| Agent deleted | The item is gone. | FR-005 |
| Jump with empty list | The current view stays; "No agent needs you". | FR-008 |
| Scene open | No Needs you list; the count in the title; the jump shortcut opens the top desk | FR-009, FR-010 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- Needs you, each item, dismiss, and the jump shortcut work from the keyboard (PB-4). Keyboard focus is always visible.
- Each item has an accessible name with the agent's name, project, latest event, and wait time.
- A change of the count is announced to assistive technology without moving focus; a new item is not announced separately from its 0003 notification.
- The wait time is text, never color alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Item | One agent that waits for the user. | agent, first unhandled event time, latest event (kind, exit code when it applies), latest event time | At most one per agent. Kept by the background server, so it survives app closure. |

## Success criteria

- **SC-001:** With 30 agents running and 5 of them with items, Needs you lists exactly those 5 in the order of FR-003, and one press of the jump shortcut shows the top agent's live terminal with keyboard focus within 1 second.
- **SC-002:** With 5 items, 5 presses of the jump shortcut open the 5 agents' terminals in the order of FR-003, and Needs you is empty afterwards.
- **SC-003:** An item shows in Needs you, and the count in the window title changes, within 2 seconds after the attention event (for `waiting for input`, after the quiet period ends).
- **SC-004:** After the desktop app was closed while 3 attention events occurred, 100% of their items show in Needs you within 3 seconds after the app opens.
- **SC-005:** An agent that produces output at least once per quiet period, rings no bell, and does not exit never gets an item.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| No agent waits. | Empty | Needs you says "No agent needs you"; the title shows no count; the jump shortcut keeps the view (FR-008). |
| An attention event occurs for the agent whose terminal is open in the focused window. | Boundary | No item; 0003/FR-006(b) shows the event as read (FR-001). |
| An agent with an item rings the bell many times. | Boundary | One item; wait time unchanged; latest event "bell" (FR-001). |
| Two items have the same wait time. | Boundary | Ordered by agent name (FR-003). |
| More items than fit on screen. | Boundary | The list scrolls; the count in the title is the full number. |
| The top item's agent is deleted while the user presses the jump shortcut. | Concurrent | The item is gone (FR-005); the shortcut opens the next item, or says that no agent needs the user. |
| An agent changes back to `working` after a directive from its lead (0002) before the user looks. | Concurrent | Its item leaves (FR-005). |
| The background server restarts while items exist. | Failure | Items survive. An agent whose status became `interrupted` keeps its item; `interrupted` adds none (PB-3). |
| The OS does not show the window title (for example a tiling layout without title bars). | Failure | The count still shows as text next to Needs you (FR-009). |
| The project switcher hides the item's project. | Boundary | Needs you still lists the item; it covers every project (A2). |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0003 PB-5). |
| The user dismisses an item, and the agent later has a new attention event. | Boundary | A new item starts with the new event's time. |
| The scene is open when the user presses the jump shortcut. | Boundary | The top agent's desk opens in the scene (FR-010). |
| The user changes the focus shortcut to the jump shortcut. | Invalid | The system refuses it and names the conflict (as 0003). |
| The focus shortcut stored before this change equals the jump shortcut. | Invalid | The focus shortcut keeps its stored key; the jump shortcut has no key until the user changes the focus shortcut, and the app says so once where shortcuts are shown. |

## Assumptions

- **A1:** The default jump shortcut is `Ctrl+Shift+F11`, in the same family as the 0003 shortcuts (F1 to F10) and the focus shortcut (F12). The 0003 per-OS fallback table and its macOS Fn note apply; the plan checks the key per OS and the webview and may change it. Fixed in the MVP, like the 0003 agent shortcuts. The plan checks that no stored focus shortcut is lost by the new reservation.
- **A2:** Needs you covers every project, independent of the project switcher. The jump shortcut opens the agent the way the 0003 agent switcher does (0003 A1).
- **A3:** An item's group follows its latest event, so a waiting agent that then fails moves to the `failed` group and keeps its wait time. The jump shortcut always opens the top item, which is the longest-waiting `failed` agent when one exists.
- **A4:** Dismiss removes the item only. It does not mark notifications read and changes no status. Dismiss is not recorded in the action record, because it changes nothing in the office.
- **A5:** No age badges, no reminders, and no repeated alerts: the wait time as text is the only age signal, so the user is not trained to ignore alerts.
- **A6:** Items live in the background server, so a second desktop app on the same machine shows the same Needs you, and opening a terminal in either removes the item.

## Clarifications

### Session 2026-11-19 (owner-decided)

- Q: Is Needs you ordered by wait time only, or grouped by event kind? → A: Grouped: `failed` first, then `waiting for input` and bell, then `exited`, longest wait first inside each group (FR-003, A3). The jump shortcut still opens the top item.
- Q: Is the automatic hold of 0007/FR-008 an attention event, and where does it sort? → A: Yes. It sorts with `waiting for input` and bell, and its item leaves when the hold is released (FR-002, FR-003, FR-005).
- Q: What does the jump shortcut do while the office scene is open? → A: It opens the top agent's desk in the scene and leaves the panel's project choice and selection unchanged. The count shows in the window title in either view (FR-009, FR-010).
- Q: What if a stored focus shortcut equals the jump shortcut? → A: The focus shortcut keeps its key, the jump shortcut has no key until the user changes the focus shortcut, and the app says so once (Edge cases, A1).
- Q: Does a bell that 0003 coalesces count, and is a new item announced twice? → A: A coalesced bell counts (FR-001). Only the count change is announced; the 0003 notification announces the event (Accessibility).

## Out of scope

- Reading or parsing CLI prompts, and telling "finished" apart from "asks a question" (0001 A1, 0003 A3).
- Answering an agent automatically.
- Sound, phone push, mail, or chat alerts (PB-5).
- Reminders, repeated alerts, and age badges (A5).
- Signals from CLI hooks or CLI configuration (PB-2).
- Terminal escape sequences for notifications (OSC 9, 99, 777) as new attention events; the bell of 0003 stays the only in-band signal.
- A "current task" line per agent; it is a separate change that amends 0003/FR-002.
- Resuming `interrupted` agents in bulk.
