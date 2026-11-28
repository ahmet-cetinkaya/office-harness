---
status: draft
shape: full
capability: task-board
plan:
---

# 0028: Safe board changes

## Intent

The board of 0027 works for one person in one window with a server that always answers. This change makes the board trustworthy when that is not so: a change that the office does not keep never leaves a card in a place that the office does not hold, and two windows that change one card at once never lose a change without a word.

## Context

- This change depends on 0027 (the board, the card, the columns, the text rules, the move, the announcements). Ids such as `0027/FR-006` point to the living spec `specs/task-board/SPEC.md` once 0027 ships. Terms of 0027 apply here.
- 0027 and 0028 release together: 0027 alone has no handling of a change that the office did not keep, so a move could look done when it was not. The plan of 0027 does not ship before 0028 is ready.
- It extends the capability `task-board`: all requirements below are ADDED; none of 0027 is modified. Changes of 0029 (edit and delete) and of 0030 (agents) use the rules of this change.
- Field-level merging of two windows' changes, and Undo, stay out, as in 0027.
- The requirements describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Card, column, status, board view, window, kept, text rules | As defined in 0027. |
| Saw | A window saw a card when it last showed that card as the office keeps it. A card is changed by anyone when a window or an agent changes it (0029, 0030). The office's clear of a gone assignee or project (0029/FR-003) is not a change for this rule. For an open edit form, the window saw the card as the form opened it (0029/FR-005). |
| Not kept | A change that the office refused, could not store, or did not answer within 2 seconds (FR-002). |

## Data flow and trust boundary

- **Boundary:** the person's desktop app connection with the office token (0001/FR-007), as in 0027.
- **Into the office:** from the person, a new card and a move, as in 0027; the office decides in the order that it receives them (FR-001).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | Two windows move one card at once, and one move is lost without a word. | A change to a card that changed after the window saw it is refused with the reason (FR-001). |
  | A move looks done in the window but the office did not keep it. | A change that is not kept puts the card back where it was and says why (FR-002). |
  | A retry of a save makes a second card. | Saving the same new card again keeps one card (FR-005). |

## User stories

### US1: A change that the office does not keep never moves a card without a word (Priority: P1)

The person moves a card while the background server is not reachable. The card goes back to where it was, and the window says that the move was not kept and why. The person has the board open in two windows and moves the same card in both at once. One move is kept, the other window says that the card changed in another window, and both windows show the card in the same column.

**Why this priority:** a board that shows a move that did not happen gives the person a false plan. The person must be able to trust each column, or the board is worse than no board.

**Independent test:** stop the background server while the board view is open, and move a card from To do to Doing. Within 2 seconds the card is back in To do, the window says that the move was not kept because the server is not reachable, and the live region says the same. Start the server again. Open two windows and move one card to Doing in one and to Blocked in the other at the same moment: within 2 seconds both windows show the card in one column, and one window says that the card changed in another window.

**Scenarios:**

1. Given the background server is not reachable, When the person makes or moves a card, Then the office keeps nothing, the board shows the card as it was, and the window says which change was not kept and why.
2. Given the office cannot store a change, for example because the disk is full, When the person moves a card, Then the card goes back to where it was and the window says that the office could not store the change.
3. Given two windows saw the same card, When both move it at the same moment, Then the office keeps the first move that it receives, refuses the second with the reason that the card changed in another window, and both windows show the kept column within 2 seconds.
4. Given the person typed a new card and the office refuses it, When the window shows the refusal, Then the form keeps the typed text and no other card changes.
5. Given the window got no answer for a new card, When the window reaches the server again and the person saves the same form again, Then the board holds one card, not two.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL apply the changes to one board one at a time, in the order that the office receives them; IF a change is to a card that anyone (another window or an agent) changed after the sending window saw that card, THEN THE SYSTEM SHALL refuse the change with the reason that the card changed, and who changed it when it was not a window, and THE SYSTEM SHALL NOT count a move of another card as a change of this card.
- **FR-002** (US1): IF the office does not keep a new card or a move, because a rule of 0027/FR-006 refuses it, the card changed (FR-001), or the office could not store it, or IF the window gets no answer from the background server within 2 seconds after the person made the change, THEN THE SYSTEM SHALL, within those 2 seconds, show the card of that change as it was before, in its column and place, and not at all for a new card, and say in that window which change was not kept, or not confirmed, and why.
- **FR-003** (US1): IF a change is not kept, THEN THE SYSTEM SHALL keep the text that the person typed in an open form and change no other card.
- **FR-004** (US1): WHEN the window reaches the background server again after a change that was not confirmed, THE SYSTEM SHALL show the board as the office keeps it.
- **FR-005** (US1): WHEN the person saves again the same new card form after a save that was not confirmed, THE SYSTEM SHALL keep one card for it, not two.
- **FR-006** (US1): WHEN the office refuses a change or FR-002 reports one as not confirmed, THE SYSTEM SHALL announce it once to assistive technology in that window within 1 second with the change and its reason, and SHALL NOT announce it in other windows.

