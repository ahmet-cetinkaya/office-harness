---
status: shipped
capability: office
plan: specs/changes/archive/0012-safe-directive-delivery/PLAN.md
supersedes: 0002/FR-010
---

<!-- Approved for implementation 2026-11-23 through the review gate (spec review r3 PASS at bb7d334, its one condition met at 4590311), under the owner's spec delegation; shipped in PR #115. -->

# 0012: Safe directive delivery

## Intent

A parent agent sends a directive, and today the office types its text and Enter into the target's terminal at once, whenever the target runs. When the target's CLI shows a question for a person at that moment, such as "Allow this command?", the Enter, or a letter of the text, can answer it: the CLI then runs a tool call that nobody saw. With many agents, nobody notices. This change puts each directive in a queue per target and types it only when the target waits for input, no person is typing in that terminal or has unsent text there, no text the office typed without Enter waits there for the person, and the screen shows no question that the office recognises. It then presses Enter only after the typed text, or the CLI's placeholder for it, shows at the target's input line, so a question of a shape the office does not know gets at most the typed text, never the Enter. The sender learns whether its directive was submitted, waits, or was left unsubmitted, and a directive that cannot be typed in time is withdrawn and recorded. What the screen checks cannot cover is stated under Residual risk.

## Context

- Source of truth: `docs/PRD.md`. Section 4.1 says office-harness writes nothing to the CLI's configuration; this change keeps that rule, because it reads only the screen copy that the office already keeps for each terminal.
- This change depends on 0001 (terminals, the agent status, the quiet period, the last output line, the action record), 0002 (directives and the descendant rule), 0007 (holds and rate limits), and 0011 (the task line). References such as `0002/FR-010` point to requirements in those changes.
- A full check of what the CLI shows would need the CLI to report its own state through hooks. That needs the CLI to read a settings file that the office provides, and the owner has not decided whether that counts as changing the CLI's configuration. This change does not depend on that decision; it reads the screen only. The residual risk is stated below.
- The idea of one serial delivery queue per terminal comes from an MIT-licensed reference office app; see the pull request. Only the behavior is taken, no code.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, terminal, status, waiting for input, quiet period, action record | As defined in 0001. |
| Directive, sender, target, descendant | As defined in 0002. The sender is the parent agent that sends the directive; the target is the agent that receives it. |
| Hold, rate limit | As defined in 0007. |
| Screen | The office's own copy of what the target's terminal shows, the same copy that gives the agent's last output line (0001). |
| Queue | The directives that the office accepted for one target and has not typed yet, oldest first. |
| Typed | The office entered the directive's text into the target's terminal, as one paste, without Enter. |
| Input line | The row of the screen where the cursor stands, together with the two rows above it, which a long line fills. |
| Submitted | The office typed the directive and then pressed Enter. |
| Unsubmitted | The office typed the directive but pressed no Enter, because neither the text nor a paste placeholder showed at the input line. |
| Person input | Input that a user's client sends to a terminal: keys, pastes, and answers. A directive is not person input. |
| Startup prompt | The text from the office additions that the office pastes, without Enter, after a session's first output, for the person to review (0001/FR-022). |
| Unsent person text | Text a person typed at the target's input line and has not sent with Enter or cleared, as the office reads it from the person's keys (A5). |
| Pending office text | Text the office typed into the target's current session without Enter, a startup prompt or an unsubmitted directive, from its typing until a person input reaches the terminal after it or the session ends. |
| Paste placeholder | A short bracketed text that some CLIs show at the input line instead of a long paste, such as `[Pasted text #1]`. |
| Open question | A question on the screen that a key answers, such as a yes-or-no choice, a choice list, or a request for permission. |
| Withdrawn | Taken out of the queue without being typed. |

## User stories

### US1 — A directive does not press Enter into a question meant for a person (Priority: P1)

