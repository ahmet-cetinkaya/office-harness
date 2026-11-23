---
status: shipped
capability: office
plan: specs/changes/archive/0016-agent-file-browser/PLAN.md
supersedes:
---

<!-- Approved under the owner's spec delegation on 2026-11-23; shipped in PR #122. -->

# 0016: Read-only file browser of an agent

## Intent

Reviewing what the agents did is the owner's main job, and today the desktop app shows only their terminals. To see which files an agent changed, he leaves the app and opens a shell or an editor in the agent's working folder, which is slow with many agents and easy to get wrong with many worktrees (0010). This change lets the user open, from the desktop app, a read-only view of one agent's working folder: a tree of its files, the content of a file, the files that differ from the last commit with their changes, and the commit history of the folder. The view writes nothing: it changes no file, no git data, and no agent. It stays inside the agent's working folder, shows a file of a secret-looking name only after the user asks for it, and shows only files up to a size limit. The idea comes from an MIT-licensed reference office app; see the pull request.

## Context

- Source of truth: `docs/PRD.md`, R1 (one panel) and R9 (practicality).
- This change depends on 0001 (agents, working folder, the desktop app, the office token, keyboard access) and 0010 (an agent's own worktree, which is then its working folder). References such as `0001/FR-012` point to requirements in those changes.
- The working folder can hold anything the agent's CLI wrote, so every file name, file content, and commit text that the view shows is untrusted data.
- Agents run on the office's own machine; agents on other machines (0006) are not built.
- Stack decision, made by the owner on 2026-11-15, as in 0001. It is a recorded decision, not a requirement of this spec.
- The requirements below describe behavior only. The plan decides how to build them.

## Terms

| Term | Meaning |
|---|---|
| Agent, working folder, desktop app, office token, action record | As defined in 0001. |
| Own worktree | As defined in 0010. An agent with an own worktree has that worktree as its working folder. |
| Root | The folder the agent's CLI starts in: its working folder, or its project's folder when it has none (0001/FR-012, FR-013), with every link in its path resolved, at the moment of a request. |
| Inside the root | A path that, after every symbolic link in it is resolved, is the root or lies below it. |
| File view | The read-only view of one agent's root in the desktop app, with three parts: Files, Changes, and History. |
| Git data | The repository's own data folder (`.git`) and every file in it, including the index and lock files. |
| Changed file | A file inside the root that differs from the last commit (HEAD): modified, added, deleted, renamed, or untracked, whether or not the change is staged. Ignored files are not changed files. |
| Diff | The lines that a change removed and added in one file, with a few unchanged lines around them. |
| Commit history | The commits of the branch checked out in the root (HEAD and its ancestors) that change at least one file inside the root, newest first. |
| Size limit | 512 KiB (524,288 bytes): the largest file content or diff that the view shows. |
| Secret-looking file | A file whose name, compared without regard to case, is `.env` or starts with `.env.`; ends with `.pem`, `.key`, `.p12`, `.pfx`, `.jks`, `.keystore`, `.kdbx`, `.ppk`, or `.tfstate`; starts with `id_rsa`, `id_dsa`, `id_ecdsa`, or `id_ed25519`; is `.netrc`, `.npmrc`, `.pypirc`, `.pgpass`, `.git-credentials`, `.htpasswd`, or `kubeconfig`; ends with `.ovpn` or `.asc`; or contains `secret` or `credential`. |
| Reveal | The user's explicit action that shows the content or the diff of one secret-looking file once. |
| Binary file | A file whose first 8,000 bytes hold a zero byte. |

## Data flow and trust boundary

- **Boundary 1, desktop app to office:** the user connection of 0001, which needs the office token (0001/FR-007) and accepts only connections from the same machine (0001/FR-008). Agents cannot use the file view: their connection (0002) does not offer it.
- **Into the office:** an agent, a path relative to the root, a commit, and the reveal choice. All are untrusted: the office checks each one before it reads anything.
- **Boundary 2, office to the root:** the office reads files and git data inside the root as the user's OS user. It follows a symbolic link only after it has checked that the link's target is inside the root.
- **Out of the office, to the desktop app:** names, sizes, kinds, and times of the entries of one folder; the content of one file up to the size limit; the changed files and the diff of one file; commits with their id, author, time, and message; and the changed files and diffs of one commit. Each is untrusted data that the agent may have written.
- **Never crosses:** anything outside the root, the git data's own files (its configuration, which can hold credentials in remote addresses), and the content or diff of a secret-looking file without a reveal (FR-004).
- **Threats and controls:**

  | Threat | Control |
  |---|---|
  | A path such as `../..`, an absolute path, or a symbolic link reads a file outside the root, for example the user's keys. | Every path is resolved, links included, and refused unless it is inside the root (FR-003, SC-001). |
  | A file holds a key or a password, and the view shows it while someone watches the screen or while the screen is recorded. | A secret-looking file shows only after a reveal, for that one file (FR-004, SC-003). |
  | A huge file or diff freezes the desktop app or fills its memory. | Content and diffs over the size limit are not sent; lists have caps (FR-002, FR-005, SC-002). |
  | The view changes the agent's work: it writes the index, leaves a lock file, runs a git hook, or runs a program that the repository's configuration names. | The view writes nothing and runs no program that the repository names (FR-007, SC-004). |
  | A commit request names an option instead of a commit, and git writes a file. | A commit is named only by its full id; anything else is refused (FR-006, Edge cases). |
  | A file, a file name, or a commit message carries markup, a script, a terminal escape code, or a bidirectional format character that reorders what the reviewer reads. | Everything is shown as plain text; control and bidirectional format characters show as visible symbols (FR-009, SC-008). |
  | Reading a special file (a named pipe or a device) blocks the office. | Only regular files are read (Edge cases). |

- **Residual risk:** the agent's CLI runs as the same OS user as the office. An agent that swaps a folder for a symbolic link between the office's check and its read, or that adds a hard link to a file outside the root, can still make the view show that file. The rule prevents mistakes and stray links, not a hostile agent (as 0002 A8). The secret-looking names are a list of common names; a secret in a file of another name shows without a reveal (A4).

## User stories

### US1 — Browse the files of an agent (Priority: P1)

The user opens the file view of an agent and walks its working folder: folders open one at a time, and a file shows its content as read-only text.

**Why this priority:** the user cannot review an agent's work without seeing its files; the other parts build on this one.

**Independent test:** create an agent whose working folder holds a folder `src` with a file `main.txt`, a file `.env`, a 1 MiB file, a binary file, and a symbolic link to a file outside the folder. Open the file view. The tree lists all five entries. `main.txt` shows its text; `.env` shows only after a reveal; the 1 MiB file and the binary file show why their content is not shown; the link shows that it points outside the working folder, and its target's content is never shown.

**Scenarios:**

1. Given an agent with a working folder, When the user opens the agent's file view, Then the Files part lists the entries of the root, folders first, then files, each group in name order, each entry with its name, its kind, and, for a file, its size.
2. Given the tree shows a folder, When the user opens it, Then the tree shows that folder's entries below it.
3. Given the tree shows a text file within the size limit, When the user opens it, Then the view shows its content as read-only text with line numbers.
4. Given a file larger than the size limit, When the user opens it, Then the view shows its size and that files over 512 KiB are not shown, and shows none of its content.
5. Given a binary file, When the user opens it, Then the view says that it is a binary file and shows none of its content.
6. Given a symbolic link whose target is outside the root, When the user opens it, Then the view says that the link points outside the agent's working folder and shows nothing of the target.
7. Given a secret-looking file, When the user opens it, Then the view says that the file may hold a secret, shows none of its content, and offers a reveal; When the user reveals it, Then the content shows; When the user opens another file and comes back, Then the content is hidden again.
8. Given the agent is `working`, `stopped`, or any other status, When the user opens the file view, Then it works the same way.

### US2 — See what an agent changed since the last commit (Priority: P1)

The user sees the list of files that differ from the last commit in the agent's working folder and the diff of each one.

**Why this priority:** this is the review itself: most of the time the user wants to know what the agent changed, not every file.

**Independent test:** in a git repository, let an agent's working folder hold one modified, one staged new, one deleted, one renamed, and one untracked file. Open the Changes part. It lists exactly those five files, each with its kind, and each diff shows the removed and added lines.

**Scenarios:**

1. Given the root is inside a git repository, When the user opens the Changes part, Then it lists every changed file inside the root with its path and its kind (modified, added, deleted, renamed with its old path, untracked, or conflicted), in path order.
2. Given the Changes part lists a file, When the user opens it, Then the view shows its diff against the last commit, with removed and added lines marked by a sign and a line number, not by color alone.
3. Given an untracked file, When the user opens it, Then its diff shows all of its lines as added.
4. Given a secret-looking changed file, When the user opens it, Then the diff shows only after a reveal, as in US1 scenario 7.
5. Given the root is a subfolder of a repository whose other folders also have changes, When the user opens the Changes part, Then it lists only the changed files inside the root.
6. Given the root is not inside a git repository, When the user opens the Changes part, Then it says that the folder is not in a git repository.
7. Given the root has no changed file, When the user opens the Changes part, Then it says that there is no change since the last commit.

### US3 — Read the commit history of an agent's folder (Priority: P2)

The user reads the commits that changed the agent's working folder and opens one to see its files and diffs.

**Why this priority:** an agent that commits its work leaves no changed files; its commits are the rest of the review. The user can still review in a shell, so it ranks below US1 and US2.

**Independent test:** in a repository with 120 commits that change the root, open the History part: it lists the 50 newest commits; "Show more" adds the next 50, then the last 20. Open one commit: it lists its changed files, and each file shows its diff.

**Scenarios:**

1. Given the root is inside a git repository, When the user opens the History part, Then it lists the 50 newest commits of the commit history, each with its short id, the first line of its message, its author name, and its time.
2. Given more commits exist, When the user chooses "Show more", Then the list adds the next 50.
3. Given the History part lists a commit, When the user opens it, Then the view shows its full id, author, time, full message, and the files inside the root that it changed, each with its kind.
4. Given an opened commit lists a file, When the user opens the file, Then the view shows the file's diff in that commit against the commit's first parent, with the reveal rule of US1 scenario 7 for a secret-looking file.
5. Given the repository has no commit yet, When the user opens the History part, Then it says that there is no commit yet.

## Requirements

### ADDED

- **FR-001** (US1): WHEN the user opens the file view of an agent, THE SYSTEM SHALL list the entries of the root, and WHEN the user opens a folder, THE SYSTEM SHALL list that folder's entries, folders first and then the other entries, each group in name order, each entry with its name, its kind (folder, file, link, or other), and, for a file, its size, and SHALL list at most 1,000 entries of one folder and say how many more exist.
- **FR-002** (US1): WHEN the user opens a file inside the root, THE SYSTEM SHALL show its content as read-only text with line numbers, IF the file is larger than the size limit THEN THE SYSTEM SHALL show its size instead, and IF the file is a binary file THEN THE SYSTEM SHALL say so instead.
- **FR-003** (US1, US2, US3): THE SYSTEM SHALL resolve every path that the file view asks for, symbolic links included, before it reads, SHALL refuse a path that is absolute, holds a `..` part, or is not inside the root, and SHALL NOT send any part of a file or a folder listing that is not inside the root.
- **FR-004** (US1, US2, US3): THE SYSTEM SHALL NOT send the content or the diff of a secret-looking file, or the diff of a rename whose old or new name is secret-looking, until the user reveals that file, and SHALL hide it again when the user opens another file, another part, or refreshes.
- **FR-005** (US2): WHERE the root is inside a git repository, WHEN the user opens the Changes part, THE SYSTEM SHALL list the changed files inside the root, at most 1,000 with the count of the others, and WHEN the user opens one, SHALL show its diff against the last commit, staged and unstaged changes together, or its size and that it is too large when the diff is over the size limit.
- **FR-006** (US3): WHERE the root is inside a git repository, WHEN the user opens the History part, THE SYSTEM SHALL list the commit history in pages of 50 commits, and WHEN the user opens a commit, SHALL show its details, the files inside the root that it changed, and each file's diff against the commit's first parent.
- **FR-007** (US1, US2, US3): THE SYSTEM SHALL NOT create, change, or delete any file in the root or in its git data, SHALL NOT change any agent, and SHALL NOT run any program that the repository's configuration or folder names, such as a hook, a diff driver, a text conversion, a content filter, a signature check, or a file system monitor, while it serves the file view.
- **FR-008** (US1, US2, US3): THE SYSTEM SHALL show a loading state while a part of the file view waits for the office, an empty state when a folder, the changes, or the history has nothing to show, and an error state with the reason when the office cannot read what was asked, and SHALL show the time of the last load with a refresh action.
- **FR-009** (US1, US2, US3): THE SYSTEM SHALL show every file name, file content, diff, and commit text in the file view as plain text, SHALL NOT render markup, run scripts, load images, or follow links found in them, and SHALL show each control character and bidirectional format character as a visible symbol.

### PRESERVED

- **PB-1:** The agent's connection to the office (0002) gets no file view request: the allowed agent requests stay as they are.
- **PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), in the agent's working folder, or in the repository's git data (0010/PB-2) because of the file view.
- **PB-3:** The desktop app still needs the office token and a connection from the same machine for every request (0001/FR-007, FR-008).
- **PB-4:** The file view changes no agent's status, session, terminal, task text, or Needs you item, and adds nothing to the action record.
- **PB-5:** The panel, the terminals, and every existing dialog work as before; the file view adds one way in from an agent and nothing else changes.

