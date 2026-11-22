---
status: draft
capability: office
plan: specs/changes/0013-bulk-resume/PLAN.md
supersedes:
---

# 0013: Resume all interrupted agents

## Intent

When the background server stops while agents run (a crash, a logout, a machine restart), every agent that ran shows `interrupted` at the next start, and none starts by itself (0001/FR-004). With 20 or 30 agents, the user must open each agent, pick its last session, and continue it, one by one. When a CLI cannot continue its session, for example because it does not accept the session id, the agent ends at once and the user must notice it and start it again. This change adds one confirmed action that resumes every interrupted agent that the user checks. When a CLI refuses to continue, the office starts that agent once in a new session. An agent that still fails is put on hold instead of being started again and again.

## Context

- Source of truth: `docs/PRD.md`. References such as `0001/FR-025` point to requirements of the approved changes.
- This change depends on 0001 (statuses, sessions, continuing a session, the action record), 0002 (the tree of CEO, leads, and members), 0003 (notifications), and 0007 (the agent hold and the office as an actor).
- The office still starts no agent by itself: the user opens the action and confirms it. 0001/FR-004 and the 0001 out-of-scope entry "Automatic restart of interrupted agents" stay as they are.
- Research R1 (`research.md`) killed the server with SIGKILL while agents ran. A CLI that ends on the terminal hang-up was gone; a CLI that ignores the hang-up kept running as an orphan, and the next server start did not see it. Continuing a session after the restart worked. Sweeping such orphans is out of scope here (see Out of scope).
- The idea comes from an MIT-licensed reference office app; the pull request names it. No code is taken from it; this spec describes behavior only.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, session, action record, office additions | As defined in 0001. |
| CEO, lead, member | The roles of the tree, as defined in 0002. |
| Hold | An agent hold as defined in 0007: agent-made actions of the agent and its descendants are refused; the user's own actions still work. |
| Interrupted agent | An agent with status `interrupted` (0001/FR-004). |
| Last session | The most recent session of an agent. |
| Resume | Start an interrupted agent again: continue its last session where the CLI supports it, otherwise start a new session. |
| Check period | The 20 seconds after a start in which the office watches whether the session ends by itself (A2). |
| Ends by itself | The CLI's process ends without a stop, a new task, or a continue from the user, and without a server stop. |

## User stories

### US1: Resume every interrupted agent with one confirmed action (Priority: P1)

After a restart, the user sees how many agents are interrupted, opens one list, checks or unchecks agents, and confirms. The office resumes each checked agent.

**Why this priority:** it is the value of the change. One confirmed action replaces one action per agent.

**Independent test:** with 5 agents running (3 that can continue a session, 2 plain commands that cannot), stop the server with SIGKILL and start it again. Open "Resume interrupted (5)" and confirm. The 3 continue their last sessions, the 2 start new sessions, and each start is in the action record with the user as actor.

**Scenarios:**

1. Given 3 agents are interrupted, When the user looks at the panel, Then it offers "Resume interrupted (3)".
2. Given no agent is interrupted, When the user looks at the panel, Then it does not offer the action.
3. Given the user opens the action, When the list shows, Then it names each interrupted agent in start order with what will happen to it: continue the session that started at a given time, or start a new session, with the reason. Every agent is checked, except an agent on hold, which is unchecked and marked "on hold".
4. Given the user unchecks one agent and confirms, When the office resumes, Then the unchecked agent stays `interrupted`.
5. Given a CEO, a lead, and two members of that lead are interrupted, When the user confirms, Then both members start before the lead and the lead starts before the CEO.
6. Given the user started one listed agent by hand after the list opened, When the user confirms, Then the office skips that agent and says that it is no longer interrupted.

### US2: Start a new session when the CLI refuses to continue (Priority: P1)

When the CLI ends at once instead of continuing the session, the office starts the agent once in a new session and tells the user why.

**Why this priority:** without it, an agent whose CLI refuses the old session stays dead after the bulk action, and the user must find it among many. It ships with US1.

**Independent test:** give one agent a CLI that ends 1 second after it is asked to continue a session. Resume it with the bulk action. The office starts it once in a new session, the result says that the CLI ended 1 second after the continue, and the agent's action record shows that start with the office as actor and the same reason.

**Scenarios:**

1. Given the office continued an agent's last session, When that session ends by itself within the check period, Then the office starts the agent once in a new session, and both the result and the agent's action record say why.
2. Given the agent's CLI cannot continue its last session (the CLI does not support it, or the office does not know the CLI's session id), When the office resumes it, Then it starts a new session at once, and both the result and the agent's action record say why.
3. Given the continued session still runs at the end of the check period, When the period ends, Then the result is "continued" and the office does nothing more.
4. Given the user stops the agent, starts a new task, or continues a session within the check period, When the session ends, Then the office does not start the agent again and the result stays as it was.

### US3: Hold an agent that keeps failing (Priority: P2)

