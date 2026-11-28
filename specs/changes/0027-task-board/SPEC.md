---
status: draft
shape: full
capability: task-board
plan:
---

# 0027: Task board

## Intent

The person keeps the plan of the office in their head or in another tool: which tasks wait, which run now, which are stuck, and which are finished. The office shows what each agent was told (0011), but not the list of tasks that the person wants done. This change adds one plain task board per office with four columns: To do, Doing, Blocked, and Done. The person makes, edits, moves, orders, and deletes cards, each with a title, an optional note, an optional project, and an optional agent of the office as assignee. The board is kept with the office, survives restarts, shows each change in every open window, and can be used with the keyboard alone and with a screen reader.

## Context

- Source of truth: `docs/PRD.md` names no release item for a task board. The owner decided on 2026-11-28 that the first version is a plain board only (Clarifications).
- This change makes the new capability `task-board`. No living spec exists for it, so the requirements are all ADDED; no existing requirement is modified or removed.
- The change depends on 0001 (the office, projects, agents, the desktop app, the office token, the owner-only storage, durability, keyboard access, and the not-reachable and not-authorized states), 0003 (the panel), 0005 (the office scene), and 0020 (a temp is removed when it ends). Ids such as `0001/FR-007` point to requirements in the living specs under `specs/`.
- The idea comes from issue boards in other tools that show an assignee and the time of the last change on each card. No code is taken from them; this spec describes behavior only.
- The task line of 0011 is a different thing: it shows the last directive an agent received. The board does not read, set, or clear it, and the task line does not read the board.
- The look of the board is not part of this change. The board uses the existing elements of the desktop app; the owner picks a look later from mockups (User experience).
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Person, office, project, agent, desktop app, background server, office token | As defined in 0001. The person is the user of 0001. |
| Panel | As defined in 0003. |
| Office scene | As defined in 0005. |
| Temp | As defined in 0020. A temp is an agent and can be an assignee. |
| Board | The one task board of an office. It has four columns and holds the office's cards. |
| Column | One of the four fixed lists of the board, in this order: To do, Doing, Blocked, Done. |
| Status | The column of a card: `todo`, `doing`, `blocked`, or `done`, shown as To do, Doing, Blocked, and Done. |
| Card | One task on the board, with a title, a note, a status, a place, a project, and an assignee. |
| Place | The position of a card inside its column, counted from 1 at the top. |
| Assignee | The one agent of the office that the person names for a card, or none. |
| Last change | The last time the person made, edited, moved, or ordered the card. An Undo keeps the last change that the card had before its delete. A change that the office makes (FR-008), or a move of another card, does not set it. |
| Board view | The desktop app view that shows the board (FR-001). |
| Window | One open desktop app window. Each window shows the board as the office keeps it. |
| Kept | The background server has stored the change, so it survives a restart. |
| Text rules of a title | After the white space at its ends, line breaks included, is removed: not blank, at most 200 characters, and no control characters. |
| Text rules of a note | After each carriage return, with or without a line feed after it, is changed to one line feed: at most 8,192 characters (an empty note is allowed), and no control characters other than line feed and tab. |
| Control character | A character of the Unicode general category Cc (C0, DEL, and C1), as in 0011/FR-006. |
| Card cap | 1,000 cards: the highest number of cards that the board holds. |

## Data flow and trust boundary

