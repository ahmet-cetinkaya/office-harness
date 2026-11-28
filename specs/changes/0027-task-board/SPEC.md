---
status: draft
shape: full
capability: task-board
plan:
---

# 0027: Task board

## Intent

The person keeps the plan of the office in their head or in another tool: which tasks wait, which run now, which are stuck, and which are finished. This change adds one plain task board per office with four columns: To do, Doing, Blocked, and Done. The person makes cards, each with a title and an optional note, and moves them between the columns. The board is kept with the office, survives restarts, shows each change in every open window, and can be used with the keyboard alone and with a screen reader.

This is the first of three changes. 0028 adds editing, a project, an assignee, and delete; 0029 adds board access for CEO and team lead agents. Each ships and merges on its own, in that order.

## Context

- Source of truth: `docs/PRD.md` names no release item for a task board. The owner decided on 2026-11-28 that the first version is a plain board only (Clarifications).
- This change makes the new capability `task-board`. No living spec exists for it, so all requirements are ADDED; no existing requirement is modified or removed.
- The change depends on 0001 (the office, the desktop app, the office token, the owner-only storage, keyboard access, and the not-reachable and not-authorized states), 0003 (the panel), and 0005 (the office scene). Ids such as `0001/FR-007` point to requirements in the living specs under `specs/`.
- The task line of 0011 is a different thing: it shows the last directive an agent received. The board does not read, set, or clear it.
- The look of the board is not part of this change. The board uses the existing elements of the desktop app; the owner picks a look later from mockups (User experience).
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Person, office, agent, desktop app, background server, office token | As defined in 0001. The person is the user of 0001. |
| Panel, office scene | As defined in 0003 and 0005. |
| Board | The one task board of an office. It has four columns and holds the cards. |
| Column | One of the four fixed lists of the board, in this order: To do, Doing, Blocked, Done. |
| Status | The column of a card: `todo`, `doing`, `blocked`, or `done`, shown as To do, Doing, Blocked, and Done. |
| Card | One task on the board, with a title, a note, and a status. |
| Place | The position of a card in its column, counted from 1 at the top. A card that enters a column takes the last place. The person cannot reorder cards inside a column in this change. |
| Board view | The desktop app view that shows the board (FR-001). |
| Window | One open desktop app window. Each window shows the board as the office keeps it. |
| Kept | The background server has stored the change, so it survives a restart. |
| Saw | A window saw a card when it last showed that card as the office keeps it. |
| Text rules of a title | After the white space at its ends, line breaks included, is removed: not blank, at most 200 characters, no control characters (Unicode general category Cc), and no invisible or direction-changing format characters (the zero-width characters and the bidi controls), as 0025/FR-001 refuses them in a name. |
| Text rules of a note | After each carriage return, with or without a line feed after it, is changed to one line feed: at most 8,192 characters (an empty note is allowed), and no control characters other than line feed and tab. |
| Card cap | 200 cards: the highest number of cards that the board holds. |

## Data flow and trust boundary

- **Boundary:** the person's desktop app connection, which presents the office token (0001/FR-007). The agent's connection (0002) gets no board request (PB-4).
- **Into the office:** from the person, a new card and a move. The office checks every field when it receives the change (FR-004).
- **Out of the office, to the person:** the board in every open window (FR-007).
- **Kept by the background server:** the board and every card, readable only by the OS user who owns the office (FR-006).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent reads or changes the board. | The agent's connection has no board request; any such request is refused and recorded (PB-4). |
  | A title or note carries control characters, escape codes, or invisible characters that change what the person sees. | The text rules refuse them when the office receives the change (FR-004); the board shows every text as plain text and applies nothing in it (FR-003). |
  | A text holds direction marks that reorder the text around it. | The direction of one text never changes the order of another text (FR-003). |
  | Two windows move one card at once, and one move is lost without a word. | A change to a card that changed after the window saw it is refused with the reason (FR-008). |
  | A move looks done in the window but the office did not keep it. | A change that is not kept puts the card back where it was and says why (FR-009). |
  | Other users of the machine read the card texts. | The board is kept readable only by the office owner (FR-006). |
  | A loop or a mistake fills the board until the office slows down. | The card cap bounds the board at 200 cards (FR-004). |