When the new session also ends at once, the office stops trying, puts the agent on hold, and tells the user.

**Why this priority:** it stops a start loop that would hide the real fault, such as a CLI that is not signed in. US1 and US2 work without it, but a failing agent would then stay in the next bulk action without a mark.

**Independent test:** give one agent a CLI that ends 1 second after every start. One bulk action starts it at most twice; then the agent is on hold with the reason, one notification names it, and the next bulk action lists it unchecked and marked "on hold".

**Scenarios:**

1. Given the office started a new session for an agent in this action, When that session ends by itself within the check period or does not start, Then the office does not start the agent again, puts it on hold with the office as actor and the reason, and notifies the user once, naming the agent.
2. Given an agent is on hold, When the user checks it in the list and confirms, Then the office resumes it like any other checked agent and the hold stays until the user releases it.

## Requirements

### ADDED

- **FR-001** (US1): WHILE at least one agent of the office is `interrupted`, THE SYSTEM SHALL offer a "Resume interrupted" action that shows the number of interrupted agents.
- **FR-002** (US1, US3): WHEN the user opens the "Resume interrupted" action, THE SYSTEM SHALL list every interrupted agent in start order, with what will happen to it (continue its last session, shown with that session's start time, or start a new session, shown with the reason) and with whether it is on hold, SHALL check every listed agent that is not on hold, and SHALL start nothing before the user confirms.
- **FR-003** (US1, US2): WHEN the user confirms, THE SYSTEM SHALL, for each checked agent that is still `interrupted`, continue its last session where the agent's CLI supports continuing it (0001/FR-025) and otherwise start a new session (0001/FR-024), SHALL start every member before its lead and every lead before the CEO, and SHALL skip each checked agent that is no longer `interrupted`, with that reason.
- **FR-004** (US2): IF a session that this action continued ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL start the agent once in a new session and SHALL record that start in the agent's action record with the office as actor and the reason (which session the CLI refused, and how).
- **FR-005** (US3): IF a new session that this action started ends by itself within the check period, or does not start, THEN THE SYSTEM SHALL NOT start the agent again, SHALL put the agent on hold (0007/FR-003) with the office as actor and the reason, and SHALL notify the user once, naming the agent.
- **FR-006** (US1, US2, US3): THE SYSTEM SHALL show the result of each checked agent as text as soon as it is known: checking (during the check period), continued, new session (with the reason), on hold (with the reason), or skipped (with the reason).
- **FR-007** (US2): WHEN this action starts a new session instead of continuing the last one, for any reason, THE SYSTEM SHALL keep the reason with that start's entry in the agent's action record, so the user sees in the agent's details that the CLI's earlier context was not carried over.

### PRESERVED

- **PB-1:** No agent starts without an explicit action of the user; the server starts no interrupted agent by itself at its start (0001/FR-004).
- **PB-2:** The user can still continue any previous session of an agent from its details, also a session that the CLI refused before (0001/FR-025).
- **PB-3:** A hold refuses only agent-made actions; the user's own actions, this action included, work as before (0007 PB-1, PB-2).
- **PB-4:** office-harness creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021).
- **PB-5:** Each start, continue, and hold is recorded in the action record with its actor (0001/FR-011, 0007/FR-010).

## User experience

**Flow:**

1. After a server start, the panel header offers "Resume interrupted (N)".
2. The user opens it. A dialog lists the interrupted agents in start order: name, role, what will happen, and "on hold" where it applies, each with a check box.
3. The user unchecks agents to leave out and confirms.
4. The dialog shows each checked agent's result and updates it: checking, then continued, new session with the reason, on hold with the reason, or skipped with the reason.
5. The user closes the dialog at any time. The office finishes the check periods without it, and the action record and the hold list keep what happened.
6. In an agent's details, the action record shows each new session that this action started instead of a continue, with its actor and its reason.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| No interrupted agent | No "Resume interrupted" action. | FR-001, US1 scenario 2 |
| Interrupted agents | "Resume interrupted (N)". | FR-001 |
| Loading the list | A loading text in the dialog. | FR-002 |
| List | Each agent with its check box, what will happen, and "on hold" where it applies. | FR-002 |
| Nothing checked | Confirm is unavailable, with the text that at least one agent must be checked. | Edge cases |
| Results | Each checked agent with its result text, updated live. | FR-006 |
| Request failed | The reason in the dialog; nothing started. | Edge cases |
| Agent details after a new session | The action record entry of that start, with its actor (user or office) and the reason. | FR-004, FR-007 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The action, the check boxes, confirm, and close work from the keyboard (0001/FR-028), focus is always visible, and Escape closes the dialog.
- Each result is text, never color alone; a change of a result is announced to assistive technology.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Resume result | What the action did to one checked agent. | agent, result (checking, continued, new session, on hold, skipped), reason | Shown while the dialog is open; not stored. What happened stays in the action record and the hold list. |
| Hold (extends 0007) | An agent hold. | reason, which can now also be a failed resume | Set by the office (FR-005). |
| Action record entry (extends 0001) | One recorded action. | reason (optional): why a start began a new session instead of a continue | Belongs to the agent it names (0001/FR-011). |

