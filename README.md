# office-harness

office-harness is a self-hosted control room for AI agent CLIs. A background server, `office-harnessd`, runs your agents in pseudo-terminals and keeps them running while the desktop app is closed. The desktop app shows every agent with its status and attaches to the live terminal of any agent. The server runs only on your machine, accepts only local clients that have the office token, and sends no office data to any outside service. It never writes the configuration files of the CLIs that it runs.

For the product goals, see [docs/PRD.md](docs/PRD.md). For the design, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). For the behavior that this release gives, see [specs/changes/0001-office-server-and-terminal/SPEC.md](specs/changes/0001-office-server-and-terminal/SPEC.md) and, for the agent hierarchy, [specs/changes/0002-agent-hierarchy/SPEC.md](specs/changes/0002-agent-hierarchy/SPEC.md).

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
5. The terminal of the agent opens in the agent details. Type into it as in a normal terminal. Every key goes to the CLI, except the focus shortcut.
6. To move focus from the terminal back to the rest of the app, press `Ctrl+Shift+F12`.
7. Close the app when you want. The agents keep running and the server keeps recording their output.

The app has no screen to change the focus shortcut. The server stores it as the setting `focus_shortcut`. The default value is `Ctrl+Shift+F12`. A value is modifiers joined with `+`, then one key, for example `Ctrl+Alt+F11`. A client can change it with the `settings.set` request (see [the client protocol](specs/changes/0001-office-server-and-terminal/contracts/client-protocol.md)). The app reads the setting when you open a terminal.

The agent and switcher shortcuts are fixed: `Ctrl+Shift+F1` to `Ctrl+Shift+F9` open the first nine agents of the list, and `Ctrl+Shift+F10` opens the agent switcher. On most macOS keyboards the F-keys send media commands by default, so the shortcuts work there only while you hold `Fn` (or after you turn on "Use F1, F2, etc. keys as standard function keys"). If you cannot change that setting, a macOS build of a later release can move the shortcuts to the fallback family `Cmd+Alt+1` to `Cmd+Alt+9` (agents) and `Cmd+Alt+0` (switcher), which no webview or macOS default takes; that fallback is not shipped yet. On KDE Plasma the default family is not verified until a hands-on QA pass; if a distribution binds it globally, the same `Cmd`/`Ctrl+Alt` fallback applies. The full fallback table is in [the panel-practicality spec](specs/changes/0003-panel-practicality/SPEC.md).

## The agent hierarchy

office-harness can organize the agents of an office as a company. The office has one CEO at the top. Under the CEO are teams, and each team has one team lead with its members under the lead. An agent without a role stays outside the hierarchy and keeps the behavior described so far.

The hierarchy view shows the office as a read-only tree: the CEO, every team with its project, its lead, and its members, and the agents without a role in their own group. Each agent shows its name, its role, and its status as text. You can move in the tree with the keyboard, as in the rest of the app. When the office has no CEO and no team, the view says so in text, and when the tree cannot load, it says so and offers to try again. A team without members shows its lead and says that it has no members.

The app has no hierarchy actions yet. Creating a CEO, a team with its team lead, or a member under a lead, the role choice that offers only the roles that the chosen place allows, and the stop and delete confirmations that show how many agents the action affects, come later in this change with task T7, "Create flows and confirmations". The server already applies the rules behind them: it refuses a second CEO and names the existing one, it refuses a team whose name another team already has, and it refuses a role that a place does not allow, naming the roles that are allowed there.

Stopping or deleting an agent that has agents under it affects them too. A confirmed delete removes the agent and every agent under it. A stop can stop the agent alone, or the agent together with the agents under it in one action. The server counts the agents under the deleted one again and refuses the delete when the number has changed since your confirmation.

Deleting a team removes the team only. Its lead and members keep their agents and show outside the hierarchy afterwards.

A client reaches the hierarchy over the protocol: the `team.create`, `team.delete`, `hierarchy.tree`, and `hierarchy.descendants` requests, and `agent.stop` and `agent.delete` extended with an `include_descendants` field; deleting an agent that has agents under it also carries `descendant_count`, the number that a confirmation showed. The shapes are in [the hierarchy protocol](specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md).

## Holding the office and limiting agents