- **Boundary:** the person's desktop app connection, which presents the office token (0001/FR-007). The agent's connection (0002) gets no board request (PB-4).
- **Into the office:** from the person, a new card, an edit, a move, an order change, a delete, and an Undo. The office checks every field when it receives the change (FR-002).
- **Out of the office, to the person:** the board in every open window (FR-007).
- **Kept by the background server:** the board and every card, readable only by the OS user who owns the office (FR-007, 0001/FR-009).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent reads or changes the board, for example to hide a blocked task. | The agent's connection has no board request; any such request is refused and recorded (PB-4). |
  | A title or note carries control characters or escape codes that change what the person sees. | The text rules refuse them when the office receives the change (FR-002); the board shows every text as plain text and applies nothing in it (FR-010). |
  | A text holds direction marks that reorder the text around it, so a card reads as something else. | The direction of one text never changes the order of another text (FR-010). |
  | Two windows change one card at once, and one change is lost without a word. | A change based on a field that changed after the window showed it is refused with the reason (FR-006). |
  | A move looks done in the window but the office did not keep it. | A change that is not kept puts the card back where it was and says why (FR-005). |
  | Other users of the machine read the card texts. | The board is kept readable only by the office owner (FR-007). |
  | A loop or a mistake fills the board until the office slows down. | The card cap bounds the board at 1,000 cards (FR-002). |

## User stories

### US1: The person puts tasks on the board and moves them through the columns (Priority: P1)

The person opens the board view from the panel. The board is empty, says so, and offers New card. The person makes the card "Fix the login test" in To do, later moves it to Doing, then to Done. The board shows the card in its column with the time of its last change. After a restart of the server and of the machine, the board shows the same cards in the same places. The person can do all of this with the keyboard alone, and a screen reader says where each moved card went.

**Why this priority:** this is the board itself. Without a card that moves through the four columns and stays after a restart, nothing else in this change has value.

**Independent test:** in a new office, open the board view with the keyboard. It shows the four columns, each with 0 cards, and the message that the board has no cards. With the keyboard alone, make the card "Fix the login test", move it to Doing, then to Done. A screen reader says "Fix the login test, moved to Doing, 1 of 1" and then the same for Done. Restart the background server: the card is in Done, place 1, with its title.

**Scenarios:**

1. Given the board has no cards, When the person opens the board view, Then it shows the four columns, each with its name, the count 0, and the text that it has no cards, and the board says that it has no cards and offers New card.
2. Given the person presses New card in a column, When the person types a title and saves, Then the card is kept at the end of that column and shows its title and the time of its last change.
3. Given a card is in To do, When the person moves it to Doing by pointer, Then it is kept in Doing at the place where the person dropped it.
4. Given a card has focus, When the person moves it to the next column with the keyboard, Then it is kept at the end of that column, keeps focus, and a screen reader says its title, its new column, and its place as "n of m".
5. Given the board has cards, When the background server or the machine restarts, Then the board shows every card with the same fields in the same column and place.
6. Given a card has a project and an assignee, When the person looks at the board, Then the card shows the project name and the assignee's name as text.

### US2: A change that the office does not keep never moves a card without a word (Priority: P1)

The person moves a card while the background server is not reachable. The card goes back to where it was, and the window says that the move was not kept and why. The person has the board open in two windows and moves the same card in both at once. One move is kept, the other window says that the card changed in another window, and both windows show the card in the same place.

**Why this priority:** a board that shows a move that did not happen gives the person a false plan. The person must be able to trust each column, or the board is worse than no board.

**Independent test:** stop the background server while the board view is open, and move a card from To do to Doing. Within 2 seconds the card is back in To do at its old place, the window says that the move was not kept because the server is not reachable, and a screen reader says the same. Start the server again. Open two windows and move one card to Doing in one and to Blocked in the other at the same moment: within 2 seconds both windows show the card in one column at one place, and one window says that the card changed in another window.

**Scenarios:**

1. Given the background server is not reachable, When the person makes, edits, moves, orders, deletes, or restores a card, Then the office keeps nothing, the board shows the card as it was before, and the window says which change was not kept and why.
2. Given the office cannot store a change, for example because the disk is full, When the person moves a card, Then the card goes back to where it was and the window says that the office could not store the change.
3. Given two windows show the board, When the person changes a card in one window, Then the other window shows the change within 2 seconds, with no reload.
4. Given two windows show the same card, When both move it at the same moment, Then the office keeps the first move that it receives, refuses the second with the reason that the card changed in another window, and both windows show the kept place within 2 seconds.
5. Given one window edits a card's title, When another window moves the same card at the same moment, Then the office keeps both changes, because they set different fields.
6. Given the person types a title with a control character, When the person saves, Then the office keeps nothing, names the title field and its rule, and the form keeps the typed text.