## Success criteria

- **SC-001:** With 30 interrupted agents, the user resumes all of them with 2 activations (open the action, confirm), and 100% of them have a started session within 60 seconds of the confirm.
- **SC-002:** With a CLI that ends within 1 second of every continue, 100% of the checked agents run a new session within 30 seconds of the confirm, and each has exactly 1 action record entry for that start with the office as actor and a reason.
- **SC-003:** An agent whose CLI ends within 1 second of every start is started at most 2 times by one bulk action, is on hold after it, and causes exactly 1 notification.
- **SC-004:** After 10 server starts that follow a SIGKILL of the server, 0 agents start without an action of the user.
- **SC-005:** In a tree of 1 CEO, 2 leads, and 4 members, all interrupted, 100% of the members start before their lead and both leads start before the CEO.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| No agent is interrupted. | Empty | The action is not offered (FR-001). |
| The user unchecks every agent. | Empty | Confirm is unavailable and says that at least one agent must be checked. |
| A listed agent was started, stopped, or deleted after the list opened. | Concurrent | It is skipped with the reason (FR-003). |
| Two clients confirm at the same time. | Concurrent | Each agent is resumed once; the later request skips the agents that are no longer interrupted (FR-003). |
| The CLI does not support continuing, or the office does not know the CLI's session id. | Boundary | A new session starts at once, with the reason (US2 scenario 2). |
| The continued CLI ends with exit code 0 within the check period. | Boundary | It counts as ending by itself (A3). |
| The agent's working folder or worktree is missing (0010/FR-008). | Failure | The continue does not start, the new session does not start either, and the agent is put on hold with the reason (FR-004, FR-005). |
| The user stops the agent within the check period. | Concurrent | The office does not start it again (US2 scenario 4). |
| The server stops during a check period. | Failure | The remaining checks end; the agents follow 0001/FR-004 at the next start. |
| An agent on hold is checked by the user. | Boundary | It is resumed; the hold stays (US3 scenario 2). |
| The office is on hold. | Boundary | The action works (a user action); the resumed agents' agent-made actions stay refused (0007/FR-002). |
| The hold of FR-005 cannot be stored. | Failure | The agent is not started again; the result says the hold failed and the storage failure shows (0001 storage alert). |
| A CLI refuses the session but keeps running, showing an error. | Boundary | Not detected; the result is "continued" (A4). |
| A CLI process from before a SIGKILL of the server still runs (R1). | Failure | Not detected. If it blocks the continue and the CLI ends, the new session of FR-004 follows (A5). |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). |

## Assumptions

- **A1:** The action covers `interrupted` agents only. Agents that are `exited`, `failed`, or `stopped` are not listed; the user ended them or they ended on their own.
- **A2:** The check period is 20 seconds from each start. It is not a setting in this change.
- **A3:** Any end of the CLI's process within the check period counts as ending by itself, whatever the exit code, because an interactive agent CLI that continues a session does not end on its own within seconds. A one-shot command that ends quickly is therefore started a second time and then put on hold; the user can uncheck it.
- **A4:** The office does not read the CLI's output to find a refusal; it watches only whether the process ends. A refusal that leaves the CLI running shows in the agent's terminal.
- **A5:** The office does not find or end CLI processes left over from an earlier server run (research R1). This change does not depend on it: a leftover that blocks a continue makes the continue fail, and FR-004 applies.
- **A6:** Members start before their lead and leads before the CEO, so that a directive that a resumed lead or CEO sends at once finds its targets running (0002 edge case: a directive to an agent that is not running is refused). Agents outside the tree start with the members. Within one level, the order is the order of the agent list.
- **A7:** The office starts the agents one after another without a pause between them.
- **A8:** The fallback of FR-004 happens at most once per agent per action. The office does not try a second continue or a second new session.
- **A9:** The hold of FR-005 uses the 0007 agent hold, so it also refuses agent-made actions of the agent's descendants (0007 A2), and the user releases it as any other hold (0007 A6). If the agent is already on hold, no second hold and no second notification are made.
- **A10:** Only the user's desktop app offers the action. Agents cannot start it; it is outside the agent call surface of 0002.
- **A11:** The result list is not stored. When the dialog closes, the action record (with the reason of each new session, FR-007), the agent statuses, and the hold list (with the reason of each hold) keep what happened.

## Out of scope

- Resuming agents automatically at server start (0001 out of scope).
- Finding and ending CLI processes left over from a server crash (research R1). A follow-up change can store each session's process identity and end only the processes whose identity still matches.
- Reading CLI output to detect a refusal (A4).
- More than one fallback per agent (A8), and a setting for the check period (A2).
- Resuming agents that are `exited`, `failed`, or `stopped` (A1).
- Starting the action from an agent (A10).
