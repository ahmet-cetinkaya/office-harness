---
status: shipped
capability: temporary-agents
plan: specs/changes/archive/0020-temporary-agents/PLAN.md
supersedes: 0015/FR-005
---

<!-- Approved for coding on 2026-11-23 through the review gate (r2 at e8a6ab3), under the owner's spec delegation; shipped in PR #131. -->

# 0020: Temporary agents

## Intent

A lead that gives a member one small job keeps that member in the tree after the job is done: only the person deletes agents (0002 A4), so helpers pile up, each with its CLI, its memory, and its place in the tree. This change adds the temporary agent, a "temp": an agent that the person or a parent agent creates for one job, with an objective. A temp starts at once, works in its own branch and worktree by default when its folder is in a git repository (0010), and reports `done` through the upward report channel (0015). The office then ends it: it stops the CLI, removes the agent, and keeps the results. A worktree or branch that holds work that is not merged stays, and the done report stays visible in a list of finished temps until the person dismisses it. A cap limits how many temps exist at once, an idle limit ends a temp that stops producing output, a temp whose parent goes away ends with it, and the person sees and can end every temp in one view.

## Context

- Source of truth: `docs/PRD.md`, R2 (deep hierarchy) and R9 (find the agent that needs the user).
- This change depends on 0001 (agents, statuses, the stop, the action record, durability), 0002 (the tree, roles, agent-made creates, deleting descendants), 0003 (notifications), 0007 (holds, rate limits, office settings), 0008 (Needs you), 0010 (own worktree, keep or remove on delete), 0012 (the queue and safe typing), 0013 (resume of interrupted agents), 0015 (reports), and 0018 (idle sleep). References such as `0010/FR-005` point to requirements in those changes.
- 0002 A4 says that agents cannot delete agents and that only the person deletes. This stays true: no agent gets a delete request. The office itself removes a temp when the temp ends (FR-005, FR-006).
- 0010 A7 says that agents create agents without own worktree. This change keeps that for agents that are not temps and gives a temp its own worktree by default, whoever creates it (FR-002).
- The office does not read a CLI's prompts (0001 A1, 0003 A3). A temp's `done` is what the temp reports (0015 A1), not what the office saw.
- The idea comes from an MIT-licensed reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, status, session, terminal, stop, action record, desktop app, background server | As defined in 0001. Running means the status `starting`, `working`, or `waiting for input`. |
| Parent, child, descendant, role, directive, agent-made create | As defined in 0002. |
| Notification | As defined in 0003. |
| Agent-made action, office hold, agent hold, rate limit, automatic hold, office settings | As defined in 0007. A hold covers an agent under the rules of 0007/FR-004. |
| Needs you, item | As defined in 0008. |
| Own worktree, base, uncommitted changes | As defined in 0010. |
| Queue | As defined in 0012: the entries that wait to be typed into one agent's terminal. |
| Report, `done` report, report for the person | As defined in 0015. |
| Asleep | As defined in 0018. |
| Temp | An agent that the person or a parent agent created for one job, with an objective, and that the office ends when the job ends. |
| Objective | The text that says what a temp must do. The office types it into the temp's terminal as the temp's first input. |
| Creator | The person, or the agent whose request created the temp. The creator can differ from the temp's parent: a CEO can create a member temp under one of its leads (0002). |
| Temp cap | An office setting: the highest number of temps that exist at the same time. |
| Idle limit | An office setting: the time in minutes after which the office ends a temp that produced no output. |
| Quiet time | The time since the temp's quiet-since moment. |
| Quiet since | The latest of these moments: the start of the temp's current run, its last terminal output, the last person input that reached its terminal, the end of the last hold that covered it, the moment its last running child stopped running, and the moment its last Needs you item left. |
| End (a temp) | The office ends the temp's child temps, stops its CLI, keeps or removes its worktree, removes the agent, and adds a finished temp entry (FR-006). |
| Keep rule | A temp's worktree and branch stay when the worktree has uncommitted changes or the branch has commits that its base lacks; otherwise the office removes both as 0010/FR-007 does. |
| Finished temp entry | What the office keeps of a temp after it ends: its name, id, objective, creator, parent, start and end time, the end reason, its last `done` report, and the worktree folder and branch that stayed, or that nothing stayed and that files git ignores were removed with the worktree. |
| End notice | One line that the office types into a parent's queue to say that one of its temps ended and why (FR-007). |
| Temps view | The desktop app view that lists every temp and every finished temp entry (FR-009). |

