---
status: draft
capability: office
plan: specs/changes/0021-worktree-dependencies/PLAN.md
---

# 0021: Installed packages in new agent worktrees

## Intent

When the office creates an agent's own worktree (0010), the new worktree has no installed packages, because git does not check out the files that it ignores. The agent must run a full install before it can build or test, and many agents mean many installs. This change gives a new worktree links to the installed Node.js packages of the base checkout, so the agent's CLI can build and test at once. The links never count as work: git ignores them, the delete question does not count them, and the office removes them without following them, so it never changes the base checkout. A new office setting, "Worktree dependencies", on by default, lets the person turn this off.

## Context

- Source of truth: `docs/PRD.md`. Section 4.1 says that office-harness writes nothing to a CLI's configuration; this change keeps that rule.
- This change amends 0010 (own worktree per agent). It depends on 0001 (agents, the background server, the desktop app, the action record, durability), 0009 (agent and team templates), and 0010 (own worktree, the create form, the delete question, the removal). References such as `0010/FR-005` point to requirements in those changes.
- It changes no requirement of 0010. The links are ignored files, which 0010 A9 already keeps out of the count and removes with the worktree. This change adds that the office removes its own links first, without following them.
- The links share the base checkout's installed packages; they are not a copy. An install that the agent runs in its worktree can change the base checkout's packages (A3).
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, working folder, action record, desktop app, background server | As defined in 0001. |
| Git repository, worktree, own worktree, base, uncommitted changes, agent worktree | As defined in 0010. |
| Person | The user of 0001 and 0010: the owner of the office. |
| Base checkout | The main checkout folder of the repository, which 0010 stores as the agent's repository. It is the folder where the person installed the packages. It is the base checkout also when the chosen folder is inside another agent's worktree. It is not the base of 0010, which is a branch or a commit. |
| Dependency folder | A folder named `node_modules` that holds installed Node.js packages, as npm, pnpm, Yarn (with `node_modules`), and Bun make it. |
| Dependency place | The top folder of a worktree, or a folder of the worktree that holds a `package.json` file that git tracks in the worktree. The same folder, relative to the top, in the base checkout is its match. |
| Dependency link | A link that the office makes in a new worktree, so that the dependency folder at a dependency place gives the installed packages of the base checkout at the same place. The office links only package entries of the base checkout's dependency folder. |
| Package entry | An entry of a dependency folder whose name does not start with a dot, or one of the package manager entries `.bin`, `.pnpm`, `.modules.yaml`, `.package-lock.json`, `.yarn-state.yml`, and `.bun`. Other entries that start with a dot, such as the tool caches `.vite`, `.vitest`, `.cache`, `.tmp`, and `.astro`, are not package entries. |
| Outside target | A place outside the base checkout, or a place that does not exist, that a link reaches after every link on its way is followed. |
| Worktree dependencies | The office setting that this change adds. It is on or off. |

## User stories

### US1: Build and test in a new own worktree without an install (Priority: P1)

The person creates an agent with own worktree in a repository where he installed the Node.js packages. The agent's CLI starts in the new worktree and can build and run the tests at once.

**Why this priority:** it is the value of the change. It removes one full install for each agent with own worktree.

**Independent test:** in a repository whose base checkout has installed, ignored dependency folders at the top and in one workspace folder, create an agent with own worktree. In the worktree, run the project's test command with no install before it. The command passes.

**Scenarios:**

1. Given Worktree dependencies is on and the base checkout has an installed dependency folder at the top that git ignores, When the person creates an agent with own worktree, Then, before the agent's CLI starts, the worktree's dependency folder at the top gives the installed packages of the base checkout, and the project's build command runs in the worktree without an install.
2. Given the base checkout also has an installed dependency folder beside a tracked `package.json` in a subfolder, When the person creates the agent, Then the worktree gets dependency links at that subfolder too.
3. Given the base checkout has no dependency folder at a dependency place, When the person creates the agent, Then the office makes nothing at that place, and the agent installs as it would without this change.
4. Given a team template has 3 member entries with own worktree on, When the person creates the team (0010/FR-009), Then each of the 3 worktrees gets its own dependency links.
5. Given the office cannot make a dependency link at a place (for example, the disk is full), When the person creates the agent, Then the agent is created, no dependency link stays at that place, and the background server's log names the place and the reason.
6. Given the chosen folder is inside another agent's worktree, When the person creates an agent with own worktree, Then the dependency links give the packages of the base checkout, not of the other agent's worktree.

