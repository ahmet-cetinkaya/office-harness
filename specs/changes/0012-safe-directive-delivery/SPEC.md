---
status: draft
capability: office
plan: specs/changes/0012-safe-directive-delivery/PLAN.md
supersedes: 0002/FR-010
---

<!-- Draft for review, 2026-11-23. Approval goes through the review gate under the owner's spec delegation. -->

# 0012: Safe directive delivery

## Intent

A parent agent sends a directive, and today the office types its text and Enter into the target's terminal at once, whenever the target runs. When the target's CLI shows a question for a person at that moment, such as "Allow this command?", the Enter, or a letter of the text, can answer it: the CLI then runs a tool call that nobody saw. With many agents, nobody notices. This change puts each directive in a queue per target and types it only when the target waits for input, no person is typing in that terminal, and the screen shows no open question. The sender learns whether its directive was typed or waits, and a directive that cannot be typed in time is withdrawn and recorded.

## Context

- Source of truth: `docs/PRD.md`. Section 4.1 says office-harness writes nothing to the CLI's configuration; this change keeps that rule, because it reads only the screen copy that the office already keeps for each terminal.
- This change depends on 0001 (terminals, the agent status, the quiet period, the action record), 0002 (directives and the descendant rule), 0007 (holds and rate limits), and 0011 (the task line). References such as `0002/FR-010` point to requirements in those changes.
- A full check of what the CLI shows would need the CLI to report its own state through hooks. That needs the CLI to read a settings file that the office provides, and the owner has not decided whether that counts as changing the CLI's configuration. This change does not depend on that decision; it reads the screen only.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, terminal, status, waiting for input, quiet period, action record | As defined in 0001. |
| Directive, sender, target, descendant | As defined in 0002. The sender is the parent agent that sends the directive; the target is the agent that receives it. |
| Hold, rate limit | As defined in 0007. |
| Queue | The directives that the office accepted for one target and has not typed yet, oldest first. |
| Typed | The office entered the directive's text and Enter into the target's terminal. |
| Person input | Input that a user's client sends to a terminal: keys, pastes, and answers. A directive is not person input. |
| Open question | A question on the target's screen that a key answers, such as a yes-or-no choice, a choice list, or a request for permission. |
| Withdrawn | Taken out of the queue without being typed. |

## User stories

### US1 — A directive never answers a question meant for a person (Priority: P1)

The office types a directive only when the target waits for input, no person is typing in its terminal, and its screen shows no open question. Until then the directive waits in the target's queue, in order.

**Why this priority:** it removes the risk that motivates the change: an Enter from a directive can approve a tool call that nobody saw.

**Independent test:** start an agent whose screen shows a permission question and stays quiet. Send it a directive. Nothing is typed into its terminal. Answer the question by hand; once the agent waits for input again, the directive is typed.

**Scenarios:**

1. Given the target waits for input, no person typed in its terminal for 30 seconds, and its screen shows no open question, When a directive arrives and no other directive waits for it, Then the office types it at once.
2. Given the target's screen shows a permission question, When a directive arrives, Then the office types nothing into that terminal, neither text nor Enter, and the directive waits.
3. Given the target is working, When a directive arrives, Then the directive waits until the target waits for input.
4. Given a person typed into the target's terminal 10 seconds ago, When a directive arrives, Then the directive waits until 30 seconds have passed since the person's last input.
5. Given three directives wait for one target, When the conditions hold, Then the office types them in the order it accepted them, each only after the target waits for input again.

### US2 — The sender knows what happened to its directive (Priority: P1)

The sender's answer says whether the directive was typed or waits, and where it stands in the queue. A waiting directive that cannot be typed is withdrawn with a reason, and the action record shows it.

**Why this priority:** a lead that believes a directive arrived, when it still waits or was withdrawn, plans on wrong facts. It ships with US1, because US1 makes waiting possible.

**Independent test:** send a directive to a working agent: the answer says it waits, first in the queue. Stop the agent: the sender's and the target's action records show the directive as withdrawn because the target stopped.

**Scenarios:**