## Data flow and trust boundary

- **Boundaries:** the agent's connection to the office (0002), and the person's desktop app connection (0001). The office knows an agent creator from the call credential of its running session (0002) and records it as the creator and the actor.
- **Into the office:** from a creator, the create fields of 0002 and 0010 plus the temp choice and the objective; from the person, the temp cap, the idle limit, an end, and a dismiss.
- **Out of the office, to the temp's terminal:** the objective, typed under the rules of 0012 as a directive's text is.
- **Out of the office, to a parent's terminal:** the temp's own reports (0015) and end notices, typed under the rules of 0012 and 0015/FR-004.
- **Out of the office, to the person:** the temps view, finished temp entries, and notifications (FR-007).
- **Into the action record:** each temp create and each refused create, each end with its actor and reason, and each dismiss of a finished entry.
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | An agent creates temps outside its subtree or with a role its place does not allow. | The 0002 tree and role rules apply to a temp create unchanged (FR-001, PB-2). |
  | A looping agent creates temps without end and fills the machine. | The temp cap (FR-003), the rate limit "agents created by one agent", the holds, and the automatic hold of 0007 apply (PB-3). |
  | An objective carries control characters or escape codes that act on the temp's terminal. | The objective must pass the text rules of a directive (FR-001); it is typed only under the 0012 rules (PB-5). |
  | The office removes a temp's work. | The keep rule keeps every worktree with uncommitted changes and every branch with commits that its base lacks (FR-006, SC-003). A dismiss removes only the entry (FR-008). |
  | A temp's name or report text acts on the parent's terminal or the person's view. | Reports keep their 0015 visible form; an end notice uses the temp's name in the visible form of 0014 (FR-007). |
  | A temp keeps itself alive without end by printing output. | Not prevented by the idle limit; the cap bounds the count and the person ends any temp from the temps view (FR-009, Residual risk). |

- **Residual risk:** an agent CLI runs as the person's OS user and can act outside the office (0002 A8). A temp that prints output in a loop never reaches the idle limit; the person sees its quiet time in the temps view and ends it. Kept worktrees use disk space until the person removes them; the office lists each one in its finished temp entry.

## User stories

### US1 — A lead gives a member one job, and the office cleans up after it (Priority: P1)

A lead creates a member temp with an objective. The temp starts at once in its own branch and worktree, does the job, and sends a `done` report. The lead's terminal shows the report. The office then ends the temp: the member leaves the tree, its branch and worktree stay because they hold its commits, and the finished temps list shows the report.

**Why this priority:** this is the gap the change closes. Without it every helper stays in the tree until the person deletes it by hand.

**Independent test:** in a git repository, lead L1 creates member temp T1 with the objective "write the file notes.txt and commit it". T1 starts in its own worktree; its terminal receives the objective. T1 commits a file and sends a `done` report. L1's terminal shows the report line. Within 10 seconds T1 is no longer an agent; its worktree and branch still exist; the temps view shows a finished entry for T1 with the report and the kept worktree and branch.

**Scenarios:**

1. Given lead L1 runs in a project whose folder is a git repository, When L1 creates a member temp with an objective, Then the office creates it under L1 with its own branch and worktree, starts it, and types the objective into its terminal under the 0012 rules.
2. Given temp T1 runs, When it sends a `done` report that the office accepts, Then the report reaches L1 as 0015 requires and the office ends T1 with the reason "done".
3. Given T1 committed work on its branch, When the office ends T1, Then the worktree and the branch stay and the finished entry names both.
4. Given T1 changed no file and made no commit, When the office ends T1, Then the office removes the worktree and the branch and the finished entry says that nothing stayed.
5. Given L1 is working when T1's `done` report arrives, When the office ends T1 while the report waits in L1's queue, Then the report stays in the queue and is typed when L1 is ready.
6. Given a member temp, When it tries to create an agent, Then the office refuses it as 0002 refuses a member's create.
7. Given lead temp LT creates a member, Then the member is a temp as well.