### US2: The links never become work (Priority: P1)

The dependency links never show as changes, the agent cannot commit them by accident, and their removal never changes the base checkout.

**Why this priority:** a link that counts as a change would make the delete question wrong and could end up in a commit. A removal that follows a link would delete the person's installed packages. It ships with US1.

**Independent test:** create an agent with own worktree and dependency links. In the worktree, `git status` lists 0 entries. Delete the agent and choose remove: the delete question shows no uncommitted changes. After the removal, the base checkout's dependency folders hold the same files with the same content as before.

**Scenarios:**

1. Given a new worktree has dependency links, When the agent runs `git status` in it, Then no dependency link and no dependency folder is listed.
2. Given a new worktree has dependency links, When the agent stages all changes in the worktree without a force option, Then no dependency link is staged.
3. Given a worktree has dependency links and no other change, When the person deletes the agent and chooses remove, Then the delete question shows no uncommitted changes and the office asks no second time (0010/FR-005, 0010/FR-006).
4. Given a worktree has dependency links and one changed file, When the person deletes the agent and chooses remove, Then the second confirmation names 1 changed file.
5. Given the person confirmed the removal, When the office removes the worktree, Then it first removes the dependency links without following them and then removes the worktree as 0010 does, and every file in the base checkout's dependency folders is still there and unchanged.
6. Given one package in the base checkout's dependency folder is a link to a folder outside the base checkout, When the person creates an agent with own worktree, Then the office makes no link for that package, makes the other dependency links, and creates the agent.
7. Given the person chose keep at the delete, When the agent is deleted, Then the worktree keeps its dependency links, and the office no longer manages them (0010 A6).

### US3: Turn the links off and know what the create does (Priority: P2)

The person turns Worktree dependencies off in the office settings. The create form says whether a new own worktree shares the installed packages.

**Why this priority:** shared packages mean that an install in a worktree changes the base checkout's packages, and some people want separate installs. US1 and US2 deliver their value without this story, with the setting at its default.

**Independent test:** turn Worktree dependencies off. Create an agent with own worktree: the create form note says that installed packages are not shared, and the worktree has 0 dependency links. Turn the setting on. The next create makes dependency links.

**Scenarios:**

1. Given the person opens the office settings, When the setting shows, Then "Worktree dependencies" is on by default, with a help text that says that new own worktrees share the checkout's installed Node.js packages and that an install, update, or removal of packages in such a worktree changes the checkout's packages.
2. Given the person turned the setting off, When he creates an agent with own worktree, Then the office makes no dependency link.
3. Given worktrees with dependency links exist, When the person turns the setting off, Then those worktrees keep their links until the office removes them.
4. Given the setting is on and own worktree is on in the create form, When the person looks at the form, Then the note says that untracked files, such as local environment files, are not copied, and that installed Node.js packages are shared from the checkout while Worktree dependencies is on.
5. Given the setting is off and own worktree is on in the create form, When the person looks at the form, Then the note says that untracked files are not copied and that installed packages are not shared, so the agent installs them.
6. Given the person turned the setting off, When the background server stops and starts again, Then the setting is still off.
7. Given the setting cannot be saved, When the person changes it, Then the reason shows and the setting keeps its previous value.

## Requirements

### ADDED