### US3: The person edits, orders, and assigns cards (Priority: P2)

The person edits a card's title and note, picks its project and its assignee from the office, moves it up or down inside its column to show what comes first, and changes its status from the card's form. When an assignee agent is deleted, the card stays with its text and has no assignee.

**Why this priority:** US1 already gives a board that tracks tasks. The note, the order, the project, and the assignee make it more useful, but a board without them still delivers value.

**Independent test:** with the keyboard alone, edit a card's title and note, set its project to P1 and its assignee to agent A1, move it from place 3 to place 1 of its column, and change its status to Blocked in its form. Each step is kept and shows in a second window within 2 seconds. Delete A1: within 2 seconds the card shows no assignee, and its title, note, project, column, and place are the same as before.

**Scenarios:**

1. Given a card, When the person edits its title, note, project, assignee, or status and saves, Then the office keeps the fields under the text rules and shows them in every window.
2. Given a card at place 3, When the person moves it one place up with the keyboard, Then it is at place 2, the card that was at place 2 is at place 3, and the order of the other cards does not change.
3. Given a card at place 1, When the person moves it up, Then nothing changes and a screen reader says that it is already first.
4. Given agent A1 is the assignee of 3 cards, When the person deletes A1, or A1 is a temp that ends and is removed, Then the 3 cards have no assignee and every other field, column, and place stays.
5. Given project P1 is the project of a card, When the person deletes P1, Then the card has no project and every other field stays.
6. Given a title of 200 characters, or a word of 200 characters with no space, When the board shows the card, Then the full title wraps inside the card and the board does not scroll sideways at 360 px width or at 200% zoom.
7. Given a note with a control character other than line feed or tab, When the person saves, Then the office keeps nothing and names the note field and its rule.

### US4: The person deletes a card and can undo the delete (Priority: P2)

The person deletes a card from its action list or with a key. The card leaves the board at once with no confirmation. The window says that the card was deleted and offers Undo. Undo puts the card back with every field in its column and place.

**Why this priority:** a board without a delete fills up, but cards can wait in Done until this story ships. A delete without Undo loses text to one wrong key, so the two ship together.

**Independent test:** make a card with a title, a note, a project, and an assignee at place 2 of Doing. Delete it with the keyboard: it leaves every window within 2 seconds, and the deleting window offers Undo. Press Undo with the keyboard: the card is back at place 2 of Doing with 100% of its fields, in every window.

**Scenarios:**