### US2 — The person starts a temp and sees every temp (Priority: P1)

The person creates a temp from the create form with an objective. The temps view lists every temp in the office, with its creator, objective, status, and quiet time, and every finished temp entry. The person ends any temp from there.

**Why this priority:** temps start and end without the person; the person must still see each one and be able to end it, or the office does work nobody watches.

**Independent test:** the person creates 2 temps; lead L1 creates 3 member temps. The temps view lists all 5 with creator, objective, status, run time, and quiet time. The person ends one of L1's temps from the view with the keyboard only; after one confirmation it leaves the list and a finished entry with the reason "ended by the person" appears.

**Scenarios:**

1. Given the person opens the create form, When the person turns on "temp" and gives an objective, Then the office creates and starts a temp and types the objective.
2. Given the person turns on "temp" with a blank objective, When the person submits the form, Then the office creates nothing and says that a temp needs an objective.
3. Given temps exist at several places in the tree, When the person opens the temps view, Then it lists every temp in the office.
4. Given the person ends a temp from the temps view and confirms, Then the office ends it with the reason that the person ended it.
5. Given the person stops a temp from its details or its terminal view, Then the office ends it in the same way.
6. Given a finished entry, When the person dismisses it, Then the entry leaves and every kept worktree and branch stays.
7. Given a temp runs, When the person or its parent starts a new task on it or continues one of its sessions, Then the temp stays a temp and does not end.

### US3 — Temps are bounded (Priority: P1)

The office never holds more temps than the cap, and it ends a temp that produced no output for the idle limit, with a reason that says so.

**Why this priority:** without the bounds temps are a new way for one looping agent to fill the machine, and a stuck temp would wait forever.

**Independent test:** set the cap to 3 and the idle limit to 5 minutes. With 3 temps, a 4th create, by an agent and by the person, is refused with the cap in the reason. Let one temp wait for input with no output and dismiss its Needs you item: within 6 minutes it ends, its finished entry says "no output for 5 minutes", and its parent's terminal shows the end notice.

**Scenarios:**

1. Given 3 temps exist and the cap is 3, When an agent or the person creates a temp, Then the office creates nothing and refuses with the cap and its value.
2. Given an agent is refused 5 times in 60 seconds by the cap, Then the office puts it on no hold (the cap is not a rate limit).
3. Given the person lowers the cap below the number of temps, Then the office ends no temp and refuses new temps until the count is below the cap.
4. Given a temp produced no output for the idle limit, no child of it runs, and no hold covers it, Then the office ends it within 60 seconds with the reason that it produced no output for that many minutes.
5. Given a temp prints output more often than the idle limit, Then the office does not end it.
6. Given a hold covers a temp, one of its children runs, or it has an item in Needs you, Then the office does not end it for the idle limit.
7. Given the office ends a temp for the idle limit and its parent runs, Then the parent's queue gets one end notice that names the temp, its id, and the reason; when it has no running parent, the person gets one notification.

### US4 — A temp whose parent goes away ends with it (Priority: P2)

When the person deletes a temp's parent, or a temp's parent is itself a temp that ends, the temp ends too, and its results stay.

**Why this priority:** without this rule a temp could outlive the agent it reports to, and nobody would read its `done` report. It is P2 because deleting a parent already deletes its descendants (0002/FR-006); this story adds the result keeping and the reason.

**Independent test:** lead L1 has 3 member temps. The person deletes L1 and keeps the worktrees. 3 finished entries appear with the reason that their parent L1 was deleted, and their worktrees and branches stay.

**Scenarios:**

1. Given lead L1 has member temps, When the person deletes L1 with its descendants (0002/FR-006), Then each temp ends with the reason that its parent was deleted, and its worktree follows the person's keep or remove answer (0010/FR-005).
2. Given lead temp LT has member temps, When LT ends for any reason, Then the office ends LT's member temps first, each with the reason that its parent ended.
3. Given the person stops lead L1 with its descendants (0002/FR-007), Then each temp among them ends, and L1 and the other descendants stop as before.

## Requirements

