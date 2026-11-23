# office-harness

office-harness is a self-hosted control room for AI agent CLIs. A background server, `office-harnessd`, runs your agents in pseudo-terminals and keeps them running while the desktop app is closed. The desktop app shows every agent with its status and attaches to the live terminal of any agent. The server runs only on your machine, accepts only local clients that have the office token, and sends no office data to any outside service. It never writes the configuration files of the CLIs that it runs.

For the product goals, see [docs/PRD.md](docs/PRD.md). For the design, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). For the behavior that this release gives, see [specs/changes/archive/0001-office-server-and-terminal/SPEC.md](specs/changes/archive/0001-office-server-and-terminal/SPEC.md) and, for the agent hierarchy, [specs/changes/archive/0002-agent-hierarchy/SPEC.md](specs/changes/archive/0002-agent-hierarchy/SPEC.md).

## Install

The manual `release` workflow in `.github/workflows/release.yml` builds the installers on Linux, macOS, and Windows. It keeps them as workflow artifacts and publishes nothing.

1. Open the Actions tab of the repository on GitHub and run the `release` workflow.
2. When the run ends, download the artifact for your OS.

| OS      | Artifact                    | Installer                          |
| ------- | --------------------------- | ---------------------------------- |
| Linux   | `installers-ubuntu-latest`  | `.deb` package, `.AppImage` file   |
| macOS   | `installers-macos-latest`   | `.dmg` image                       |
| Windows | `installers-windows-latest` | `.exe` installer (NSIS)            |

The installers are not signed. Your OS can warn you when you open them. Use an installer only if you built it yourself or got it from a source that you trust.

- **Linux, deb:** run `sudo apt install ./<file>.deb`.
- **Linux, AppImage:** run `chmod +x <file>.AppImage`, then start the file.
- **macOS:** open the `.dmg` and move the app to the Applications folder. macOS can refuse to open an app that is not signed. If it does, open System Settings, go to Privacy & Security, and choose the option to open the app anyway.
- **Windows:** start the `.exe`. Windows SmartScreen can show a "Windows protected your PC" notice for an unsigned installer. Choose "More info", then "Run anyway".