1. Given a card has focus, When the person deletes it, Then the card leaves the board, focus moves to the next card in the column, or to the previous one, or to the column's New card, and the window says that the card was deleted and offers Undo.
2. Given the window offers Undo, When the person presses Undo, Then the card is back in its column and place with its title, note, status, project, assignee, and last change.
3. Given the card's column now holds fewer cards than its old place, When the person presses Undo, Then the card is back at the end of that column.
4. Given the card's assignee or project was deleted after the delete of the card, When the person presses Undo, Then the card is back without that assignee or project, and the window says which one is gone.
5. Given the window offers Undo for card C1, When the person deletes card C2 in the same window, Then the window offers Undo for C2 only.
6. Given another window shows the board, When the person deletes a card in one window, Then the other window shows the card gone within 2 seconds and does not offer Undo.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep one board per office with the four columns To do, Doing, Blocked, and Done in this fixed order; SHALL show it in one board view of the desktop app that the person opens from the panel and from the office scene, and leaves for the panel, with one action each, keyboard included; WHILE the board view waits for the board, THE SYSTEM SHALL say that it loads the board and show no column count; SHALL show each card in the column of its status at its place, with its title, its project name and its assignee's name when set, and the time since its last change, all as text; SHALL show each column with its name and its number of cards, and a column with no cards with the text that it has no cards; and WHEN the board has no cards, THE SYSTEM SHALL say that the board has no cards and offer New card.
- **FR-002** (US1, US2, US3): WHEN the office receives a new or edited card, THE SYSTEM SHALL keep it only IF its title meets the text rules of a title, its note meets the text rules of a note, its status is one of the four, its project, when set, is a project of the office, its assignee, when set, is an agent of the office, and, for a new card, the board holds fewer cards than the card cap; IF any rule breaks, THEN THE SYSTEM SHALL keep nothing, name each failing field and its rule, or the card cap and its value, and leave an edited card as it was; and THE SYSTEM SHALL put a new card at the end of the column of its status.
- **FR-003** (US1, US3): WHEN the person moves a card to another column or to another place in its column, by pointer or by keyboard, THE SYSTEM SHALL keep the new column as the card's status and the new place, keep the order of the other cards in both columns, and keep focus on the moved card; THE SYSTEM SHALL let the person, with the keyboard alone, move the focused card one place up or down, to the previous or the next column, and to any column from the card's action list, where a move to another column without a pointer, a change of status in the card's form included, puts the card at the end of that column; and IF a keyboard move would take a card past the first or last place, or past the first or last column, THEN THE SYSTEM SHALL change nothing and say that the card is already there.
- **FR-004** (US1, US2): WHEN the office keeps or refuses a change that the person made in a window, or FR-005 reports it as not confirmed, THE SYSTEM SHALL announce it once to assistive technology in that window within 1 second: for a move, the card's title, its column, and its place as "n of m"; for a new card, a delete, or an Undo, the card's title and its column; for a refusal, the change and its reason; THE SYSTEM SHALL NOT announce changes made in other windows; and IF the focused card is deleted in another window, THEN THE SYSTEM SHALL move focus as a delete does (US4 scenario 1) and announce once that the card was deleted in another window; and IF the focused card is moved to another column in another window, THEN THE SYSTEM SHALL keep focus on the card and announce once its new column and its place as "n of m".
- **FR-005** (US2): IF the office does not keep a new card, an edit, a move, an order change, a delete, or an Undo, because a rule of FR-002 refuses it, the card changed or was deleted in another window (FR-006), or the office could not store it, or IF the window gets no answer from the background server within 2 seconds after the person made the change, THEN THE SYSTEM SHALL, within those 2 seconds, show the card of that change in that window as it was before the change: in the column and place it had, not at all for a new card, and again for a delete; SHALL say in that window which change was not kept, or not confirmed, and why; SHALL keep the text that the person typed in an open form; and SHALL change no other card; and WHEN the window reaches the background server again, THE SYSTEM SHALL show the board as the office keeps it.
- **FR-006** (US2): THE SYSTEM SHALL apply the changes to one board one at a time, in the order that the office receives them; IF a change sets a field of a card (the title, the note, the status and place together, the project, or the assignee) that a change from another window set after the sending window last showed that card, or the card was deleted, THEN THE SYSTEM SHALL refuse the change under FR-005 with the reason that the card changed, or was deleted, in another window; and THE SYSTEM SHALL keep a change that sets only fields that nobody else changed, also when another window changed other fields of the same card; a move of one card shifts the places of the cards after it but does not count as a change of those cards.
- **FR-007** (US1, US2): THE SYSTEM SHALL show each kept change in every open window within 2 seconds, with no reload; SHALL keep the board, every card with its fields, and the order of every column across server and machine restarts, readable only by the OS user who owns the office (0001/FR-009); and SHALL let only a client that presents a valid office token (0001/FR-007) read or change the board.
- **FR-008** (US3, US4): WHEN an agent that is the assignee of cards is deleted, or removed because it was a temp that ended (0020/FR-006), THE SYSTEM SHALL clear the assignee of those cards, and WHEN a project is deleted, THE SYSTEM SHALL clear the project of its cards, in both cases keeping every other field, the column, the place, and the last change, and showing the result in every window within 2 seconds; and THE SYSTEM SHALL show an assignee by the agent's current name and a project by its current name.
- **FR-009** (US4): WHEN the person deletes a card, THE SYSTEM SHALL remove it at once with no confirmation, SHALL say in that window that the card was deleted, and SHALL offer Undo there, reachable from the keyboard and with no time limit, until the person deletes another card in that window or closes the window; WHEN the person presses Undo, THE SYSTEM SHALL restore the card with its title, note, status, project, assignee, and last change at its old place, or at the end of its column when the column holds fewer cards than that place, SHALL leave out an assignee or project that no longer exists and say which one; THE SYSTEM SHALL count an Undo as a new card for the card cap (FR-002); and IF the Undo is not kept, THEN THE SYSTEM SHALL handle it under FR-005 and keep offering Undo.
- **FR-010** (US1, US3): THE SYSTEM SHALL show every title and note exactly as kept, as plain text, without applying any control character, terminal escape code, or markup in it; SHALL keep the direction characters of one text from changing the order of any other text on the screen; SHALL wrap a long title, and a long word with no space, inside its card so that the board view does not scroll sideways at 360 px width or at 200% zoom; and SHALL show the full note in the card's details.