The office types a directive only when the target waits for input, no person is typing in its terminal or has unsent text there, no pending office text waits there, and its screen shows no open question. Until then the directive waits in the target's queue, in order. After typing, the office presses Enter only when the text, or the CLI's paste placeholder, shows at the target's input line, so a question the screen check did not recognise gets no Enter.

**Why this priority:** it removes the risk that motivates the change: an Enter from a directive can approve a tool call that nobody saw.

**Independent test:** start an agent whose screen shows a permission question and stays quiet. Send it a directive. Nothing is typed into its terminal. Answer the question by hand; once the agent waits for input again, the directive is typed, shows at the input line, and is submitted. Then start an agent that shows a question the check does not know and that does not echo input: the directive is typed, no Enter follows, and the record says it was left unsubmitted.

**Scenarios:**

1. Given the target waits for input, no person typed in its terminal for 30 seconds, and its screen shows no open question, When a directive arrives and no other directive waits for it, Then the office types it at once, sees the text at the input line, and presses Enter.
2. Given the target's screen shows a permission question, When a directive arrives, Then the office types nothing into that terminal, neither text nor Enter, and the directive waits.
3. Given the target is working, When a directive arrives, Then the directive waits until the target waits for input.
4. Given a person typed into the target's terminal 10 seconds ago, When a directive arrives, Then the directive waits until 30 seconds have passed since the person's last input.
5. Given three directives wait for one target, When the conditions hold, Then the office types them in the order it accepted them, each only after the target waits for input again.
6. Given the office typed a directive, When neither its text nor a paste placeholder shows at the input line within 2 seconds, Then the office presses no Enter, leaves the text in the terminal for the person to submit or clear, and records the directive as unsubmitted.
7. Given the office pasted a startup prompt into the target's session and no person input reached the terminal since, When a directive arrives, Then the directive waits, so its Enter cannot submit the startup prompt.
8. Given a directive was left unsubmitted and no person input reached the terminal since, When the next directive's turn comes, Then it waits, so its Enter cannot submit both texts.
9. Given a person typed part of a line into the target's terminal and has not sent or cleared it, When a directive arrives, Then the directive waits, also after 30 seconds, until the person sends or clears the line.

### US2 — The sender knows what happened to its directive (Priority: P1)

The sender's answer says whether the directive was submitted, waits (with its place in the queue), or was left unsubmitted. A waiting directive that cannot be typed is withdrawn with a reason, and the action record shows it.

**Why this priority:** a lead that believes a directive arrived, when it still waits or was withdrawn, plans on wrong facts. It ships with US1, because US1 makes waiting possible.

**Independent test:** send a directive to a working agent: the answer says it waits, first in the queue. Stop the agent: the sender's and the target's action records show the directive as withdrawn because the target stopped.

**Scenarios:**

1. Given the office submits a directive at once, When the sender reads the answer, Then it says the directive was submitted.
2. Given a directive must wait, When the sender reads the answer, Then it says the directive waits and gives its place in the queue.
3. Given the office typed a directive at once but neither its text nor a paste placeholder showed at the input line, When the sender reads the answer, Then it says the directive was left unsubmitted.
4. Given a directive waited 10 minutes without being typed, When the time passes, Then the office withdraws it and records the withdrawal with the reason in the sender's and the target's action records.
5. Given directives wait, When the background server stops normally, Then the office withdraws them and records each withdrawal with the reason that the office server stopped.
6. Given directives wait for an agent, When the agent stops, exits, or fails, Then the office withdraws them and records each withdrawal with the reason.
7. Given a directive waits, When the user puts a hold on the office, on the sender or one of its ancestors, or on the target, Then the office withdraws the directive instead of typing it and records the hold as the reason.
8. Given 20 directives wait for one target, When a 21st arrives, Then the office refuses it, says that the target's queue is full, and records the refusal; the refusal is not a rate-limit refusal (A6).

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one left the queue and the conditions of FR-002 hold again.
- **FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, no unsent person text waits at its input line, no pending office text (a startup prompt or an unsubmitted directive) waits in the target's current session, and the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **FR-003** (US1): WHEN the system has typed a directive, THE SYSTEM SHALL press Enter only IF the end of the directive's text, or a new paste placeholder, appears at the input line of the target's screen within 2 seconds, and otherwise SHALL press no Enter, leave the typed text in the terminal, and record the directive as unsubmitted with the reason in the sender's and the target's action records.
- **FR-004** (US2): WHEN the system accepts a directive, THE SYSTEM SHALL answer the sender that the directive was submitted, that it waits with its place in the target's queue, or that it was left unsubmitted; IF 20 directives already wait for the target, THEN THE SYSTEM SHALL refuse the directive, say that the target's queue is full, and record the refusal.
- **FR-005** (US2): IF a waiting directive was not typed within 10 minutes, its target stopped running, the target is no longer the sender's descendant, a hold would now refuse the directive (0007/FR-002, 0007/FR-004), or the background server stops normally, THEN THE SYSTEM SHALL withdraw the directive without typing it and SHALL record the withdrawal with the directive text and the reason in the sender's and the target's action records.

