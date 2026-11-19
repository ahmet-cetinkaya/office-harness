---
status: draft
capability: office
plan:
supersedes:
---

# 0010: Opt-in own worktree per agent

## Intent

The owner runs several agents on one git repository. Today they share one working folder, so two agents that edit the same file overwrite each other's work. This change lets him give an agent its own git branch and worktree when he creates it in a git repository. The agent then works in its own folder on its own branch, and the user's own folder stays unchanged. The choice is off by default, so an agent still starts as the user's shell would. Deleting the agent never removes uncommitted work without a second confirmation.

## Context

- Source of truth: `docs/PRD.md`. Section 4.1 says an agent starts as the user's shell would and office-harness writes nothing to the CLI's configuration; this change keeps both because it is opt-in and touches only the repository's git data and a folder that office-harness owns.
- This change depends on 0001 (agents, working folder, the action record, durability) and 0002 (deleting an agent with descendants). References such as `0001/FR-012` point to requirements in those changes.
- A worktree separates files, not the repository's history: every worktree of a repository shares its branches and commits. The office promises separate folders and branches, not a separate history.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, working folder, action record | As defined in 0001. |
| Git repository | A folder that git manages, with its history and branches. |
| Worktree | A second checked-out folder of a git repository, on its own branch, that shares the repository's history. |
| Own worktree | The choice, at agent creation, that gives the agent a new branch and a worktree of its own. |
| Base | The branch or commit from which the agent's new branch starts. |
| Uncommitted changes | Changed, added, deleted, or untracked files in a worktree that no commit holds. |

## User stories

### US1 — Give an agent its own branch and folder (Priority: P1)

When the user creates an agent in a git repository, he can turn on "own worktree". The office creates a new branch and a worktree for it, and the agent's CLI starts there.

**Why this priority:** it is the value of the change: two agents on one repository stop overwriting each other.

**Independent test:** in one git repository, create agents A and B with own worktree on. Let both change the same file. Each change shows only in its own worktree, and the user's repository folder has no change.

**Scenarios:**

1. Given the user creates an agent with a working folder inside a git repository, When he looks at the create form, Then "own worktree" is offered and is off.
2. Given the working folder is not inside a git repository, When the user creates an agent, Then "own worktree" is not offered.
3. Given "own worktree" is on, When the user looks at the create form, Then it shows the base (the branch checked out in the chosen folder) and the branch name that will be created, and says that untracked files such as local environment files are not copied.
4. Given "own worktree" is on, When the user creates agent A, Then the office creates a new branch from the base and a worktree of it in a folder that office-harness owns, and A's working folder is that worktree.
5. Given agent A has an own worktree, When the user opens A's details, Then they show A's branch, base, and worktree folder.
6. Given agents A and B have own worktrees on one repository, When both change the same file, Then each change is only in its own worktree.

### US2 — Delete an agent without losing its work (Priority: P1)

When the user deletes an agent that has an own worktree, he chooses whether to keep or remove the worktree. The office never removes uncommitted changes without a second confirmation and never deletes a branch that holds commits its base lacks.

**Why this priority:** a worktree holds the agent's work; a delete that loses it would make the feature unsafe. It ships with US1.

**Independent test:** create agent A with own worktree, add an uncommitted file, and delete A choosing "remove". The office asks a second time and names the change. Decline: the worktree stays. Delete agent B with own worktree and no changes, choosing "remove": its folder, the repository's record of it, and its branch are gone.

**Scenarios:**

1. Given agent A has an own worktree, When the user deletes A, Then the office asks whether to keep or remove the worktree, with keep as the default.
2. Given A's worktree has uncommitted changes and the user chose remove, When the office asks the second time, Then it names the number of changed files, and on cancel it keeps the worktree.
3. Given A's worktree has no uncommitted changes and A's branch has no commits that its base lacks, When the user removes the worktree, Then the folder, the repository's record of the worktree, and the branch are gone.
4. Given A's branch has commits that its base lacks, When the user removes the worktree, Then the folder and the record are gone, the branch stays, and the office says that it kept the branch and names it.
5. Given the user chose keep, When the agent is deleted, Then the folder and the branch stay, and the office shows their place and name.
6. Given a lead with 3 members that have own worktrees, When the user deletes the lead with its descendants (0002/FR-006), Then one question lists each worktree with whether it has uncommitted changes.

