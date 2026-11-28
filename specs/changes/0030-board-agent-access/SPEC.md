---
status: draft
shape: full
capability: task-board
plan:
---

# 0030: Board agent access

## Intent

The person works on the task board of 0027, 0028, and 0029. This change lets the agents that run the work, the CEO and the team leads, read the board and change its cards under the same rules as the person. A member agent cannot. Each card shows who made its last change, and the person learns of an agent's delete, so that the person can tell an agent's change from their own.

## Context

- The owner decided on 2026-11-28 that the board is read and changed by the person and by agents whose role is CEO or team lead (0002/FR-001); a member agent cannot (Clarifications).
- This change depends on 0027, 0028, and 0029 (the board, the cards, the rules, the conflict refusal, the editing), on 0002 (the roles and the refusal of an agent request), on 0007 (the hold and the rate limits), and on 0001 (the action record). It extends the capability `task-board`. Ids such as `0028/FR-001` point to the living spec `specs/task-board/SPEC.md` once 0027, 0028, and 0029 ship.
- This change modifies four living requirements (MODIFIED below) and changes two preserved statements of the earlier changes: 0027/PB-4 and 0029/PB-3 state that the agent's connection gets no board request; from this change they hold for every agent except a board agent.
- The requirements describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Board, card, window, kept, board view | As defined in 0027. |
| Role, CEO, team lead, member | As defined in 0002/FR-001. |
| Hold, rate limit, agent-made action | As defined in 0007. Each kept board change by an agent is one agent-made action; a read of the board is not. |
| Board agent | An agent whose role is CEO or team lead. Only a board agent may read or change the board. |
| Card version | The way an agent shows what it saw (0028, Terms), as a window shows it by what it last displayed: a number that the office keeps for each card. It rises by one with each kept change to that card; a move of another card does not raise it. A read of the board gives the version of each card. |
| Changer | Who made the last change of a card: the person, a board agent by the name that it had at that change, or the office when it cleared an assignee or a project (0029/FR-003). A later rename or delete of the agent does not change it. A card that was made before this change shows the person. |
| Board change | A new card, an edit, a move, or a delete of a card. A read is not a board change. |

## Data flow and trust boundary

- **Boundary:** the connection of an agent (0002), whose role the office knows. The person's connection stays as in 0027.
- **Into the office:** from a board agent, a request to read the board, and board changes that name the card version that the agent read. The office checks the role, the hold, the rate limit, the version, and every field when it receives the request (FR-001 and FR-002, FR-004).
- **Out of the office:** to a board agent, the board with the card versions in its answer; to the person, each change in every open window (FR-003).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | A member agent, or an agent without a role, reads or changes the board, for example to hide a blocked task. | Refused with the reason and recorded (FR-002). |
  | A board agent loops over the board, or acts while it or the office is on hold. | The hold and a rate limit for board changes apply; the refusal is recorded (FR-002). |
  | An agent changes a card from an old view of it and overwrites a newer change. | A board change that names an older card version is refused with the reason (FR-004). |
  | An agent changes a card and the person cannot tell who did it. | Each card records and shows the changer (FR-005). |
  | An agent deletes a card and leaves no trace. | A delete by an agent is recorded in the action record and announced in every open window (FR-006). |

## User stories

### US1: A CEO or a team lead agent works on the board (Priority: P1)

A team lead agent reads the board, makes a card for one of its members, and moves it to Doing. The person sees the card appear and move in the open window, with the lead named as the changer. A member agent that asks for the board is refused with the reason, and the refusal is recorded.

**Why this priority:** it is the owner's access rule. The leads that run the work keep the board current, and a member cannot hide or change a task.

**Independent test:** with a team lead agent L and a member agent M, L reads the board, makes a card assigned to M, and moves it to Doing. Within 2 seconds the open window shows the card in Doing with L as the changer, gives 0 announcements and 0 Needs you items. M asks to read the board and to make a card: both are refused with the reason that its role is member, and 2 refusals are recorded. The person puts L on hold, then L asks for the board: it is refused with the reason that it is on hold.