### MODIFIED

- **0002/FR-010** (US2): WHEN a parent agent sends a directive, THE SYSTEM SHALL enter the directive text into the target agent's terminal as one unbroken input, at the time that 0012/FR-001 and 0012/FR-002 allow, followed by Enter when 0012/FR-003 allows it.

### PRESERVED

- **PB-1:** The checks at sending stay as they are, with their refusals and records: the target must be a running descendant, the text must follow the directive rules (0002/FR-009, 0002/FR-011), and holds and rate limits apply (0007).
- **PB-2:** An accepted directive is recorded once in the target's action record, with the sender as the actor (0002/FR-011). A directive that waits is accepted.
- **PB-3:** The task line shows the text of the last directive that was submitted (0011/FR-002). A directive that waits, was withdrawn, or was left unsubmitted does not change it.
- **PB-4:** The office writes nothing into a CLI's configuration (0001/FR-021). Both screen checks read only the screen copy that the office keeps.
- **PB-5:** Person input reaches the terminal at once and unchanged (0001); the queue never holds it back.

## User experience

**Flow:**

1. A parent agent sends a directive through the office.
2. The office checks it as before (PB-1). If the target waits for input, nobody types there, no question shows, and nothing waits before it, the office types the text, waits until it shows at the input line, presses Enter, and answers "submitted".
3. Otherwise the office answers "waiting" with the place in the queue, and types the directive as soon as the conditions hold.
4. If neither the text nor a paste placeholder shows at the input line, the office presses no Enter, answers or records "unsubmitted", and leaves the text in the terminal, where the person sees it and can submit or clear it.
5. If a waiting directive cannot be typed in time, or its target stops, or a hold, a tree change, or a server stop stops it, the office withdraws it and records why.

**Visible states:**

| State | What the sender or the user sees | Covered by |
|---|---|---|
| Submitted | The sender's answer says submitted; the target's task line shows the text. | FR-004, PB-3 |
| Waiting | The sender's answer says waiting, with the place in the queue. | FR-004 |
| Unsubmitted | The sender's answer or the action records say unsubmitted, with the reason; the text stays visible in the target's terminal. | FR-003, FR-004 |
| Queue full | The sender's answer refuses with the reason; the record has the refusal. | FR-004 |
| Withdrawn | The sender's and the target's action records show the text and the reason. | FR-005 |

**Design:** no new screen. The sender reads the answer of its request; the user reads the action record, the task line, and the terminal that exist today.

**Accessibility:** no new control. The action record, the task line, and the terminal keep their 0001, 0003, and 0011 accessibility.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Waiting directive | A directive the office accepted and has not typed yet. | sender, target, text, time accepted, place in the queue | Belongs to one target's queue until it is typed or withdrawn. Lives only while the background server runs; a normal stop withdraws it (FR-005, A4). |