## User stories

### US1: The person puts tasks on the board and moves them through the columns (Priority: P1)

The person opens the board view from the panel. The board is empty, says so, and offers New card. The person makes the card "Fix the login test" in To do, then moves it to Doing, then to Done. After a restart of the server and of the machine, the board shows the same cards in the same columns. The person can do all of this with the keyboard alone, and a screen reader says where each moved card went.

**Why this priority:** this is the board itself. Without a card that moves through the four columns and stays after a restart, nothing else has value.

**Independent test:** in a new office, open the board view with the keyboard. It shows the four columns, each with 0 cards, and the message that the board has no cards. With the keyboard alone, make the card "Fix the login test", move it to Doing, then to Done. A screen reader says "Fix the login test, moved to Doing, 1 of 1" and then the same for Done. Restart the background server: the card is in Done with its title.

**Scenarios:**

1. Given the board has no cards, When the person opens the board view, Then it shows the four columns, each with its name, the count 0, and the text that it has no cards, and the board says that it has no cards and offers New card.
2. Given the person presses New card in a column, When the person types a title and saves, Then the card is kept at the end of that column and shows its title.
3. Given a card is in To do, When the person drops it on Doing with a pointer, Then it is kept in Doing at the last place.
4. Given a card has focus, When the person moves it to the next column with the keyboard, Then it is kept at the last place of that column, keeps focus, and a screen reader says its title, its new column, and its place as "n of m".
5. Given the board has cards, When the background server or the machine restarts, Then the board shows every card with the same fields in the same column and place.
6. Given the person types a title with a control character, When the person saves, Then the office keeps nothing, names the title field and its rule, and the form keeps the typed text.
7. Given a title of 200 characters, or a word of 200 characters with no space, When the board shows the card, Then the full title wraps inside the card and the board does not scroll sideways at 360 px width or at 200% zoom.

### US2: A change that the office does not keep never moves a card without a word (Priority: P1)

The person moves a card while the background server is not reachable. The card goes back to where it was, and the window says that the move was not kept and why. The person has the board open in two windows and moves the same card in both at once. One move is kept, the other window says that the card changed in another window, and both windows show the card in the same column.

**Why this priority:** a board that shows a move that did not happen gives the person a false plan. The person must be able to trust each column, or the board is worse than no board.

**Independent test:** stop the background server while the board view is open, and move a card from To do to Doing. Within 2 seconds the card is back in To do, the window says that the move was not kept because the server is not reachable, and the live region says the same. Start the server again. Open two windows and move one card to Doing in one and to Blocked in the other at the same moment: within 2 seconds both windows show the card in one column, and one window says that the card changed in another window.

**Scenarios:**

