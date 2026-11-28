# Temporary agents

## Intent

A temp is a short-lived agent that one lead starts for one job and that the office ends when the job is done. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0020-temporary-agents](../changes/archive/0020-temporary-agents/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002, 0012: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0003: [Agent panel](../agent-panel/SPEC.md)
- 0015: [Agent state and reports in the hierarchy](../agent-reports/SPEC.md)
- 0010: [Agent worktrees and dependencies](../agent-worktree/SPEC.md)
- 0013: [Resume all interrupted agents](../bulk-resume/SPEC.md)
- 0007: [Hold switch and agent action limits](../hold-and-limits/SPEC.md)
- 0008: [Needs-you queue](../needs-you-queue/SPEC.md)
- 0017: [Leftover processes and ended commands](../process-cleanup/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0020: Temporary agents

### Intent

A lead that gives a member one small job keeps that member in the tree after the job is done: only the person deletes agents (0002 A4), so helpers pile up, each with its CLI, its memory, and its place in the tree. This change adds the temporary agent, a "temp": an agent that the person or a parent agent creates for one job, with an objective. A temp starts at once, works in its own branch and worktree by default when its folder is in a git repository (0010), and reports `done` through the upward report channel (0015). The office then ends it: it stops the CLI, removes the agent, and keeps the results. A worktree or branch that holds work that is not merged stays, and the done report stays visible in a list of finished temps until the person dismisses it. A cap limits how many temps exist at once, an idle limit ends a temp that stops producing output, a temp whose parent goes away ends with it, and the person sees and can end every temp in one view.

### Requirements

- **0020/FR-001** (US1, US2): WHEN the person, or a running agent as 0002/FR-008 allows, creates an agent with the temp choice on and an objective that follows the text rules of a directive (0002), THE SYSTEM SHALL create the agent as a temp at the place the 0002 rules allow, start it at once, and put the objective into the temp's queue as its first entry, typed under the rules of 0012 as for a directive whose sender is the creator; IF the objective is missing or breaks those rules, THEN THE SYSTEM SHALL create nothing, give the reason, and record the refusal; and WHEN a temp creates an agent, THE SYSTEM SHALL create it as a temp.
- **0020/FR-002** (US1): WHERE the working folder of a new temp is inside a git repository, THE SYSTEM SHALL create the temp with its own branch and worktree as 0010/FR-002 does, unless the creator turns own worktree off in the create; WHERE the folder is not inside a git repository, THE SYSTEM SHALL create the temp without one.
- **0020/FR-003** (US3): IF a create would make the number of temps in the office greater than the temp cap, THEN THE SYSTEM SHALL create nothing, refuse it with a reason that names the cap and its value, record the refusal, and SHALL NOT count the refusal toward any rate limit or the automatic hold (0007/FR-008); THE SYSTEM SHALL let the person set the temp cap in the office settings as a whole number from 1 to 100, 8 by default, and SHALL apply a changed value to the next create without ending any temp.
- **0020/FR-004** (US3): WHILE a temp is running or asleep, no child of it runs, no hold covers it, and it has no item in Needs you (0008), THE SYSTEM SHALL end the temp within 60 seconds after the time since its quiet-since moment reaches the idle limit, with the reason that it produced no output for that many minutes; THE SYSTEM SHALL let the person set the idle limit in the office settings as a whole number of minutes from 5 to 1440, 30 by default, and SHALL apply a changed value at the next check.
- **0020/FR-005** (US1, US2, US3, US4): THE SYSTEM SHALL end a temp, with the reason in brackets, WHEN it accepts a `done` report from the temp (done), WHEN FR-004 applies (no output for N minutes), WHEN the temp's CLI exits or fails other than by a stop or a sleep (exited with its exit code, or failed with its reason), WHEN the person or an agent stops the temp, alone or with an ancestor (stopped by the actor), WHEN the person ends it from the temps view (ended by the person), WHEN the person deletes it, alone or with an ancestor (deleted by the person, or its parent was deleted), and WHEN its parent temp ends (its parent ended); SHALL decide each end on the temp's status as it is under the office's order of status changes, so that a status that a new task or a continued session (0001/FR-024, 0001/FR-025) passes through ends no temp; and SHALL NOT end a temp only because it is `interrupted`.
- **0020/FR-006** (US1, US4): WHEN the system ends a temp, THE SYSTEM SHALL end its child temps first, stop its CLI as the person's stop does when it runs, apply the keep rule to its own worktree (or, for a delete, the person's keep or remove answer of 0010/FR-005), remove the agent as a delete does, add one finished temp entry, and add one action record entry for the end with the reason and with the person, the stopping agent, or the office as the actor; and IF the office cannot remove a worktree or a branch that the keep rule removes, THEN THE SYSTEM SHALL keep it and name it and the failure in the finished entry.
- **0020/FR-007** (US3): WHEN the system ends a temp because of the idle limit or because its CLI exited or failed, THE SYSTEM SHALL put one end notice, with the temp's name in visible form, its id, and the reason, into the parent's queue under 0015/FR-004 and 0015/FR-010 as a report of that temp WHERE the parent runs and its queue has room; and, for an end because of the idle limit whose notice reaches no queue, THE SYSTEM SHALL notify the person once with the temp's name and the reason (an exit or a failure notifies the person under 0003/FR-006 as before).
- **0020/FR-008** (US1, US2): THE SYSTEM SHALL keep each finished temp entry across server and machine restarts until the person dismisses it, and a dismiss SHALL remove only the entry, never a worktree, a branch, or a file.
- **0020/FR-009** (US2): THE SYSTEM SHALL show the person, in one temps view of the desktop app, every temp with its name, its creator, its parent, its objective, its status, its run time, and its quiet time, and every finished temp entry, newest first; SHALL let the person end each temp and dismiss each entry from that view with the keyboard alone, ending after one confirmation that says the temp will be removed, that work not merged stays, and that files git ignores are removed with a removed worktree, and SHALL ask the same confirmation when the person stops a single temp from its details or its terminal view; and SHALL mark a temp as "temp", as text, in its details.

This change modified: 0015/FR-005. The final text is under the change that owns that id.

### Preserved behaviors

- **0020/PB-1:** An agent that is not a temp keeps every behavior it has: the office does not end, remove, or idle-limit it, and an agent-made create of an agent that is not a temp has no own worktree (0010 A7).
- **0020/PB-2:** The 0002 tree rules stay as they are: the CEO creates team leads, a team lead creates members of its team, a member creates no agents, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009). A temp create follows them.
- **0020/PB-3:** Holds and rate limits apply to a temp create as to any agent-made create (0007); the person's own actions are never held or rate-limited (0007/PB-1). The temp cap is not a rate limit; it applies to every create of a temp.
- **0020/PB-4:** A delete by the person still asks once whether to keep or remove own worktrees, with keep as the default, and asks again for a worktree with uncommitted changes (0010/FR-005, 0010/FR-006). The office removes no worktree with uncommitted changes and no branch with commits that its base lacks without that answer.
- **0020/PB-5:** A temp's reports follow every 0015 rule: the parent from the tree, the visible form, the queue and its limits, the rate limit, and Needs you for a report for the person. An objective and an end notice are typed under the 0012 rules: queue, readiness, question check, and the input-line check before Enter.
- **0020/PB-6:** The office starts no agent by itself after a restart (0001/FR-004, 0013/PB-1); an `interrupted` temp waits until the person resumes, ends, or deletes it.
- **0020/PB-7:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Temp (an agent, extended) | An agent for one job. | the temp mark, objective, creator, created time | An agent of 0001 and 0002; its children are temps; removed when it ends. |
| Finished temp entry | What stays of a temp after it ends. | temp name and id, objective, creator, parent, start and end time, end reason, last `done` report (kind, text), kept worktree folder and branch or none, a removal failure | One per ended temp; kept until the person dismisses it; no link to a live agent. |
| Temp settings | The office bounds of temps. | temp cap (1 to 100, default 8), idle limit (5 to 1440 minutes, default 30) | One set per office, with the 0007 settings. |