### PRESERVED

- **PB-1:** The board of 0027 stays as before when no change fails: the view, the text rules, the card cap, the move, the keeping across restarts, the live update, and the announcements of kept changes (0027/FR-001 to 0027/FR-010).
- **PB-2:** The agent panel, the Needs-you queue, and scheduled directives stay as before (0027/PB-1 to 0027/PB-3). A refused change adds no Needs you item and no notification.
- **PB-3:** The agent's connection gets no board request (0027/PB-4), until 0030 ships; office-harness sends no board data off the machine (0027/PB-5).
- **PB-4:** Every action of this change is available from the keyboard, and each message shows as text (0001/FR-028).

## User experience

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Change not kept | The card is where it was before; the window says which change was not kept and why. | FR-002, US1 scenarios 1 and 2 |
| Changed in another window | The window shows the card as the office keeps it and says that the card changed in another window. | FR-001, US1 scenario 3 |
| Form refused, text kept | The form shows the refusal and still holds the typed text. | FR-003 |
| Server back | The window shows the board as the office keeps it. | FR-004 |

**Design:** the messages use the existing elements of the desktop app and add no new visual style (0027, A10).

**Accessibility:** a refusal is announced once with its reason (FR-006); the message is text, never color or an icon alone; focus stays on the card or in the form.

## Key entities

None new. A card of 0027 is the entity; its state is what the office keeps.

## Success criteria

- **SC-001:** With the background server stopped, 5 of 5 moves end with the card back in its old column and place within 2 seconds and a text that says the server is not reachable; after the server starts again, the board holds 0 of those moves.
- **SC-002:** In 20 trials in which two windows move the same card to two different columns at the same moment, 20 of 20 end with the card in exactly 1 column, the same in both windows within 2 seconds, and exactly 1 window says that the card changed in another window; 0 cards are lost or doubled.
- **SC-003:** After a new card gets no answer and the person saves the same form again, the board holds 1 card with that title in 5 of 5 trials.
- **SC-004:** 3 of 3 refused changes (a rule, a conflict, a store that fails) put the card back within 2 seconds, keep the typed text, and change 0 other cards; the live region holds each reason once within 1 second (the Linux residual of 0027, A13, applies).
- **SC-005:** The tests of 0027 pass unchanged (PB-1 to PB-3).

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| Two windows move the same card at once. | Concurrent | The first move that the office receives is kept; the other is refused with the reason; both windows show the kept column (FR-001). |
| Two windows move different cards into one column at once. | Concurrent | Both are kept, one after the other, in the order that the office receives them; every window shows the same order within 2 seconds; no card is lost or doubled (FR-001, 0027/FR-009). |
| The window gets no answer for a new card, and the person saves again. | Failure | The board holds one card with that title (FR-005). |
| The background server is not reachable, or stops during a change. | Failure | With no answer within 2 seconds, the card shows where it was before and the window says that the change was not confirmed and why; when the server is back, the window shows the board as the office keeps it (FR-002, FR-004). |
| The office cannot store a change, for example because the disk is full. | Failure | The same as above, with the reason that the office could not store it (FR-002). |
| A refused change while a form is open. | Invalid | The form keeps the typed text (FR-003). |

## Assumptions

- **A1:** A change to a card is refused when anyone (another window or an agent) changed the card after the window saw it; moving another card does not change this card. This is one rule for the whole card, not per field.
- **A2:** The window that gets no answer treats the change as not confirmed; the office may have kept it, and the window shows the truth when the server is back (FR-004).
- **A3:** "The same new card form" is one save action of one form; the office recognizes a repeat of it, so a retry never makes a second card. A title that the person retyped or edited after the failed save is a new save, and makes a second card.

## Clarifications

### Session 2026-11-28 (owner decision)

- Q: How are two windows' changes handled? → A: A change to a card that changed since this window saw it is refused with the reason; there is no field-level merge in the first version.

## Out of scope

- Field-level merging of two windows' changes, and Undo.
- A retry that the office makes for the person; the person saves again.
- Changes of editing and agents (0029, 0030) beyond using these rules.

## Deviations

None yet.