- **FR-001** (US3): THE SYSTEM SHALL let the person turn "Worktree dependencies" on and off in the office settings, on by default, with a help text that says that new own worktrees share the checkout's installed Node.js packages and that an install, update, or removal of packages in such a worktree changes the checkout's packages; SHALL keep the setting in the background server across server and machine restarts; SHALL apply a change to the creates that begin after it and leave the dependency links of existing worktrees as they are; and SHALL show the state of the setting as text: loading, on, off, the reason when it cannot be loaded, and the reason when a change cannot be saved, with the previous value kept.
- **FR-002** (US1): WHEN the office creates an agent's own worktree (0010/FR-002, 0010/FR-009) WHILE Worktree dependencies is on, THE SYSTEM SHALL make dependency links at each dependency place that meets the conditions of FR-003, before the agent's CLI starts, so that the worktree's dependency folder at that place gives the installed packages of the base checkout at the same place.
- **FR-003** (US1, US2): THE SYSTEM SHALL make dependency links at a dependency place only when all of these hold: (a) Worktree dependencies is on; (b) the office does not run on Windows; (c) the base checkout has a dependency folder at that place; (d) git ignores that dependency folder in the base checkout; (e) git, inside the new worktree, ignores a dependency folder at that place; (f) the new worktree has nothing (no file, folder, or link) named `node_modules` at that place; and at any other place THE SYSTEM SHALL make, change, and remove nothing, and SHALL create the agent as 0010 does.
- **FR-004** (US2): IF a dependency link would reach an outside target after every link on its way is followed, THEN THE SYSTEM SHALL NOT make that link, SHALL write the place and the reason to the background server's log, and SHALL go on with the other dependency links and with the create.
- **FR-005** (US2): WHILE a worktree has dependency links, THE SYSTEM SHALL keep them out of every list of changes: git inside the worktree ignores them, so the delete question (0010/FR-005, 0010/FR-006) counts none of them and the agent's own `git status` lists none of them; and THE SYSTEM SHALL write nothing for them into the repository's git data or configuration beyond the branch and the worktree record of 0010 (no ignore or exclude entries), and nothing into the base checkout.
- **FR-006** (US2): WHEN the office removes a worktree (0010/FR-007) or undoes a create that failed (0010/FR-004), THE SYSTEM SHALL first remove each dependency link that it made in that worktree and that still points into the base checkout, without following it, also after a restart of the background server; SHALL then remove the worktree as 0010 does; and SHALL follow no link during the removal, so that it changes and removes no file or folder in the base checkout or anywhere else outside the worktree folder.
- **FR-007** (US1): IF the office cannot make a dependency link at a dependency place (for example, the disk is full or the file system has no links), THEN THE SYSTEM SHALL remove the dependency links that it made at that place and leave nothing else that it made there, SHALL write the place and the reason to the background server's log, SHALL go on with the other places, and SHALL create the agent; THE SYSTEM SHALL NOT fail a create because of a dependency link.
- **FR-008** (US3): WHILE own worktree is on in the create form, THE SYSTEM SHALL show a note that says that untracked files, such as local environment files, are not copied (0010 A8), and, WHILE Worktree dependencies is on and the office does not run on Windows, that installed Node.js packages are shared from the checkout and that an install, update, or removal of packages in the worktree changes the checkout's packages; otherwise, that installed packages are not shared, so the agent installs them.

### PRESERVED

- **PB-1:** An agent created without own worktree starts in its chosen folder as the person's shell would, with no dependency link (0010 PB-1, 0001/FR-020).
- **PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), and nothing in the person's own repository folder; it adds only a branch and a worktree record to the repository's git data (0010 PB-2).
- **PB-3:** The delete question still lists each worktree with whether it has uncommitted changes, and the second confirmation still counts every changed, added, deleted, or untracked file that git does not ignore (0010/FR-005, 0010/FR-006).
- **PB-4:** IF an agent's worktree folder is missing when the agent starts, THEN the office does not start the CLI and says that the worktree folder is missing (0010/FR-008).
- **PB-5:** Other files that git ignores in a worktree are still not counted as uncommitted changes and are still removed with the worktree (0010 A9).
- **PB-6:** A create that fails for a reason of 0010 still creates no agent and leaves no branch, folder, or dependency link behind (0010/FR-004).
- **PB-7:** Each create and delete is recorded in the action record with its actor, as before (0001/FR-011, 0010 PB-4). The dependency links add no entry.

## User experience

**Flow:**

1. In the office settings, the person sees "Worktree dependencies", on. The help text says that new own worktrees share the checkout's installed Node.js packages, and that an install, update, or removal of packages in such a worktree changes the checkout's packages.
2. He creates an agent and turns on own worktree. The note under the choice says that untracked files are not copied and that installed Node.js packages are shared from the checkout while Worktree dependencies is on.
3. He creates the agent. The office makes the dependency links before the CLI starts. The agent builds and tests without an install.
4. He deletes the agent and chooses remove. The delete question counts no dependency link. The office removes the links without following them, and then the worktree.
5. To use separate installs, he turns Worktree dependencies off. The create form note then says that installed packages are not shared.

**Visible states:**

