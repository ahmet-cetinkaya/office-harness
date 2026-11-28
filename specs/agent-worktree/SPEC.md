# Agent worktrees and dependencies

## Intent

An agent can work in its own git worktree, which the person opts into. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0010-agent-worktree](../changes/archive/0010-agent-worktree/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0009: [Agent and team templates](../agent-templates/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)
- 0020: [Temporary agents](../temporary-agents/SPEC.md)

## Change 0010: Opt-in own worktree per agent

### Intent

The person runs several agents on one git repository. Without this change they share one working folder, so two agents that edit the same file overwrite each other's work. This change lets the person give an agent its own git branch and worktree when they create it in a git repository. The agent then works in its own folder on its own branch, and the person's own folder stays unchanged. The choice is off by default, so an agent still starts as the person's shell would. Deleting the agent never removes uncommitted work without a second confirmation.

### Terms

| Term | Meaning |
|---|---|
| Agent, project, working folder, action record | As defined in 0001. |
| Git repository | A folder that git manages, with its history and branches. |
| Worktree | A second checked-out folder of a git repository, on its own branch, that shares the repository's history. |
| Own worktree | The choice, at agent creation, that gives the agent a new branch and a worktree of its own. |
| Base | The branch or commit from which the agent's new branch starts. |
| Uncommitted changes | Changed, added, deleted, or untracked files in a worktree that no commit holds. |

### Requirements

- **0010/FR-001** (US1): WHERE the working folder of a new agent is inside a git repository, THE SYSTEM SHALL offer an "own worktree" choice, off by default, that shows the base and the branch name it will create.
- **0010/FR-002** (US1): WHEN the user creates an agent with own worktree on, THE SYSTEM SHALL create a new branch from the base and a worktree of that branch in a folder that office-harness owns outside the repository, and SHALL use that worktree, at the same subfolder as the chosen folder, as the agent's working folder.
- **0010/FR-003** (US1): THE SYSTEM SHALL show the branch, the base, and the worktree folder in the details of each agent that has an own worktree.
- **0010/FR-004** (US1): IF the office cannot create the branch or the worktree, THEN THE SYSTEM SHALL create no agent, leave no branch or folder behind, and say why.
- **0010/FR-005** (US2): WHEN the user deletes one or more agents that have own worktrees, THE SYSTEM SHALL ask once whether to keep or remove those worktrees, with keep as the default, and SHALL list each worktree with whether it has uncommitted changes.
- **0010/FR-006** (US2): IF the user chooses remove and a worktree has uncommitted changes, THEN THE SYSTEM SHALL name the number of changed files and remove that worktree only after a second confirmation.
- **0010/FR-007** (US2): WHEN the office removes a worktree, THE SYSTEM SHALL remove its folder and the repository's record of it, SHALL delete the agent's branch only when the branch has no commits that its base lacks, and otherwise SHALL keep the branch and name it.
- **0010/FR-008** (US1): IF an agent's worktree folder is missing when the agent starts, THEN THE SYSTEM SHALL NOT start the CLI, and SHALL say that the worktree folder is missing.
- **0010/FR-009** (US1): WHEN the user creates an agent or a team from a template whose entry has own worktree on, THE SYSTEM SHALL create each such agent with its own branch and worktree as FR-002 does, from the base of the chosen folder; IF the folder is not inside a git repository, THEN THE SYSTEM SHALL create nothing (0009/FR-005) and say that own worktree needs a git repository.
- **0009/FR-001** (US1): THE SYSTEM SHALL let the user save an agent template with a template name, an agent name pattern, a CLI command, an optional working folder, office additions, an optional role, and the own worktree choice (0010), either as a new template or as a copy of an existing agent's definition.
- **0009/FR-003** (US2): THE SYSTEM SHALL let the user save a team template with a template name, one lead entry, and one or more member entries, where each entry is an agent template, with its own worktree choice, and each member entry has a count.

Modified by later changes: none.

This change modified 0009/FR-001 and 0009/FR-003; their final text is the last two entries above.

### Preserved behaviors

- **0010/PB-1:** An agent created without own worktree starts in its chosen folder as the user's shell would, as before (0001/FR-020).
- **0010/PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), and nothing in the user's own repository folder; it adds only a branch and a worktree record to the repository's git data.
- **0010/PB-3:** Deleting an agent with descendants still shows how many agents will be deleted and needs the user's confirmation (0002/FR-006).
- **0010/PB-4:** Each create and delete is recorded in the action record with its actor (0001/FR-011).
- **0010/PB-5:** The office keeps each agent's definition across restarts (0001/FR-003), including its branch, base, and worktree folder.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Agent (extends 0001) | One agent CLI that office-harness runs. | own worktree (yes or no) | As in 0001. |
| Agent worktree | The branch and folder of an agent with own worktree. | repository, base, branch name, worktree folder | Belongs to exactly one agent while the agent exists. A kept worktree belongs to no agent after the delete. |

### Success criteria