## Success criteria

- **SC-001:** Of 50 directives sent to an agent whose screen shows a permission question, 0 characters are typed into its terminal while the question shows.
- **SC-002:** Of 20 directives sent to an agent that shows an unrecognised question and does not echo input, 0 are followed by Enter.
- **SC-003:** Of 20 directives sent while a person types into the target's terminal, 0 are typed earlier than 30 seconds after the person's last input.
- **SC-004:** 10 directives sent in a row to one working agent are each typed exactly once, 100% in the order they were sent.
- **SC-005:** 100% of withdrawn and unsubmitted directives have an entry with the text and the reason in the sender's and the target's action records.
- **SC-006:** A directive sent to an agent that waits for input and echoes input, with no open question and no person input in the last 30 seconds, is submitted within 2 seconds of sending.
- **SC-007:** Of 10 directives sent to an agent with pending office text, a pasted startup prompt or an unsubmitted directive, 0 are typed before a person input reaches that terminal.
- **SC-008:** Of 10 directives sent while a person's half-typed line waits unsent, 0 are typed until the person sends or clears the line.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The target is starting and its first screen is a trust question. | Boundary | The directive waits; the question check holds it while the question shows (FR-002). |
| A question appears while directives wait. | Concurrent | Nothing is typed while it shows (FR-002). |
| The person answers the question and the target works again. | Concurrent | The directive waits until the target waits for input and 30 seconds passed since the person's last input (FR-002). |
| A person types while a directive arrives. | Concurrent | The person's input reaches the terminal at once (PB-5); the directive waits (FR-002). This replaces the 0002 edge case "the directive arrives as one unbroken input" for that moment. |
| A person leaves a half-typed line for longer than 30 seconds. | Boundary | Directives wait until the person sends or clears the line (FR-002, A5); a directive that waits 10 minutes is withdrawn (FR-005). |
| A person recalls an earlier line with an arrow key, or erases a word. | Boundary | The office cannot follow that edit, so it holds directives until the person sends the line or clears it with a line-clearing key (A5). |
| The screen shows a question shape that the check does not know, and the question does not echo typed text. | Boundary | The text is typed, no Enter follows, and the directive is recorded as unsubmitted (FR-003). |
| The CLI shows a long paste as a short placeholder instead of the text. | Boundary | A new placeholder at the input line counts like the text, and Enter follows (FR-003, A9). |
| The office pasted a startup prompt and the person has not touched the terminal since. | Concurrent | Directives wait until a person input reaches the terminal (FR-002), so a directive's Enter never submits the startup prompt. |
| A directive was left unsubmitted and more directives wait. | Concurrent | They wait until a person input reaches the terminal (FR-002), so no Enter submits the unsubmitted text with the next one. |
| The input line already ended with the same text before the office typed. | Boundary | That does not count as the text appearing; the office looks for a change (A9). |
| Output earlier on the screen looks like a question, but the CLI waits for input. | Boundary | The directive waits, and is withdrawn after 10 minutes with the reason (A3). |
| The target stops, exits, or fails with directives waiting. | Failure | Each is withdrawn and recorded (FR-005). |
| The sender is deleted, or the target is moved out of the sender's subtree, while a directive waits. | Concurrent | The directive is withdrawn and recorded (FR-005). |
| The user puts a hold on the office while directives wait. | Concurrent | Each is withdrawn when its turn comes and recorded with the hold as the reason (FR-005). |
| 20 directives already wait for one target. | Boundary | The next one is refused and recorded (FR-004). |
| The background server stops normally while directives wait. | Failure | Each is withdrawn and recorded with the reason that the office server stopped (FR-005). |
| A directive arrives after a normal stop of the server has begun. | Failure | The office refuses it with the reason that the office server is stopping, and records the refusal (FR-004, FR-005, A4). |
| The background server crashes while directives wait. | Failure | They are lost; their acceptance stays in the record (A4). |
| The text breaks the directive rules, or the target is not a running descendant. | Invalid | Refused at sending, as before (PB-1). |
| An agent that is not the sender's ancestor asks about or changes the queue. | Unauthorized | Not possible: there is no request for the queue (A7). |