**Scenarios:**

1. Given a board agent, When it asks for the board, Then it gets every card with its fields, its changer, and its version, in the order of each column.
2. Given a board agent, When it makes, edits, moves, or deletes a card with the version that it read, Then the office applies the rules of 0027, 0028, and 0029 as for the person, and shows the result in every open window within 2 seconds; the confirmation of a delete is for the person only.
3. Given a card that a board agent changed last, When the person looks at the card, Then the card shows that agent's name as the changer.
4. Given a member agent, or an agent without a role, When it asks to read or change the board, Then the office refuses it, tells it the reason in the answer, and records the refusal; no card changes.
5. Given a board agent is on hold, or the office is on hold, or it is beyond the rate limit for board changes, When it asks to read or change the board, Then the office refuses it with the reason and records the refusal.
6. Given a board agent changes a card with a version older than the card's version, When the office receives the change, Then it refuses it with the reason that the card changed, and the agent gets the reason in its answer.
7. Given a card that a board agent changed, When the agent is deleted, Then the card shows the name it had at the change.

### US2: The person learns when an agent deletes a card (Priority: P2)

A team lead deletes a card. Every open window announces that the lead deleted the card, and the action record of the lead holds the delete.

**Why this priority:** a delete leaves nothing on the board, so without a trace the person could not learn that it happened. It is a safeguard of US1, but US1 is useful before it.

**Independent test:** a board agent L deletes a card "Fix the login test". Within 2 seconds each open window says once that L deleted "Fix the login test", and the details of L list the delete with its time and the card's title.

**Scenarios:**

1. Given a board agent deletes a card, When the office keeps the delete, Then every open window announces once that the agent deleted the card, naming the card's title, and the delete is in the action record of the agent.
2. Given the focused card is deleted by an agent, When the window shows it, Then focus moves as after a delete (0029/FR-004) and the announcement names the agent.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let an agent whose role is CEO or team lead read the board, with the version of each card, and make, edit, move, and delete cards on its own connection, under the rules of 0027, 0028, and 0029, without the confirmation of a delete that the person gets.
- **FR-002** (US1): IF an agent whose role is member, or that has no role, asks to read or change the board, or IF a board agent asks while it or the office is on hold, or asks for a board change beyond the rate limit for board changes (0007/FR-007), THEN THE SYSTEM SHALL refuse the request, tell the agent the reason in its answer, record the refusal (0002/FR-009), and change no card.
- **FR-003** (US1): THE SYSTEM SHALL show each board change by a board agent in every open window within 2 seconds, with no reload, and without an announcement, except for a delete (FR-006).
- **FR-004** (US1): IF a board change by an agent names a card version older than the current version of the card, or names a card that was deleted, THEN THE SYSTEM SHALL refuse the change with the reason that the card changed, or was deleted, and tell the agent in its answer.
- **FR-005** (US1): THE SYSTEM SHALL record on each card the changer of its last change, and show it as text on the card in every window.
- **FR-006** (US2): WHEN a board agent deletes a card, THE SYSTEM SHALL record the delete in the action record with the agent as actor, the card's title, and the time, show it in the agent's details, and announce once in every open window that the agent deleted the card, naming its title.

### MODIFIED