- **0010/SC-001:** Two agents with own worktrees on one repository change the same file; 0 changes of one show in the other's worktree, and the user's repository folder shows 0 changed files.
- **0010/SC-002:** After the user deletes 10 agents with own worktrees and no uncommitted changes, choosing remove, 0 of their worktree folders, 0 worktree records, and 0 of their branches without own commits remain.
- **0010/SC-003:** 0 worktrees with uncommitted changes are removed without the second confirmation.
- **0010/SC-004:** In a folder that is not inside a git repository, "own worktree" is offered 0 times, and with the choice left at its default, 0 worktrees are created.
- **0010/SC-005:** After a restart of the background server, 100% of agents with own worktrees show the same branch, base, and worktree folder in their details.

### Assumptions

The bare `A1` to `A9` in the text above and in the notes name these assumptions of 0010. Later changes cite A1, A4, A6, A7, A8, and A9 by id.

- **A1:** The office-owned folder for worktrees is outside every repository, so the user's repository needs no ignore entry and the CLI's configuration is untouched.
- **A2:** The base is the branch checked out in the chosen folder, or its commit when no branch is checked out. The user can pick another local branch in the create form. The base is never stored in a template; it is read from the chosen folder at creation (FR-009). The 0009 template file carries the own worktree choice as one more template value, so 0009 SC-002 covers its export and import.
- **A3:** The branch name is `office-harness/<agent name>-<short id>`; when that name exists, the office uses the next free short id. Shipped differently, as the plan's D3 records (accepted in the plan review r2): the branch is `office-harness/<slug>` (the agent name lower-cased), or the lowest free `office-harness/<slug>-<n>` from 2, with no short id, because the form shows the name before the agent id exists. Characters that git does not allow in a branch name are replaced by `-`; the form shows the resulting name before create (FR-001).
- **A4:** A new task and a continued session (0001/FR-024, 0001/FR-025) keep the same worktree and branch. The office does not commit, merge, push, or rebase anything.
- **A5:** The choice exists only when an agent is created. Turning it on or off for an existing agent is out of scope.
- **A6:** A kept worktree is no longer managed by the office after the delete: the office shows its folder and branch once, and the user owns them from then on.
- **A7:** Only the user's own create offers the choice in this change. Agents that create agents under 0002 create them without own worktree. Later, 0020/FR-002 gives a temp an own worktree by default when its folder is in a git repository, unless its creator turns it off; an agent that is not a temp still gets none from another agent (0020/PB-1).
- **A8:** Untracked files of the user's folder, such as local environment files, are not copied into a worktree; the create form says so.
- **A9:** Files that git ignores are not counted as uncommitted changes and are removed with the worktree; the delete question says so (FR-005).

### Out of scope

- Merging branches, pull requests, rebasing, and conflict help.
- Committing, pushing, or cleaning up an agent's work (A4).
- Separate history per agent; worktrees share the repository's branches and commits.
- Turning own worktree on or off for an existing agent (A5).
- Own worktree for agents that agents create (A7). Temps are the exception that 0020/FR-002 adds.
- Managing kept worktrees after the agent is deleted (A6).
- Copying untracked files into a worktree (A8).
- Version control systems other than git.

### Notes from Deviations

- **Branch name (A3).** `office-harness/<slug>`, then `-2`, `-3`, and so on, with no short id, because the form shows the name before the agent exists (plan D3).
- **Delete answer.** `agent.delete` answers `{deleted_branches, kept_branches, kept_folders}` for every delete, also for an agent without a worktree (empty lists), instead of `null`.
- **Delete refusal text.** A subtree delete passes the agent module's refusal through without a `conflict: ` prefix, so a changed worktree reads `the worktree of <name> changed; confirm again` on a single and on a subtree delete.
- **Unknown own commits.** When git cannot count the branch's own commits, `own_commits` is `null` and the branch is kept.
- **Hooks off (D1).** The plan named an empty `.no-hooks` folder in the data folder as `core.hooksPath`. An agent can write into that folder, so every command now uses the null device instead, and also switches off hooks that git's configuration names (`hook.<event>.enabled=false`).
- **Stale subtree member (D6).** The plan kept a member whose worktree changed and deleted the others. A subtree delete now refuses as a whole and deletes nobody, so one confirmation still covers one known set; members already stopped stay stopped. The worktree protocol states this.
- **Partial delete.** A delete that fails after some agents went names, in its error text, the branches it deleted and the branches and folders it kept for them. A single delete removes the agent's row before git touches its worktree. The branch is deleted with `update-ref -d` and the commit its own commits were counted at, so a commit made after the count keeps the branch.
- **Template from an agent with its own worktree (FR-009).** `template.from_agent` copies the choice and writes the repository folder, with the agent's subfolder, as the working folder of the body, not the worktree folder that the office owns, because the base and the worktree are made again at each creation (A2).
- **Empty repository folder after an undo.** A discard removes the agent's worktree folder and its branch. The empty per-repository folder in the worktrees folder stays, as it does after any discard.
- **Repository of a linked worktree.** When the chosen folder is inside a linked worktree (for example another agent's), the stored repository is the main worktree, so the agent stays reachable after that linked worktree is removed.
- **None of the notes defers or changes a requirement.**
- **Edge cases, user stories, the user experience, and the Clarifications** are in the archived spec.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0010-agent-worktree - 2026-11-28
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