## Assumptions

- **A1:** "Waits for input" is the 0001 status, which follows a quiet period of the terminal (0001 A1). A CLI that shows a question is also quiet, so the status alone cannot keep a directive out of a question; the question check (FR-002) and the input-line check (FR-003) do that.
- **A2:** The question check knows the common shapes: a yes-or-no choice such as `(y/n)` or `[Y/n]`, a numbered choice list with a selection mark, and words that ask for permission or approval. A question drawn in another shape may not be recognised; the input-line check is the second guard.
- **A3:** A false match of the question check only delays a directive: it is withdrawn after 10 minutes with the reason, and the sender can send it again.
- **A4:** Waiting directives live in the background server's memory. A normal stop withdraws and records them (FR-005); a crash drops them without a withdrawal entry, and their acceptance stays in the target's record.
- **A5:** Any person input counts as typing for 30 seconds, also a key that answers a question or a paste. Besides that pause, the office reads unsent person text from the person's keys: typed characters add to it, an erase key takes one off, and Enter outside a paste or a line-clearing key (Ctrl+C, Ctrl+U) empties it. A key whose effect it cannot follow (an arrow, a lone Escape, a word erase) keeps the line held until Enter or a line-clearing key. While unsent person text waits, directives wait with no time limit of their own; the office never sends Escape and never clears the line. A new session starts with an empty line. Decided by god and Efe under the owner's spec delegation, 2026-11-23; before, a half-typed line held directives for 30 seconds only.
- **A6:** Rate limits (0007) count a directive when the office accepts it, not when it is typed. A queue-full refusal is not a rate-limit refusal: it does not count toward a limit and not toward the automatic hold (0007/FR-008). The hold check runs at sending and again before typing.
- **A7:** The queue has no view and no request of its own. The sender learns the place from the answer; the user withdraws waiting directives by stopping the target or by a hold (FR-005), and clears an unsubmitted text in the terminal like any typed text; that input also lets the queue go on (A11).
- **A8:** The 30-second typing pause, the 2-second wait for the input line, the 10-minute limit, and the 20-directive queue size are fixed in this change, not settings.
- **A9:** The input-line check compares the end of the text, without spaces and frame-drawing characters, with the input line before and after typing, so a line that wraps or sits in a frame still matches, and a line that already ended with the text does not. A paste placeholder counts when the input line shows one more of them after typing than before.
- **A10:** The office types the text as one paste, marked as a paste when the CLI asked for that, so a CLI that tells pastes from keys reads it as text.
- **A11:** Pending office text waits for the person from its typing until the first person input that reaches the terminal after it, or until its session ends; a new session starts with none. The office cannot see whether the person submitted or cleared it; any person input ends the wait, and the 30-second typing pause (FR-002) follows. Waiting directives keep counting toward the 10-minute limit meanwhile (FR-005).

## Residual risk

What this change does not cover, stated plainly:

- **Letters into an unknown question.** When a question shape is not recognised (A2), the office types the text into it. A question that reads single keys, in a CLI that did not ask for marked pastes (A10), can take a letter of the text as an answer before the office can see anything. No Enter follows unless the text or a paste placeholder shows at the input line.
- **A free-text question.** A question that asks for typed text, such as "what should I do instead?", echoes the directive at its own input line and then receives the Enter. The directive then becomes the answer to that question.
- **Line edits the keys do not show.** The unsent-text count reads the person's keys only. A CLI that fills the line by itself, for example with a suggestion, is not seen; a key the count cannot follow holds the line until Enter or a line-clearing key (A5).
- **The moment between check and typing.** The checks read the screen right before typing. A question that the CLI opens within that moment, while it was quiet for the whole quiet period, is not seen.
- **A crash of the background server** loses waiting directives without a record (A4).
- Closing these needs the CLI to report its own state, which waits on the owner's decision about an office settings file for the CLI.