1. Given the background server is not reachable, When the person makes or moves a card, Then the office keeps nothing, the board shows the card as it was, and the window says which change was not kept and why.
2. Given the office cannot store a change, for example because the disk is full, When the person moves a card, Then the card goes back to where it was and the window says that the office could not store the change.
3. Given two windows show the board, When the person changes a card in one window, Then the other window shows the change within 2 seconds, with no reload.
4. Given two windows saw the same card, When both move it at the same moment, Then the office keeps the first move that it receives, refuses the second with the reason that the card changed in another window, and both windows show the kept column within 2 seconds.
5. Given the window got no answer for a new card, When the person saves the same form again, Then the board holds one card, not two.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep one board per office with the four columns To do, Doing, Blocked, and Done in this fixed order, and SHALL show it in one board view of the desktop app that the person opens from the panel and from the office scene, and leaves for the panel, with one action each, keyboard included.
- **FR-002** (US1): WHILE the board view waits for the board, THE SYSTEM SHALL say that it loads the board and show no column count; WHEN it has the board, THE SYSTEM SHALL show each column with its name and its number of cards, a column with no cards with the text that it has no cards, and, when the board has no cards, the text that the board has no cards and the offer New card.
- **FR-003** (US1): THE SYSTEM SHALL show each card in the column of its status, in the order of its place, with its title, and its full note in the card's details, exactly as kept, as plain text, without applying any control character, terminal escape code, or markup in it; SHALL keep the direction characters of one text from changing the order of any other text on the screen; and SHALL wrap a long title and a long word with no space inside the card so that the board view does not scroll sideways at 360 px width or at 200% zoom.
- **FR-004** (US1): WHEN the person saves a new card with New card in a column, THE SYSTEM SHALL keep it at the last place of the column of its status, which starts as the column where the person pressed New card, or To do, only IF its title meets the text rules of a title, its note meets the text rules of a note, and the board holds fewer cards than the card cap; IF a rule breaks, THEN THE SYSTEM SHALL keep nothing and name each failing field and its rule, or the card cap and its value.
- **FR-005** (US1): WHEN the person moves a card by pointer drop, or with the keyboard to the previous column, to the next column, or to any column from the card's action list, THE SYSTEM SHALL keep the new column as the card's status at the last place of that column, keep the places of the other cards in order, and keep focus on the moved card; IF a keyboard move would take a card past the first or last column, THEN THE SYSTEM SHALL change nothing and say that the card is already there.
- **FR-006** (US1): THE SYSTEM SHALL keep the board and every card with its fields, and the place of every card, across server and machine restarts, readable only by the OS user who owns the office, and SHALL let only a client that presents a valid office token (0001/FR-007) read or change the board.
- **FR-007** (US2): THE SYSTEM SHALL show each kept change in every open window within 2 seconds, with no reload.
- **FR-008** (US2): THE SYSTEM SHALL apply the changes to one board one at a time, in the order that the office receives them; IF a change is to a card that another window changed after the sending window saw that card, THEN THE SYSTEM SHALL refuse the change with the reason that the card changed in another window, and THE SYSTEM SHALL NOT count a move of another card as a change of this card.
- **FR-009** (US2): IF the office does not keep a new card or a move, because a rule of FR-004 refuses it, the card changed in another window (FR-008), or the office could not store it, or IF the window gets no answer from the background server within 2 seconds after the person made the change, THEN THE SYSTEM SHALL, within those 2 seconds, show the card of that change as it was before, in its column and place, and not at all for a new card; SHALL say in that window which change was not kept, or not confirmed, and why; SHALL keep the text that the person typed in an open form; SHALL change no other card; and, WHEN the window reaches the background server again, SHALL show the board as the office keeps it, so that saving the same new card form again keeps one card.
- **FR-010** (US1, US2): WHEN the office keeps or refuses a change that the person made in a window, or FR-009 reports it as not confirmed, THE SYSTEM SHALL announce it once to assistive technology in that window within 1 second: for a move, the card's title, its column, and its place as "n of m"; for a new card, the card's title and its column; for a refusal, the change and its reason; and SHALL NOT announce changes made in other windows.

### PRESERVED

- **PB-1:** The agent panel stays as before: the agent list shows the name, project, status, last output line, and task line of every agent (0003/FR-002), and the task line comes only from a received directive or the person's own edit (0011/FR-002, 0011/FR-003). A card or a move of it changes no task text and types nothing into any terminal.
- **PB-2:** The Needs-you queue stays as before: it adds, orders, and removes items only for the attention events of 0008 (0008/FR-001 to 0008/FR-005). A board change adds no Needs you item and no notification (0003/FR-006).
- **PB-3:** Scheduled directives stay as before: schedules, their runs, their holds and limits, and the schedules view (0025/FR-001 to 0025/FR-014) do not read or change the board, and a board change does not make, change, or run a schedule.
- **PB-4:** The agent's connection to the office gets no new request. An agent request to read or change the board is refused and recorded as every request outside the allowlist is (0002/FR-009, 0025/PB-5).
- **PB-5:** office-harness sends no office data, the board included, outside the user's machine (0001/FR-005).
- **PB-6:** Every desktop app action is available from the keyboard, the actions of this change included (0001/FR-028), and the column of a card shows as text, never color alone.
- **PB-7:** When the desktop app opens, it opens the panel or the office scene as the last view says (0003/FR-008). The board view is not a last view (A8).