## Requirements

### ADDED

- **FR-001** (US1): WHERE the working folder of a new agent is inside a git repository, THE SYSTEM SHALL offer an "own worktree" choice, off by default, that shows the base and the branch name it will create.
- **FR-002** (US1): WHEN the user creates an agent with own worktree on, THE SYSTEM SHALL create a new branch from the base and a worktree of that branch in a folder that office-harness owns outside the repository, and SHALL use that worktree, at the same subfolder as the chosen folder, as the agent's working folder.
- **FR-003** (US1): THE SYSTEM SHALL show the branch, the base, and the worktree folder in the details of each agent that has an own worktree.
- **FR-004** (US1): IF the office cannot create the branch or the worktree, THEN THE SYSTEM SHALL create no agent, leave no branch or folder behind, and say why.
- **FR-005** (US2): WHEN the user deletes one or more agents that have own worktrees, THE SYSTEM SHALL ask once whether to keep or remove those worktrees, with keep as the default, and SHALL list each worktree with whether it has uncommitted changes.
- **FR-006** (US2): IF the user chooses remove and a worktree has uncommitted changes, THEN THE SYSTEM SHALL name the number of changed files and remove that worktree only after a second confirmation.
- **FR-007** (US2): WHEN the office removes a worktree, THE SYSTEM SHALL remove its folder and the repository's record of it, SHALL delete the agent's branch only when the branch has no commits that its base lacks, and otherwise SHALL keep the branch and name it.
- **FR-008** (US1): IF an agent's worktree folder is missing when the agent starts, THEN THE SYSTEM SHALL NOT start the CLI, and SHALL say that the worktree folder is missing.

### PRESERVED

- **PB-1:** An agent created without own worktree starts in its chosen folder as the user's shell would, as before (0001/FR-020).
- **PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), and nothing in the user's own repository folder; it adds only a branch and a worktree record to the repository's git data.
- **PB-3:** Deleting an agent with descendants still shows how many agents will be deleted and needs the user's confirmation (0002/FR-006).
- **PB-4:** Each create and delete is recorded in the action record with its actor (0001/FR-011).
- **PB-5:** The office keeps each agent's definition across restarts (0001/FR-003), including its branch, base, and worktree folder.

## User experience

**Flow:**

1. The user creates an agent and picks a working folder. If the folder is inside a git repository, "own worktree" shows, off.
2. He turns it on. The form shows the base, the new branch name, and a note that untracked files are not copied.
3. He creates the agent. Its details show the branch, base, and worktree folder. The CLI starts in the worktree.
4. He deletes the agent. The office asks keep or remove, with keep as the default. If he removes a worktree with uncommitted changes, it asks a second time and names the number of changed files.
5. After removal, the office says which branches it deleted and which it kept.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Not a git repository | No "own worktree" choice. | US1 scenario 2 |
| Choice off | The choice, off. | FR-001 |
| Choice on | Base, new branch name, untracked-files note. | FR-001, US1 scenario 3 |
| Creation failed | The reason; no agent created. | FR-004 |
| Agent details | Branch, base, worktree folder. | FR-003 |
| Delete question | Keep or remove, keep default; each worktree with its change state. | FR-005 |
| Second confirmation | The number of changed files; confirm or cancel. | FR-006 |
| Removal result | Branches deleted and branches kept, by name. | FR-007 |
| Worktree folder missing | The agent does not start; the reason. | FR-008 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The choice, the delete question, and the second confirmation work from the keyboard (0001/FR-028). Keyboard focus is always visible, and keep has the initial focus in the delete question.
- The change state of each worktree is text, never color alone.
- The second confirmation is announced to assistive technology with the number of changed files.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | own worktree (yes or no) | As in 0001. |
| Agent worktree | The branch and folder of an agent with own worktree. | repository, base, branch name, worktree folder | Belongs to exactly one agent while the agent exists. A kept worktree belongs to no agent after the delete. |

