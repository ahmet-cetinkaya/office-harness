---
status: draft
capability: office
plan: specs/changes/0012-safe-directive-delivery/PLAN.md
supersedes: 0002/FR-010
---

<!-- Draft for review, 2026-11-23. Approval goes through the review gate under the owner's spec delegation. -->

# 0012: Safe directive delivery

## Intent

A parent agent sends a directive, and today the office types its text and Enter into the target's terminal at once, whenever the target runs. When the target's CLI shows a question for a person at that moment, such as "Allow this command?", the Enter, or a letter of the text, can answer it: the CLI then runs a tool call that nobody saw. With many agents, nobody notices. This change puts each directive in a queue per target and types it only when the target waits for input, no person is typing in that terminal, and the screen shows no open question. It then presses Enter only after the typed text shows at the target's input line. The sender learns whether its directive was submitted, waits, or was left unsubmitted, and a directive that cannot be typed in time is withdrawn and recorded.

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
| Unsubmitted | The office typed the directive but pressed no Enter, because the text did not show at the input line. |
| Person input | Input that a user's client sends to a terminal: keys, pastes, and answers. A directive is not person input. |
| Open question | A question on the screen that a key answers, such as a yes-or-no choice, a choice list, or a request for permission. |
| Withdrawn | Taken out of the queue without being typed. |

## User stories

### US1 — A directive never answers a question meant for a person (Priority: P1)

The office types a directive only when the target waits for input, no person is typing in its terminal, and its screen shows no open question. Until then the directive waits in the target's queue, in order. After typing, the office presses Enter only when the text shows at the target's input line, so a question the screen check did not recognise gets no Enter.

**Why this priority:** it removes the risk that motivates the change: an Enter from a directive can approve a tool call that nobody saw.

**Independent test:** start an agent whose screen shows a permission question and stays quiet. Send it a directive. Nothing is typed into its terminal. Answer the question by hand; once the agent waits for input again, the directive is typed, shows at the input line, and is submitted. Then start an agent that shows a question the check does not know and that does not echo input: the directive is typed, no Enter follows, and the record says it was left unsubmitted.

**Scenarios:**

1. Given the target waits for input, no person typed in its terminal for 30 seconds, and its screen shows no open question, When a directive arrives and no other directive waits for it, Then the office types it at once, sees the text at the input line, and presses Enter.
2. Given the target's screen shows a permission question, When a directive arrives, Then the office types nothing into that terminal, neither text nor Enter, and the directive waits.
3. Given the target is working, When a directive arrives, Then the directive waits until the target waits for input.
4. Given a person typed into the target's terminal 10 seconds ago, When a directive arrives, Then the directive waits until 30 seconds have passed since the person's last input.
5. Given three directives wait for one target, When the conditions hold, Then the office types them in the order it accepted them, each only after the target waits for input again.
6. Given the office typed a directive, When its text does not show at the input line within 2 seconds, Then the office presses no Enter, leaves the text in the terminal for the person to submit or clear, and records the directive as unsubmitted.

### US2 — The sender knows what happened to its directive (Priority: P1)

The sender's answer says whether the directive was submitted, waits (with its place in the queue), or was left unsubmitted. A waiting directive that cannot be typed is withdrawn with a reason, and the action record shows it.

**Why this priority:** a lead that believes a directive arrived, when it still waits or was withdrawn, plans on wrong facts. It ships with US1, because US1 makes waiting possible.

**Independent test:** send a directive to a working agent: the answer says it waits, first in the queue. Stop the agent: the sender's and the target's action records show the directive as withdrawn because the target stopped.

**Scenarios:**