- **0002/FR-008** (as modified by 0014, 0015, and 0026) (US1): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that 0002/FR-001 allows, send reports only to its own parent or to the person, read only the secrets that the person granted to that agent, and, for an agent whose role is CEO or team lead, read the board and make, edit, move, and delete cards of the board of the office (0030/FR-001).
- **0007/FR-007** (as modified by 0026) (US1): IF an agent's agent-made action would exceed one of the rate limits in the table under Assumption A3 of 0007, with the row added by 0015, the row added by 0026, and the row added by 0030, THEN THE SYSTEM SHALL refuse the action with a reason that names the limit and its value, counting each kept board change by an agent as one action and a read of the board as none.
- **0007/FR-009** (as modified by 0026) (US1): THE SYSTEM SHALL let the user change each rate limit in the office settings, the limit of reads of secrets by one agent (`limit_secret_reads_per_agent`) and the limit of board changes by one agent (`limit_board_changes_per_agent`) included, and SHALL apply a changed value to the next agent-made action.
- **0001/FR-011** (US2): THE SYSTEM SHALL record each create, start, stop, and delete action with its actor, its target agent, and its time, and each delete of a board card by an agent with the agent as actor, the card's title, and its time, and SHALL show these entries in the details of the target agent, or of the actor agent for a card.

The table of rate limits under Assumption A3 of 0007 gains one row: board changes by one agent (added by 0030), 20 per rolling 60 seconds by default, because a lead that sets up the cards of a 5-member plan fits and a loop stops.

### PRESERVED

- **PB-1:** The board of 0027, 0028, and 0029 stays as before for the person: every requirement there works as it did (0027/FR-001 to 0027/FR-010, 0028/FR-001 to 0028/FR-006, 0029/FR-001 to 0029/FR-009).
- **PB-2:** The tree rules, the holds, and the rate limits of 0002 and 0007 stay as before for every other action: a third refusal within 60 seconds puts the agent on hold (0007/FR-008). The agent panel, the Needs-you queue, and scheduled directives stay as before (0027/PB-1 to 0027/PB-3).
- **PB-3:** An agent request other than the board request of a board agent is refused and recorded as before (0002/FR-009, 0025/PB-5). A board change by an agent types nothing into any terminal and sets no task text.
- **PB-4:** office-harness sends no office data, the board included, outside the user's machine (0001/FR-005).

## User experience

**Flow (a board agent changes a card):** the agent changes a card. The open window shows the card in its new place within 2 seconds with the agent's name as the changer; the window announces nothing and moves no focus, except for a delete.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Changed by an agent | The card shows the agent's name as the changer. | FR-003, FR-005 |
| Changed by the person or the office | The card shows the person, or the office. | FR-005 |
| Deleted by an agent | The card is gone; the window says that the agent deleted it. | FR-006 |

**Design:** the changer is plain text on the card; it adds no new visual style (0027, A10).

**Accessibility:** the changer is text in the card's accessible description, never color or an icon alone. An agent's change does not interrupt a screen reader user except for a delete, because the card leaves the board.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Card (extends 0027 and 0029) | One task on the board. | adds: changer of the last change, version | The changer is the person, an agent's name at the change, or the office, as text. The version rises with each kept change. |
| Action record entry (extends 0001) | One recorded action. | adds: delete of a board card, with the card's title | Belongs to the office; shows in the details of the actor agent. |

## Success criteria