### PRESERVED

- **PB-1:** The agent panel stays as before: the agent list shows the name, project, status, last output line, and task line of every agent (0003/FR-002), and the task line comes only from a received directive or the person's own edit (0011/FR-002, 0011/FR-003). A card, its assignee, or a move of it changes no task text and types nothing into any terminal.
- **PB-2:** The Needs-you queue stays as before: it adds, orders, and removes items only for the attention events of 0008 (0008/FR-001 to 0008/FR-005). A board change adds no Needs you item and no notification (0003/FR-006).
- **PB-3:** Scheduled directives stay as before: schedules, their runs, their holds and limits, and the schedules view (0025/FR-001 to 0025/FR-014) do not read or change the board, and a board change does not make, change, or run a schedule.
- **PB-4:** The agent's connection to the office gets no new request. An agent request to read or change the board is refused and recorded as every request outside the allowlist is (0002/FR-009, 0025/PB-5).
- **PB-5:** office-harness sends no office data, the board included, outside the user's machine (0001/FR-005).
- **PB-6:** Every desktop app action is available from the keyboard, the actions of this change included, and every status shows as text, never color alone (0001/FR-014, 0001/FR-028).
- **PB-7:** When the desktop app opens, it opens the panel or the office scene as the last view says (0003/FR-008). The board view is not a last view (A9).

## User experience

**Flow (the person puts a task on the board):**

1. The person opens the board view from the panel or the office scene.
2. The person presses New card on the board or in a column, types a title, and can add a note, a project, an assignee, and a status. The status starts as the column where the person pressed New card, or To do.
3. The person saves. The card shows at the end of its column.

**Flow (the person moves a card):**

1. By pointer: the person drags the card to another place or column and drops it.
2. Without a pointer drag: the person gives the card focus and uses a key, or opens the card's action list and picks a column or a move up or down.
3. The card shows at its new place; a screen reader says where it went.

**Flow (the person deletes a card):**

1. The person deletes the focused card with a key or from its action list.
2. The window says that the card was deleted and offers Undo. Undo puts it back.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Loading | The board view says that it loads the board; no column shows a count until the board arrives. | FR-001 |
| Empty board | The four columns, each with the count 0 and the text that it has no cards; the board says that it has no cards and offers New card. | US1 scenario 1, FR-001 |
| Empty column | The column's name, the count 0, and the text that it has no cards. | FR-001 |
| Board with cards | Each card in its column with title, project, assignee, and the time since its last change, as text. | FR-001, US1 scenario 6 |
| Card details | The full title and the full note as plain text, with every field. | FR-010 |
| Form refused | The form names each failing field and its rule, or the card cap and its value; the typed text stays. | FR-002, FR-005 |
| Change not kept | The card is where it was before; the window says which change was not kept and why. | FR-005, US2 scenarios 1 and 2 |
| Changed in another window | The window shows the card as the office keeps it and says that the card changed, or was deleted, in another window. | FR-006, US2 scenario 4 |
| Already first or last | Nothing moves; the window and a screen reader say that the card is already there. | FR-003, US3 scenario 3 |
| Card deleted | The card is gone; the window says so and offers Undo. | FR-009, US4 scenario 1 |
| Undo restored part | The card is back; the window names the assignee or project that is gone. | FR-009, US4 scenario 4 |
| Server not reachable or not authorized | The 0001 states show before the board view (0001/FR-026, 0001/FR-027); a change made while the server is lost is not kept and says why. | FR-005, US2 scenario 1 |