## User experience

**Flow (the person puts a task on the board):**

1. The person opens the board view from the panel or the office scene.
2. The person presses New card on the board or in a column, types a title, and can add a note and a status. The status starts as the column where the person pressed New card, or To do.
3. The person saves. The card shows at the end of its column.

**Flow (the person moves a card):**

1. By pointer: the person drags the card to a column and drops it.
2. Without a pointer drag: the person gives the card focus and uses a key, or opens the card's action list and picks a column.
3. The card shows at the end of its new column; a screen reader says where it went.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Loading | The board view says that it loads the board; no column shows a count until the board arrives. | FR-002 |
| Empty board | The four columns, each with the count 0 and the text that it has no cards; the board says that it has no cards and offers New card. | FR-002, US1 scenario 1 |
| Empty column | The column's name, the count 0, and the text that it has no cards. | FR-002 |
| Board with cards | Each card in its column with its title as text. | FR-003 |
| Card details | The full title and the full note as plain text. | FR-003 |
| Form refused | The form names each failing field and its rule, or the card cap and its value; the typed text stays. | FR-004, FR-009 |
| Change not kept | The card is where it was before; the window says which change was not kept and why. | FR-009, US2 scenarios 1 and 2 |
| Changed in another window | The window shows the card as the office keeps it and says that the card changed in another window. | FR-008, US2 scenario 4 |
| Already last column | Nothing moves; the window and the live region say that the card is already there. | FR-005 |
| Server not reachable or not authorized | The 0001 states show before the board view (0001/FR-026, 0001/FR-027); a change made while the server is lost is not kept and says why. | FR-009, US2 scenario 1 |

**Design:** the board view is one new view of the desktop app, built from its existing list, form, and menu elements, and adds no new visual style. The look of the columns and cards is not fixed by this change; the owner picks one from mockups later. A look picked after approval is a spec change.

**Accessibility:**

- The person can do every action from the keyboard, with a visible focus: open the board view, New card, each move, and the action list. Escape closes the form and the action list and changes nothing.
- A drag is never the only way to move a card: the card's action list moves it with single actions (WCAG 2.2 success criterion 2.5.7).
- Each column is a labelled group with its name and its count. A card's accessible name is its title; its accessible description holds its column and its place as "n of m".
- Announcements follow FR-010: one per change made in that window, none for other windows, and no focus moves on its own.
- The column is text, never color or an icon alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text). At 360 px width and at 200% zoom the board view and the form reflow without sideways scrolling (FR-003).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Board | The one task board of an office. | the four columns in fixed order, the order of the cards in each column | One per office. Kept with the office (FR-006). |
| Card | One task on the board. | title, note (optional), status, place | Unique id. Belongs to the board; in exactly one column at exactly one place. The board holds at most 200 cards. |

## Success criteria