1. Given the office types a directive at once, When the sender reads the answer, Then it says the directive was typed.
2. Given a directive must wait, When the sender reads the answer, Then it says the directive waits and gives its place in the queue.
3. Given a directive waited 10 minutes without being typed, When the time passes, Then the office withdraws it and records the withdrawal with the reason in the sender's and the target's action records.
4. Given directives wait for an agent, When the agent stops, exits, or fails, Then the office withdraws them and records each withdrawal with the reason.
5. Given a directive waits, When the user puts a hold on the office, on the sender or one of its ancestors, or on the target, Then the office withdraws the directive instead of typing it and records the hold as the reason.
6. Given 20 directives wait for one target, When a 21st arrives, Then the office refuses it, says that the target's queue is full, and records the refusal.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep one queue of directives per target and SHALL type the directives of a queue one at a time, in the order it accepted them, each only after the previous one was typed and the conditions of FR-002 hold again.
- **FR-002** (US1): THE SYSTEM SHALL type a directive only WHILE the target waits for input, no person input reached the target's terminal in the last 30 seconds, and the target's screen shows no open question (FR-003); otherwise THE SYSTEM SHALL keep the directive in the queue.
- **FR-003** (US1): THE SYSTEM SHALL treat the target's screen as showing an open question WHEN the bottom lines of the screen show a yes-or-no choice, a choice list with a selection mark, or a request for permission or approval, and SHALL then type nothing into that terminal, neither the text nor Enter.
- **FR-004** (US2): WHEN the system accepts a directive, THE SYSTEM SHALL answer the sender either that the directive was typed or that it waits, with its place in the target's queue; IF 20 directives already wait for the target, THEN THE SYSTEM SHALL refuse the directive, say that the target's queue is full, and record the refusal.
- **FR-005** (US2): IF a waiting directive was not typed within 10 minutes, its target stopped running, the target is no longer the sender's descendant, or a hold would now refuse the directive (0007/FR-002, 0007/FR-004), THEN THE SYSTEM SHALL withdraw the directive without typing it and SHALL record the withdrawal with the directive text and the reason in the sender's and the target's action records.

### MODIFIED

- **0002/FR-010** (US2): WHEN a parent agent sends a directive, THE SYSTEM SHALL enter the directive text into the target agent's terminal as one unbroken input followed by Enter, at the time that 0012/FR-001 and 0012/FR-002 allow.

### PRESERVED

- **PB-1:** The checks at sending stay as they are, with their refusals and records: the target must be a running descendant, the text must follow the directive rules (0002/FR-009, 0002/FR-011), and holds and rate limits apply (0007).
- **PB-2:** An accepted directive is recorded once in the target's action record, with the sender as the actor (0002/FR-011). A directive that waits is accepted.
- **PB-3:** The task line shows the text of the last directive that was typed into the terminal (0011/FR-002). A directive that waits or was withdrawn does not change it.
- **PB-4:** The office writes nothing into a CLI's configuration (0001/FR-021). The question check reads only the screen copy that the office keeps.
- **PB-5:** Person input reaches the terminal at once and unchanged (0001); the queue never holds it back.

## User experience

**Flow:**

1. A parent agent sends a directive through the office.
2. The office checks it as before (PB-1). If the target waits for input, nobody types there, and no question shows, the office types it and answers "typed".
3. Otherwise the office answers "waiting" with the place in the queue, and types the directive as soon as the conditions hold.
4. If the directive cannot be typed in time, or its target stops, or a hold or a tree change stops it, the office withdraws it and records why.

**Visible states:**

| State | What the sender or the user sees | Covered by |
|---|---|---|
| Typed | The sender's answer says typed; the target's task line shows the text. | FR-004, PB-3 |
| Waiting | The sender's answer says waiting, with the place in the queue. | FR-004 |
| Queue full | The sender's answer refuses with the reason; the record has the refusal. | FR-004 |
| Withdrawn | The sender's and the target's action records show the text and the reason. | FR-005 |

**Design:** no new screen. The sender reads the answer of its request; the user reads the action record and the task line that exist today.

**Accessibility:** no new control. The action record and the task line keep their 0001, 0003, and 0011 accessibility.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Waiting directive | A directive the office accepted and has not typed yet. | sender, target, text, time accepted, place in the queue | Belongs to one target's queue until it is typed or withdrawn. Lives only while the background server runs (A4). |

## Success criteria