### ADDED

- **FR-001** (US1, US2): WHEN the person, or a running agent as 0002/FR-008 allows, creates an agent with the temp choice on and an objective that follows the text rules of a directive (0002), THE SYSTEM SHALL create the agent as a temp at the place the 0002 rules allow, start it at once, and put the objective into the temp's queue as its first entry, typed under the rules of 0012 as for a directive whose sender is the creator; IF the objective is missing or breaks those rules, THEN THE SYSTEM SHALL create nothing, give the reason, and record the refusal; and WHEN a temp creates an agent, THE SYSTEM SHALL create it as a temp.
- **FR-002** (US1): WHERE the working folder of a new temp is inside a git repository, THE SYSTEM SHALL create the temp with its own branch and worktree as 0010/FR-002 does, unless the creator turns own worktree off in the create; WHERE the folder is not inside a git repository, THE SYSTEM SHALL create the temp without one.
- **FR-003** (US3): IF a create would make the number of temps in the office greater than the temp cap, THEN THE SYSTEM SHALL create nothing, refuse it with a reason that names the cap and its value, record the refusal, and SHALL NOT count the refusal toward any rate limit or the automatic hold (0007/FR-008); THE SYSTEM SHALL let the person set the temp cap in the office settings as a whole number from 1 to 100, 8 by default, and SHALL apply a changed value to the next create without ending any temp.
- **FR-004** (US3): WHILE a temp is running or asleep, no child of it runs, no hold covers it, and it has no item in Needs you (0008), THE SYSTEM SHALL end the temp within 60 seconds after the time since its quiet-since moment reaches the idle limit, with the reason that it produced no output for that many minutes; THE SYSTEM SHALL let the person set the idle limit in the office settings as a whole number of minutes from 5 to 1440, 30 by default, and SHALL apply a changed value at the next check.
- **FR-005** (US1, US2, US3, US4): THE SYSTEM SHALL end a temp, with the reason in brackets, WHEN it accepts a `done` report from the temp (done), WHEN FR-004 applies (no output for N minutes), WHEN the temp's CLI exits or fails other than by a stop or a sleep (exited with its exit code, or failed with its reason), WHEN the person or an agent stops the temp, alone or with an ancestor (stopped by the actor), WHEN the person ends it from the temps view (ended by the person), WHEN the person deletes it, alone or with an ancestor (deleted by the person, or its parent was deleted), and WHEN its parent temp ends (its parent ended); SHALL decide each end on the temp's status as it is under the office's order of status changes, so that a status that a new task or a continued session (0001/FR-024, 0001/FR-025) passes through ends no temp; and SHALL NOT end a temp only because it is `interrupted`.
- **FR-006** (US1, US4): WHEN the system ends a temp, THE SYSTEM SHALL end its child temps first, stop its CLI as the person's stop does when it runs, apply the keep rule to its own worktree (or, for a delete, the person's keep or remove answer of 0010/FR-005), remove the agent as a delete does, add one finished temp entry, and add one action record entry for the end with the reason and with the person, the stopping agent, or the office as the actor; and IF the office cannot remove a worktree or a branch that the keep rule removes, THEN THE SYSTEM SHALL keep it and name it and the failure in the finished entry.
- **FR-007** (US3): WHEN the system ends a temp because of the idle limit or because its CLI exited or failed, THE SYSTEM SHALL put one end notice, with the temp's name in visible form, its id, and the reason, into the parent's queue under 0015/FR-004 and 0015/FR-010 as a report of that temp WHERE the parent runs and its queue has room; and, for an end because of the idle limit whose notice reaches no queue, THE SYSTEM SHALL notify the person once with the temp's name and the reason (an exit or a failure notifies the person under 0003/FR-006 as before).
- **FR-008** (US1, US2): THE SYSTEM SHALL keep each finished temp entry across server and machine restarts until the person dismisses it, and a dismiss SHALL remove only the entry, never a worktree, a branch, or a file.
- **FR-009** (US2): THE SYSTEM SHALL show the person, in one temps view of the desktop app, every temp with its name, its creator, its parent, its objective, its status, its run time, and its quiet time, and every finished temp entry, newest first; SHALL let the person end each temp and dismiss each entry from that view with the keyboard alone, ending after one confirmation that says the temp will be removed, that work not merged stays, and that files git ignores are removed with a removed worktree, and SHALL ask the same confirmation when the person stops a single temp from its details or its terminal view; and SHALL mark a temp as "temp", as text, in its details.

