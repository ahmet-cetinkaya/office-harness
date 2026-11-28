---
status: draft
shape: full
capability: task-board
plan:
---

# 0029: Board agent access

## Intent

The person works on the task board of 0027 and 0028. This change lets the agents that run the work, the CEO and the team leads, read the board and change its cards under the same rules as the person. A member agent cannot. Each card shows who made its last change, so the person can tell an agent's change from their own.

## Context

- The owner decided on 2026-11-28 that the board is read and changed by the person and by agents whose role is CEO or team lead (0002/FR-001); a member agent cannot (Clarifications).
- This change depends on 0027 and 0028 (the board, the cards, the rules, the conflict refusal), on 0002 (the roles and the refusal of an agent request, 0002/FR-009), and on 0007 (the hold and the rate limits). It extends the capability `task-board`: all requirements below are ADDED. Ids such as `0027/FR-008` point to the living spec `specs/task-board/SPEC.md` once 0027 and 0028 ship.
- The requirements describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Board, card, window, saw, kept, board view | As defined in 0027. |
| Role, CEO, team lead, member | As defined in 0002/FR-001. |
| Hold, rate limit | As defined in 0007. |
| Board agent | An agent whose role is CEO or team lead. Only a board agent may read or change the board. |
| Changer | Who made the last change of a card: the person, or a board agent by the name that it had at that change. A later rename or delete of the agent does not change it. |

## Data flow and trust boundary

- **Boundary:** the connection of an agent (0002), whose role the office knows. The person's connection stays as in 0027.
- **Into the office:** from a board agent, a request to read the board, and to make, edit, move, and delete cards. The office checks the role, the hold, the rate limit, and every field when it receives the request (FR-001, FR-003).
- **Out of the office:** to a board agent, the board in its answer; to the person, each change in every open window (FR-004).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | A member agent, or an agent without a role, reads or changes the board, for example to hide a blocked task. | Refused with the reason and recorded (FR-002). |
  | A board agent loops over the board, or acts while it or the office is on hold. | The hold and the rate limits of 0007 apply; the refusal is recorded (FR-003). |
  | An agent changes a card and the person cannot tell who did it. | Each card records and shows the changer (FR-005). |
  | A board agent and a window change one card at once. | The conflict rule of 0027/FR-008 applies to both (FR-004). |

## User stories

### US1: A CEO or a team lead agent works on the board (Priority: P1)

A team lead agent reads the board, makes a card for one of its members, and moves it to Doing. The person sees the card appear and move in the open window, with the lead named as the changer. A member agent that asks for the board is refused with the reason, and the refusal is recorded.

**Why this priority:** it is the owner's access rule. The leads that run the work keep the board current, and a member cannot hide or change a task.

**Independent test:** with a team lead agent L and a member agent M, L reads the board, makes a card assigned to M, and moves it to Doing. Within 2 seconds the open window shows the card in Doing with L as the changer, gives 0 announcements and 0 Needs you items. M asks to read the board and to make a card: both are refused with the reason that its role is member, and 2 refusals are recorded. The person puts L on hold, then L asks for the board: it is refused with the reason that it is on hold.

**Scenarios:**

1. Given a board agent, When it asks for the board, Then it gets every card with its fields and its changer, in the order of each column.
2. Given a board agent, When it makes, edits, moves, or deletes a card, Then the office applies the rules of 0027 and 0028 as for the person, and shows the result in every open window within 2 seconds; the confirmation of a delete is for the person only.
3. Given a card that a board agent changed last, When the person looks at the card, Then the card shows that agent's name as the changer.
4. Given a member agent, or an agent without a role, When it asks to read or change the board, Then the office refuses it, tells it the reason in the answer, and records the refusal; no card changes.
5. Given a board agent is on hold, or the office is on hold, or it is beyond a rate limit, When it asks to read or change the board, Then the office refuses it with the reason and records the refusal.
6. Given a board agent edits a card that the person moved after the agent last read it, When both change the card, Then the office keeps the first change it receives and refuses the other with the reason that the card changed (0027/FR-008); the agent gets the reason in its answer.
7. Given a card that a board agent changed, When the agent is deleted, Then the card shows the name it had at the change.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let an agent whose role is CEO or team lead read the board, and make, edit, move, and delete cards, on its own connection, under the rules of 0027 and 0028, without the confirmation of a delete that the person gets.
- **FR-002** (US1): IF an agent whose role is member, or that has no role, asks to read or change the board, THEN THE SYSTEM SHALL refuse the request, tell the agent the reason in its answer, record the refusal, and change no card.
- **FR-003** (US1): IF a board agent asks to read or change the board while it or the office is on hold, or beyond a rate limit (0002/FR-009, 0007), THEN THE SYSTEM SHALL refuse the request, tell the agent the reason in its answer, record the refusal, and change no card.
- **FR-004** (US1): THE SYSTEM SHALL show each change by a board agent in every open window within 2 seconds, without an announcement and without a move of focus, except for a focused card that the change deletes or moves (0027/FR-010, 0028/FR-006), and SHALL apply the conflict rule of 0027/FR-008 to the changes of a board agent and of a window alike, refusing the later one with the reason in the answer of the agent or in the window.
- **FR-005** (US1): THE SYSTEM SHALL record on each card the changer of its last change, and show it as text on the card in every window.