1. Given the office submits a directive at once, When the sender reads the answer, Then it says the directive was submitted.
2. Given a directive must wait, When the sender reads the answer, Then it says the directive waits and gives its place in the queue.
3. Given the office typed a directive at once but its text did not show at the input line, When the sender reads the answer, Then it says the directive was left unsubmitted.
4. Given a directive waited 10 minutes without being typed, When the time passes, Then the office withdraws it and records the withdrawal with the reason in the sender's and the target's action records.
5. Given directives wait for an agent, When the agent stops, exits, or fails, Then the office withdraws them and records each withdrawal with the reason.
6. Given a directive waits, When the user puts a hold on the office, on the sender or one of its ancestors, or on the target, Then the office withdraws the directive instead of typing it and records the hold as the reason.
7. Given 20 directives wait for one target, When a 21st arrives, Then the office refuses it, says that the target's queue is full, and records the refusal.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one left the queue and the conditions of FR-002 hold again.
- **FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, and the bottom lines of the target's screen show no open question (a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **FR-003** (US1): WHEN the system has typed a directive, THE SYSTEM SHALL press Enter only IF the end of the directive's text appears at the input line of the target's screen within 2 seconds, and otherwise SHALL press no Enter, leave the typed text in the terminal, and record the directive as unsubmitted with the reason in the sender's and the target's action records.
- **FR-004** (US2): WHEN the system accepts a directive, THE SYSTEM SHALL answer the sender that the directive was submitted, that it waits with its place in the target's queue, or that it was left unsubmitted; IF 20 directives already wait for the target, THEN THE SYSTEM SHALL refuse the directive, say that the target's queue is full, and record the refusal.
- **FR-005** (US2): IF a waiting directive was not typed within 10 minutes, its target stopped running, the target is no longer the sender's descendant, or a hold would now refuse the directive (0007/FR-002, 0007/FR-004), THEN THE SYSTEM SHALL withdraw the directive without typing it and SHALL record the withdrawal with the directive text and the reason in the sender's and the target's action records.

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
4. If the text does not show at the input line, the office presses no Enter, answers or records "unsubmitted", and leaves the text in the terminal, where the person sees it and can submit or clear it.
5. If a waiting directive cannot be typed in time, or its target stops, or a hold or a tree change stops it, the office withdraws it and records why.

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
| Waiting directive | A directive the office accepted and has not typed yet. | sender, target, text, time accepted, place in the queue | Belongs to one target's queue until it is typed or withdrawn. Lives only while the background server runs (A4). |

## Success criteria

- **SC-001:** Of 50 directives sent to an agent whose screen shows a permission question, 0 characters are typed into its terminal while the question shows.
- **SC-002:** Of 20 directives sent to an agent that shows an unrecognised question and does not echo input, 0 are followed by Enter.
- **SC-003:** Of 20 directives sent while a person types into the target's terminal, 0 are typed earlier than 30 seconds after the person's last input.
- **SC-004:** 10 directives sent in a row to one working agent are each typed exactly once, 100% in the order they were sent.
- **SC-005:** 100% of withdrawn and unsubmitted directives have an entry with the text and the reason in the sender's and the target's action records.
- **SC-006:** A directive sent to an agent that waits for input and echoes input, with no open question and no person input in the last 30 seconds, is submitted within 2 seconds of sending.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The target is starting and its first screen is a trust question. | Boundary | The directive waits; the question check holds it while the question shows (FR-002). |
| A question appears while directives wait. | Concurrent | Nothing is typed while it shows (FR-002). |
| The person answers the question and the target works again. | Concurrent | The directive waits until the target waits for input and 30 seconds passed since the person's last input (FR-002). |
| A person types while a directive arrives. | Concurrent | The person's input reaches the terminal at once (PB-5); the directive waits (FR-002). This replaces the 0002 edge case "the directive arrives as one unbroken input" for that moment. |
| A person leaves a half-typed line for longer than 30 seconds. | Boundary | The directive is typed after it, and the Enter sends both (A5). |
| The screen shows a question shape that the check does not know, and the question does not echo typed text. | Boundary | The text is typed, no Enter follows, and the directive is recorded as unsubmitted (FR-003). |
| The CLI shows a long paste as a short placeholder instead of the text. | Boundary | The end of the text does not appear, so no Enter follows; the person can submit it (FR-003, A9). |
| The input line already ended with the same text before the office typed. | Boundary | That does not count as the text appearing; the office looks for a change (A9). |
| Output earlier on the screen looks like a question, but the CLI waits for input. | Boundary | The directive waits, and is withdrawn after 10 minutes with the reason (A3). |
| The target stops, exits, or fails with directives waiting. | Failure | Each is withdrawn and recorded (FR-005). |
| The sender is deleted, or the target is moved out of the sender's subtree, while a directive waits. | Concurrent | The directive is withdrawn and recorded (FR-005). |
| The user puts a hold on the office while directives wait. | Concurrent | Each is withdrawn when its turn comes and recorded with the hold as the reason (FR-005). |
| 20 directives already wait for one target. | Boundary | The next one is refused and recorded (FR-004). |
| The background server stops while directives wait. | Failure | They are lost; their acceptance stays in the record (A4). |
| The text breaks the directive rules, or the target is not a running descendant. | Invalid | Refused at sending, as before (PB-1). |
| An agent that is not the sender's ancestor asks about or changes the queue. | Unauthorized | Not possible: there is no request for the queue (A7). |