### MODIFIED

- **0015/FR-005** (US1): IF the sender's parent is not running, THEN THE SYSTEM SHALL refuse a report to it, say that the parent is not running, and record the refusal; IF a waiting report's sender is no longer a child of the report's parent, THEN THE SYSTEM SHALL withdraw it as 0012/FR-005 withdraws a directive whose target left the sender's subtree, EXCEPT a waiting report or end notice of a temp that the system ended, which stays in the queue.

### PRESERVED

- **PB-1:** An agent that is not a temp keeps every behavior it has: the office does not end, remove, or idle-limit it, and an agent-made create of an agent that is not a temp has no own worktree (0010 A7).
- **PB-2:** The 0002 tree rules stay as they are: the CEO creates team leads, a team lead creates members of its team, a member creates no agents, and every other agent request is refused and recorded (0002/FR-008, 0002/FR-009). A temp create follows them.
- **PB-3:** Holds and rate limits apply to a temp create as to any agent-made create (0007); the person's own actions are never held or rate-limited (0007/PB-1). The temp cap is not a rate limit; it applies to every create of a temp.
- **PB-4:** A delete by the person still asks once whether to keep or remove own worktrees, with keep as the default, and asks again for a worktree with uncommitted changes (0010/FR-005, 0010/FR-006). The office removes no worktree with uncommitted changes and no branch with commits that its base lacks without that answer.
- **PB-5:** A temp's reports follow every 0015 rule: the parent from the tree, the visible form, the queue and its limits, the rate limit, and Needs you for a report for the person. An objective and an end notice are typed under the 0012 rules: queue, readiness, question check, and the input-line check before Enter.
- **PB-6:** The office starts no agent by itself after a restart (0001/FR-004, 0013/PB-1); an `interrupted` temp waits until the person resumes, ends, or deletes it.
- **PB-7:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).

## User experience

**Flow (a lead and a member temp):**

1. Lead L1 creates a member temp with an objective.
2. The office creates the temp with its own worktree, starts it, and types the objective when the temp waits for input.
3. The temp works and sends a `done` report.
4. L1's terminal shows the report; the office ends the temp.
5. The person opens the temps view and sees the finished entry with the report and the kept branch.

**Flow (the person ends a temp):**

1. The person opens the temps view; it lists every temp with its quiet time.
2. The person picks a temp and presses End; a confirmation says the temp will be removed and that work not merged stays.
3. The person confirms; the temp leaves the list and a finished entry appears.

**Visible states:**

| State | What the person or the parent sees | Covered by |
|---|---|---|
| Temp running | The temps view row: name, creator, parent, objective, status, run time, quiet time; the details say "temp". | FR-009 |
| Objective waiting | The temp's queue holds the objective until the temp is ready, as 0012 shows a waiting directive. | FR-001 |
| Create refused | The form or the agent's answer gives the reason: no objective, objective rules, the cap with its value, or a 0002 or 0007 refusal. | FR-001, FR-003 |
| Ended: done | A finished entry with the report's text and what stayed. | FR-005, FR-006, FR-008 |
| Ended: no output | A finished entry with "no output for N minutes"; an end notice in the parent's terminal or one notification. | FR-004, FR-007 |
| Ended: exited or failed | A finished entry with the exit code or the reason; an end notice or one notification. | FR-005, FR-007 |
| Ended: stopped, ended, or deleted | A finished entry that names the actor. | FR-005 |
| Ended: parent went away | A finished entry that names the parent and what happened to it. | FR-005 |
| Kept work | The finished entry names the worktree folder and branch. | FR-006 |
| Removal failed | The finished entry names the worktree or branch that stayed and the failure. | FR-006 |
| No temps | The temps view says that there are no temps and no finished entries. | FR-009 |
| Interrupted temp | The temps view row shows `interrupted`; it waits for the person. | PB-6 |