**Design:** the board view is one new view of the desktop app, built from its existing list, form, and menu elements, and adds no new visual style. The look of the columns and cards is not fixed by this change; the owner picks one from mockups later. A look picked after approval is a spec change.

**Accessibility:**

- The person can do every action from the keyboard, with a visible focus: open the board view, New card, edit, each move, delete, Undo, and the action list. Escape closes the form and the action list and changes nothing.
- A drag is never the only way to move a card: the card's action list moves it with single actions (WCAG 2.2 success criterion 2.5.7).
- Each column is a labelled group with its name and its count. A card's accessible name is its title; its accessible description holds its column, its place as "n of m", its project, its assignee, and the time since its last change.
- Announcements follow FR-004: one per change made in that window, none for other windows, and no focus moves on its own except after a delete.
- The status, the project, the assignee, and the time are text, never color or an icon alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text). At 360 px width and at 200% zoom the board view and the form reflow without sideways scrolling (FR-010).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Board | The one task board of an office. | the four columns in fixed order, the order of the cards in each column | One per office. Kept with the office (FR-007). |
| Card | One task on the board. | title, note (optional), status, place, project (optional), assignee (optional), last change | Unique id. Belongs to the board; in exactly one column at exactly one place. Refers to at most one project and at most one agent, each by id. The board holds at most 1,000 cards. |
| Deleted card offered for Undo | The last card that one window deleted. | every field of the card as it was at the delete | At most one per window; gone when the window deletes another card or closes. |

## Success criteria

