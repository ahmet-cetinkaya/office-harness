# Needs-you queue

## Intent

The Needs-you queue gathers the agents that wait for the person, so that nothing that needs an answer stays hidden. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0008-needs-you-queue](../changes/archive/0008-needs-you-queue/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Modified requirements:

| Requirement | Modified by | Final text is under |
|---|---|---|
| 0008/FR-001 | 0022 | Change 0008, Requirements |
| 0008/FR-002 | 0015, 0024 | Change 0008, Requirements |
| 0008/FR-003 | 0015 | Change 0008, Requirements |
| 0008/FR-005 | 0015, 0022 | Change 0008, Requirements |

Changes of other capabilities that this file names:

- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0024: [Exact agent status from CLI hooks](../agent-status/SPEC.md)
- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0022: [Idle sleep](../idle-sleep/SPEC.md)
- 0005: [Office scene](../office-scene/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0008: Needs-you queue

### Intent

With 12 to 30 agents, the owner must find the agents that wait for him. Today 0003 gives him one notification per event, but a notification list is ordered by arrival, keeps items after the agent moved on, and has no key that takes him to the next agent that waits. This change adds one standing list in the panel, "Needs you", with one item per agent that waits for him, grouped by what happened and ordered by how long it has waited, and one key that opens the terminal of the agent at the top. Each item says what the office saw, not what the agent meant. This change amends the approved change 0003.

### Terms

| Term | Meaning |
|---|---|
| Agent, project, status, quiet period, desktop app | As defined in 0001. |
| Panel, agent list, notification | As defined in 0003. |
| Attention event | One of the events of 0003/FR-006 as modified by 0007: an agent changes from `working` to `waiting for input`, exits, fails, rings the terminal bell, or is put on hold automatically (0007/FR-008). |
| Needs you | The list in the panel that this change adds. It holds at most one item per agent. |
| Item | The entry of one agent in Needs you. |
| Wait time | The time since the oldest attention event of the item that the user has not handled. |
| Jump shortcut | The app shortcut that opens the terminal of the agent at the top of Needs you. |

### Requirements

- **0008/FR-001** (US1): WHEN an attention event occurs that 0003/FR-006 does not show as read on arrival, THE SYSTEM SHALL add an item for that agent to Needs you, or, if the agent already has an item, SHALL keep that one item and its wait time and record the new event as its latest event. A bell that 0003/FR-006 (a) coalesces still counts as an attention event for Needs you. An unprompted change to `waiting for input` is not an attention event for Needs you: THE SYSTEM SHALL NOT add an item for it and SHALL NOT change an existing item, its wait time, or its latest event because of it.
- **0008/FR-002** (US1): THE SYSTEM SHALL show each item with the agent's name, project, latest event as the office saw it (for an exact session `finished its turn`, `its turn ended with an error`, or `asks permission`; otherwise quiet for the quiet period, bell, exit with its exit code, put on hold by the office with the limit it hit, or a report with its kind), and wait time, as text, and, while the item holds a report, the report's kind and its whole text in visible form.
- **0008/FR-003** (US1): THE SYSTEM SHALL order the items in groups by their latest event, `failed` first, then `waiting for input`, bell, automatic hold (the resume hold of 0013 included), and report, then `exited`, and inside each group by wait time, longest first, and items with the same wait time by agent name.
- **0008/FR-004** (US1, US3): WHEN the user opens an agent's terminal by any route, or dismisses the agent's item, THE SYSTEM SHALL remove that agent's item from Needs you.
- **0008/FR-005** (US1): WHEN an agent with an item is deleted or changes to `stopped`, or, for an item that holds no report, changes to `starting`, makes a prompted change to `working`, gets a person input or office text in its terminal while it shows `working`, or has shown `working` without a break for longer than the work bound, or, for an item that holds no report and whose latest event is an automatic hold (the resume hold of 0013 included), has its hold released, THE SYSTEM SHALL remove that agent's item from Needs you; an unprompted change to `working` SHALL leave the item, its wait time, and its latest event unchanged while the stretch of `working` stays within the work bound.
- **0008/FR-006** (US1): IF an attention event occurs while the desktop app is closed, THEN THE SYSTEM SHALL show its item in Needs you, with the wait time counted from the event, when the app opens.
- **0008/FR-007** (US2): WHEN the user presses the jump shortcut and Needs you has items, THE SYSTEM SHALL open the terminal of the agent at the top of Needs you with keyboard focus.
- **0008/FR-008** (US2): IF the user presses the jump shortcut while Needs you is empty, THEN THE SYSTEM SHALL keep the current view and say that no agent needs the user.
- **0008/FR-009** (US3): WHILE Needs you has one or more items, THE SYSTEM SHALL show the number of items as text next to Needs you in the panel and in the desktop app's window title in either view.
- **0008/FR-010** (US2): WHILE the office scene (0005) is open, WHEN the user presses the jump shortcut and Needs you has items, THE SYSTEM SHALL open the top agent's desk in the scene with keyboard focus, as 0005/FR-005 does, and SHALL leave the panel's project choice and selected agent unchanged (0005/FR-006).
- **0001/FR-017** (as modified by 0003) (US2): WHILE an agent's terminal has keyboard focus, THE SYSTEM SHALL send every key to the agent's CLI except the documented app shortcuts that approved changes let out of a terminal (the focus, agent, and switcher shortcuts of 0003 A1, and any that another approved change adds, such as the 0007 office hold shortcut) and the jump shortcut.

Modified by later changes: 0008/FR-001 (by 0022), 0008/FR-002 (by 0015, 0024), 0008/FR-003 (by 0015), 0008/FR-005 (by 0015, 0022). This change modified 0001/FR-017; its final text is the last entry above, and the entry under [Change 0001](../server-and-terminal/SPEC.md) is to be read with it.

### Preserved behaviors

- **0008/PB-1:** Notifications keep their 0003 behavior: each attention event notifies once (0003/FR-006), unread notifications survive app closure (0003/FR-007), and opening a terminal marks them read (0003/FR-009). Needs you adds no OS notification and no repeated alert.
- **0008/PB-2:** The office does not read CLI prompts and does not create, change, or delete anything in a CLI's configuration (0001/FR-021).
- **0008/PB-3:** `stopped` and `interrupted` cause no attention event (0003/FR-006).
- **0008/PB-4:** Every desktop app action, including the jump shortcut and dismiss, is available from the keyboard (0001/FR-028), and every status and event shows as text, never color alone (0001/FR-014).
- **0008/PB-5:** No office data leaves the user's machine (0001/FR-005).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Item | One agent that waits for the user. | agent, first unhandled event time, latest event (kind, exit code when it applies), latest event time | At most one per agent. Kept by the background server, so it survives app closure. |

### Success criteria

- **0008/SC-001:** With 30 agents running and 5 of them with items, Needs you lists exactly those 5 in the order of FR-003, and one press of the jump shortcut shows the top agent's live terminal with keyboard focus within 1 second.
- **0008/SC-002:** With 5 items, 5 presses of the jump shortcut open the 5 agents' terminals in the order of FR-003, and Needs you is empty afterwards.
- **0008/SC-003:** An item shows in Needs you, and the count in the window title changes, within 2 seconds after the attention event (for `waiting for input`, after the quiet period ends).
- **0008/SC-004:** After the desktop app was closed while 3 attention events occurred, 100% of their items show in Needs you within 3 seconds after the app opens.
- **0008/SC-005:** An agent that produces output at least once per quiet period, rings no bell, and does not exit never gets an item.

### Notes from Deviations

- **Name lookup port.** The notification module reads an agent's name through a port, `AgentNameLookup`, so that it does not depend on the agent module. The server adapter is in the background server's bridges.
- **Two plan tasks in one commit.** The server hooks task and the protocol task landed in one commit.
- **Text of a failed item.** The detail of a `failed` item is the status text that the server sends, such as "the command ended with exit code 2", not a text that the desktop builds from the exit code. The contract says so.
- **Free key in the test.** The end-to-end test of a key that reaches the CLI uses `Control+Alt+F11`, because `Ctrl+Shift+F12` is the default focus shortcut and so is not free.
- **Focus shortcut module moved.** `focus-shortcut.ts` moved from the desktop terminal module to `src/core/shortcuts/`, beside a new `focus-shortcut-state.ts`, so that the shared state loads the focus shortcut once at app start. A change of the setting applies at the next app start.
- **Dismiss of an item that is gone.** A dismiss that the server answers with `not_found` is a success in the desktop, because the item is already gone.
- **Jump focus for an agent that is not running.** The terminal of an agent that is not running is read-only and takes no keyboard focus. The jump then puts focus on the terminal view heading, whose accessible name names the agent ("Terminal, <agent name>"). This refines FR-007 for that case.
- **Criteria that tests ran in a narrower form (QA report).** The 1 second bound of 0008/SC-001 is tested on a mock core only. The 3 second bound of 0008/SC-004 is not timed by a test. 0008/SC-005 is stated per quiet period, and its test uses a quiet period of 2000 ms with an agent that writes every 100 ms, watched for 3 seconds. The server runs use fake CLIs.
- **Rows that no person has run.** A screen reader pass, 200 percent zoom, the real window title and the real jump key on a desktop environment, and a run with a real agent CLI are not run. macOS and Windows hands-on rows are waived, as for 0003, 0005, and 0007.
- **Edge cases without a full test.** No test removes the title bar of the window, and no 0008 test targets the unreachable-server and rejected-token states of 0001.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0008-needs-you-queue - 2026-11-28
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