### PRESERVED

- **PB-1:** The board of 0027 and 0028 stays as before for the person: every requirement there works as it did (0027/FR-001 to 0027/FR-010, 0028/FR-001 to 0028/FR-006).
- **PB-2:** The tree rules, the holds, and the rate limits of 0002 and 0007 stay as before; this change adds no new limit. The agent panel, the Needs-you queue, and scheduled directives stay as before (0027/PB-1 to 0027/PB-3).
- **PB-3:** An agent request other than the board request of a board agent is refused and recorded as before (0002/FR-009, 0025/PB-5). A board change by an agent types nothing into any terminal and sets no task text.
- **PB-4:** office-harness sends no office data, the board included, outside the user's machine (0001/FR-005).

## User experience

**Flow (a board agent changes a card):** the agent changes a card. The open window shows the card in its new place within 2 seconds with the agent's name as the changer; the window announces nothing and moves no focus.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Changed by an agent | The card shows the agent's name as the changer. | FR-004, FR-005 |
| Person changed | The card shows the person as the changer. | FR-005 |

**Design:** the changer is plain text on the card; it adds no new visual style (0027, A10).

**Accessibility:** the changer is text in the card's accessible description, never color or an icon alone.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Card (extends 0027 and 0028) | One task on the board. | adds: changer of the last change | Holds the person or the agent's name at the change, as text. |

## Success criteria

- **SC-001:** A board agent reads the board and makes, edits, moves, and deletes 1 card each: 5 of 5 are kept under the rules of the person and show in an open window within 2 seconds with the agent as the changer, 0 announcements, 0 Needs you items, and 0 inputs in any terminal.
- **SC-002:** 5 of 5 requests of a member agent and 2 of 2 of an agent without a role to read or change the board are refused with the reason, and 7 refusals are recorded; 0 cards change.
- **SC-003:** 3 of 3 requests of a board agent are refused with the reason and recorded: one while the agent is on hold, one while the office is on hold, one beyond a rate limit; 0 cards change.
- **SC-004:** In 20 trials in which a board agent and a window change the same card at the same moment, 20 of 20 keep 1 change and refuse the other with the reason; 0 cards are lost or doubled.
- **SC-005:** Each of 5 cards changed by an agent shows that agent's name as the changer; after the agent is renamed or deleted, 5 of 5 cards show the name that it had.
- **SC-006:** The tests of 0027 and 0028 pass unchanged (PB-1 to PB-3).

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| A member agent, or an agent without a role, asks for the board. | Unauthorized | Refused with the reason and recorded (FR-002). |
| A board agent asks while it or the office is on hold, or beyond a rate limit. | Failure | Refused with the reason and recorded (FR-003). |
| A board agent and a window change one field of one card at once. | Concurrent | The first change that the office receives is kept; the other is refused with the reason (FR-004). |
| A board agent deletes a card that a window shows. | Concurrent | The window shows the card gone within 2 seconds and moves focus as after a delete (FR-004). |
| The agent that made the last change is renamed or deleted. | Boundary | The card keeps the name it had at the change (FR-005). |
| The role of an agent changes between two requests. | Boundary | Each request is judged by the role at that request. |

## Assumptions

- **A1:** The role is the one the agent has when it asks. A refusal follows 0002/FR-009: the agent gets the reason in its answer and the refusal is recorded.
- **A2:** A board agent has no Undo; there is none for the person either (0028, A3). An agent that deletes a card by mistake makes the card again.
- **A3:** A board agent may assign a card to any agent of the office and set any project of the office, as the person may; a lead is not limited to its own descendants.
- **A4:** The changer is the person or the agent's name at the change, as text.

## Clarifications

### Session 2026-11-28 (owner decision)

- Q: Who may read and change the board? → A: The person, and agents whose role is CEO or team lead (0002/FR-001); a member agent cannot. Agent changes show in every window like the person's, and each card records who made its last change.

## Out of scope

- Board access for a member agent or an agent without a role, and any agent action on the board other than reading it and making, editing, moving, and deleting cards.
- A tree rule for the board: a lead changes the cards of the whole office, not only of its descendants (A3).
- A limit for the board only; the existing holds and rate limits apply.
- A history of changes per card.

## Deviations

None yet.
