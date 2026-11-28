# Installed packages in new agent worktrees

## Intent

A new agent worktree gets the installed packages of its project, so an agent can build and test at once. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0021-worktree-dependencies](../changes/archive/0021-worktree-dependencies/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0010: [Agent worktrees and dependencies](../agent-worktree/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0021: Installed packages in new agent worktrees

### Intent

When the office creates an agent's own worktree (0010), the new worktree has no installed packages, because git does not check out the files that it ignores. The agent must run a full install before it can build or test, and many agents mean many installs. This change gives a new worktree links to the installed Node.js packages of the base checkout, so the agent's CLI can build and test at once. The links never count as work: git ignores them, the delete question does not count them, and the office removes them without following them, so it never changes the base checkout. A new office setting, "Worktree dependencies", on by default, lets the person turn this off.

### Requirements

- **0021/FR-001** (US3): THE SYSTEM SHALL let the person turn "Worktree dependencies" on and off in the office settings, on by default, with a help text that says that new own worktrees share the checkout's installed Node.js packages and that an install, update, or removal of packages in such a worktree changes the checkout's packages; SHALL keep the setting in the background server across server and machine restarts; SHALL apply a change to the creates that begin after it and leave the dependency links of existing worktrees as they are; and SHALL show the state of the setting as text: loading, on, off, the reason when it cannot be loaded, and the reason when a change cannot be saved, with the previous value kept.
- **0021/FR-002** (US1): WHEN the office creates an agent's own worktree (0010/FR-002, 0010/FR-009) WHILE Worktree dependencies is on, THE SYSTEM SHALL make dependency links at each dependency place that meets the conditions of FR-003, before the agent's CLI starts, so that the worktree's dependency folder at that place gives the installed packages of the base checkout at the same place.
- **0021/FR-003** (US1, US2): THE SYSTEM SHALL make dependency links at a dependency place only when all of these hold: (a) Worktree dependencies is on; (b) the office does not run on Windows; (c) the base checkout has a dependency folder at that place; (d) git ignores that dependency folder in the base checkout; (e) git, inside the new worktree, ignores a dependency folder at that place; (f) the new worktree has nothing (no file, folder, or link) named `node_modules` at that place; and at any other place THE SYSTEM SHALL make, change, and remove nothing, and SHALL create the agent as 0010 does.
- **0021/FR-004** (US2): IF a dependency link would reach an outside target after every link on its way is followed, THEN THE SYSTEM SHALL NOT make that link, SHALL write the place and the reason to the background server's log, and SHALL go on with the other dependency links and with the create.
- **0021/FR-005** (US2): WHILE a worktree has dependency links, THE SYSTEM SHALL keep them out of every list of changes: git inside the worktree ignores them, so the delete question (0010/FR-005, 0010/FR-006) counts none of them and the agent's own `git status` lists none of them; and THE SYSTEM SHALL write nothing for them into the repository's git data or configuration beyond the branch and the worktree record of 0010 (no ignore or exclude entries), and nothing into the base checkout.
- **0021/FR-006** (US2): WHEN the office removes a worktree (0010/FR-007) or undoes a create that failed (0010/FR-004), THE SYSTEM SHALL first remove each dependency link that it made in that worktree and that still points into the base checkout, without following it, also after a restart of the background server; SHALL then remove the worktree as 0010 does; and SHALL follow no link during the removal, so that it changes and removes no file or folder in the base checkout or anywhere else outside the worktree folder.
- **0021/FR-007** (US1): IF the office cannot make a dependency link at a dependency place (for example, the disk is full or the file system has no links), THEN THE SYSTEM SHALL remove the dependency links that it made at that place and leave nothing else that it made there, SHALL write the place and the reason to the background server's log, SHALL go on with the other places, and SHALL create the agent; THE SYSTEM SHALL NOT fail a create because of a dependency link.
- **0021/FR-008** (US3): WHILE own worktree is on in the create form, THE SYSTEM SHALL show a note that says that untracked files, such as local environment files, are not copied (0010 A8), and, WHILE Worktree dependencies is on and the office does not run on Windows, that installed Node.js packages are shared from the checkout and that an install, update, or removal of packages in the worktree changes the checkout's packages; otherwise, that installed packages are not shared, so the agent installs them.

### Preserved behaviors

- **0021/PB-1:** An agent created without own worktree starts in its chosen folder as the person's shell would, with no dependency link (0010 PB-1, 0001/FR-020).
- **0021/PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), and nothing in the person's own repository folder; it adds only a branch and a worktree record to the repository's git data (0010 PB-2).
- **0021/PB-3:** The delete question still lists each worktree with whether it has uncommitted changes, and the second confirmation still counts every changed, added, deleted, or untracked file that git does not ignore (0010/FR-005, 0010/FR-006).
- **0021/PB-4:** IF an agent's worktree folder is missing when the agent starts, THEN the office does not start the CLI and says that the worktree folder is missing (0010/FR-008).
- **0021/PB-5:** Other files that git ignores in a worktree are still not counted as uncommitted changes and are still removed with the worktree (0010 A9).
- **0021/PB-6:** A create that fails for a reason of 0010 still creates no agent and leaves no branch, folder, or dependency link behind (0010/FR-004).
- **0021/PB-7:** Each create and delete is recorded in the action record with its actor, as before (0001/FR-011, 0010 PB-4). The dependency links add no entry.

### Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Worktree dependencies setting | Whether the office makes dependency links in new own worktrees. | on or off | One per office; kept by the background server. |
| Dependency link | One link that the office made in an agent worktree to installed packages of the base checkout. | place in the worktree, target in the base checkout | Belongs to exactly one agent worktree; removed before that worktree. In a kept worktree, it belongs to no agent. |
| Agent worktree (extends 0010) | The branch and folder of an agent with own worktree. | the dependency links that the office made in it, kept across restarts | As in 0010. |

### Success criteria

- **0021/SC-001:** In a repository whose base checkout has installed, ignored dependency folders at the top and in one workspace folder, an agent in a new own worktree runs the project's test command with 0 installs before it, and the command passes.
- **0021/SC-002:** Right after the create, `git status` in the worktree lists 0 entries, and the delete question counts 0 changed files for the worktree.
- **0021/SC-003:** After the office removes 10 such worktrees, 5 of them after a restart of the background server, and undoes 3 failed creates, 0 files in the base checkout's dependency folders are changed or missing, compared by name, size, and content with a listing made before the creates.
- **0021/SC-004:** With Worktree dependencies off, 10 creates with own worktree make 0 dependency links.
- **0021/SC-005:** In a base checkout where one package of the top dependency folder is a link to a folder outside the base checkout, and the dependency folder of a second place is itself a link to a folder outside it, a create makes 0 dependency links that reach an outside target, makes a link for every other package, and creates the agent.
- **0021/SC-006:** On Windows, 10 creates with own worktree make 0 dependency links, and 100% of them create the agent as 0010 does.
- **0021/SC-007:** On a file system without links, 10 creates with own worktree create 10 agents, leave 0 dependency links, and write 1 log line with the reason for each place that met the conditions of FR-003.

### Notes from Deviations

- **Log line.** The reasons of 0021/FR-004 and 0021/FR-007 reach the background server's log as plain lines. Each line starts with `dependency links: ` and then names the place and the reason. No storage alert shows in the app.
- **Where a place's links are found at removal (0021/FR-006).** The office finds the dependency places again from the worktree's tracked `package.json` files. A place whose `package.json` the agent deleted keeps its links until git removes the worktree. Git's removal deletes a link without following it, so the base checkout is still never changed.
- **None of the notes defers or changes a requirement.**

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0021-worktree-dependencies - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
