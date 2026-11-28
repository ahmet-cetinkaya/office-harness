---
status: draft
shape: full
capability: task-board
plan:
---

# 0028: Card details

## Intent

The task board of 0027 holds cards with a title, an optional note, and a status. This change lets the person edit a card, name its project and one assignee agent, and delete a card after a plain confirmation. A card keeps its text when its agent or project goes away.

## Context

- This change depends on 0027 (the board, the card, the columns, the text rules, the conflict refusal, the not-kept handling, and the announcements) and on 0001 and 0020 (agents, projects, and a temp that ends). The requirements of 0027 are in the living spec `specs/task-board/SPEC.md` once 0027 ships; ids such as `0027/FR-008` point to them. Terms of 0027 apply here.
- It extends the capability `task-board`: all requirements below are ADDED; none of 0027 is modified. The conflict rule of 0027/FR-008 and the not-kept handling of 0027/FR-009 apply to every change below.
- Undo of a delete, ordering inside a column, and a time of the last change stay out, as in 0027.
- The requirements describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Card, column, status, board view, window, saw, kept, text rules of a title, text rules of a note | As defined in 0027. |
| Project, agent, temp | As defined in 0001 and 0020. A temp is an agent and can be an assignee. |
| Assignee | The one agent of the office that the person names for a card, or none. |
| Card project | The one project of the office that the person names for a card, or none. |

## Data flow and trust boundary

- **Boundary:** the person's desktop app connection with the office token (0001/FR-007), as in 0027. The agent's connection gets no board request (0027/PB-4).
- **Into the office:** from the person, an edit of a card, a project, an assignee, a status, and a delete. The office checks every field when it receives the change (FR-001, FR-002).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | A card names a project or an agent that does not exist, or one that went away. | The office checks that they exist when it receives the change, and clears them when they go away (FR-002, FR-003). |
  | A delete by one wrong key loses a card. | A delete asks the person to confirm it (FR-004). |
  | An edit overwrites what another window just changed. | The conflict rule of 0027/FR-008 refuses it with the reason (FR-005). |

## User stories

### US1: The person edits a card and gives it a project and an assignee (Priority: P1)

The person opens a card, changes its title and note, picks a project and an agent of the office, and saves. The card shows the project and the agent as text in every window. When the agent is deleted, the card stays with its text and has no assignee.

**Why this priority:** a card that cannot be corrected or tied to an agent is only a sticky note. The two edits make the board usable for real work, and they come before delete.

**Independent test:** with the keyboard alone, edit a card's title and note, set its project to P1 and its assignee to agent A1, and change its status to Blocked in its form. Each step is kept and shows in a second window within 2 seconds. Delete A1: within 2 seconds the card shows no assignee, and its title, note, project, and column are the same as before.

**Scenarios:**

1. Given a card, When the person edits its title, note, project, assignee, or status and saves, Then the office keeps the fields under the rules and shows them in every window; a status change puts the card at the last place of the new column.
2. Given a title or note that breaks its text rules, When the person saves, Then the office keeps nothing, names the field and its rule, and the form keeps the typed text.
3. Given agent A1 is the assignee of 3 cards, When the person deletes A1, or A1 is a temp that ends and is removed, Then the 3 cards have no assignee and every other field, column, and place stays.
4. Given project P1 is the project of a card, When the person deletes P1, Then the card has no project and every other field stays.
5. Given the assignee or project is deleted while the form that sets it is open, When the person saves, Then the office refuses the save with the field and the reason that it no longer exists, and the other typed text stays.
6. Given another window changed the card after this window saw it, When the person saves an edit, Then the office refuses it with the reason that the card changed in another window.

### US2: The person deletes a card after a confirmation (Priority: P2)

The person deletes a card from its action list or with a key. The window asks to confirm the delete. After the confirmation the card leaves every window.

**Why this priority:** a board without a delete fills up, but cards can wait in Done until this story ships.

**Independent test:** delete a card with the keyboard. The window asks to confirm and names the card. Press Escape: the card stays. Delete again and confirm: the card leaves both open windows within 2 seconds, and focus moves to a card of the same column or to its New card.

**Scenarios:**

1. Given a card has focus, When the person deletes it, Then the window asks to confirm and names the card; Escape or Cancel changes nothing.
2. Given the person confirms, When the office keeps the delete, Then the card leaves every window within 2 seconds and focus moves to the next card in the column, or the previous one, or the column's New card.
3. Given another window deleted the card first, When this window saves an edit or confirms a delete of it, Then the office refuses with the reason that the card was deleted in another window.

## Requirements

### ADDED

- **FR-001** (US1): WHEN the person saves an edit of a card, THE SYSTEM SHALL keep the new title, note, and status only IF they meet the text rules and the four columns, put a card whose status changed at the last place of the new column, and, IF a rule breaks, THEN THE SYSTEM SHALL keep nothing, name each failing field and its rule, and leave the card as it was.
- **FR-002** (US1): WHEN the person saves a card with a project or an assignee, THE SYSTEM SHALL keep it only IF the project is a project of the office and the assignee is an agent of the office; IF one does not exist, THEN THE SYSTEM SHALL keep nothing and name the field and the reason that it does not exist; and THE SYSTEM SHALL show the project and the assignee by their current names, as text, on the card.
- **FR-003** (US1): WHEN an agent that is the assignee of cards is deleted, or removed because it was a temp that ended (0020/FR-006), THE SYSTEM SHALL clear the assignee of those cards, and WHEN a project is deleted, THE SYSTEM SHALL clear the project of its cards, in both cases keeping every other field, the column, and the place, and showing the result in every window within 2 seconds.
- **FR-004** (US2): WHEN the person deletes a card, THE SYSTEM SHALL ask the person to confirm, naming the card, and change nothing on Escape or Cancel; WHEN the person confirms, THE SYSTEM SHALL remove the card and move focus to the next card of the column, or the previous one, or the column's New card.
- **FR-005** (US1, US2): IF a change of this spec is to a card that another window changed or deleted after the sending window saw it, THEN THE SYSTEM SHALL refuse it with the reason that the card changed, or was deleted, in another window, under 0027/FR-008 and 0027/FR-009.
- **FR-006** (US1, US2): WHEN the office keeps, refuses, or does not confirm an edit or a delete that the person made in a window, THE SYSTEM SHALL announce it once to assistive technology in that window within 1 second with the card's title and the change, or the reason for a refusal; IF the focused card is deleted in another window, THEN THE SYSTEM SHALL move focus as FR-004 does and announce once that the card was deleted in another window.

