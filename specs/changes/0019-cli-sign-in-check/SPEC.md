---
status: draft
capability: office
plan: specs/changes/0019-cli-sign-in-check/PLAN.md
supersedes:
---

# 0019: Check that the CLI is signed in before a start

## Intent

When an agent's CLI is not signed in, a start today looks like a success: the agent shows `starting`, then `waiting for input`, and the CLI waits at its sign-in screen. Nobody is there to sign in. The user finds out only when the agent does no work, and a parent agent or a bulk resume cannot tell at all. This change asks the CLI before each start whether it is signed in, with the CLI's own read-only status command, and also watches the first seconds of the CLI's screen for a sign-in screen. When the CLI is not signed in, the start ends with one clear reason that says so and what to do. When the check cannot tell, the start works as it does today.

## Context

- Source of truth: `docs/PRD.md`. References such as `0001/FR-021` point to requirements of approved changes; the living spec is `specs/office/SPEC.md`.
- This change depends on 0001 (starts, sessions, statuses, the action record, no change to CLI configuration), 0002 (agent-made starts), 0003 (notifications), 0011 (the visible form of a text), 0012 (the bottom rows of a screen that the office reads), and 0013 (bulk resume).
- Research (`research.md`) found the read-only status command of each supported CLI (R1), that a CLI's own program can write its own files when it runs (R2), how long a status command takes (R3), and the words of each sign-in screen (R4).
- The idea comes from an MIT-licensed reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, start, action record, office additions | As defined in 0001. |
| Status command | The CLI's own read-only command that tells whether it is signed in (A1). |
| Sign-in check | Running the status command before a start and reading its answer. |
| Answer | What the sign-in check found: signed in, not signed in, or no answer. |
| No answer | The status command timed out, could not run, or gave an answer that is neither signed in nor not signed in (A2). |
| Sign-in screen | A screen of the CLI that asks the person to sign in, known by the words in research R4. |
| Watch period | The first 20 seconds of a session, in which the office reads the CLI's screen for a sign-in screen (A5). |
| Visible form | The text with every control and bidirectional format character shown as a visible symbol (0011/FR-006). |
| Not-signed-in reason | The one text that says which CLI is not signed in, how the office found it, and what to do (FR-007). |

## User stories

### US1: A start of a CLI that is not signed in ends with a clear reason (Priority: P1)

The user, a parent agent, or a bulk resume starts an agent whose CLI is not signed in. The start ends at once, the agent shows `failed`, and the not-signed-in reason shows where the start's result shows.

**Why this priority:** it is the value of the change. Without it, an agent waits at a sign-in screen and looks alive.

**Independent test:** give one agent a test CLI named `claude` whose status command ends with exit 1 and the not-signed-in words. Start the agent. The agent is `failed`, no CLI session runs, and the not-signed-in reason shows in the answer to the start, in the agent's session list, and in its action record.

**Scenarios:**

1. Given the agent's CLI has a known status command and the command says the CLI is not signed in, When the user starts the agent, Then the office does not start the CLI, shows the agent as `failed`, and shows the not-signed-in reason.
2. Given the same agent, When the user starts a new task or continues a session, Then the result is the same as in scenario 1.
3. Given the same agent is a member, When its lead starts it with an agent call (0002), Then the lead's answer is a failed start with the not-signed-in reason.
4. Given the status command says the CLI is signed in, When the user starts the agent, Then the CLI starts as before.
5. Given the status command printed text with control characters or bidirectional marks, When the reason shows the CLI's line, Then the line shows in visible form and at most 200 characters long.

### US2: A slow or unknown check never blocks a start (Priority: P1)

A status command that hangs, is missing, or gives an answer the office does not know never stops or long delays a start.

**Why this priority:** without it, the check itself could stop agents that work. It ships with US1.

**Independent test:** give one agent a test CLI named `claude` whose status command never ends. Start the agent. The CLI starts within 6 seconds, the status command no longer runs, and the start's action record entry says that the sign-in check gave no answer because it timed out.

**Scenarios:**