- **SC-001:** Of 50 directives sent to an agent whose screen shows a permission question, 0 characters are typed into its terminal while the question shows.
- **SC-002:** Of 20 directives sent while a person types into the target's terminal, 0 are typed earlier than 30 seconds after the person's last input.
- **SC-003:** 10 directives sent in a row to one working agent are each typed exactly once, 100% in the order they were sent.
- **SC-004:** 100% of withdrawn directives have an entry with the text and the reason in the sender's and the target's action records.
- **SC-005:** A directive sent to an agent that waits for input, with no open question and no person input in the last 30 seconds, is typed within 1 second of sending.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The target is starting and its first screen is a trust question. | Boundary | The directive waits; the question check holds it while the question shows (FR-003). |
| A question appears while directives wait. | Concurrent | Nothing is typed while it shows (FR-003). |
| The person answers the question and the target works again. | Concurrent | The directive waits until the target waits for input and 30 seconds passed since the person's last input (FR-002). |
| A person types while a directive arrives. | Concurrent | The person's input reaches the terminal at once (PB-5); the directive waits (FR-002). This replaces the 0002 edge case "the directive arrives as one unbroken input" for that moment. |
| A person leaves a half-typed line for longer than 30 seconds. | Boundary | The directive is typed after it, and the Enter sends both (A5). |
| The screen shows a question shape that the check does not know. | Boundary | The directive may be typed into it (A2). |
| Output earlier on the screen looks like a question, but the CLI waits for input. | Boundary | The directive waits, and is withdrawn after 10 minutes with the reason (A3). |
| The target stops, exits, or fails with directives waiting. | Failure | Each is withdrawn and recorded (FR-005). |
| The sender is deleted, or the target is moved out of the sender's subtree, while a directive waits. | Concurrent | The directive is withdrawn and recorded (FR-005). |
| The user puts a hold on the office while directives wait. | Concurrent | Each is withdrawn when its turn comes and recorded with the hold as the reason (FR-005). |
| 20 directives already wait for one target. | Boundary | The next one is refused and recorded (FR-004). |
| The background server stops while directives wait. | Failure | They are lost; their acceptance stays in the record (A4). |
| The text breaks the directive rules, or the target is not a running descendant. | Invalid | Refused at sending, as before (PB-1). |
| An agent that is not the sender's ancestor asks about or changes the queue. | Unauthorized | Not possible: there is no request for the queue (A7). |

## Assumptions

- **A1:** "Waits for input" is the 0001 status, which follows a quiet period of the terminal (0001 A1). A CLI that shows a question is also quiet, so the status alone cannot keep a directive out of a question; the question check (FR-003) does that.
- **A2:** The question check reads the office's copy of the screen and knows the common shapes: a yes-or-no choice such as `(y/n)` or `[Y/n]`, a numbered choice list with a selection mark, and words that ask for permission or approval. A question drawn in another shape may not be recognised. A full check through the CLI's own hooks is out of scope; it waits on the owner's decision about an office settings file for the CLI.
- **A3:** A false match only delays a directive: it is withdrawn after 10 minutes with the reason, and the sender can send it again.
- **A4:** Waiting directives live in the background server's memory. A server stop drops them without a withdrawal entry; their acceptance stays in the target's record.
- **A5:** Any person input counts as typing for 30 seconds, also a key that answers a question or a paste. The office cannot see whether a person left a half-typed line, so after 30 seconds a directive is typed after it.
- **A6:** Rate limits (0007) count a directive when the office accepts it, not when it is typed. The hold check runs at sending and again before typing.
- **A7:** The queue has no view and no request of its own. The sender learns the place from the answer; the user withdraws waiting directives by stopping the target or by a hold (FR-005).
- **A8:** The 30-second typing pause, the 10-minute limit, and the 20-directive queue size are fixed in this change, not settings.

## Clarifications

None yet.

## Out of scope

- A view of waiting directives, and withdrawing one directive by hand (A7).
- Checking what the CLI shows through CLI hooks or any office file that the CLI reads (A2).
- Keeping waiting directives across a restart of the background server (A4).
- Detecting a half-typed line that a person left in the terminal (A5).
- Settings for the typing pause, the time limit, or the queue size (A8).
- Holding back input that a person types or pastes (PB-5).
- Startup prompts, part of the office additions (0001/FR-022); they are pasted without Enter already.