**Design:** the temps view is one new view of the desktop app, built from the existing list and details elements; it adds no new visual style. The create form gets the temp choice and an objective field. The office settings get the temp cap and the idle limit beside the 0007 limits. The plan must not add other visuals without a design that the owner approves.

**Accessibility:**

- Status, creator, quiet time, and end reasons are text, never color or an icon alone (0001/FR-014).
- Every control of the temps view, the End confirmation, and Dismiss works from the keyboard with a visible focus; Escape closes the confirmation and ends nothing.
- A row's accessible name holds the temp's name, status, and creator; the objective and the report text are its accessible description.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text). At 360 px width and at 200% zoom the view reflows without horizontal scrolling.
- An end is announced only through its notification (FR-007) and the list change; the view does not move focus on its own.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Temp (an agent, extended) | An agent for one job. | the temp mark, objective, creator, created time | An agent of 0001 and 0002; its children are temps; removed when it ends. |
| Finished temp entry | What stays of a temp after it ends. | temp name and id, objective, creator, parent, start and end time, end reason, last `done` report (kind, text), kept worktree folder and branch or none, a removal failure | One per ended temp; kept until the person dismisses it; no link to a live agent. |
| Temp settings | The office bounds of temps. | temp cap (1 to 100, default 8), idle limit (5 to 1440 minutes, default 30) | One set per office, with the 0007 settings. |

## Success criteria