## Clarifications

### Session 2026-11-23 (review guidance before the review)

- Q: Is the question check enough to keep an Enter out of a prompt? → A: No; a quiet CLI with an unrecognised prompt reads as waiting for input. The office types the text, confirms that it shows at the input line, and only then presses Enter; otherwise it holds the Enter (FR-003). The residual risk is stated in its own section.

### Session 2026-11-23 (review r1)

- Q: Can a directive's Enter submit a startup prompt the person has not reviewed? → A: No; a pasted startup prompt holds directives until a person input reaches the terminal after it (FR-002, A11).
- Q: Does a CLI's paste placeholder count as the text showing? → A: Yes, a new placeholder at the input line counts (FR-003, A9).
- Q: Is a normal server stop recorded? → A: Yes, as a withdrawal with the reason that the office server stopped; only a crash stays unrecorded (FR-005, A4).
- Q: Does a queue-full refusal count toward the automatic hold? → A: No (A6).

### Session 2026-11-23 (decision after review r4)

- Q: May a directive be typed after a person's half-typed line once 30 seconds have passed? → A: No; delivery holds until that text is sent or cleared, and the office never sends Escape or clears the line (A5, FR-002). Decided by god and Efe under the owner's spec delegation.

### Session 2026-11-23 (review r2)

- Q: After a directive is left unsubmitted, can the next one's Enter submit both? → A: No; any text the office typed without Enter, a startup prompt or an unsubmitted directive, holds the queue until a person input reaches the terminal after it or the session ends (FR-002, A11).

## Out of scope

- A view of waiting directives, and withdrawing one directive by hand (A7).
- Checking what the CLI shows through CLI hooks or any office file that the CLI reads (Residual risk).
- Keeping waiting directives across a restart of the background server (A4).
- Reading a person's half-typed line from the screen; the office counts the person's keys instead (A5).
- Erasing an unsubmitted text from the terminal; the person submits or clears it (A7).
- Settings for the typing pause, the input-line wait, the time limit, or the queue size (A8).
- Holding back input that a person types or pastes (PB-5).
- Changing how a startup prompt is pasted (0001/FR-022); this change only holds directives back while one waits for the person (FR-002, A11).

## Deviations

Implementation notes. None of them defers or changes a requirement. The last note records a gap found after the change shipped and closed by #127; the requirement stays as written.

- **One delivery per target, not one write lock for all.** The plan kept the handler's single write lock. A delivery can wait up to 2 seconds for the input line, so the handler now marks one delivery per target instead; directives to other targets go on meanwhile, and writes and receipts of one target keep their order (0011 D2).
- **Readiness has three answers.** The plan named the reasons a target is busy. The handler needs only ready, busy, or gone, so the reasons are not carried.
- **Input-line check by count.** The plan compared the text's end with the end of the line before typing. The check counts how often the text's end, and a paste placeholder, show at the input line before and after typing, so a line that already held the same text elsewhere does not count (A9).
- **Commits.** The plan named one commit per task. The hierarchy change and the server change do not build apart, so T2 to T4 code and tests are one commit, and the documents are another.
- **Found after ship (2026-11-23): FR-002 is not met for an unnumbered choice list.** In a hands-on run, the start question of `claude` in a new folder, a choice list with a cursor mark and no numbers, did not hold a directive; the office typed it after the quiet period. FR-003 held: no Enter was sent and the question stayed unanswered. #127 fixed the question check on 2026-11-23; the same hands-on run then held the directive (qa-report, Known gaps).


## Updates

- 2026-11-23: a hands-on run showed that the question check missed a choice list whose entries have no numbers, as in a trust question at start. FR-002 already names "a choice list with a selection mark"; the check now reads such a list also without numbers, when it has 2 or more entries and ends the screen. No requirement changed.