- **SC-001:** In a new office, the person opens the board view with 1 action from the panel and with 1 action from the office scene; the view shows 4 columns, each with the count 0 and the empty text, and the New card offer; while the board loads, 0 counts show.
- **SC-002:** With the keyboard alone, the person makes a card, moves it through all 4 columns, and sees it at the last place each time, with 0 pointer actions; a pointer drop of 5 of 5 cards lands each in the dropped column; a keyboard move past the last column changes 0 cards and says that the card is already there.
- **SC-003:** The live region holds the title, column, and place as "n of m" for 10 of 10 keyboard moves within 1 second and the reason for 3 of 3 refused moves, and a change made in a second window gives 0 announcements in the first. Screen readers on Linux do not receive live-region text from the platform web view (the 2026-11-28 update of the QA reports of 0003, 0005, and 0007); on that platform this criterion is verified by the live-region text in an automated test, and a screen reader check stays a residual that the QA report states.
- **SC-004:** With 200 cards (50 per column, each title 200 characters, each note 8,192 characters), the board view shows all 200 within 3 seconds after it opens, and each of 20 moves shows in a second window within 2 seconds.
- **SC-005:** 200 cards in 4 columns survive 5 server restarts and 1 machine restart with 100% of their fields, columns, and places unchanged.
- **SC-006:** With the background server stopped, 5 of 5 moves end with the card back in its old column and place within 2 seconds and a text that says the server is not reachable; after the server starts again, the board holds 0 of those moves.
- **SC-007:** In 20 trials in which two windows move the same card to two different columns at the same moment, 20 of 20 end with the card in exactly 1 column, the same in both windows within 2 seconds, and exactly 1 window says that the card changed in another window; 0 cards are lost or doubled.
- **SC-008:** A title of 0 characters, of 201 characters, of 200 characters with one control character, or of only zero-width characters, and a note of 8,193 characters or with an escape character, are refused 5 of 5 with the field and the rule named and 0 cards added; a title of 200 characters and a note of 8,192 characters with line feeds and tabs are kept and shown with 0 applied codes.
- **SC-009:** With 200 cards on the board, a new card is refused with the card cap and its value, and 0 cards are added.
- **SC-010:** After a new card gets no answer and the person saves the same form again, the board holds 1 card with that title in 5 of 5 trials.
- **SC-011:** The person makes and moves 10 cards: the agents' terminals receive 0 inputs, task texts change 0 times, and Needs you and the notifications get 0 new entries. The existing tests of 0003, 0008, and 0025 pass unchanged (PB-1 to PB-4, PB-7); an agent request for the board is refused and recorded 3 of 3 times (PB-4); the board view reflows at 360 px width and at 200% zoom with 0 sideways scrolling (FR-003).

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The board has no cards, or a column has none. | Empty | The empty texts and New card show (FR-002). |
| The title is blank, only spaces and line breaks, or only invisible characters. | Empty | Nothing is kept; the title field and its rule are named (FR-004). |
| The title is over 200 characters, or holds a control character, an invisible character, or an escape code. | Invalid | Nothing is kept; the field and its rule are named; the typed text stays in the form (FR-004, FR-009). |
| The note is over 8,192 characters, or holds a control character other than line feed and tab. | Invalid | Nothing is kept; the field and its rule are named (FR-004). |
| The note has Windows line ends. | Boundary | Each carriage return with its line feed counts and shows as one line feed (Terms). |
| A title of exactly 200 characters, or a word of 200 characters with no space. | Boundary | Kept; the full title wraps inside the card with no sideways scrolling (FR-003). |
| A text holds direction marks or characters from right-to-left scripts. | Boundary | Shown without changing the order of any other text (FR-003). |
| A text holds characters that look like markup or a link. | Invalid | Shown as plain text; nothing is applied or opened (FR-003). |
| The board holds 200 cards. | Boundary | A new card is refused with the card cap and its value (FR-004). |
| A keyboard move past the last column. | Boundary | Nothing changes; the window says that the card is already there (FR-005). |
| Two windows move the same card at once. | Concurrent | The first move that the office receives is kept; the other is refused with the reason; both windows show the kept column (FR-008). |
| Two windows move different cards into one column at once. | Concurrent | Both are kept, one after the other, in the order that the office receives them; every window shows the same order within 2 seconds; no card is lost or doubled (FR-007, FR-008). |
| The window gets no answer for a new card, and the person saves again. | Failure | The board holds one card with that title (FR-009). |
| The background server is not reachable, or stops during a change. | Failure | With no answer within 2 seconds, the card shows where it was before and the window says that the change was not confirmed and why; when the server is back, the window shows the board as the office keeps it (FR-009). |
| The office cannot store a change, for example because the disk is full. | Failure | The same as above, with the reason that the office could not store it (FR-009). |
| The background server stops without a clean stop. | Failure | At the next start, the board shows every kept change and none that was not kept (FR-006). |
| A client without a valid office token asks for the board. | Unauthorized | Refused; the client gets no card (FR-006, 0001/FR-007). |
| An agent asks the office to read or change the board. | Unauthorized | Refused and recorded (PB-4). |
| A column holds many cards. | Boundary | The column shows every card in its order, and scrolls only up and down (FR-002, FR-003). |