| State | What the person sees | Covered by |
|---|---|---|
| Setting loading | The setting with the text that it is loading; the switch is not available. | FR-001 |
| Setting failed to load | The reason; the switch is not available. | FR-001 |
| Setting on (default) | The switch on, with the help text. | FR-001, US3 scenario 1 |
| Setting off | The switch off, with the help text. | FR-001, US3 scenario 2 |
| Setting not saved | The reason; the setting keeps its previous value. | FR-001, US3 scenario 7 |
| Create form note, setting on | Untracked files are not copied; installed Node.js packages are shared from the checkout while Worktree dependencies is on. | FR-008, US3 scenario 4 |
| Create form note, setting off or Windows | Untracked files are not copied; installed packages are not shared, so the agent installs them. | FR-008, US3 scenario 5 |
| Dependency link not made | Nothing new in the app: the agent is created; the background server's log names the place and the reason. | FR-004, FR-007, US1 scenario 5 |
| Delete question | As in 0010; dependency links are not counted. | FR-005, US2 scenarios 3 and 4 |
| Server not reachable or token rejected | The 0001 states; the setting is not available. | 0001/FR-026, 0001/FR-027 |

**Design:** no fixed design exists. The setting reuses the pattern of the existing office settings dialog, and the note reuses the 0010 create form note. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The person can use the Worktree dependencies switch from the keyboard (0001/FR-028), and keyboard focus is always visible.
- The switch has the text label "Worktree dependencies", and its help text is linked to it for assistive technology.
- The on and off state of the switch, the loading and error states, and the create form note are text, never color alone.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Worktree dependencies setting | Whether the office makes dependency links in new own worktrees. | on or off | One per office; kept by the background server. |
| Dependency link | One link that the office made in an agent worktree to installed packages of the base checkout. | place in the worktree, target in the base checkout | Belongs to exactly one agent worktree; removed before that worktree. In a kept worktree, it belongs to no agent. |
| Agent worktree (extends 0010) | The branch and folder of an agent with own worktree. | the dependency links that the office made in it, kept across restarts | As in 0010. |

## Success criteria