1. Given the status command does not end within 5 seconds, When the time passes, Then the office ends the status command and starts the CLI as before.
2. Given the CLI has no known status command (`gemini`, `opencode`, or any other command), When the user starts the agent, Then the office runs no status command and starts the CLI as before.
3. Given the status command cannot run, ends with a non-zero exit without the not-signed-in words, or prints nothing, When the user starts the agent, Then the office starts the CLI as before.
4. Given the sign-in check gave no answer, When the start's entry shows in the action record, Then it says that the sign-in check gave no answer and why (timed out, could not run, or unclear answer), without the command's output.

### US3: A sign-in screen in the first seconds ends the start (Priority: P2)

When the CLI starts and shows a sign-in screen, the office ends it and shows the same kind of reason.

**Why this priority:** it covers a CLI with no status command (`gemini`) and a status command that gave no answer. US1 and US2 work without it.

**Independent test:** give one agent a test CLI named `gemini` that prints `How would you like to authenticate for this project?` and waits. Start the agent. Within 3 seconds of that output the CLI no longer runs, the agent is `failed`, its last session ended as not signed in with the reason, the action record has one entry with the office as actor and the reason, and exactly one notification names the agent.

**Scenarios:**

1. Given a session started less than 20 seconds ago and nobody typed into it, When the bottom of its screen shows a sign-in screen of that CLI, Then the office ends the CLI, shows the agent as `failed`, and records the not-signed-in reason with the office as actor.
2. Given the person typed into the session's terminal, When a sign-in screen shows later, Then the office does not end the CLI.
3. Given the session runs for more than 20 seconds, When a sign-in screen shows after that, Then the office does not end the CLI.
4. Given the CLI's command is not one of the four supported CLIs, When it prints the words of a sign-in screen, Then the office does not end it.

## Requirements

### ADDED

- **FR-001** (US1, US2): WHEN a session of an agent is about to start (a start, a new task, a continue, an agent-made start, or a start of a bulk resume) AND the agent's CLI has a known status command (A1), THE SYSTEM SHALL run that status command before it starts the CLI, and SHALL start the CLI only when the answer is not "not signed in".
- **FR-002** (US1): IF the status command ends with a non-zero exit AND its output contains the CLI's not-signed-in words (A2), THEN THE SYSTEM SHALL NOT start the CLI, SHALL store the session as one that never started with the not-signed-in reason, SHALL show the agent as `failed`, and SHALL record the start in the action record with the not-signed-in reason as its failure.
- **FR-003** (US2): IF the status command has not ended 5 seconds after it began, THEN THE SYSTEM SHALL end the status command and every process it started, and SHALL go on with the start as if no status command were known.
- **FR-004** (US2): IF the sign-in check gives no answer (A2), THEN THE SYSTEM SHALL go on with the start as if no status command were known, and SHALL note in the start's action record entry that the sign-in check gave no answer and why: timed out, could not run, or unclear answer.
- **FR-005** (US3): WHILE a session is in its watch period AND nobody typed into its terminal, WHEN the bottom of the CLI's screen shows the sign-in screen words of that CLI (A5), THE SYSTEM SHALL end the CLI, SHALL end the session as not signed in with the not-signed-in reason, SHALL show the agent as `failed`, SHALL record the end in the action record with the office as actor and the not-signed-in reason, and SHALL notify the user once as for a failed agent (0003/FR-006).
- **FR-006** (US1, US2): THE SYSTEM SHALL run the status command without input and without a terminal, with the agent's program, working folder, login environment, and office additions (0001/FR-020), without the office's agent-call credential (0002), SHALL keep at most 64 KiB of its output in memory only, and SHALL read and write no file for the sign-in check.
- **FR-007** (US1, US3): THE SYSTEM SHALL use one not-signed-in reason that names the CLI, says that it is not signed in, says how the office found it (the status command it ran, or the sign-in screen), says to sign in to the CLI in the user's own terminal and then start the agent again, and quotes at most one line of what the CLI showed, in visible form and at most 200 characters long.
- **FR-008** (US1, US2, US3): THE SYSTEM SHALL NOT show, record, store, or log any output of a status command except the one quoted line of FR-007 after a not-signed-in answer, and SHALL NOT show, record, store, or log any screen text of the watch except the one quoted row of FR-007 after a match.

### PRESERVED