## User experience

**Flow:**

1. The user picks an agent in the panel and opens its file view with the "Browse files" action of the agent's details, by mouse or keyboard.
2. The view opens on the Files part with the root listed. The header names the agent and its working folder, and shows the time of the last load and a refresh action.
3. The user opens folders and files. A file's content shows beside the tree, read-only.
4. He switches to Changes: the changed files, each with its kind. Opening one shows its diff.
5. He switches to History: the newest commits. Opening one shows its details and changed files; opening a file shows its diff.
6. For a secret-looking file he chooses Reveal to see it once.
7. He closes the view; focus returns to the control that opened it.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading | A loading indicator in the part that waits, with text for assistive technology. | FR-008 |
| Folder listed | The entries, folders first; "N more not shown" past 1,000. | FR-001 |
| Empty folder | "This folder is empty." | FR-008, US1 |
| File shown | Read-only text with line numbers. | FR-002 |
| File too large | Its size and the 512 KiB limit; no content. | FR-002, US1 scenario 4 |
| Binary file | "Binary file, not shown"; no content. | FR-002, US1 scenario 5 |
| Link outside | The link points outside the agent's working folder; nothing shown. | FR-003, US1 scenario 6 |
| Secret-looking file | "This file may hold a secret" and a Reveal action; no content until then. | FR-004, US1 scenario 7 |
| Not a git repository | Changes and History say that the folder is not in a git repository. | FR-008, US2 scenario 6 |
| No changes | "No change since the last commit." | FR-008, US2 scenario 7 |
| No commits | "No commit yet." | FR-008, US3 scenario 5 |
| Diff shown | Removed and added lines with signs and line numbers. | FR-005, FR-006 |
| Diff too large | Its size and the 512 KiB limit; no lines. | FR-005 |
| Error | The reason, for example that the working folder does not exist, that access was denied, that git is not available, or that git did not answer in time, with the refresh action. | FR-008, Edge cases |