## Success criteria

- **SC-001:** Two agents with own worktrees on one repository change the same file; 0 changes of one show in the other's worktree, and the user's repository folder shows 0 changed files.
- **SC-002:** After the user deletes 10 agents with own worktrees and no uncommitted changes, choosing remove, 0 of their worktree folders, 0 worktree records, and 0 of their branches without own commits remain.
- **SC-003:** 0 worktrees with uncommitted changes are removed without the second confirmation.
- **SC-004:** In a folder that is not inside a git repository, "own worktree" is offered 0 times, and with the choice left at its default, 0 worktrees are created.
- **SC-005:** After a restart of the background server, 100% of agents with own worktrees show the same branch, base, and worktree folder in their details.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The chosen folder is not in a git repository. | Empty | No choice shown (FR-001). |
| The folder's repository has a detached checkout (no branch). | Boundary | The base is the checked-out commit, shown by its short id (A2). |
| The new branch name already exists. | Boundary | The office picks the next free name and shows it before create (A3). |
| The chosen folder is a subfolder of the repository. | Boundary | The agent starts in the same subfolder inside its worktree (FR-002). |
| The disk is full or the office cannot write its worktree folder. | Failure | No agent, no branch, no folder left behind; the reason shows (FR-004). |
| The user deletes the worktree folder by hand while the agent exists. | Failure | The next start is refused with the reason (FR-008); delete still offers remove, which clears the repository's record. |
| The user removes the repository, or moves it. | Failure | Start is refused because the worktree is missing or broken (FR-008); delete removes what remains. |
| The agent is running when the user deletes it. | Concurrent | The 0001 delete stops it first; the worktree question follows. |
| Deleting a lead and members with own worktrees, some with uncommitted changes. | Boundary | One question for all; the second confirmation names each worktree with changes (FR-005, FR-006). |
| The agent's branch is checked out somewhere else when the office removes the worktree. | Concurrent | The office keeps the branch and names it (FR-007). |
| The user wants own worktree on an existing agent. | Invalid | Not offered; the choice exists only at creation (A5). |
| The desktop app cannot reach the background server or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). |

## Assumptions

- **A1:** The office-owned folder for worktrees is outside every repository, so the user's repository needs no ignore entry and the CLI's configuration is untouched.
- **A2:** The base is the branch checked out in the chosen folder, or its commit when no branch is checked out. The user can pick another local branch in the create form.
- **A3:** The branch name is `office-harness/<agent name>-<short id>`; when that name exists, the office uses the next free short id.
- **A4:** A new task and a continued session (0001/FR-024, 0001/FR-025) keep the same worktree and branch. The office does not commit, merge, push, or rebase anything.
- **A5:** The choice exists only when an agent is created. Turning it on or off for an existing agent is out of scope.
- **A6:** A kept worktree is no longer managed by the office after the delete: the office shows its folder and branch once, and the user owns them from then on.
- **A7:** Only the user's own create offers the choice in this change. Agents that create agents under 0002 create them without own worktree.
- **A8:** Untracked files of the user's folder, such as local environment files, are not copied into a worktree; the create form says so.

## Clarifications

None yet.

## Open questions

- [NEEDS CLARIFICATION: Templates. The owner's main case is a team of members on one repository, and 0009 (draft) creates such teams from templates. Should an agent template and a team template entry also carry the "own worktree" choice, so a team created from a template gets one worktree per member? Recommended: yes, as a small amendment to 0009 once both changes are approved; this change keeps it out of scope.]

## Out of scope

- Merging branches, pull requests, rebasing, and conflict help.
- Committing, pushing, or cleaning up an agent's work (A4).
- Separate history per agent; worktrees share the repository's branches and commits.
- Turning own worktree on or off for an existing agent (A5).
- Own worktree for agents that agents create (A7).
- Managing kept worktrees after the agent is deleted (A6).
- Copying untracked files into a worktree (A8).
- Version control systems other than git.