- **PB-1:** office-harness creates, changes, or deletes no file in an agent CLI's configuration (0001/FR-021). A status command is the CLI's own program; what it writes into its own folders is the same as at any start of that CLI (A4).
- **PB-2:** A start of an agent whose CLI has no known status command, and that shows no sign-in screen, works as before: no command runs before it and nothing ends it.
- **PB-3:** The startup prompt is typed in after the first output and is never submitted (0001, D5); the check does not change this.
- **PB-4:** A start of a bulk resume that ends as not signed in counts as a start that does not start (0013/FR-004, 0013/FR-005): the fallback start is tried once, and then the agent is put on hold with the reason.
- **PB-5:** No agent starts without an action of the user or of an agent (0001/FR-004). The sign-in check starts no CLI session.
- **PB-6:** Each start, stop, and failure is recorded in the action record with its actor (0001/FR-011).

## User experience

**Flow:**

1. The user, a parent agent, or a bulk resume starts an agent as today.
2. When the CLI is not signed in, the agent shows `failed`. The answer to the start shows the not-signed-in reason, as other start failures show today.
3. In the agent's details, the session list shows the session as never started (status command) or ended as not signed in (sign-in screen), with the reason. The action record shows the entry with its actor and the reason.
4. When the office ended a CLI at a sign-in screen, the user gets one notification for the failed agent.
5. The user signs in to the CLI in a terminal of their own and starts the agent again.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Signed in | The start works as before. | US1 scenario 4 |
| Not signed in, found by the status command | The agent is `failed`; the start's answer, the session list, and the action record show the not-signed-in reason. | FR-002, FR-007 |
| Not signed in, found on the screen | The agent is `failed`; the session list and the action record show the not-signed-in reason; one notification. | FR-005, FR-007 |
| No answer | The start works as before; its action record entry says the check gave no answer and why. | FR-003, FR-004 |
| No status command known | The start works as before; nothing new shows. | PB-2 |

**Design:** no new view. The reason shows in the places where start failures and session ends show today. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The reason is text, never color alone, and it reads in the same places as other failure texts, with the same keyboard reach (0001/FR-028).
- The reason quotes the CLI's line in visible form, so no control character acts on the screen or on assistive technology.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text), as the existing failure texts do.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Sign-in check result | What the check found for one start. | answer (signed in, not signed in, no answer), reason of no answer (timed out, could not run, unclear answer), the quoted line | Not stored by itself. A not-signed-in answer is kept as the never-started reason of the session and the failure of the action record entry; a no answer is kept as a note on the entry. |
| Session (extends 0001) | One run of an agent CLI. | end result: also "not signed in", with the reason, when the office ended it at a sign-in screen | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | note (optional): the sign-in check gave no answer, and why | Belongs to the agent it names (0001/FR-011). |

## Success criteria