**Design:** no fixed design exists. The view follows the existing look of the desktop app's dialogs and panel; the plan must not add new visual styles beyond this flow.

**Accessibility:**

- Every action works from the keyboard (0001/FR-028): the parts are tabs, the tree follows the tree pattern (arrow keys move and open, Enter opens a file), lists and diffs scroll by keyboard, and Escape closes the view.
- Keyboard focus is always visible, and when the view closes, focus returns to the control that opened it.
- A diff line's kind is a sign (`+` or `-`) and text for assistive technology, never color alone; an entry's kind and a change's kind are text or an icon with a text name.
- Loading, empty, and error states are announced to assistive technology.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text), and the view reflows at 200% zoom without hiding text.

## Key entities

None stored. Every answer is read from the root and its git data at the moment of the request; the office keeps no copy.

## Success criteria

- **SC-001:** Of 20 requests for paths outside the root (parent parts, absolute paths, a link to a file outside, a link to a folder outside, a link inside a linked folder, and a link chain that leaves the root), 100% are refused, and 0 bytes of the outside files reach the desktop app.
- **SC-002:** A file of exactly 512 KiB shows in full; a file of 512 KiB plus 1 byte shows its size and 0 bytes of content; a diff over 512 KiB shows its size and 0 lines.
- **SC-003:** In a folder with 10 secret-looking files that are also changed files, opening the tree, each file, the Changes part, and each diff sends 0 bytes of their content to the desktop app until a reveal; after one reveal, exactly that one file's content arrives; after opening another file, it is hidden again. A `.env` renamed to `notes.txt`, in the Changes part and in a commit, sends 0 bytes of its diff until a reveal.
- **SC-004:** After a session that lists every folder, opens every file, the changes, every diff, the history, and every commit of a test repository, the root and its git data are byte for byte the same as before, with the same modification times, no lock file was created, and a hook, a diff driver, a text conversion, a content filter, a signature check program, and a file system monitor that the repository names each wrote 0 marker files.
- **SC-005:** In a repository with one modified, one staged new, one deleted, one renamed, and one untracked file inside the root and one modified file outside it, the Changes part lists exactly the 5 files inside the root, each with its kind, and each diff holds the removed and added lines of that file.
- **SC-006:** With 120 commits that change the root and 10 that change only another folder of the repository, the History part shows 50, then 100, then 120 commits, newest first, none of the 10 others.
- **SC-007:** Each state of the Visible states table appears in a test with its text, and every action of the flow completes with the keyboard alone.
- **SC-008:** A file and a commit message that hold `<script>`, an image tag, a terminal escape code (U+001B), and a right-to-left override (U+202E) show as text, with U+001B and U+202E as visible symbols, and 0 scripts run and 0 images load.
- **SC-009:** In a working folder of 50,000 files, a folder of 1,000 entries lists within 2 seconds, a 512 KiB file shows within 2 seconds, and the Changes part of 1,000 changed files lists within 5 seconds.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The folder is empty. | Empty | "This folder is empty." (FR-008). |
| The root is not inside a git repository. | Empty | Files works; Changes and History say that the folder is not in a git repository (FR-008). |
| The repository has no commit yet. | Empty | Changes lists every file inside the root as added or untracked; History says that there is no commit yet. |
| A file is empty (0 bytes). | Empty | The view shows that the file is empty. |
| A folder has more than 1,000 entries. | Boundary | The first 1,000 in the order of FR-001, then "N more not shown" (FR-001). |
| More than 1,000 changed files. | Boundary | The first 1,000 in path order, then the count of the others (FR-005). |
| A file or a diff of exactly the size limit, or one byte over. | Boundary | Exactly the limit shows; one byte over shows its size only (FR-002, FR-005, SC-002). |
| A text file is not valid UTF-8. | Boundary | It shows with each byte sequence that is not valid UTF-8 replaced by a visible replacement mark; nothing is guessed. |
| A file or folder name is not valid UTF-8. | Boundary | It is listed with each invalid byte sequence replaced by a visible replacement mark; opening it says that the office cannot open a name that is not valid UTF-8. |
| A line is very long. | Boundary | The line scrolls inside the content area; the page does not scroll sideways. |
| A file name or a commit text holds a control character, an escape code, or a bidirectional format character. | Boundary | It shows as a visible symbol (FR-009, SC-008). |
| A path is absolute, holds `..`, holds a zero byte, or is empty for a file. | Invalid | Refused with the reason; nothing is read (FR-003). |
| A symbolic link's target is outside the root, does not exist, or forms a loop. | Invalid | The link is listed as a link; opening it says that its target is outside the working folder, missing, or a loop, and shows nothing of it (FR-003). |
| A symbolic link's target is inside the root. | Boundary | Opening it shows the target, as if the user opened the target. |
| A named pipe, a socket, or a device file. | Invalid | Listed as "other"; it is never opened, so the office never waits on it. |
| The repository's git data folder (`.git`). | Boundary | It is not listed in the tree and cannot be opened; Changes and History are the way to see git state. |
| A commit is named by anything other than a full commit id (a branch name, a short id, an option such as `--output`, or an object that is not a commit). | Invalid | Refused with the reason; git does not run with it (FR-006). |
| A commit is a merge commit. | Boundary | Its files and diffs are those against its first parent (FR-006). |
| A secret-looking file is renamed to a harmless name, or the other way round. | Boundary | The diff of the rename is treated as secret-looking: it shows only after a reveal (FR-004). |
| A changed file is a submodule. | Boundary | Listed with the kind "submodule"; the view does not open the submodule's own changes. |
| A file is changed, renamed, or deleted between the listing and the opening. | Concurrent | The view shows the file as it is when opened, or says that it no longer exists; the refresh action reloads the list. |
| The agent writes while the view is open. | Concurrent | The view shows what it loaded and the time of that load; it changes only on refresh or when the user opens something (A3). |
| Another git command holds the repository's lock. | Concurrent | The view still reads, because it takes no lock (FR-007). |
| The agent is deleted while its file view is open. | Concurrent | The next request says that the agent no longer exists; the view shows the error state. |
| The working folder does not exist or the user's OS account cannot read it or a file in it. | Failure | The error state names the reason; nothing else changes (FR-008). |
| Git is not installed, or git does not answer within 10 seconds. | Failure | The error state says so; the Files part still works (FR-008). |
| The desktop app is not connected to the office, or the token was rejected. | Unauthorized | The app's existing not-reachable or not-authorized state shows (0001/FR-026, FR-027). |
| An agent asks for the file view from its session. | Unauthorized | The agent connection does not offer it (PB-1). |
| A request names an agent that does not exist. | Invalid | Refused with "no such agent"; nothing is read. |