The exact wording of these OS notices depends on the OS version. The macOS and Windows installers have not been tested by hand yet (see [Known limits](#known-limits)).

## First use

1. Start the office-harness app. If the office server is not running, the app shows "The office server is not running". Choose "Start server".
2. The app connects to the server. A new office shows "The office is empty". Choose "Create the first agent".
3. Enter a name, a command (the CLI to run, for example `claude`), and a working folder. You can also choose a project.
4. Select the agent and choose "Start agent".
5. The terminal of the agent opens in the agent details. Type into it as in a normal terminal. Every key goes to the CLI, except the focus shortcut and the app shortcuts (`Ctrl+Shift+F1` to `Ctrl+Shift+F11` and `Ctrl+Shift+H`, described below), which leave the terminal for the app.
6. To move focus from the terminal back to the rest of the app, press `Ctrl+Shift+F12`.
7. Close the app when you want. The agents keep running and the server keeps recording their output.

The app has no screen to change the focus shortcut. The server stores it as the setting `focus_shortcut`. The default value is `Ctrl+Shift+F12`. A value is modifiers joined with `+`, then one key, for example `Ctrl+Alt+F11`. A client can change it with the `settings.set` request (see [the client protocol](specs/changes/archive/0001-office-server-and-terminal/contracts/client-protocol.md)). The app loads the setting once, when it starts, so a change applies at the next app start.

The agent and switcher shortcuts are fixed: `Ctrl+Shift+F1` to `Ctrl+Shift+F9` open the first nine agents of the list, `Ctrl+Shift+F10` opens the agent switcher, and `Ctrl+Shift+F11` jumps to the agent at the top of Needs you (see below). On most macOS keyboards the F-keys send media commands by default, so the shortcuts work there only while you hold `Fn` (or after you turn on "Use F1, F2, etc. keys as standard function keys"). If you cannot change that setting, a macOS build of a later release can move the shortcuts to the fallback family `Cmd+Alt+1` to `Cmd+Alt+9` (agents) and `Cmd+Alt+0` (switcher), which no webview or macOS default takes; that fallback is not shipped yet, and the jump key has no entry in it yet, so on macOS it also works only while you hold `Fn`. On KDE Plasma the default family is not verified until a hands-on QA pass; if a distribution binds it globally, the same `Cmd`/`Ctrl+Alt` fallback applies. The full fallback table is in [the panel-practicality spec](specs/changes/archive/0003-panel-practicality/SPEC.md).

## Needs you

The panel has a "Needs you" section with the agents that wait for you. An agent gets one item when it asks for input (it was quiet for the quiet period), rings the terminal bell, fails, exits, is put on hold by the office after it hit a limit, or sends you a report. Each item shows the agent's name, its project, what the office saw, and how long the agent has waited, as text. A deleted agent, an agent that starts working again, and an agent that you stop have no item.

The items are in groups: failed agents first, then agents that wait for input, ring the bell, or were held by the office, then exited agents. Inside a group the longest wait comes first. The number of items shows next to "Needs you" and in the window title, in the panel and in the office scene. The server keeps the items, so events that happened while the app was closed show when it opens.

Press `Ctrl+Shift+F11` to open the terminal of the agent at the top of the list, with keyboard focus, from the panel, from a focused terminal, or from the office scene (there it opens the top agent's desk). The item leaves the list. Press the key again for the next agent. When the list is empty the view stays and the app says "No agent needs you". An exited agent that you will not restart can leave the list with its "Dismiss" button, which changes nothing else. Opening an agent's terminal by any route also removes its item.

If the stored `focus_shortcut` is `Ctrl+Shift+F11`, the focus shortcut keeps the key and the jump has no key. The Needs you section then says: "The jump shortcut is off because the focus shortcut uses Ctrl+Shift+F11. Change the focus_shortcut setting (see the README) and reopen the app to turn the jump on." Change the setting, as described above, and restart the app.

A report from an agent shows its kind and text on the item; a long text shows its first three lines and a "Show the whole report" button. The item stays when the agent works again, and leaves when you open its terminal or dismiss it. Answer the agent in its terminal.

A client reaches the list with the `needs_you.list` and `needs_you.dismiss` requests and the `needs_you.changed` event. The shapes are in [the needs-you protocol](specs/changes/0008-needs-you-queue/contracts/needs-you-protocol.md).

## The agent hierarchy

office-harness can organize the agents of an office as a company. The office has one CEO at the top. Under the CEO are teams, and each team has one team lead with its members under the lead. An agent without a role stays outside the hierarchy and keeps the behavior described so far.

The hierarchy view shows the office as a read-only tree: the CEO, every team with its project, its lead, and its members, and the agents without a role in their own group. Each agent shows its name, its role, and its status as text. You can move in the tree with the keyboard, as in the rest of the app. When the office has no CEO and no team, the view says so in text, and when the tree cannot load, it says so and offers to try again. A team without members shows its lead and says that it has no members.

The app has no hierarchy actions yet. Creating a CEO, a team with its team lead, or a member under a lead, the role choice that offers only the roles that the chosen place allows, and the stop and delete confirmations that show how many agents the action affects, come later in this change with task T7, "Create flows and confirmations". The server already applies the rules behind them: it refuses a second CEO and names the existing one, it refuses a team whose name another team already has, and it refuses a role that a place does not allow, naming the roles that are allowed there.

Stopping or deleting an agent that has agents under it affects them too. A confirmed delete removes the agent and every agent under it. A stop can stop the agent alone, or the agent together with the agents under it in one action. The server counts the agents under the deleted one again and refuses the delete when the number has changed since your confirmation.

Deleting a team removes the team only. Its lead and members keep their agents and show outside the hierarchy afterwards.

A client reaches the hierarchy over the protocol: the `team.create`, `team.delete`, `hierarchy.tree`, and `hierarchy.descendants` requests, and `agent.stop` and `agent.delete` extended with an `include_descendants` field; deleting an agent that has agents under it also carries `descendant_count`, the number that a confirmation showed. The shapes are in [the hierarchy protocol](specs/changes/archive/0002-agent-hierarchy/contracts/hierarchy-protocol.md).

## Templates

A template is a saved definition that you reuse. An agent template holds a name pattern, a CLI command, an optional working folder, the office additions (flags, environment variables, startup prompt), an optional role, and the [own worktree](#own-worktree-per-agent) choice, which is off by default. A team template holds one lead entry and one or more member entries. Each member entry has a count, and a team can have at most 50 agents. Templates are copies. Editing or deleting a template changes no agent or team that was made from it, and a team template keeps its own copy of each entry. The server keeps the templates in the office, so they survive a restart. Only you use templates: agents cannot.

Choose "Templates" in the panel to see them. The view says "No template exists yet." when there is none. It has the buttons "New template", "Import templates", and "Export templates". Every part of the flow works from the keyboard.

**Name patterns.** A pattern is the text that the office makes an agent name from. `{team}` becomes the name of the team, and `{n}` becomes the lowest number that makes the name unique. A pattern without `{n}` gets `-<number>` added only when the name is taken. An agent that you save as a template gets the pattern `<agent name>-{n}`. In a team template, the lead entry creates a team lead and each member entry creates members, whatever role the entry holds. A pattern with `{team}` needs a team: used for a direct creation without one, it is refused and names the pattern.

**Save an agent as a template.** Open the agent details and choose "Save as template". The dialog copies the command, working folder, office additions, and role of the agent. Give the template a name and a pattern, then save. The agent does not change. You can also build an agent template from nothing with "New template" in the templates view.

**Own worktree in a template.** The agent editor and each entry of a team template have an "own worktree" checkbox, off by default. A template file without the field imports with the choice off. "Save as template" copies the choice from an agent that has its own worktree, and writes the repository folder (with its subfolder) as the working folder, not the worktree folder. A template never stores the base or a branch name: each creation reads the base from the working folder and takes the next free branch name (`office-harness/<agent name>`, then `-2`, `-3`). Each agent that is made from an entry with the choice on gets its own branch and worktree. If the folder of such an entry is not inside a git repository, the office refuses the whole creation with "own worktree needs a git repository" beside the other reasons and creates nothing. If a late step fails, the office removes every agent, worktree, and branch that the creation made, without asking. The create dialog and the import preview show the choice as "Own worktree: yes" or "no".

**Build a team template.** In the templates view, choose "New template" and pick the team kind. Fill in the lead entry and each member entry with its count. The office refuses a team template that would create more than 50 agents and names the limit.

**Create from a template.** Choose "Create from template" beside the create buttons of the hierarchy view, then pick a template.
- For an agent template, pick the place (a team, when the template has a role) and an optional project. The office creates one agent and does not start it. When the template has no working folder, the folder of the project is the working folder.
- For a team template, type the team name and pick an optional project. The office creates the team, its lead under the CEO, and each member under the lead in one action. A CEO and two teams of one lead and six members take three create actions.

Creation is all-or-nothing. If the team or any agent cannot be created, the office creates none of them and names each reason, for example a taken name, a role that the place does not allow, or a folder that does not exist. When a working folder is missing and no project gives one, the office says that a working folder is needed. Each created agent is recorded in the action record with you as the actor.

**Export.** Choose "Export templates". The app first says that the file leaves out every environment value, and that flags and startup prompts are written as they are, so you must remove a secret from them first. Then choose "Choose a file and export" and pick the file in the system file dialog. The file is plain text with a format version. Each environment variable keeps its name and has no value. The text of the file is the `text` that the server returns for the `template.export` request, and the app writes exactly that text to the path you picked. If the file cannot be written, the app says that it was not written, and no template changes.

**Import.** Choose "Import templates" and pick a file. The app shows an import preview with every template: its name in the file, the name it will be saved under, its kind, CLI commands, working folders, flags, environment variable names, and startup prompts. Control characters in the text show as visible symbols and are never applied. The preview saves nothing. Choose "Save this template" (or "Save these <n> templates") and the office saves the templates. It creates no agent. Cancel (or press Escape) and the office saves nothing. An imported template never replaces one: a name that exists gets ` (2)`, ` (3)`, and so on. The office refuses the whole file, names the template and the reason, and saves nothing when the format version is unknown, the text cannot be read as a template file, the file is larger than 256 KiB, a team template holds more than 50 agents, or a template breaks a save rule (for example an empty command or a count below 1). Treat a file from another person like a program: the preview shows every command that the office could run later.

**Environment variables without a value.** The values of an imported file are never saved: every imported environment variable is saved with no value. The template and the create dialog show it as `NAME: no value`. The details of an agent list it as `NAME: no value`. In the editor of the additions it is a bare `NAME` line, and a line `NAME=` is an empty value. The office does not pass a variable without a value to the CLI, so the value from your login environment stays in effect. A variable with an empty value is different: the office passes it as empty. Inside the office, templates keep the values that you type, as agents do. Only export and import leave them out.

A template holds only definitions of office-harness. The office never writes the configuration of a CLI because of a template, and an export writes only the file that you pick. The shapes of the requests are in [the template protocol](specs/changes/0009-agent-templates/contracts/template-protocol.md). The behavior is in [the templates spec](specs/changes/0009-agent-templates/SPEC.md), and the QA status is in [the QA report](specs/changes/0009-agent-templates/qa-report.md).

## Holding the office and limiting agents

An agent can create agents, start and stop them, start new tasks, and send directives without you. If one agent loops, it can flood a terminal or churn agents faster than you can read. A hold stops these agent-made actions. The limits slow an agent that acts too fast.

**Hold the office.** Choose "Hold office" in the panel header. The header then says "Office on hold" as text and offers "Release office". The shortcut is `Ctrl+Shift+H` while the keyboard focus is in the panel. It also works inside a focused terminal: every key of the shortcut table leaves a terminal and is not sent to the CLI.

**Hold one agent.** Open the agent details and choose the hold action. The agent and every agent under it are held. The agent's row says "on hold" as text. Choose the release action in the details to end it. Releasing the office does not release agent holds.

**What a hold refuses.** While the office is on hold, the office refuses every agent-made `directive.send`, `agent.create`, `agent.start`, `agent.stop`, and `agent.new_task`. While one agent is on hold, the office refuses these actions when the held agent or an agent under it asks for them, and when the held agent or an agent under it is the target. The parent and the CEO of a held agent are refused too when they act on it. Each refusal reaches the asking agent as the answer to its request, and the action record keeps it.

**What a hold never refuses.** Your own actions in the app (create, start, stop, delete, new task) work as without a hold. Your typing reaches the terminals. The terminals and the CLIs keep running. To stop a CLI, use the stop action.

**Holds survive restarts.** A hold stays until you release it, also after a server or machine restart. It never ends on a timer.

**Limits.** An agent that acts too fast is refused. The office counts each limit over a rolling 60 seconds:

| Limit | Default |
| ----- | ------- |
| Directives sent by one agent | 10 |
| Directives sent to one target agent | 3 |
| Agents created by one agent | 5 |
| Starts, stops, and new tasks by one agent | 10 |
| Reports sent by one agent | 5 |

A limit is a whole number from 1 to 1000. Choose "Limits" in the panel header to see each limit with its default and to change it. A client can also change a limit with `settings.set` and the keys `limit_directives_per_agent`, `limit_directives_per_target`, `limit_creates_per_agent`, `limit_lifecycle_per_agent`, and `limit_reports_per_agent`. A new value applies to the next agent-made action. The office does not queue or retry a refused action. Rate counts start again at 0 after a server restart.

**Automatic hold.** When an agent is refused by a limit 3 times within 60 seconds, the office puts that agent on hold and sends you one notification. The notification names the agent and the limit. Refusals that come from a hold do not count. You release an automatic hold the same way as any other hold.

**Office record.** The "Office record" list in the panel header, beside the office hold control, shows the entries that have no agent: each office hold and release. A hold or release of one agent shows in that agent's details. The user is the actor of a hold or release that you take. The office is the actor of an automatic hold.

The holds and the limits are a brake against mistakes and loops. They are not a security boundary: an agent CLI runs as your OS user and can act outside the office. The shapes of the requests are in [the hold protocol](specs/changes/archive/0007-hold-and-limits/contracts/hold-protocol.md).

## The task line

Each agent row can show one more line under the last output line: the last task the office gave the agent, with its age, for example `Task given 3 min ago: Fix the login form`. The line shows the first line of the text, at most 80 characters, with an ellipsis when it is cut. The age counts whole minutes ("0 min ago" to "59 min ago") and then hours ("2 h ago").

**Where the text comes from.**
- A directive that the office entered into the agent's terminal sets the text, from the time it arrived. A directive that the office refused sets nothing.
- You can set, edit, or clear the text in the agent's details, in the "Task" section: choose "Set task" or "Edit task", type, and choose "Save task"; choose "Clear task" to remove it. The text can have at most 8,192 characters. Empty text clears it. This writes nothing to the terminal.
- A new task (the "Start new task" action) clears the text. Starting an agent, continuing a session, and a restart of the server keep it.
- The office never reads the text from terminal output, prompts, or window titles.

**Safe display.** The row and the details show control characters and terminal escape codes as visible symbols (for example ESC shows as the symbol U+241B) and never apply them. The details show the full text. The scene's desks and the "Needs you" list do not show the task text.

The shapes of the request and the event are in [the task protocol](specs/changes/0011-task-line/contracts/task-protocol.md).

## Own worktree per agent

Two agents that work in one git repository folder change the same files. To keep them apart, turn on "Own worktree" when you create an agent whose working folder is inside a git repository. The office then creates a new branch and a git worktree of it for that agent, and the agent's CLI starts there. Your own repository folder does not change; the repository only gets the new branch and the worktree record.

**At create.**
- "Own worktree" shows only when the folder is inside a git repository, and it is off by default. It needs git 2.22 or newer on the login `PATH`.
- The base is the branch checked out in that folder (or its commit, when no branch is checked out). You can pick another local branch.
- The branch is named `office-harness/<agent name>`, in lower case with every other character replaced by `-`. When that name is taken, the office adds `-2`, `-3`, and so on. The form shows the name before you create the agent.
- The worktree is in the `worktrees/` folder of the [data folder](#data-folder), never inside your repository. When you chose a subfolder of the repository, the agent starts in the same subfolder of its worktree.
- Untracked files, such as local environment files, are not copied into the worktree.
- If the branch or the worktree cannot be made, no agent is created and no branch or folder is left.
- The office runs git without hooks and without asking for credentials, and it writes no git configuration. Checkout filters that your repository configures, such as Git LFS, still run during the checkout, as in your own checkout, and can download files. Later reads, such as the change count before a delete, run no filter; a file that only a filter changes, such as a Git LFS file, can then count as changed.
- Only you can turn the choice on. Agents that create agents never get an own worktree.

**In the details.** The agent's details show its branch, its base, and the worktree folder. If the worktree folder is gone or no longer linked to its repository (for example after you moved the repository), the agent does not start and says why.

**At delete.** Deleting an agent with an own worktree asks once, for the agent and every descendant that the delete covers, whether to keep or remove the worktrees. Keep is the default.
- **Keep:** the folder and the branch stay, and the office shows where they are. From then on they are yours; the office does not manage them.
- **Remove:** the office stops the agent, counts the changes in the worktree again, and removes it only if nothing changed since the question. A worktree with uncommitted changes (changed, added, deleted, or untracked files) is removed only after a second confirmation that names the number of changed files. Files that git ignores are not counted and are removed with the worktree.
- The branch is deleted only when it has no commits that its base lacks and the base still exists. Otherwise it stays, and the office names it. A branch checked out in another folder also stays.
- A worktree whose repository cannot be read is kept, and the agent is still deleted.

The office never commits, merges, pushes, or rebases. Worktrees share the repository's history, so their branches and commits show in your repository too. To remove a kept worktree yourself, run `git worktree remove <folder>` in the repository and `git branch -D <branch>` if you no longer need the branch.

The shapes of the requests are in [the worktree protocol](specs/changes/0010-agent-worktree/contracts/worktree-protocol.md).

## Browse an agent's files

To review what an agent did, choose "Browse files" in its details. A read-only view of the folder the agent starts in (its working folder, its own worktree, or its project's folder) opens with three tabs:

- **Files:** the folder as a tree, folders first. Open a file to read it with line numbers.
- **Changes:** the files that differ from the last commit, staged and unstaged together, with what kind of change each is. Open one to see its diff. Ignored files are not listed.
- **History:** the commits that changed this folder, 50 at a time ("Show more" adds the next 50). Open a commit to see its message and its files, and a file to see its diff against the commit's first parent.

**What it shows and what it does not.**
- Only the agent's folder: a path that leads outside it, also through a link, is refused. The repository's `.git` folder is never listed or opened.
- Files and diffs up to 512 KiB; a larger one shows its size only. Binary files are not shown. A folder lists at most 1,000 entries and the Changes tab at most 1,000 files; the rest is counted.
- Files with names that often hold secrets, such as `.env`, `*.pem`, `*.key`, `id_rsa`, or names containing `secret` or `credential`, show their content or diff only after you choose "Reveal". The reveal lasts until you open another file, switch tabs, or refresh.
- Everything shows as plain text: no markup is rendered, no image loads, and control characters show as visible symbols.
- The view writes nothing. It changes no file, takes no git lock, and runs no git hook, filter, external diff, text conversion, or signature program that a repository configures. It shows the state at the time of the last load; choose "Refresh" to load again.

Agents cannot use this view. The shapes of the requests are in [the file view protocol](specs/changes/archive/0016-agent-file-browser/contracts/file-view-protocol.md).

## Resume interrupted agents

When the background server stops while agents run (a crash, a logout, a restart of the machine), those agents show `interrupted` at the next start, and none starts by itself. The panel header then offers **Resume interrupted (N)**.

- The dialog lists the interrupted agents in start order: members first, then leads, then the CEO, so a resumed lead finds its members running. Each row says what will happen: continue the session that started at a given time, or start a new session, with the reason (for example, the CLI cannot continue a session). Agents on hold are listed unchecked.
- After an unclean stop (for example a SIGKILL of the server, a crash, or a power loss), the agents whose last session was still open are marked. A warning says that their CLI processes from before may still run, and Confirm waits until you tick that you understand that a second copy may start. The session list shows such a session as `lost`. Before this warning, the server ends the leftover processes that it can prove are its own (see below).
- After Confirm, each agent's result shows live: checking, continued, new session (with the reason), on hold (with the reason), or skipped (with the reason). You can close the dialog; the server finishes the checks.
- The office watches each start for 20 seconds. When a continued session ends by itself in that time, the office starts the agent once in a new session. That start is in the agent's action record with the office as actor and the reason, because the CLI's earlier context was not carried over. When that new session also ends within 20 seconds, the office does not start the agent again: it puts the agent on hold with the reason and notifies you once.
- Any end of the CLI within the 20 seconds counts, whatever its exit code. A command that ends quickly by design is started twice and then held; uncheck such agents.

The shapes of the requests are in [the resume protocol](specs/changes/archive/0013-bulk-resume/contracts/resume-protocol.md).

### Leftover processes after an unclean stop

While a session runs, the server stores the identity of its CLI process and of the helper processes that descend from it. After an unclean stop, the server ends those processes before any agent can start again. The panel shows "Ending processes left from the last run" until this is done, and your start requests wait.

- The server ends a process only when its start time, its command line, and the machine run all match what it stored. It asks the process to stop, and ends it by force after the stop grace period. On Windows it ends it by force at once.
- A process that the server cannot prove is its own is left alone. This includes a process with a reused process id.
- Each result is in the action record as `end_leftover`. When a process cannot be ended, you get a `leftover_kept` notification for the agent.
- After the sweep, the resume warning names only the agents for which the server cannot rule out a running process.

The shapes are in [the sweep protocol](specs/changes/0017-orphan-sweep/contracts/sweep-protocol.md).

## Idle sleep

Idle sleep is off by default. When you turn it on, the office puts an agent to sleep after it stays idle for a set time. A sleeping agent uses no memory for its CLI or for the helper servers that the CLI started.

Press **Idle sleep** in the panel header to open the setting. Turn the switch on and set the idle time in minutes, a whole number from 5 to 1440. The field shows 30 when you first turn it on. A change applies at once, and the count starts again from that moment, so no agent that was idle for hours sleeps before the idle time has passed. The background server keeps the setting and applies it while the desktop app is closed.

- An agent is idle when its status is `waiting for input` without a break. The idle time counts from the latest of three moments: the agent changed to `waiting for input`, your last key reached its terminal, and you last turned idle sleep on or changed the idle time.
- An idle agent stays awake when it has an item in Needs you, when a hold covers it, when a directive or a report waits in its queue, when pending office text or unsent text of yours waits at its input line, or when one of its children runs. The office checks these again right before it ends the CLI.
- A sleep ends the CLI as your stop does. The agent shows `asleep`. The terminal view keeps the last output, with the lines that the CLI printed while it ended. The office keeps the session so that a wake can continue it. The action record gets one `sleep` entry with the office as actor and the reason "idle for N minutes". A sleep sends no notification and adds no Needs you item.
- **Warning:** a sleep also ends every program that the CLI started and that still runs, such as a background task or a development server. The office does not detect these programs, and their work is lost.
- An asleep agent stays asleep when the server restarts and when you turn idle sleep off. It is not listed in **Resume interrupted**. Nothing starts it except your action.

To wake an agent, give its terminal view keyboard focus and press Enter, or press **Wake** in the terminal view or in the agent's details. No other key reaches the CLI while the agent is asleep. The office continues the last session. When the CLI cannot continue it, the office starts one new session with the reason. When a continued session ends within 20 seconds, the office starts one new session. When that session also ends within 20 seconds, the office puts the agent on hold with the reason and notifies you once, as it does for a resume. The result shows as text in the details and in the terminal view. A hold does not stop a wake, and the hold stays. The wake is in the action record with you as actor.

A directive or a report to an agent that is asleep, or is going to sleep, is refused. The answer says that the agent is not running because it is asleep. The refusal is in the action records.

The shapes of the requests are in [the sleep protocol](specs/changes/0018-idle-sleep/contracts/sleep-protocol.md).

## Start at login

The server runs only while you are logged in. Agents end when you log out or shut the machine down. After the next login the server starts again, and the agents that were running show `interrupted`. No agent starts without your action.

The app registers the server for login on its first start and on the first start after an upgrade, so you do not need to do it yourself. After you run `office-harnessd unregister`, the app does not register it again; the button below or `office-harnessd register` turns login start on again. The app also shows a "Server at login" section with the button "Start the server at login".

You can also register it from a terminal. `office-harnessd` has the login subcommands:

```sh
office-harnessd register
office-harnessd unregister
office-harnessd status
```

It has one more subcommand, `agent-call`, which runs inside an agent session (see [Calling the office from an agent](#calling-the-office-from-an-agent)).

- `register` copies the server program to `<data folder>/bin/<version>/` and points the login item at that copy. The copy is stable if the app moves or an AppImage unmounts. Each version gets its own folder. Running `register` again gives the same result.
- `unregister` removes the login item and every copy in `<data folder>/bin`, and records in the data folder that login start is off. The other files in the data folder stay. On Windows, stop the server first: Windows does not delete the file of a running program, so the command exits with code 1 when the server runs.
- `status` prints `registered` or `not registered`.

The subcommands always use the default data folder, because the login item starts the server without arguments. On Linux, run them in your desktop session: without a `systemctl --user` connection they do not see a systemd user unit. On Windows the program does not open a console, so redirect the output to read it, for example `office-harnessd status > status.txt`.

The login item is a systemd user unit (or an XDG autostart entry when `systemctl --user` does not answer) on Linux, a LaunchAgent on macOS, and a value under the current user's `Run` registry key on Windows. On Linux, `register` refuses a program path that contains `"`, `\`, `$`, `` ` ``, or `%`, because the login item would read them as special characters. The systemd unit restarts the server after a failure, but not after exit code 75, which means that a server already runs.

Exit codes:

| Code | Meaning                                                           |
| ---- | ----------------------------------------------------------------- |
| 0    | The command worked. For `status`, the server is registered.       |
| 1    | The command failed, for example a file or the OS refused it.      |
| 2    | An argument is wrong. The subcommands take no arguments.          |
| 3    | `status` only: the server is not registered.                      |

The program is in the folder where the installer put the app (the app calls it the sidecar), and a registered copy is in the data folder.

## Data folder

The server keeps its state in your local application data folder:

| OS      | Folder                                                          |
| ------- | --------------------------------------------------------------- |
| Linux   | `$XDG_DATA_HOME/office-harness`, or `~/.local/share/office-harness` |
| macOS   | `~/Library/Application Support/dev.office-harness.office-harness` |
| Windows | `%LOCALAPPDATA%\office-harness\office-harness\data`             |

The folder holds `office.db` (the office), `token` (the office token), `server.json` (the port of the running server), `server.lock`, `server.log`, `output/` (one capped output file for each session), `worktrees/` (the agents' own git worktrees, see [Own worktree per agent](#own-worktree-per-agent)), and `bin/` (the login copies). The token file and the stored output are readable only by your OS user.

The server accepts `--data-folder <path>` to use another folder. A server started at login always uses the default folder.

## Regenerate the token

A client needs the office token before the server accepts it. The app reads the token from the `token` file in the data folder. The server writes the file when it first starts. To make a new token, for example after the server rejected the app:

1. Stop the office server program (`office-harnessd`). The app then shows "The office server is not running".
2. Delete the file `token` in the data folder (the folder that holds `server.json`).
3. In the app, choose "Start server". The server writes a new token and the app reads it.

A client that is already connected can also send the `token.regenerate` request. The server then rejects the old token and closes every connection that used it. The app has no button for this.

## Troubleshooting

**"The office server is not running"** (server not reachable). The app cannot reach the server. Choose "Start server". If the app shows "The server did not start" with a reason, read the reason and the last lines of `server.log` in the data folder.

**"The server did not accept this app"** (not authorized). The server rejected the token that the app reads. Follow the steps in [Regenerate the token](#regenerate-the-token). If you changed the token file another way, choose "Check again".

**"The office server is busy"**. The server has no free connection slot. The app tries again on its own. Wait a few seconds.

**"The server has another version"**. The app and the running server have different versions. Choose "Restart server" and confirm. This stops every running agent and ends its session. Then start the server again. If the server speaks a protocol that the app does not know, the app cannot restart it. Stop the server program yourself, then reopen the app.

## Supported CLIs

| CLI        | Command      | Continue a session |
| ---------- | ------------ | ------------------ |
| Claude Code | `claude`    | Yes                |
| Codex      | `codex`      | Yes                |
| Gemini CLI | `gemini`     | Yes                |
| OpenCode   | `opencode`   | Yes                |
| Other      | any command  | No                 |

You can run any other command as an agent. It starts as a generic command and cannot continue a session.

office-harness starts each CLI with your home folder, your login environment, and your CLI configuration. It adds only the flags, environment variables, and startup prompt that you set for that agent. It never creates, changes, or deletes a file in a CLI's configuration. It reads some session files of a CLI to tell its sessions apart.

The startup prompt is pasted into the terminal after the first output of the CLI. office-harness never presses Enter for you. If the CLI shows a question first, for example a trust question, you answer it and press Enter to submit the prompt. A change to the additions of a running agent applies at the next session start.

### Sign-in check

Before a session of `claude` or `codex` starts, the office asks the CLI whether it is signed in, with its own read-only command (`claude auth status --text`, `codex login status`). The command gets the agent's folder, login environment, and additions, never the office's agent credential, and runs for at most 5 seconds. Other requests do not wait for it.

- When the CLI says it is not signed in, the agent shows `failed`, and the reason says so and tells you to sign in to the CLI in your own terminal. On a running agent, a new task or a continue is refused with that reason, and the running session goes on.
- When the command times out, cannot run, or gives an answer the office does not know, the start goes on as before, with a note in the action record.
- No check runs when the agent's command has its own arguments, the additions hold flags, or a sign-in key variable is set (for example `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`), because the status command cannot see what those change.
- For the first 20 seconds of a `claude`, `codex`, or `gemini` session, the office reads the bottom of the screen for the CLI's sign-in screen. When it shows and nobody typed into the terminal, the office ends the CLI, the session ends as not signed in, and you get one notification. A startup prompt that holds the sign-in words turns this watch off.

The office shows at most one line of what the CLI printed, at most 200 characters, with control characters made visible. It never shows or keeps the output of a signed-in answer.

## Calling the office from an agent

While an agent's session runs, office-harness gives that session three environment variables. They are additions to the session environment only; office-harness writes nothing to the configuration of the CLI.

| Variable | Value |
| -------- | ----- |
| `OFFICE_HARNESS_AGENT_TOKEN` | The call credential of this session, as `<agent id>:<secret>`. A new session of the same agent gets a new one, and it stops working when the session ends. |
| `OFFICE_HARNESS_CLI` | The absolute path of the running `office-harnessd` program. |
| `OFFICE_HARNESS_SERVER_FILE` | The path of the `server.json` file in the data folder, from which the helper reads the port. |

The office token is not in any agent environment. An agent connection proves its own credential, reaches only the local server, and gets no events and no terminals.

An agent asks the office with the helper:

```sh
office-harnessd agent-call <request type> [<json fields>]
```

The helper reads the credential and the server file from the environment, connects to the server, sends one request, and prints the answer as JSON. It exits with code 0 when the answer is a success, code 2 when the command is used wrong or runs outside an agent session, and code 1 when anything else fails, for example when the server refuses the request or cannot be reached.

The office answers `agent.whoami` with the agent's own id:

```sh
office-harnessd agent-call agent.whoami
```

```json
{"id":1,"ok":true,"result":{"agent_id":"<agent id>"}}
```

The command works the same in every supported CLI, because it is an ordinary program and the credential lives in the session environment, which the CLI and everything that it starts share. An agent runs it as a shell command in its session, in the way that its CLI runs shell commands. Outside a session the environment has no credential, so the command exits with code 2.

An agent reads the state of the agents below it with `agent.subtree`, which takes no fields:

```sh
office-harnessd agent-call agent.subtree
```

The answer lists each descendant after its parent, with its name, role, status, the time its terminal last showed output, whether it waits in Needs you, whether a hold covers it, and its task line. It never carries terminal output, and it lists no agent outside the caller's own subtree. A hold does not refuse the read, the read counts toward no rate limit, and an answered read is not in the action record. The shapes are in `specs/changes/archive/0014-subtree-status/contracts/subtree-protocol.md`.

An agent reports to its parent with `report.send`, with a kind (`done`, `question`, `blocked`, or `info`) and a text of up to 1,000 characters:

```sh
office-harnessd agent-call report.send '{"kind":"done","text":"tests pass"}'
```

The office types the report into the parent's terminal as one line, `[report from <name> (<agent id>): <kind>] <text>`, with the same checks as a directive: it waits while the parent works, shows a question, or a person types there. The parent answers with `directive.send` to the id in the line. A report from the CEO, from an agent without a parent, or with `"for_person":true` goes to you instead: it joins Needs you and notifies you once. Control characters, line breaks, and bidirectional characters in the text show as visible symbols. One sender has at most one report waiting in its parent's queue (a newer one replaces it), and reports take at most 10 of the queue's 20 places. Holds and the reports limit apply. The shapes are in `specs/changes/archive/0015-upward-reports/contracts/report-protocol.md`.

An agent also creates, starts, and stops agents below it, starts new tasks on them, and sends them directives; the protocol contract lists the request shapes. Every other request type is refused, and the refusal names the rule: an agent may only create, start, stop, give new tasks to, send directives to, and read the state of its own descendants, and send reports to its parent or the person.

A hold or a limit can refuse these requests as well. The answer is `refused`, and its message is one of these, checked after the rules above in this order:

| Message | Meaning |
| ------- | ------- |
| `the office is on hold` | You hold the office. |
| `the agent <id> is on hold` | The asking agent, the target, or one of their ancestors is on hold. |
| `the limit "<name>" allows <n> per 60 seconds` | The agent is over a limit. `<name>` is `directives sent by one agent`, `directives sent to one target agent`, `agents created by one agent`, `starts, stops, and new tasks by one agent`, or `reports sent by one agent`. |

The agent decides whether to try again. See [Holding the office and limiting agents](#holding-the-office-and-limiting-agents).

An agent acts only within its own descendants: the agents under it in the tree. When the actions of that list arrive, a request about an agent that is not the asking agent's descendant will be refused and recorded.

## Known limits

- A server that the OS starts at login has no `DISPLAY` or `WAYLAND_DISPLAY` variable, so the agents that it runs get none. A CLI can then lose clipboard and browser-open features.
- The installers are not signed (see [Install](#install)).
- The hands-on checks on macOS and Windows are still open. CI runs the automated tests on all three OSes, but nobody has used the installers by hand on macOS or Windows yet. The QA status is in [the QA report](specs/changes/archive/0001-office-server-and-terminal/qa-report.md).
- Windows stops an agent with `taskkill /T /F`. It can miss a child process whose parent already ended.
- The quiet period and the output limit apply after a server restart.
- After an unclean stop of the server, a CLI that ignores the terminal hang-up can keep running until the next server start. The `opencode` background service does. The next start ends it (see [Resume interrupted agents](#resume-interrupted-agents)). On macOS and on a Linux kernel before 5.3, no process handle exists, so a process that exits and whose id is reused in the short time before the end could be ended by mistake.
- The app has no update check, because office-harness opens no connection to an outside service.
- The database upgrade is forward-only. An older program refuses a database that a newer one upgraded. To go back, use the previous program together with a copy of `office.db` that you made before the upgrade (see [Data folder](#data-folder)). The program does not make that copy for you, so copy the file before you install a new version. Restore the copy and the previous program as one step.

## Development

You need Rust 1.100.0 (the repository pins it in `rust-toolchain.toml`), Bun 1.4.4, and Node 24. On Linux you also need the Tauri system packages. The list is in the `Install Linux system packages for Tauri` step of `.github/workflows/ci.yml`.

On a machine without rustup, run the Rust lint gates with scripts/lint-rust.sh through the pinned toolchain, so that a wrong rustc cannot hide lints from CI:

```sh
nix-shell -p rustup --run "rustup run 1.100.0 scripts/lint-rust.sh -p <crate>"
```

Install the JavaScript dependencies from the repository root:

```sh
bun install --frozen-lockfile
```

Run the checks. CI runs these on Linux, macOS, and Windows. Build the server program for the desktop app first, because Tauri needs it to exist:

```sh
cd src/apps/desktop
bun scripts/prepare-sidecar.ts
cd ../../..
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-boundaries.sh --self-test
scripts/check-boundaries.sh
cd src/apps/desktop
bun run check
bun run lint
bun run format:check
bun test src
bunx playwright install --with-deps chromium
bun run e2e
bun run tauri build --no-bundle
```

Run the desktop app in development mode from `src/apps/desktop`:

```sh
bun run tauri dev
```

`bun run tauri build` builds the installers for your OS. They appear under `target/release/bundle/`.

## License

office-harness is free software under the GNU Affero General Public License, version 3 or later (`AGPL-3.0-or-later`). See [LICENSE](LICENSE).