### PRESERVED

- **PB-1:** The living spec `task-board` of 0027 stays as before: the board, the move, the conflict refusal, the not-kept handling, and the announcements work as they do (0027/FR-001 to 0027/FR-010).
- **PB-2:** The agent panel, the Needs-you queue, and scheduled directives stay as before (0027/PB-1 to 0027/PB-3). An assignee or a project on a card changes no task text, types nothing into any terminal, and adds no Needs you item.
- **PB-3:** The agent's connection gets no board request (0027/PB-4); office-harness sends no board data off the machine (0027/PB-5).
- **PB-4:** Every action of this change is available from the keyboard, and the project and the assignee show as text (0001/FR-028).

## User experience

**Flow (the person edits a card):** the person gives a card focus, opens its form from the action list or a key, changes fields, and saves. The form shows the project and the assignee as lists of the office. The card shows the fields in every window.

**Flow (the person deletes a card):** the person gives the card focus, chooses Delete, and confirms in the confirmation. The card leaves the board.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Edit form | Title, note, status, project, and assignee, with the card's values. | FR-001, FR-002 |
| Form refused | Each failing field and its rule; the typed text stays. | FR-001, FR-002 |
| Assignee or project gone | The card shows no assignee or project; the other text stays. | FR-003 |
| Confirm delete | A confirmation that names the card, with Delete and Cancel. | FR-004 |
| Card deleted in another window | The card is gone; the window says so. | FR-005, FR-006 |

**Design:** the form and the confirmation use the existing elements of the desktop app and add no new visual style (0027, A10).

**Accessibility:** every action runs from the keyboard with a visible focus; Escape closes the form and the confirmation and changes nothing; the confirmation takes focus and returns it to the card on Cancel; project and assignee are text.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Card (extends 0027) | One task on the board. | adds: project (optional), assignee (optional) | Refers to at most one project and at most one agent, each by id. |

## Success criteria

- **SC-001:** With the keyboard alone, the person edits a card's title, note, project, assignee, and status, and each step is kept and shows in a second window within 2 seconds.
- **SC-002:** A title, note, project, or assignee that breaks a rule is refused 5 of 5 with the field and the rule named, and the card is unchanged.
- **SC-003:** When the person deletes an agent that is the assignee of 3 cards, within 2 seconds the 3 cards show no assignee in every window, and 100% of their other fields, columns, and places are unchanged; the same holds for a deleted project and for a temp that ends.
- **SC-004:** With the keyboard alone, the person deletes a card: it stays after Escape in 5 of 5 trials, and leaves every open window within 2 seconds after the confirmation.
- **SC-005:** In 10 trials, an edit of a card that another window changed or deleted is refused 10 of 10 with the reason that the card changed or was deleted in another window.
- **SC-006:** The live region holds the title and the change for 10 of 10 edits and deletes within 1 second (the Linux residual of 0027, A13, applies). The tests of 0027 pass unchanged (PB-1 to PB-3); the agents' terminals receive 0 inputs from 10 assignments.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The title or note breaks its text rules on edit. | Invalid | Nothing is kept; the field and rule are named (FR-001). |
| The assignee or project is deleted while the form is open. | Concurrent | The save is refused with the field and the reason (FR-002). |
| The assignee is a temp that ends. | Boundary | The card has no assignee; other fields stay (FR-003). |
| A card is deleted in another window while this one edits it. | Concurrent | The save is refused with the reason that the card was deleted (FR-005). |
| The person presses delete twice, fast. | Boundary | One confirmation shows; one card is deleted after the person confirms (FR-004). |
| The server is not reachable during an edit or a delete. | Failure | The card stays as it was; the window says the change was not kept (0027/FR-009). |

## Assumptions

- **A1:** The board does not check that the assignee belongs to the card's project; a person can give a task of one project to an agent of another.
- **A2:** A card follows its project and its assignee by id; when one goes away the card keeps its text and loses only that link, because the task is still the person's.
- **A3:** A delete has no Undo; the confirmation guards it. Undo is a later addition.
- **A4:** A change of status in the form puts the card at the last place of the new column, as a move does (0027, A4).
- **A5:** The project switcher of the panel does not filter the board (0027, A9; still open with the owner).

## Clarifications

### Session 2026-11-28 (owner decision)

- Q: Undo of a delete? → A: Not in the first version; use a plain confirmation.

## Out of scope

- Undo of a delete, and ordering of cards inside a column (0027).
- Agent access to the board (0029).
- Labels, due dates, priorities, sub-tasks, comments, and links between cards.
- A visual design of the form or the board beyond the existing elements.

## Deviations

None yet.
