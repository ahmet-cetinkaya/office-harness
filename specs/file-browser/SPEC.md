# Read-only file browser of an agent

## Intent

The person looks at the files of an agent in a read-only view. This file is the current truth of the part of that capability that has shipped and merged here. Each requirement id has the number of its source change as a prefix, so every id is unique and traceable.

## Sources

Archived change specs merged here:

- [0016-agent-file-browser](../changes/archive/0016-agent-file-browser/SPEC.md)

## How to read this file

- An id such as `0003/FR-004` names the requirement `FR-004` of change 0003.
- When a later change modified a requirement, this file keeps one entry under the original id with the final text, and a note names the change that modified it. The earlier text is not repeated.
- A bare id inside a requirement text, such as `FR-001`, names a requirement of the same change as that entry.
- Requirements removed by a later change: none.

Changes of other capabilities that this file names:

- 0002: [Agent hierarchy and directives](../agent-hierarchy/SPEC.md)
- 0010: [Agent worktrees and dependencies](../agent-worktree/SPEC.md)
- 0001: [Server and terminal](../server-and-terminal/SPEC.md)

## Change 0016: Read-only file browser of an agent

### Intent

Reviewing what the agents did is the owner's main job, and the desktop app showed only the terminals. This change adds a read-only file view of one agent's root, the folder its CLI starts in (its working folder, or its project's folder). The view has three parts: Files (a tree and the content of a file), Changes (the files that differ from the last commit, with their diffs), and History (the commits that change files inside the root, with each commit's files and diffs). It writes nothing, stays inside the root, shows a file of a secret-looking name only after the user reveals it, and shows content and diffs up to 512 KiB. Agents cannot use it.

### Requirements

- **0016/FR-001** (US1): WHEN the user opens the file view of an agent, THE SYSTEM SHALL list the entries of the root, and WHEN the user opens a folder, THE SYSTEM SHALL list that folder's entries, folders first and then the other entries, each group in name order, each entry with its name, its kind (folder, file, link, or other), and, for a file, its size, and SHALL list at most 1,000 entries of one folder and say how many more exist.
- **0016/FR-002** (US1): WHEN the user opens a file inside the root, THE SYSTEM SHALL show its content as read-only text with line numbers, IF the file is larger than the size limit THEN THE SYSTEM SHALL show its size instead, and IF the file is a binary file THEN THE SYSTEM SHALL say so instead.
- **0016/FR-003** (US1, US2, US3): THE SYSTEM SHALL resolve every path that the file view asks for, symbolic links included, before it reads, SHALL refuse a path that is absolute, holds a `..` part, or is not inside the root, and SHALL NOT send any part of a file or a folder listing that is not inside the root.
- **0016/FR-004** (US1, US2, US3): THE SYSTEM SHALL NOT send the content or the diff of a secret-looking file, or the diff of a rename whose old or new name is secret-looking, until the user reveals that file, and SHALL hide it again when the user opens another file, another part, or refreshes.
- **0016/FR-005** (US2): WHERE the root is inside a git repository, WHEN the user opens the Changes part, THE SYSTEM SHALL list the changed files inside the root, at most 1,000 with the count of the others, and WHEN the user opens one, SHALL show its diff against the last commit, staged and unstaged changes together, or its size and that it is too large when the diff is over the size limit.
- **0016/FR-006** (US3): WHERE the root is inside a git repository, WHEN the user opens the History part, THE SYSTEM SHALL list the commit history in pages of 50 commits, and WHEN the user opens a commit, SHALL show its details, the files inside the root that it changed, and each file's diff against the commit's first parent.
- **0016/FR-007** (US1, US2, US3): THE SYSTEM SHALL NOT create, change, or delete any file in the root or in its git data, SHALL NOT change any agent, and SHALL NOT run any program that the repository's configuration or folder names, such as a hook, a diff driver, a text conversion, a content filter, a signature check, or a file system monitor, while it serves the file view.
- **0016/FR-008** (US1, US2, US3): THE SYSTEM SHALL show a loading state while a part of the file view waits for the office, an empty state when a folder, the changes, or the history has nothing to show, and an error state with the reason when the office cannot read what was asked, and SHALL show the time of the last load with a refresh action.
- **0016/FR-009** (US1, US2, US3): THE SYSTEM SHALL show every file name, file content, diff, and commit text in the file view as plain text, SHALL NOT render markup, run scripts, load images, or follow links found in them, and SHALL show each control character and bidirectional format character as a visible symbol.

### Preserved behaviors