- **SC-001:** In a new office, the person opens the board view with 1 action from the panel and with 1 action from the office scene; the view shows 4 columns, each with the count 0 and the empty text, and the New card offer.
- **SC-002:** With the keyboard alone, the person makes a card, edits its title and note, sets its project and assignee, moves it through all 4 columns, moves it up and down in a column, deletes it, and restores it with Undo, with 0 pointer actions, and each step is kept.
- **SC-003:** With a screen reader, 10 of 10 keyboard moves are announced within 1 second with the card's title, column, and place as "n of m", and 3 of 3 refused moves are announced with their reason; a change made in a second window gives 0 announcements in the first.
- **SC-004:** With 100 cards (25 per column, each title 200 characters, each note 8,192 characters), the board view shows all 100 within 3 seconds after it opens, and each of 20 moves shows in a second window within 2 seconds.
- **SC-005:** 100 cards in 4 columns survive 5 server restarts and 1 machine restart with 100% of their fields, columns, and places unchanged.
- **SC-006:** With the background server stopped, 5 of 5 moves end with the card back in its old column and place within 2 seconds and a text that says the server is not reachable; after the server starts again, the board holds 0 of those moves.
- **SC-007:** In 20 trials in which two windows move the same card to two different columns at the same moment, 20 of 20 end with the card in exactly 1 column at 1 place, the same in both windows within 2 seconds, and exactly 1 window says that the card changed in another window; 0 cards are lost or doubled.
- **SC-008:** In 5 of 5 trials, a deleted card restored with Undo is back with 100% of its fields at its old place, and in every open window within 2 seconds.
- **SC-009:** A title of 0, 201, or 200 characters with one control character, and a note of 8,193 characters or with an escape character, are refused 5 of 5 with the field and the rule named and 0 cards changed; a title of 200 characters and a note of 8,192 characters with line feeds and tabs are kept and shown with 0 applied codes.
- **SC-010:** When the person deletes an agent that is the assignee of 3 cards, within 2 seconds the 3 cards show no assignee in every window, and 100% of their other fields, columns, and places are unchanged.
- **SC-011:** The person assigns 10 cards to agents and moves them through the columns: the agents' terminals receive 0 inputs, their task texts change 0 times, and Needs you and the notifications get 0 new entries.
- **SC-012:** With 1,000 cards on the board, a new card is refused with the card cap and its value, and 0 cards are added; a delete then lets 1 new card be kept.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The board has no cards, or a column has none. | Empty | The empty texts and New card show (FR-001). |
| The title is blank, or only spaces and line breaks. | Empty | Nothing is kept; the title field and its rule are named (FR-002). |
| The title is over 200 characters, or holds a control character, a line break inside it, or an escape code. | Invalid | Nothing is kept; the field and its rule are named; the typed text stays in the form (FR-002, FR-005). |
| The note is over 8,192 characters, or holds a control character other than line feed and tab. | Invalid | Nothing is kept; the field and its rule are named (FR-002). |
| The note has Windows line ends. | Boundary | Each carriage return with its line feed counts and shows as one line feed (Terms, text rules of a note). |
| A title of exactly 200 characters, or a word of 200 characters with no space. | Boundary | Kept; the full title wraps inside the card with no sideways scrolling (FR-010). |
| A text holds direction marks, joiners, or characters from right-to-left scripts. | Boundary | Kept as typed; shown without changing the order of any other text (FR-010). |
| A text holds characters that look like markup or a link. | Invalid | Shown as plain text; nothing is applied or opened (FR-010). |
| The board holds 1,000 cards. | Boundary | A new card or an Undo is refused with the card cap and its value (FR-002, FR-009). |
| A keyboard move past the first or last place, or past the first or last column. | Boundary | Nothing changes; the window says that the card is already there (FR-003). |
| Two windows move or edit the same field of one card at once. | Concurrent | The first change that the office receives is kept; the other is refused with the reason, and the card shows as kept in both (FR-006). |
| Two windows change different fields of one card at once. | Concurrent | Both changes are kept (FR-006). |
| Two windows order different cards of one column at once. | Concurrent | Both are kept, one after the other; every window shows the same order within 2 seconds; no card is lost or doubled (FR-006, FR-007). |
| A window edits a card that another window deletes. | Concurrent | The save is refused with the reason that the card was deleted in another window; the typed text stays in the form (FR-005, FR-006). |
| The focused card is deleted in another window. | Concurrent | Focus moves as after a delete; one announcement (FR-004). |
| The assignee agent is deleted, or is a temp that ends. | Boundary | The card keeps every other field and has no assignee (FR-008). |
| The card's project is deleted. | Boundary | The card keeps every other field and has no project (FR-008). |
| The assignee or project is deleted while the form that sets it is open. | Concurrent | The save is refused with the field and the reason that it no longer exists; the other typed text stays (FR-002, FR-005). |
| Undo after the card's assignee or project was deleted, or after its column got shorter. | Boundary | The card is back without that assignee or project, or at the end of the column, and the window says so (FR-009). |
| The window that deleted a card closes, or deletes another card. | Boundary | Undo for the first card is gone (FR-009). |
| The background server is not reachable, or stops during a change. | Failure | With no answer within 2 seconds, the card shows where it was before and the window says that the change was not confirmed and why; when the server is back, the window shows the board as the office keeps it, which can hold the change if the office stored it before it stopped (FR-005). |
| The office cannot store a change, for example because the disk is full. | Failure | The same as above, with the reason that the office could not store it (FR-005). |
| The background server stops without a clean stop. | Failure | At the next start, the board shows every kept change and none that was not kept (FR-007). |
| A client without a valid office token asks for the board. | Unauthorized | Refused; the client gets no card (FR-007, 0001/FR-007). |
| An agent asks the office to read or change the board. | Unauthorized | Refused and recorded (PB-4). |
| The board holds many cards in one column. | Boundary | The column shows every card in its order, and scrolls only up and down (FR-001, FR-010). |