- **SC-001:** In a repository whose base checkout has installed, ignored dependency folders at the top and in one workspace folder, an agent in a new own worktree runs the project's test command with 0 installs before it, and the command passes.
- **SC-002:** Right after the create, `git status` in the worktree lists 0 entries, and the delete question counts 0 changed files for the worktree.
- **SC-003:** After the office removes 10 such worktrees, 5 of them after a restart of the background server, and undoes 3 failed creates, 0 files in the base checkout's dependency folders are changed or missing, compared by name, size, and content with a listing made before the creates.
- **SC-004:** With Worktree dependencies off, 10 creates with own worktree make 0 dependency links.
- **SC-005:** In a base checkout where one package of the top dependency folder is a link to a folder outside the base checkout, and the dependency folder of a second place is itself a link to a folder outside it, a create makes 0 dependency links that reach an outside target, makes a link for every other package, and creates the agent.
- **SC-006:** On Windows, 10 creates with own worktree make 0 dependency links, and 100% of them create the agent as 0010 does.
- **SC-007:** On a file system without links, 10 creates with own worktree create 10 agents, leave 0 dependency links, and write 1 log line with the reason for each place that met the conditions of FR-003.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The base checkout has no dependency folder at a dependency place. | Empty | The office makes nothing at that place; the agent installs as without this change (FR-003 c). |
| The repository has no `package.json` and no dependency folder at the top. | Empty | No dependency link is made; the create is as in 0010 (FR-003). |
| The base checkout's dependency folder is tracked by git, or git does not ignore it. | Invalid | No dependency link at that place (FR-003 d). |
| Git, inside the new worktree, does not ignore a dependency folder at that place (for example, the base branch has no ignore rule for it). | Invalid | No dependency link at that place, so no link can show as a change (FR-003 e, A9). |
| The new worktree already has a file, folder, or link named `node_modules` at that place (for example, the branch tracks one). | Boundary | The office makes, changes, and removes nothing at that place (FR-003 f). |
| The base checkout's dependency folder is itself a link to a folder outside the base checkout. | Invalid | No dependency link at that place; the reason goes to the log; the other places and the create go on (FR-004). |
| One package inside the base checkout's dependency folder is a link to a folder outside the base checkout, or a link to nothing. | Invalid | No link for that package; the reason goes to the log; the other links and the create go on (FR-004). The package is missing in the worktree until the agent installs it. |
| The base checkout's dependency folder holds tool caches, such as `.vite` or `.cache`. | Boundary | They are not package entries and get no link. A tool in the worktree makes its own cache in the worktree's dependency folder, and the base checkout's caches stay unchanged (Terms). |
| The repository has nested workspaces with several tracked `package.json` files. | Boundary | Each dependency place that meets the conditions of FR-003 gets its own dependency links (FR-002). |
| The chosen base branch is not the branch checked out in the base checkout. | Boundary | The links give the packages that the base checkout has installed. When the base branch needs other package versions, the agent may need an install in the worktree (A4). |
| The chosen folder is inside another agent's worktree. | Boundary | The base checkout is the main checkout of the repository, not the other worktree (Terms, US1 scenario 6). |
| The agent deletes a dependency link, replaces it with a real file or folder, or adds real files beside the links. | Boundary | At removal, the office first removes only the links that it made and that still point into the base checkout, without following them. The rest is ignored content that the 0010 removal removes with the worktree, and no link is followed, also when the agent pointed one elsewhere (FR-006, PB-5). |
| The agent runs an install, an update, or a package removal in the worktree. | Boundary | It can change the base checkout's installed packages, because the packages are shared (A3). The office does not prevent it; the help text and the create form note say so. |
| The agent force-adds a dependency link to a commit. | Invalid | It is the agent's own change. It counts as a change in the delete question as any staged file does (PB-3). |
| The person deletes or reinstalls the base checkout's dependency folder after the create. | Concurrent | The links show what the base checkout now has, or point at nothing. The removal still removes them without following them (FR-006). |
| The person runs an install in the base checkout while the office makes the links. | Concurrent | The links show what the base checkout had at that moment; packages added after it can be missing in the worktree (A4). |
| The worktree folder is missing at the removal. | Failure | There is no link to remove; the 0010 removal clears what remains (0010 Edge cases). |
| Two agents with own worktree are created at the same time from one repository. | Concurrent | Each worktree gets its own links; both share the base checkout's packages (A3). |
| The person turns the setting off after a create. | Boundary | The existing links stay until the office removes the worktree; the next creates make none (FR-001). |
| The person changes the setting while the create form is open. | Concurrent | The create uses the value that the background server holds when the create runs (A10). |
| The disk is full, or the file system has no links. | Failure | The office removes the links it made at that place, writes the reason to the log, and creates the agent (FR-007). |
| The office runs on Windows. | Boundary | No dependency link; the create is as in 0010; the create form note says that packages are not shared (FR-003 b, FR-008, A6). |
| A team template creates many members with own worktree. | Boundary | Each member's worktree gets its own links; a link failure in one worktree fails no create (FR-002, FR-007). |
| The person chose keep at the delete. | Boundary | The kept worktree keeps its links; the office no longer manages it (0010 A6, A8). |
| The setting cannot be saved (the server is not reachable, or its storage fails). | Failure | The reason shows; the setting keeps its previous value, and the office goes on applying that value (FR-001). |
| The desktop app cannot reach the background server, or its token is rejected. | Unauthorized | The 0001 states show (0001/FR-026, 0001/FR-027). The server keeps applying the stored setting. |

## Assumptions