- **0016/PB-1:** The agent's connection to the office (0002) gets no file view request: the allowed agent requests stay as they are.
- **0016/PB-2:** The office creates, changes, or deletes nothing in a CLI's configuration (0001/FR-021), in the agent's working folder, or in the repository's git data (0010/PB-2) because of the file view.
- **0016/PB-3:** The desktop app still needs the office token and a connection from the same machine for every request (0001/FR-007, FR-008).
- **0016/PB-4:** The file view changes no agent's status, session, terminal, task text, or Needs you item, and adds nothing to the action record.
- **0016/PB-5:** The panel, the terminals, and every existing dialog work as before; the file view adds one way in from an agent and nothing else changes.

### Key entities

None stored. Every answer is read from the root and its git data at the moment of the request; the office keeps no copy.

### Success criteria

- **0016/SC-001:** Of 20 requests for paths outside the root (parent parts, absolute paths, a link to a file outside, a link to a folder outside, a link inside a linked folder, and a link chain that leaves the root), 100% are refused, and 0 bytes of the outside files reach the desktop app.
- **0016/SC-002:** A file of exactly 512 KiB shows in full; a file of 512 KiB plus 1 byte shows its size and 0 bytes of content; a diff over 512 KiB shows its size and 0 lines.
- **0016/SC-003:** In a folder with 10 secret-looking files that are also changed files, opening the tree, each file, the Changes part, and each diff sends 0 bytes of their content to the desktop app until a reveal; after one reveal, exactly that one file's content arrives; after opening another file, it is hidden again. A `.env` renamed to `notes.txt`, in the Changes part and in a commit, sends 0 bytes of its diff until a reveal.
- **0016/SC-004:** After a session that lists every folder, opens every file, the changes, every diff, the history, and every commit of a test repository, the root and its git data are byte for byte the same as before, with the same modification times, no lock file was created, and a hook, a diff driver, a text conversion, a content filter, a signature check program, and a file system monitor that the repository names each wrote 0 marker files.
- **0016/SC-005:** In a repository with one modified, one staged new, one deleted, one renamed, and one untracked file inside the root and one modified file outside it, the Changes part lists exactly the 5 files inside the root, each with its kind, and each diff holds the removed and added lines of that file.
- **0016/SC-006:** With 120 commits that change the root and 10 that change only another folder of the repository, the History part shows 50, then 100, then 120 commits, newest first, none of the 10 others.
- **0016/SC-007:** Each visible state appears in a test with its text, and every action of the flow completes with the keyboard alone.
- **0016/SC-008:** A file and a commit message that hold `<script>`, an image tag, a terminal escape code (U+001B), and a right-to-left override (U+202E) show as text, with U+001B and U+202E as visible symbols, and 0 scripts run and 0 images load.
- **0016/SC-009:** In a working folder of 50,000 files, a folder of 1,000 entries lists within 2 seconds, a 512 KiB file shows within 2 seconds, and the Changes part of 1,000 changed files lists within 5 seconds.

### Notes from Deviations

- **One server commit for the folder reader and the git reader.** They share the port types of the application layer, so they do not build apart.
- **Filter switches reused.** The 0010 function that empties filter drivers lists the configuration with `config --list --name-only -z`, and it now also empties `smudge`. The 0010 worktree status and remove runs get the same switch, which changes nothing for them.
- **Submodules.** Every status and diff passes `--ignore-submodules=dirty`, so git never runs inside a submodule. A submodule shows as changed only when its recorded commit changed.
- **Root commit.** A commit without a parent is compared with the empty tree, in both the file list and the diff.
- **Renames across the root.** git pairs renames only inside the pathspec of the root, so in a subfolder root a file moved across the root's edge shows as added or deleted, never as a rename with an outside old path.
- **Pure rename.** A rename or a mode change without content change has no hunk, so its diff answers `none`, also after a reveal.
- **Read time.** The History part shows the desktop's load time, because the history answer has no read time.
- **Large commit message.** A message over the git runner's 1 MiB output limit answers `internal` instead of a shortened message.
- **Race and hard links.** An agent that swaps a folder for a link between the check and the read, or that adds a hard link to an outside file, can still make the view show that file.
- **Unix only.** The link, named pipe, and hook marker tests run on Linux and macOS, not on Windows. The `.git` refusal compares ASCII case only on Windows.
- **No hands-on run of the packaged app.** The desktop flow ran against the mock core and the server against real repositories, not together on a real agent.

## History

Each line is a change that was folded into this file, with the date its spec was archived. The last line records the split of the office spec.

- 0016-agent-file-browser - 2026-11-23
- split - 2026-11-28 - from `specs/office/SPEC.md` (capability `office`)