## Assumptions

- **A1:** Only the person makes, edits, moves, orders, and deletes cards, through the desktop app. The first version is the person's own plan of the office, not an agent tool (Out of scope).
- **A2:** A card has a title, an optional note, a status that is one of the four columns, an optional project of the office, and an optional single assignee that is an agent of the office. The board does not check that the assignee belongs to the card's project, because a person can give a task of one project to an agent of another.
- **A3:** The four columns are fixed in name, order, and count. Blocked has no reason field; the person writes the reason in the note.
- **A4:** A card follows its project and its assignee by id. When the agent or project is deleted, the card keeps its text and loses only that link, because the task is still the person's even when its agent is gone. That clear is the office's change, so it does not set the last change.
- **A5:** Cards have an order inside a column, and the person can change it. A new card, and a card moved to another column without a pointer, go to the end of the column, so a keyboard move is predictable; a pointer drop puts the card where it was dropped.
- **A6:** Each window shows the board as the office keeps it, and every kept change reaches every open window within 2 seconds. A change is refused only when it sets a field that another change set after the window last showed the card; this keeps a long edit in an open form from being refused because another window only moved the card.
- **A7:** A delete has no confirmation, because Undo restores the card. Undo has no time limit (WCAG 2.2 success criterion 2.2.1) and is offered only in the window that deleted the card, for its last delete, while that window is open.
- **A8:** Board changes add no entry to the action record. The action record holds the actions on agents and shows them in an agent's details (0001/FR-011); a card is not an agent, and its last change is shown on the card.
- **A9:** The board view is not part of the last view of 0003/FR-008; the desktop app still opens the panel or the scene. This keeps 0003 unchanged; a later change can add the board to the last view.
- **A10:** The board shows every card of the office. The project switcher of the panel (0003/FR-005) does not filter the board in this version; each card shows its project as text.
- **A11:** The title limit of 200 characters keeps a card readable; the note limit of 8,192 characters matches the directive and task text limit (0011/FR-003, 0025 Terms). Characters are counted as Unicode code points, so one emoji can count as more than one character.
- **A12:** The card cap of 1,000 is ten times the 100 cards that this change measures, and keeps a mistake from filling the board without end. It is fixed, not a setting.
- **A13:** The look of the board stays the existing plain style of the desktop app until the owner picks a look from mockups.
- **A14:** The task line of 0011 is not related to the board and stays as it is (PB-1).

## Clarifications

### Session 2026-11-28 (owner decision, given through the caller)

- Q: What does the first version of the task board hold? → A: Only a plain board, one per office, with the four columns To do, Doing, Blocked, and Done. No cost or token view, no meeting room, no work across machines, and no scheduled temps (Out of scope).
- Q: Does the spec fix the look of the board? → A: No. The spec gives behavior only; the owner picks a look from mockups later (User experience, A13).

## Out of scope

- Agents that read or change the board, and any agent request about it (PB-4). An agent that changes the board would need the tree rules, holds, and rate limits of 0002 and 0007, as schedules needed in 0025; the first version stays the person's own tool. A later change can add it.
- A card that sends a directive, starts an agent, or sets an agent's task line (PB-1).
- A cost or token view per card, the meeting room (0004), work across machines (0006), and scheduled temps.
- More than one board per office, columns that the person names, adds, or removes, and limits per column.
- Labels, due dates, priorities, sub-tasks, checklists, comments, attachments, and links between cards.
- A card history, an archive of Done cards, and a search or filter of the board, the project filter included (A10).
- Undo of an edit or a move, and Undo in another window or after the window closes (A7).
- Import, export, and copying of the board.
- Notifications and Needs you items for board changes (PB-2).
- A visual design of the board beyond the existing elements of the desktop app (A13).
- Size: 10 ADDED requirements, within the limit of 10; no split is proposed.

## Deviations

None yet.