- **A1:** The base checkout is the repository folder that 0010 stores for the agent, which is the main checkout also when the chosen folder is inside a linked worktree. A dependency place in the worktree matches the folder at the same place, relative to the top, in the base checkout.
- **A2:** Only Node.js dependency folders (`node_modules`) are linked, as npm, pnpm, Yarn with `node_modules`, and Bun make them. Other ecosystems are out of scope, each for the reason under Out of scope.
- **A3:** The links share the base checkout's packages; they are not a copy. An install, update, or package removal that the agent runs in its worktree can change the base checkout's installed packages. The help text of the setting and the create form note say so (FR-001, FR-008). An agent that wants separate packages can remove its dependency folder and install, or the person turns the setting off.
- **A4:** The office makes the links once, at create time, and never brings them up to date. Packages that the base checkout installs after the create can be missing in the worktree. A worktree whose base branch needs other package versions than the base checkout has installed can need an install. A `package.json` added to the worktree after the create gets no links.
- **A5:** Workspace packages that a dependency folder links to (for example, the repository's own packages in a workspace) resolve to the base checkout's copy, not the worktree's copy. Other packages see the agent's edits to such a package only after an install in the worktree.
- **A6:** On Windows, the office makes no links: a link to a folder needs Developer Mode or administrator rights, a junction needs a platform call that the office does not have, and some tools delete through a junction. The create goes on as in 0010.
- **A7:** A change of the Worktree dependencies setting is a settings change and is not recorded in the action record (as 0018 A11). A dependency link adds no action record entry; a link that the office did not make is written only to the background server's log (FR-004, FR-007).
- **A8:** A kept worktree keeps its dependency links. The office no longer manages it (0010 A6); the person owns it, and an install in it can change the base checkout's packages (A3).
- **A9:** Condition FR-003 (e) exists because the office writes no ignore or exclude entry (FR-005). Only a place where git already ignores the dependency folder inside the new worktree gets links, so a link can never show as a change.
- **A10:** A create uses the value of the setting that the background server holds when the create runs. The create form note can show the value from when the form opened.
- **A11:** A link in the base checkout's dependency folder that points at nothing, or that git cannot follow to an end, counts as an outside target and gets no dependency link (FR-004).
- **A12:** IF the office cannot remove the links that it made at a failed place (FR-007), THEN those links stay as ignored content in the worktree, and FR-006 removes them before the worktree.

## Out of scope

- Python virtual environments: they record their own absolute folder, so a link to one does not run from another folder.
- Rust build folders: they are build output, not dependencies, and two builds would share one lock and mix branches.
- PHP and Ruby vendor folders: their loaders resolve the project's own code to the base checkout.
- Go, Java, .NET, and Dart dependencies: they already live in a cache for each user, or their vendor folders are tracked.
- Yarn Plug'n'Play: it makes no `node_modules` folder.
- Links on Windows, for example with junctions (A6). Junction support is a later change.
- Copying packages into a worktree, or running an install for the agent.
- Bringing links up to date after the create (A4).
- Linking or copying other ignored or untracked files, such as local environment files or build output (0010 A8).
- A Worktree dependencies setting per agent or per template.
- Showing a link that was not made in the desktop app (A7).

## Clarifications

### Session 2026-11-23

- Q: Which folder do the links point into? → A: The base checkout, the main checkout that 0010 stores as the repository, also when the chosen folder is inside another agent's worktree (Terms, A1). Decided under the owner's spec delegation, 2026-11-23.
- Q: Which ecosystems get links? → A: Only Node.js dependency folders, at the top and beside each tracked `package.json`; the others are out of scope with their reasons (A2, Out of scope). Decided under the owner's spec delegation, 2026-11-23.
- Q: Why are the links not removed before the delete question counts the changes? → A: The delete question reads the worktree state while the agent may still run, and the person can cancel the delete. Removing the links at the check would take the packages away from a working agent. So the check does not count them (FR-005), and the office removes them only right before it removes the worktree (FR-006). Decided under the owner's spec delegation, 2026-11-23.
- Q: What does the office do with a link that would reach a place outside the base checkout? → A: It does not make that link, and makes the others; the create goes on (FR-004). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens when a link cannot be made? → A: The office removes the links it made at that place, writes the reason to the server log, and creates the agent. No create fails because of this change (FR-007). Decided under the owner's spec delegation, 2026-11-23.
- Q: Is Worktree dependencies on or off by default, and when does a change apply? → A: On; a change applies to the next creates, and existing worktrees keep their links until removal (FR-001). Decided under the owner's spec delegation, 2026-11-23.
- Q: What happens on Windows? → A: The office makes no links, and the create goes on as in 0010 (FR-003 b, A6). Decided under the owner's spec delegation, 2026-11-23.
- Q: Are tool caches inside the dependency folder shared too? → A: No. Only package entries are linked (Terms), so concurrent builds in the base checkout and in each worktree never write one cache. Decided at the review gate (r1), 2026-11-23.
- Q: Must git ignore the dependency folder inside the new worktree too? → A: Yes. Without an ignore or exclude entry from the office, that is the only way that a link never shows as a change (FR-003 e, A9). Decided under the owner's spec delegation, 2026-11-23.

## Deviations