## Assumptions

- **A1:** "Waits for input" is the 0001 status, which follows a quiet period of the terminal (0001 A1). A CLI that shows a question is also quiet, so the status alone cannot keep a directive out of a question; the question check (FR-002) and the input-line check (FR-003) do that.
- **A2:** The question check knows the common shapes: a yes-or-no choice such as `(y/n)` or `[Y/n]`, a numbered choice list with a selection mark, and words that ask for permission or approval. A question drawn in another shape may not be recognised; the input-line check is the second guard.
- **A3:** A false match of the question check only delays a directive: it is withdrawn after 10 minutes with the reason, and the sender can send it again.
- **A4:** Waiting directives live in the background server's memory. A server stop drops them without a withdrawal entry; their acceptance stays in the target's record.
- **A5:** Any person input counts as typing for 30 seconds, also a key that answers a question or a paste. The office cannot see whether a person left a half-typed line, so after 30 seconds a directive is typed after it.
- **A6:** Rate limits (0007) count a directive when the office accepts it, not when it is typed. The hold check runs at sending and again before typing.
- **A7:** The queue has no view and no request of its own. The sender learns the place from the answer; the user withdraws waiting directives by stopping the target or by a hold (FR-005), and clears an unsubmitted text in the terminal like any typed text.
- **A8:** The 30-second typing pause, the 2-second wait for the input line, the 10-minute limit, and the 20-directive queue size are fixed in this change, not settings.
- **A9:** The input-line check compares the end of the text, without spaces and frame-drawing characters, with the input line before and after typing, so a line that wraps or sits in a frame still matches, and a line that already ended with the text does not.
- **A10:** The office types the text as one paste, marked as a paste when the CLI asked for that, so a CLI that tells pastes from keys reads it as text.

## Residual risk

What this change does not cover, stated plainly:

- **Letters into an unknown question.** When a question shape is not recognised (A2), the office types the text into it. A question that reads single keys, and does not treat a paste as text (A10), can take a letter of the text as an answer before the office can see anything. No Enter follows unless the text shows at the input line.
- **A free-text question.** A question that asks for typed text, such as "what should I do instead?", echoes the directive at its own input line and then receives the Enter. The directive then becomes the answer to that question.
- **The moment between check and typing.** The checks read the screen right before typing. A question that the CLI opens within that moment, while it was quiet for the whole quiet period, is not seen.
- **A server stop** loses waiting directives (A4).
- Closing these needs the CLI to report its own state, which waits on the owner's decision about an office settings file for the CLI.

## Clarifications

### Session 2026-11-23 (review guidance before the review)

- Q: Is the question check enough to keep an Enter out of a prompt? → A: No; a quiet CLI with an unrecognised prompt reads as waiting for input. The office types the text, confirms that it shows at the input line, and only then presses Enter; otherwise it holds the Enter (FR-003). The residual risk is stated in its own section.

## Out of scope

- A view of waiting directives, and withdrawing one directive by hand (A7).
- Checking what the CLI shows through CLI hooks or any office file that the CLI reads (Residual risk).
- Keeping waiting directives across a restart of the background server (A4).
- Detecting a half-typed line that a person left in the terminal (A5).
- Erasing an unsubmitted text from the terminal; the person submits or clears it (A7).
- Settings for the typing pause, the input-line wait, the time limit, or the queue size (A8).
- Holding back input that a person types or pastes (PB-5).
- Startup prompts, part of the office additions (0001/FR-022); they are pasted without Enter already.