- **SC-001:** A board agent reads the board and makes, edits, moves, and deletes 1 card each: 5 of 5 are kept under the rules of the person and show in an open window within 2 seconds with the agent as the changer, 0 announcements except for the delete, 0 Needs you items, and 0 inputs in any terminal.
- **SC-002:** 5 of 5 requests of a member agent and 2 of 2 of an agent without a role to read or change the board are refused with the reason, and 7 refusals are recorded; 0 cards change.
- **SC-003:** 3 of 3 requests of a board agent are refused with the reason and recorded: one while the agent is on hold, one while the office is on hold, and the 21st board change within 60 seconds under the default limit of 20; 0 cards change; a change of the limit in the settings applies to the next board change.
- **SC-004:** In 20 trials in which a board agent changes a card with a version older than the card's version, 20 of 20 are refused with the reason, and 0 cards are lost or doubled; a change with the current version is kept 5 of 5.
- **SC-005:** Each of 5 cards changed by an agent shows that agent's name as the changer; after the agent is renamed or deleted, 5 of 5 cards show the name that it had; a card made before this change shows the person, and a card whose assignee the office cleared shows the office.
- **SC-006:** 5 of 5 deletes by a board agent are in the action record with the agent, the card's title, and the time, and each open window announces each once within 2 seconds.
- **SC-007:** The tests of 0027, 0028, and 0029 pass unchanged (PB-1 to PB-3), and the tests of 0002 and 0007 pass with the modified requirements.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| A member agent, or an agent without a role, asks for the board. | Unauthorized | Refused with the reason and recorded (FR-002). |
| A board agent asks while it or the office is on hold, or beyond the rate limit for board changes. | Failure | Refused with the reason and recorded (FR-002). |
| A board agent reads the board many times. | Boundary | Reads are not counted against the limit (FR-002). |
| A board agent changes a card with an old version. | Concurrent | Refused with the reason (FR-004). |
| A board agent changes a card that was deleted. | Concurrent | Refused with the reason (FR-004). |
| A board agent and a window change the same card at once. | Concurrent | The first change that the office receives is kept; the other is refused with the reason (0028/FR-001, FR-004). |
| A board agent deletes a card that a window shows. | Concurrent | The window shows the card gone within 2 seconds, moves focus as after a delete, and announces once (FR-006). |
| The agent that made the last change is renamed or deleted. | Boundary | The card keeps the name it had at the change (FR-005). |
| The office cleared an assignee or a project. | Boundary | The changer is the office (FR-005). |
| The role of an agent changes between two requests. | Boundary | Each request is judged by the role at that request. |

## Assumptions

- **A1:** The role is the one the agent has when it asks. A refusal follows 0002/FR-009: the agent gets the reason in its answer and the refusal is recorded.
- **A2:** A board agent has no Undo; there is none for the person either (0029, A3). An agent that deletes a card by mistake makes the card again, and the person sees the delete in the action record.
- **A3:** A board agent may assign a card to any agent of the office and set any project of the office, as the person may; a lead is not limited to its own descendants. The owner confirmed this on 2026-11-28.
- **A4:** The changer is the person, the agent's name at the change, or the office, as text. A card made before this change shows the person.
- **A5:** The project filter of 0029 (FR-007) is a view of one window. A board agent reads and changes the whole board, whatever project the person picked.
- **A6:** An agent's change shows on the card and does not interrupt a screen reader user, so that a lead that changes many cards does not flood the person; a delete is announced because the card leaves the board and the person could not otherwise find it.
- **A7:** The rate limit of 20 board changes per rolling 60 seconds is a default that the person can change in the office settings, as the other limits of 0007.
- **A9:** A lead that deletes many cards gives one announcement per card in each window; the limit of 20 board changes per 60 seconds bounds it, and the person accepts it as the price of seeing every agent delete.
- **A10:** The action record entry of a delete stays in the action record kept with the office after the agent is deleted; the details of the agent are gone, so it shows only while the agent exists.
- **A11:** The conflict rule is one rule for windows and agents: a change is refused when anyone changed the card after the sender saw it (0028/FR-001). A window shows what it saw by what it displayed, an agent by the card version it read.
- **A8:** An agent names the card version that it read; an agent that makes a new card names no version.

## Clarifications

### Session 2026-11-28 (owner decision)

- Q: Who may read and change the board? → A: The person, and agents whose role is CEO or team lead (0002/FR-001); a member agent cannot. Agent changes show in every window like the person's, and each card records who made its last change.
- Q: Are an agent's deletes recorded? → A: Yes. A delete by an agent is recorded in the action record and shown to the person; an agent gets no confirmation.

## Out of scope

- Board access for a member agent or an agent without a role, and any agent action on the board other than reading it and making, editing, moving, and deleting cards.
- A tree rule for the board: a lead changes the cards of the whole office, not only of its descendants (A3).
- A history of changes per card, and a record of agent edits and moves in the action record; only deletes are recorded.

## Deviations

None yet.