- **SC-001:** The person creates a temp with an objective in a git repository; 1 of 1 starts in its own worktree, and its terminal receives the objective with Enter once its CLI waits for input.
- **SC-002:** A lead creates 3 member temps; each sends a `done` report. 3 of 3 reports reach the lead's terminal, 3 of 3 temps are no longer agents within 10 seconds after their report, and 3 finished entries show the reports.
- **SC-003:** Of 4 temps that end with a clean worktree, an uncommitted change, a new commit, and both, the clean one's worktree and branch are removed and 3 of 3 others keep both, with 0 changed files lost.
- **SC-004:** With the cap at 3 and 3 temps in the office, a 4th create by an agent and one by the person are refused with the cap and its value in the reason (2 of 2), 0 agents are created, and 5 such refusals of one agent within 60 seconds put it on 0 holds.
- **SC-005:** With the idle limit at 5 minutes, a temp with no output and no Needs you item ends within 6 minutes, its finished entry says "no output for 5 minutes", and its running parent's terminal shows 1 end notice. A temp that prints one line every 60 seconds for 15 minutes, a temp under a hold for 10 minutes, and a temp whose Needs you item stays for 10 minutes, are not ended (0 of 3).
- **SC-006:** The person deletes a lead with 3 member temps and keeps the worktrees: 3 of 3 finished entries name the deleted parent, and 3 of 3 worktrees and branches stay.
- **SC-007:** A lead temp with 2 running member temps sends `done`: both members end before the lead, with the reason that their parent ended (2 of 2).
- **SC-008:** 5 finished entries survive a server restart (5 of 5); dismissing 1 removes the entry and 0 files, worktrees, or branches.
- **SC-009:** With 10 temps in the office at several places in the tree, the temps view lists 10 of 10, and the person ends 1 with the keyboard alone.
- **SC-010:** A temp's `done` report that waits in a working parent's queue when the temp ends is typed into the parent's terminal once the parent is ready (1 of 1).
- **SC-011:** A temp whose CLI exits with code 1 before any `done` report ends within 5 seconds, and its finished entry names exit code 1 (1 of 1).
- **SC-012:** Of 10 new tasks and 10 continued sessions on running temps, 0 end a temp.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The objective is missing, blank, longer than the directive limit, or holds a control character. | Invalid | Nothing is created; the reason is given and recorded (FR-001). |
| A member, or a member temp, asks to create a temp. | Unauthorized | Refused and recorded as 0002 refuses a member's create (PB-2). |
| An agent creates a temp outside its subtree, or with a role its place does not allow. | Unauthorized | Refused and recorded (0002/FR-009). |
| A temp asks to create an agent that is not a temp. | Boundary | The agent is created as a temp, with an objective required (FR-001). |
| The create would exceed the cap. | Boundary | Refused with the cap and its value; no rate limit or automatic hold counts it (FR-003). |
| The person lowers the cap below the number of temps. | Boundary | No temp ends; creates are refused until the count is below the cap (FR-003). |
| A temp setting outside its range, or not a whole number. | Invalid | Refused with the range; the old value stays (FR-003, FR-004). |
| The folder is not in a git repository. | Boundary | The temp has no own worktree; its results are its reports and the files it wrote (FR-002). |
| The office cannot create the branch or the worktree. | Failure | No temp is created, nothing stays behind, and the reason is given (0010/FR-004). |
| The temp's CLI cannot start (missing CLI, not signed in, missing worktree folder). | Failure | The temp ends with the reason "failed" and the cause (FR-005). |
| The objective waits in the queue longer than 0012 allows and is withdrawn. | Failure | The withdrawal is recorded as 0012 requires; the temp keeps running, and the idle limit ends it if it stays quiet (FR-004). |
| A `done` report is refused (hold, rate limit, parent not running). | Failure | The temp does not end; it may send again or mark the report for the person (0015). |
| A `done` report for the person from a temp. | Boundary | Needs you gets it and the person is notified once (0015/FR-007); the temp then ends, its item leaves (0008/FR-005), and the finished entry keeps the report (FR-008). |
| A `question`, `blocked`, or `info` report from a temp. | Boundary | Delivered as 0015 requires; the temp does not end. |
| A `done` report waits in the parent's queue when the temp ends. | Concurrent | It stays and is typed when the parent is ready (0015/FR-005 as modified). |
| A `done` report and the idle limit arrive at the same moment. | Concurrent | The temp ends once, with the reason of the event that the office handled first. |
| The person ends a temp that is already ending. | Concurrent | It ends once; the person is told that it already ended. |
| The person or the parent starts a new task on a temp, or continues one of its sessions. | Concurrent | The temp stays a temp; the stop and start inside that action end nothing (FR-005). |
| A temp is waiting for a person's answer to a question in its terminal. | Boundary | While its Needs you item stays, the idle limit does not run; after the person opens its terminal or dismisses the item, its quiet time counts from then, and the person's input resets it (FR-004). |
| A temp is asleep (0018). | Boundary | Its quiet time keeps growing, and the idle limit ends it (FR-004). |
| A hold covers the temp, one of its children runs, or it has a Needs you item. | Boundary | The idle limit does not end it; its quiet-since moment moves to the end of the hold, the child's run, or the item (FR-004). |
| The server or the machine restarts with temps. | Failure | Temps are `interrupted` and are not ended or started by the office (FR-005, PB-6); they still count toward the cap. |
| The temp's parent is stopped, but not deleted or ended. | Boundary | The temp keeps running; a `done` report to the stopped parent is refused (0015/FR-005), and the temp may mark it for the person. If the temp later ends for the idle limit, the person gets the notification (FR-007). |
| The person deletes a temp. | Boundary | The 0010 keep or remove question applies; the temp ends with the reason that the person deleted it (FR-005, PB-4). |
| The office cannot remove a worktree or a branch that the keep rule removes. | Failure | It stays; the finished entry names it and the failure (FR-006). |
| A temp wrote its results only to files that git ignores. | Boundary | Ignored files are not uncommitted changes (0010 A9), so a worktree with no other change is removed with them; the End confirmation and the finished entry say that ignored files are removed with a removed worktree. |
| The server stops while a temp ends. | Failure | An end that did not finish leaves the temp as an agent (`interrupted` after the restart); the person ends it again. |
| The parent's queue has no room for an end notice. | Boundary | The notice is not queued; for an idle-limit end the person is notified once instead (FR-007), and the finished entry keeps the reason (FR-008). |
| The person stops a lead with its descendants (0002/FR-007) and temps are among them. | Boundary | The 0002 offer applies as before; each temp among them ends with the reason that the person stopped it (FR-005, A6). |
| Many finished entries. | Boundary | The temps view lists them newest first; each stays until dismissed (FR-008). |

## Assumptions