### Success criteria

- **0020/SC-001:** The person creates a temp with an objective in a git repository; 1 of 1 starts in its own worktree, and its terminal receives the objective with Enter once its CLI waits for input.
- **0020/SC-002:** A lead creates 3 member temps; each sends a `done` report. 3 of 3 reports reach the lead's terminal, 3 of 3 temps are no longer agents within 10 seconds after their report, and 3 finished entries show the reports.
- **0020/SC-003:** Of 4 temps that end with a clean worktree, an uncommitted change, a new commit, and both, the clean one's worktree and branch are removed and 3 of 3 others keep both, with 0 changed files lost.
- **0020/SC-004:** With the cap at 3 and 3 temps in the office, a 4th create by an agent and one by the person are refused with the cap and its value in the reason (2 of 2), 0 agents are created, and 5 such refusals of one agent within 60 seconds put it on 0 holds.
- **0020/SC-005:** With the idle limit at 5 minutes, a temp with no output and no Needs you item ends within 6 minutes, its finished entry says "no output for 5 minutes", and its running parent's terminal shows 1 end notice. A temp that prints one line every 60 seconds for 15 minutes, a temp under a hold for 10 minutes, and a temp whose Needs you item stays for 10 minutes, are not ended (0 of 3).
- **0020/SC-006:** The person deletes a lead with 3 member temps and keeps the worktrees: 3 of 3 finished entries name the deleted parent, and 3 of 3 worktrees and branches stay.
- **0020/SC-007:** A lead temp with 2 running member temps sends `done`: both members end before the lead, with the reason that their parent ended (2 of 2).
- **0020/SC-008:** 5 finished entries survive a server restart (5 of 5); dismissing 1 removes the entry and 0 files, worktrees, or branches.
- **0020/SC-009:** With 10 temps in the office at several places in the tree, the temps view lists 10 of 10, and the person ends 1 with the keyboard alone.
- **0020/SC-010:** A temp's `done` report that waits in a working parent's queue when the temp ends is typed into the parent's terminal once the parent is ready (1 of 1).
- **0020/SC-011:** A temp whose CLI exits with code 1 before any `done` report ends within 5 seconds, and its finished entry names exit code 1 (1 of 1).
- **0020/SC-012:** Of 10 new tasks and 10 continued sessions on running temps, 0 end a temp.

### Notes from Deviations

- **Dismiss record.** A dismiss adds a `dismiss` entry (actor `user`, detail the entry id) to the ended temp's action record.
- **Refused from the role path.** An agent's create through the role path returns the cap refusal as `refused`, not as `internal`.
- **Temp create and the 0017 sweep gate.** A person's temp create starts its CLI inside the create, so it waits for the sweep gate like `agent.start`: no CLI starts while the sweep runs. An agent cannot create a temp during the sweep, because no agent CLI runs before a gated start.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0020-temporary-agents - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
