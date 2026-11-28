# CLI sign-in check before a start

## Intent

The office checks that the CLI of an agent is signed in before it starts a session, and tells the person when it is not. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0019-cli-sign-in-check](../changes/archive/0019-cli-sign-in-check/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0019: Check that the CLI is signed in before a start

### Intent

When an agent's CLI is not signed in, a start today looks like a success: the agent shows `starting`, then `waiting for input`, and the CLI waits at its sign-in screen. Nobody is there to sign in. The user finds out only when the agent does no work, and a parent agent or a bulk resume cannot tell at all. This change asks the CLI before each start whether it is signed in, with the CLI's own read-only status command, and also watches the first seconds of the CLI's screen for a sign-in screen. When the CLI is not signed in, the start ends with one clear reason that says so and what to do. When the check cannot tell, the start works as it does today.

### Requirements

- **0019/FR-001** (US1, US2): WHEN a session of an agent is about to start (a start, a new task, a continue, an agent-made start, or a start of a bulk resume) AND the agent's CLI has a known status command (A1) AND none of the conditions of FR-010 holds, THE SYSTEM SHALL run that status command before it starts the CLI, and SHALL start the CLI only when the answer is not "not signed in".
- **0019/FR-002** (US1): IF the status command ends with a non-zero exit AND its output contains the CLI's not-signed-in words (A2), THEN THE SYSTEM SHALL NOT start the CLI, and SHALL record the request in the action record with the not-signed-in reason as its failure; for an agent that does not run, it SHALL store the session as one that never started with that reason and show the agent as `failed`; for an agent that runs (a new task or a continue), it SHALL refuse the request with that reason and leave the running session as it is.
- **0019/FR-003** (US2): IF the status command has not ended 5 seconds after it began, THEN THE SYSTEM SHALL end the status command and every process it started, and SHALL go on with the start as if no status command were known.
- **0019/FR-004** (US2): IF the sign-in check gives no answer (A2), or the agent's command, working folder, or office additions changed between the check and the start, THEN THE SYSTEM SHALL go on with the start as if no status command were known, and SHALL note in the start's action record entry that the sign-in check gave no answer and why: timed out, could not run, unclear answer, or the agent changed during the check.
- **0019/FR-005** (US3): WHILE a session is in its watch period AND neither a person nor the office typed into its terminal (the startup prompt aside, A5), WHEN the bottom of the CLI's screen shows the sign-in screen words of that CLI (A5), THE SYSTEM SHALL end the CLI, SHALL end the session as not signed in with the not-signed-in reason, SHALL show the agent as `failed`, SHALL record the end in the action record with the office as actor and the not-signed-in reason, and SHALL notify the user once as for a failed agent (0003/FR-006).
- **0019/FR-006** (US1, US2): THE SYSTEM SHALL run the status command without input and without a terminal, with the agent's program, working folder, login environment, and office additions (0001/FR-020), without the office's agent-call credential (0002), SHALL keep at most 64 KiB of its output in memory only, and SHALL read and write no file for the sign-in check.
- **0019/FR-007** (US1, US3): THE SYSTEM SHALL use one not-signed-in reason that names the CLI, says that it is not signed in, says how the office found it (the status command it ran, or the sign-in screen), says to sign in to the CLI in the user's own terminal and then start the agent again, and quotes at most one line of what the CLI showed, in visible form and at most 200 characters long.
- **0019/FR-008** (US1, US2, US3): THE SYSTEM SHALL NOT show, record, store, or log any output of a status command except the one quoted line of FR-007 after a not-signed-in answer, and SHALL NOT show, record, store, or log any screen text of the watch except the one quoted row of FR-007 after a match.
- **0019/FR-009** (US2): WHILE a status command runs, THE SYSTEM SHALL let other start, stop, new task, continue, and exit handling go on without waiting for it.
- **0019/FR-010** (US2): IF the agent's command has arguments of its own, OR the office additions hold flags, OR a sign-in key variable of the CLI (A11) is set in the office additions or the login environment, THEN THE SYSTEM SHALL run no status command for that start and SHALL start the CLI as if no status command were known.
- **0019/FR-011** (US3): IF the startup prompt holds one of the CLI's sign-in screen words, THEN THE SYSTEM SHALL NOT watch that session for a sign-in screen.

### Preserved behaviors

- **0019/PB-1:** office-harness creates, changes, or deletes no file in an agent CLI's configuration (0001/FR-021). A status command is the CLI's own program; what it writes into its own folders is the same as at any start of that CLI (A4).
- **0019/PB-2:** A start of an agent whose CLI has no known status command, and that shows no sign-in screen, works as before: no command runs before it and nothing ends it.
- **0019/PB-3:** The startup prompt is typed in after the first output and is never submitted (0001, D5); the check does not change this.
- **0019/PB-4:** A start of a bulk resume that ends as not signed in counts as a start that does not start (0013/FR-004, 0013/FR-005): the fallback start is tried once, and then the agent is put on hold with the reason.
- **0019/PB-5:** No agent starts without an action of the user or of an agent (0001/FR-004). The sign-in check starts no CLI session.
- **0019/PB-6:** Each start, stop, and failure is recorded in the action record with its actor (0001/FR-011).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Sign-in check result | What the check found for one start. | answer (signed in, not signed in, no answer), reason of no answer (timed out, could not run, unclear answer), the quoted line | Not stored by itself. A not-signed-in answer is kept as the never-started reason of the session and the failure of the action record entry; a no answer is kept as a note on the entry. |
| Session (extends 0001) | One run of an agent CLI. | end result: also "not signed in", with the reason, when the office ended it at a sign-in screen | As in 0001. |
| Action record entry (extends 0001) | One recorded action. | note (optional): the sign-in check gave no answer, and why | Belongs to the agent it names (0001/FR-011). |

### Success criteria

- **0019/SC-001:** With a test CLI whose status command says it is not signed in, 100% of 12 starts of agents that do not run (3 each of start, new task, continue, and agent-made start) end `failed` with the not-signed-in reason in the answer, the session list, and the action record, and 0 CLI sessions run.
- **0019/SC-002:** With a test status command that never ends, 100% of 5 starts begin their CLI within 6 seconds of the request, and 1 second after each timeout 0 processes of the status command run.
- **0019/SC-003:** With a test CLI that shows a known sign-in screen within its first 2 seconds, 100% of 5 starts end within 3 seconds of the screen, each with exactly 1 action record entry with the office as actor and exactly 1 notification.
- **0019/SC-004:** With a test CLI that shows the sign-in words after the person typed into its terminal, or after its first 20 seconds, 0 of 5 sessions are ended by the office.
- **0019/SC-005:** With test CLIs whose status command and screen print 10 KiB that hold control characters, bidirectional marks, and a line shaped like a secret key, 0 bytes of a signed-in or no-answer output appear in an answer, the action record, the session list, or the server log, and 100% of shown reasons quote at most 200 characters with 0 raw control or bidirectional characters.
- **0019/SC-006:** A sign-in check against a test CLI whose status command writes nothing leaves its home folder and the office's data folder with 0 created, changed, or deleted files.
- **0019/SC-007:** With a real `claude` and a real `codex`, each signed in and each not signed in, 100% of the 4 starts behave as US1 says (checked by hand on Linux; the result goes into the QA report).
- **0019/SC-008:** While a test status command hangs for one agent, 100% of 5 stops of another running agent end within 1 second of the request.
- **0019/SC-009:** With a test status command that always says not signed in, 0 of 9 starts are refused when the agent's command has its own arguments (3), the office additions hold a flag (3), or a sign-in key variable is set (3), and each of them starts its CLI.
- **0019/SC-010:** With a test CLI that shows the sign-in words only inside the startup prompt or inside a directive typed in its first 20 seconds, 0 of 6 sessions are ended by the office.

### Notes from Deviations

- **A stop between the mark and the exit.** A person's stop that comes after the watch marked the run but before the run exits leaves the agent `stopped`: the exit then gives no `failed` status and the office adds no `stop` entry. The office records its `stop` entry only after the status followed.
- **Group signal before the child is reaped.** Where the system can look at an exit without reaping it (Linux, Android, FreeBSD), the group of a status command or git child is signalled before the child is reaped, and the caller takes its status after that. On other systems the process id of a reaped child could be reused before the signal; the window needs a helper that holds a pipe after the child exited and then left the group.
- **Assumptions that the requirements name.** A bare `A1`, `A2`, `A4`, `A5`, or `A11` in the requirements above names an assumption of 0019, kept here:
  - **A1:** The known status commands are `claude auth status --text` for `claude` and `codex login status` for `codex`. `gemini` and every other command have none. The CLI is known by the program name of the agent's command.
  - **A2:** The answer is "not signed in" only when the status command ends with a non-zero exit and its output contains the CLI's not-signed-in words: `not logged in` for both `claude` and `codex`, compared without case. Exit 0 is "signed in". Every other result is "no answer" (unclear answer).
  - **A4:** The status command is the CLI's own program, run with the agent's environment. Like any start of the CLI, it can write the CLI's own files in the CLI's own folders. The office itself opens no file of a CLI for the check and writes none.
  - **A5:** The watch period is the first 20 seconds of a session. The office reads the bottom 15 rows that show anything, as 0012 does, and compares the sign-in words of the CLI without case. The watch ends at the first key the person types into the terminal, at the first directive the office types into it, at the end of the watch period, or when the session ends. The startup prompt does not end the watch, because a prompt that holds a sign-in word turns the watch off for that session (0019/FR-011).
  - **A11:** The sign-in key variables are the names that the archived 0019 spec lists, for `claude` and for `codex`. With one of them, or with arguments of its own, a CLI can work while its status command says it is not signed in. The office skips the check (0019/FR-010). Only a variable's name is checked, never its value.
  - The timeout of 5 seconds (0019/FR-003) is not a setting.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0019-cli-sign-in-check - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