An agent can create agents, start and stop them, start new tasks, and send directives without you. If one agent loops, it can flood a terminal or churn agents faster than you can read. A hold stops these agent-made actions. The limits slow an agent that acts too fast.

**Hold the office.** Choose "Hold office" in the panel header. The header then says "Office on hold" as text and offers "Release office". The shortcut is `Ctrl+Shift+H` while the keyboard focus is in the panel. It does not work inside a terminal, because a terminal gets every key. From a terminal, press the focus shortcut first, then `Ctrl+Shift+H`.

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

A limit is a whole number from 1 to 1000. Choose "Limits" in the panel header to see each limit with its default and to change it. A client can also change a limit with `settings.set` and the keys `limit_directives_per_agent`, `limit_directives_per_target`, `limit_creates_per_agent`, and `limit_lifecycle_per_agent`. A new value applies to the next agent-made action. The office does not queue or retry a refused action. Rate counts start again at 0 after a server restart.

**Automatic hold.** When an agent is refused by a limit 3 times within 60 seconds, the office puts that agent on hold and sends you one notification. The notification names the agent and the limit. Refusals that come from a hold do not count. You release an automatic hold the same way as any other hold.

**Office record.** The "Office record" list in the panel header, beside the office hold control, shows the entries that have no agent: each office hold and release. A hold or release of one agent shows in that agent's details. The user is the actor of a hold or release that you take. The office is the actor of an automatic hold.

The holds and the limits are a brake against mistakes and loops. They are not a security boundary: an agent CLI runs as your OS user and can act outside the office. The shapes of the requests are in [the hold protocol](specs/changes/0007-hold-and-limits/contracts/hold-protocol.md).

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

The folder holds `office.db` (the office), `token` (the office token), `server.json` (the port of the running server), `server.lock`, `server.log`, `output/` (one capped output file for each session), and `bin/` (the login copies). The token file and the stored output are readable only by your OS user.

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

Today the office answers an agent only for `agent.whoami`, which returns the agent's own id:

```sh
office-harnessd agent-call agent.whoami
```

```json
{"id":1,"ok":true,"result":{"agent_id":"<agent id>"}}
```

The command works the same in every supported CLI, because it is an ordinary program and the credential lives in the session environment, which the CLI and everything that it starts share. An agent runs it as a shell command in its session, in the way that its CLI runs shell commands. Outside a session the environment has no credential, so the command exits with code 2.

Every other request type is refused. The refusal of a type outside the agent list names the rule: an agent may only create, start, stop, and give new tasks to its own descendants, and send them directives. A type inside that list is refused for now with the reason that it is not available to agents yet. Later in this change, an agent will be able to create, start, and stop agents below it, start new tasks on them, and send them directives; the protocol contract lists the request shapes.

A hold or a limit can refuse these requests as well. The answer is `refused`, and its message is one of these, checked after the rules above in this order:

| Message | Meaning |
| ------- | ------- |
| `the office is on hold` | You hold the office. |
| `the agent <id> is on hold` | The asking agent, the target, or one of their ancestors is on hold. |
| `the limit "<name>" allows <n> per 60 seconds` | The agent is over a limit. `<name>` is `directives sent by one agent`, `directives sent to one target agent`, `agents created by one agent`, or `starts, stops, and new tasks by one agent`. |

The agent decides whether to try again. See [Holding the office and limiting agents](#holding-the-office-and-limiting-agents).

An agent acts only within its own descendants: the agents under it in the tree. When the actions of that list arrive, a request about an agent that is not the asking agent's descendant will be refused and recorded.

## Known limits

- A server that the OS starts at login has no `DISPLAY` or `WAYLAND_DISPLAY` variable, so the agents that it runs get none. A CLI can then lose clipboard and browser-open features.
- The installers are not signed (see [Install](#install)).
- The hands-on checks on macOS and Windows are still open. CI runs the automated tests on all three OSes, but nobody has used the installers by hand on macOS or Windows yet. The QA status is in [the QA report](specs/changes/0001-office-server-and-terminal/qa-report.md).
- Windows stops an agent with `taskkill /T /F`. It can miss a child process whose parent already ended.
- The quiet period and the output limit apply after a server restart.
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