- **SC-001:** With a test CLI whose status command says it is not signed in, 100% of 12 starts (3 each of start, new task, continue, and agent-made start) end `failed` with the not-signed-in reason in the answer, the session list, and the action record, and 0 CLI sessions run.
- **SC-002:** With a test status command that never ends, 100% of 5 starts begin their CLI within 6 seconds of the request, and 1 second after each timeout 0 processes of the status command run.
- **SC-003:** With a test CLI that shows a known sign-in screen within its first 2 seconds, 100% of 5 starts end within 3 seconds of the screen, each with exactly 1 action record entry with the office as actor and exactly 1 notification.
- **SC-004:** With a test CLI that shows the sign-in words after the person typed into its terminal, or after its first 20 seconds, 0 of 5 sessions are ended by the office.
- **SC-005:** With test CLIs whose status command and screen print 10 KiB that hold control characters, bidirectional marks, and a line shaped like a secret key, 0 bytes of a signed-in or no-answer output appear in an answer, the action record, the session list, or the server log, and 100% of shown reasons quote at most 200 characters with 0 raw control or bidirectional characters.
- **SC-006:** A sign-in check against a test CLI whose status command writes nothing leaves its home folder and the office's data folder with 0 created, changed, or deleted files.
- **SC-007:** With a real `claude` and a real `codex`, each signed in and each not signed in, 100% of the 4 starts behave as US1 says (checked by hand on Linux; the result goes into the QA report).

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The program of the agent's command is missing. | Failure | The start fails as today with the reason that the program cannot start. |
| The status command prints more than 64 KiB. | Boundary | The office keeps the first 64 KiB and drops the rest; the answer is read from what it kept (FR-006). |
| The status command ends with exit 0 and also prints "not logged in". | Boundary | Signed in: the exit code decides (A2). |
| An older CLI version has no status subcommand and ends with a usage text and a non-zero exit. | Boundary | No answer (unclear answer); the start goes on (FR-004). |
| The status command starts child processes and does not end. | Failure | All of them end at the timeout (FR-003). |
| The CLI is not signed in, and the user continues a session. | Boundary | The continue fails with the not-signed-in reason (US1 scenario 2); the session stays continuable later. |
| A new task on a running agent whose CLI is not signed in. | Boundary | The check runs before the running session ends. A not-signed-in answer then fails the new task as other start failures do today. |
| A bulk resume of agents whose CLI is not signed in. | Boundary | Each start runs its own check; each agent ends on hold with the not-signed-in reason (PB-4). |
| The office additions set the CLI's configuration folder (for example a second account). | Boundary | The status command gets the same additions, so it checks the same account (FR-006). |
| A running agent prints the sign-in words within its first 20 seconds, before anyone typed. | Boundary | The office ends it (a false match); the reason quotes the row, so the user sees why (A5). |
| The CLI shows a trust or permission question before its sign-in screen. | Boundary | The question is not a sign-in screen; the office ends the CLI only when the sign-in words show within the watch period. |
| The server stops during a status command. | Failure | The status command ends with the server; the agent stays as it was (no session started). |
| The status command's output is not valid text. | Boundary | The office reads it with invalid bytes replaced; the answer follows A2. |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). |

## Assumptions

- **A1:** Known status commands: `claude auth status --text` for `claude` and `codex login status` for `codex` (research R1). `gemini` and `opencode` have none, and every other command has none. The CLI is known by the program name of the agent's command, as the office already picks a CLI's session handling (0001).
- **A2:** The answer is "not signed in" only when the status command ends with a non-zero exit AND its output contains the CLI's not-signed-in words: `not logged in` for both `claude` and `codex`, compared without case (research R1). Exit 0 is "signed in". Every other result is "no answer" (unclear answer). Reason: a CLI version without the subcommand ends with a non-zero exit and a usage text (R1), and a refused start of a signed-in CLI would be worse than today's behavior.
- **A3:** The timeout is 5 seconds; it is not a setting. Research R3 measured 0.03 s to 2.3 s.
- **A4:** The status command is the CLI's own program, run with the agent's environment. Like any start of the CLI, it can write the CLI's own files in the CLI's own folders (research R2). The office itself opens no file of a CLI for the check and writes none. A start of the CLI, which follows a signed-in answer, writes the same folders anyway.
- **A5:** The watch period is the first 20 seconds of a session. The office reads the bottom 15 rows that show anything, as 0012 does, and compares the sign-in words of research R4 without case. The watch ends at the first key the person types into the terminal, at the end of the watch period, or when the session ends. Text that the office types in, such as the startup prompt, does not end the watch.
- **A6:** The office ends a CLI at its sign-in screen instead of letting the agent wait there; the user signs in in a terminal of their own. Signing in through the agent's terminal is out of scope.
- **A7:** Each start runs its own check; no answer is reused. When every check of a bulk resume of N agents times out, the bulk action takes up to N times 5 seconds longer.
- **A8:** Only a session start of an agent runs the check; no other command does.
- **A9:** The words of the status answers and of the sign-in screens can change with a CLI version. A word that is not found gives no answer or misses the screen, so the start behaves as before this change; it never refuses a start by mistake on that ground.
- **A10:** Starts and stops of the office run one after another (0001). While a status command runs, a later start or stop request waits for it, at most the 5 seconds of A3.

## Out of scope

- Signing in for the user, or a sign-in flow inside the agent's terminal (A6).
- Reading a CLI's credential or configuration files to find the sign-in state.
- Checking whether a key is valid, has quota, or can reach the CLI's service.
- Checking the sign-in state again while a session runs (for example after a sign-in expires).
- A setting for the timeout, the watch period, or the words (A3, A5).
- Reusing an answer across starts (A7).
- A status command for `gemini` or `opencode` (research R1).
