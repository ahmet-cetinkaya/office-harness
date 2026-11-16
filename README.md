# office-harness

office-harness is a self-hosted control room for AI agent CLIs. A background server, `office-harnessd`, runs your agents in pseudo-terminals and keeps them running while the desktop app is closed. The desktop app shows every agent with its status and attaches to the live terminal of any agent. The server runs only on your machine, accepts only local clients that have the office token, and sends no office data to any outside service. It never writes the configuration files of the CLIs that it runs.

For the product goals, see [docs/PRD.md](docs/PRD.md). For the design, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). For the behavior that this release gives, see [specs/changes/0001-office-server-and-terminal/SPEC.md](specs/changes/0001-office-server-and-terminal/SPEC.md).

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

## Start at login

The server runs only while you are logged in. Agents end when you log out or shut the machine down. After the next login the server starts again, and the agents that were running show `interrupted`. No agent starts without your action.

The app registers the server for login on its first start and on the first start after an upgrade, so you do not need to do it yourself. After you run `office-harnessd unregister`, the app does not register it again; the button below or `office-harnessd register` turns login start on again. The app also shows a "Server at login" section with the button "Start the server at login".

You can also register it from a terminal. `office-harnessd` has three subcommands:

```sh
office-harnessd register
office-harnessd unregister
office-harnessd status
```

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

## Known limits

- A server that the OS starts at login has no `DISPLAY` or `WAYLAND_DISPLAY` variable, so the agents that it runs get none. A CLI can then lose clipboard and browser-open features.
- The installers are not signed (see [Install](#install)).
- The hands-on checks on macOS and Windows are still open. CI runs the automated tests on all three OSes, but nobody has used the installers by hand on macOS or Windows yet. The QA status is in [the QA report](specs/changes/0001-office-server-and-terminal/qa-report.md).
- Windows stops an agent with `taskkill /T /F`. It can miss a child process whose parent already ended.
- The quiet period and the output limit apply after a server restart.
- The app has no update check, because office-harness opens no connection to an outside service.

## Development

You need Rust 1.100.0 (the repository pins it in `rust-toolchain.toml`), Bun 1.4.4, and Node 24. On Linux you also need the Tauri system packages. The list is in the `Install Linux system packages for Tauri` step of `.github/workflows/ci.yml`.

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