## Assumptions

- **A1:** The root is the folder the agent's CLI starts in: its working folder, which for an agent with an own worktree is its worktree (0010/FR-002), or its project's folder when it has no working folder. The view never shows a folder above the root, even when the repository's top folder is above it.
- **A2:** "Against the last commit" means against HEAD of the branch checked out in the root, with staged and unstaged changes shown together. Seeing staged and unstaged changes apart is out of scope.
- **A3:** The view shows the state at the time of each load. It does not watch the folder; the user refreshes. With many agents writing all the time, a live view would redraw without end.
- **A4:** The secret-looking names are a fixed list of common names for keys, credentials, and environment files. A secret in a file of another name shows without a reveal. The reveal protects a screen that others can see; it does not keep the file from the user, who owns it.
- **A5:** A reveal is not stored and not recorded: it lasts until the user opens another file or part, or refreshes. The user owns the files, so a record of his own reads adds nothing.
- **A6:** The office reads the files and git data as the user's OS user, so the view shows only what the user can read.
- **A7:** Reading is not an action under 0001/FR-011 or 0007, so a hold does not block the file view, and no read is in the action record (PB-4).
- **A8:** The history follows the first parent of a merge for diffs and lists every commit of HEAD that changes the root, as git lists them by default, newest first.
- **A9:** A file counts as text when its first 8,000 bytes hold no zero byte. A text file in an encoding other than UTF-8 shows with replacement marks; detecting encodings is out of scope.
- **A10:** The limits (512 KiB, 1,000 entries, 1,000 changed files, 50 commits per page, 10 seconds for git) keep one view from freezing the desktop app or the office with an agent's large files; they are not settings.

## Clarifications

### Session 2026-11-23 (drafted under the owner's delegation)

- Q: Is the view read-only in this change? → A: Yes. Writing, staging, committing, checking out, and discarding are out of scope (FR-007, Out of scope).
- Q: How far can the view see? → A: The agent's working folder only, with links resolved before the check (FR-003, A1).
- Q: How are secret files handled? → A: Listed, but their content and diff show only after a reveal of that one file, every time (FR-004, A4, A5).
- Q: How large a file does it show? → A: Up to 512 KiB (Terms, FR-002, A10). Source files are far smaller; a larger file is usually generated or data. To be confirmed at the spec review gate.

## Out of scope

- Writing: editing files, creating or deleting files, staging, committing, checking out, branching, stashing, discarding, or any other change to files or git data.
- Branch comparison, blame, search in files, and seeing staged and unstaged changes apart (A2).
- A live view that follows the agent's writes (A3).
- Showing images, rendered Markdown, or other rendered previews (FR-009).
- File views for agents on other machines (0006).
- The file view for agents: an agent reading files of another agent through the office (PB-1).
- Settings for the limits (A10) and for the list of secret-looking names (A4).