## Assumptions

- **A1:** Only the person makes and moves cards in this change, through the desktop app. Editing, delete, project, and assignee come in 0028; agent access comes in 0029.
- **A2:** A card has a title, an optional note, and a status that is one of the four columns.
- **A3:** The four columns are fixed in name, order, and count. A reason for a blocked card goes in the note.
- **A4:** A card that enters a column goes to the last place, by pointer drop too. The person cannot order cards inside a column in this change, so a move is predictable and two windows never disagree about a place.
- **A5:** Each window shows the board as the office keeps it. A change to a card is refused when the card changed after the window saw it; moving another card does not change this card.
- **A6:** The card cap of 200 is four times the 50 cards per column that SC-004 measures, and keeps a mistake from filling the board without end. It is fixed, not a setting.
- **A7:** The title limit of 200 characters keeps a card readable; the note limit of 8,192 characters matches the directive and task text limit (0011/FR-003). Characters are counted as Unicode code points, so one emoji can count as more than one character. A title refuses invisible format characters, so an emoji joined with a zero-width joiner is refused in a title.
- **A8:** The board view is not part of the last view of 0003/FR-008; the desktop app still opens the panel or the scene. This keeps 0003 unchanged.
- **A9:** The board shows every card of the office. A card has no project in this change; 0028 adds the project and makes the project switcher of the panel (0003/FR-005) filter the board.
- **A10:** The look of the board stays the existing plain style of the desktop app until the owner picks a look from mockups.
- **A11:** The time of a card's last change is not shown. A fixed or relative time is an addition for a later change.
- **A12:** Board changes add no entry to the action record, which holds the actions on agents (0001/FR-011).
- **A13:** The platform web view on Linux sends no screen reader event for live-region text, so SC-003 is verified on Linux by the live-region text in an automated test. The residual is stated in the QA report.

## Clarifications

### Session 2026-11-28 (owner decision)

- Q: What does the first version of the task board hold? → A: Only a plain board, one per office, with the four columns To do, Doing, Blocked, and Done. No cost or token view, no meeting room, no work across machines, and no scheduled temps. Enrichment is after the first release (Out of scope).
- Q: Does the spec fix the look of the board? → A: No. The spec gives behavior only; the owner picks a look from mockups later (A10).
- Q: Which features wait? → A: Undo of a delete, field-level merging of two windows' changes, ordering of cards inside a column, the time of the last change, and the project filter wait for a later change (the project filter then came in 0028 by owner decision).

## Out of scope

- Editing a card, a project, an assignee, and delete (0028).
- Board access for agents and the record of who changed a card (0029).
- A card that sends a directive, starts an agent, or sets an agent's task line (PB-1).
- A cost or token view per card, the meeting room (0004), work across machines (0006), and scheduled temps.
- More than one board per office, columns that the person names, adds, or removes, and limits per column.
- Ordering cards inside a column, Undo of a delete, field-level merging of two windows' changes, and the time of the last change (A4, A5, A11).
- Labels, due dates, priorities, sub-tasks, checklists, comments, attachments, and links between cards.
- A card history, an archive of Done cards, and a search of the board. The filter by the project switcher is in 0028.
- Import, export, and copying of the board.
- Notifications and Needs you items for board changes (PB-2).
- A visual design of the board beyond the existing elements of the desktop app (A10).
- Split: the first draft held about 35 single behaviors. This spec keeps 10 requirements, each with one set of rules for one part of the behavior; editing, project, assignee, and delete move to 0028, and agent access moves to 0029.

## Deviations

None yet.