- **A1:** The cap counts every temp that exists, whatever its status. Since a temp ends when its CLI exits or fails, only `interrupted` temps exist without running; they count so that a restart cannot double the number of temps.
- **A2:** The default cap of 8 lets a lead run a 5-member team of temps with room for the person's own temps, and is low enough that one looping agent cannot fill a laptop. The default idle limit of 30 minutes matches the 0018 default idle time.
- **A3:** The objective follows the text rules of a directive (0002): not blank, at most 8,192 characters, and no control characters. It is the temp's first queue entry, so it is typed only when the temp's CLI waits for input and shows no question (0012); the 0001 startup prompt, which is never submitted, is not used for it. The objective counts toward no directive rate limit; the create counts toward "agents created by one agent".
- **A4:** The temp choice and the objective exist only at create. Turning an existing agent into a temp, or a temp into a lasting agent, is out of scope.
- **A5:** Output means any bytes that the temp's CLI writes to its terminal. A CLI that redraws while it waits for input does not count as quiet; the idle limit then does not end it (Residual risk).
- **A6:** A stop of a temp, by anyone and alone or with an ancestor, ends it. A temp has one job, so a stopped temp would only hold a place under the cap.
- **A7:** A `done` report ends the temp even if it still has work in its terminal; the sender decides when the job is done (0015 A1).
- **A8:** The end notice is a report of the ended temp for queue purposes, so it takes the temp's one report place in the parent's queue (0015/FR-010). It has its own heading, which the contract fixes, so the parent can tell it from the temp's own report.
- **A9:** A finished entry keeps the last `done` report only, not the other reports and not the terminal output. The action record keeps every report (0015/FR-009).
- **A10:** The office never removes a kept worktree or branch later, and never merges, pushes, or rebases anything (0010 A4, A6).
- **A11:** An agent-made create of a temp passes the same fields as a create of another agent (0002, 0010) plus the temp choice and the objective, and may turn own worktree off. The details of the request are in the contract.
- **A12:** The idle limit counts only while the temp has no Needs you item, as 0018/FR-002 (b) keeps an agent with an item awake. A temp that went quiet with an item waits until the person opens its terminal or dismisses the item; only then can the idle limit end it.

## Clarifications

### Session 2026-11-23 (drafted under the owner's delegation)

- Q: What happens to a temp whose parent is deleted? → A: It ends with the reason that its parent was deleted; its worktree follows the person's keep or remove answer of the delete (FR-005, FR-006, US4). Moving temps under another parent was considered and not chosen: 0002 A9 has no move.
- Q: What does a stop of a temp do? → A: It ends the temp (A6). Keeping stopped temps was considered and not chosen: they would hold places under the cap until the person deletes them.
- Q: Does the cap apply to the person's own creates? → A: Yes; it bounds the machine, not the agent's behavior. The person can raise it (FR-003). It is not a rate limit, so 0007/PB-1 stays true.
- Q: Does a temp exit end it? → A: Yes, with the exit code as the reason (FR-005); the person learns of it through the end notice or one notification (FR-007).
- Q: Does idle sleep (0018) apply to temps? → A: It may put a temp to sleep; the idle limit counts the sleep as quiet time and ends the temp (FR-004), so 0018 needs no change.

### Session 2026-11-23 (spec review r1)

- Q: Does a new task or a continued session on a temp end it? → A: No; the end is decided on the status under the office's order of status changes, so the stop and start inside those actions end nothing (FR-005, SC-012).
- Q: Does the idle limit end a temp that waits on a person's question? → A: Not while its Needs you item stays; the quiet time counts from when the item leaves (FR-004, A12). The first draft ended it.
- Q: Are a temp's results in files that git ignores kept? → A: No; they are not uncommitted changes (0010 A9), so the confirmation and the finished entry say that they go with a removed worktree.

## Out of scope

- Starting temps on a schedule or from outside the office.
- Waiting for a place under the cap: a create over the cap is refused, not queued.
- Moving a temp under another parent, or turning a temp into a lasting agent (A4).
- Keeping a temp's terminal output after it ends (A9).
- Merging, pushing, or cleaning up kept branches (A10).
- A cap per parent or per team.
- Filters, search, or bulk actions in the temps view beyond End and Dismiss per row.
- An agent request to read finished entries or to end a temp other than by a stop.
