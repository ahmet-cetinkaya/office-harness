# 0001 research

Decisions for change 0001 that needed research. Date: 2026-11-15. Version numbers are the latest that research found on that date. Task T1 confirms each pin before the first dependency is added.

## R1: How does the server keep a terminal that a new client can redraw?

- **Decision:** the server feeds all terminal output into a server-side terminal emulator (the `vt100` crate) and keeps the raw output in a ring of the last 10,000 lines (A2). When a client attaches, the server sends the emulator's full screen state (`Screen::state_formatted()`) and then streams live output. The scrollback above the screen comes from the ring.
- **Rationale:** full-screen terminal user interfaces such as `claude` do not redraw correctly when a client only replays raw bytes from the middle of a session. tmux uses the same approach. Serves FR-016, FR-019, and SC-003.
- **Alternatives considered:**
  - Raw byte replay only: breaks full-screen interfaces after the ring drops old bytes.
  - The xterm.js serialize addon: it is experimental, has alternate-buffer bugs, and runs in Node, not in a Rust server.
  - `alacritty_terminal`: heavier, with no ready-made redraw serializer.
- **Sources:** https://docs.rs/vt100/latest/vt100/struct.Screen.html , https://github.com/xtermjs/xterm.js/issues/6228 , https://github.com/xtermjs/xterm.js/issues/6234
- **Risk:** no source tests this with `claude`. `state_formatted()` does not include input modes, so the snapshot adds `input_mode_formatted()` and builds the scrollback from emulator rows, not raw bytes. Some CLIs send terminal queries at start, so the server answers them itself (contract, Terminal queries). Task T5(h) tests real `claude` and `codex` on all three OSes, and it is a gate before T6.

## R2: PTY library and Windows ConPTY

- **Decision:** `portable-pty` 0.9.0, with these known Windows problems handled in office-harness code:
  1. ConPTY may send a cursor-position query at startup and wait for an answer. The server answers it.
  2. Reading output may never reach the end after the child exits. The server waits on the child in its own thread and then drops the PTY master.
  3. Resizing to the current size repaints the whole screen. The server skips resizes that do not change the size.
  4. Grandchild processes can keep the PTY open. On stop (A3), the server ends the whole process tree: a Job Object on Windows, and the process group and then the remaining descendants on Unix.
  5. npm-installed CLIs are `.cmd` shims on Windows, and `cmd.exe` cannot safely escape `%`, `!`, `"`, or line breaks (the CVE-2024-24576 class of flaw). `portable-pty` builds the command line itself, so the Rust standard library's protection does not apply. For a `.cmd` or `.bat` target, the server therefore:
     - sends the startup prompt as typed input after the first output, the same way it does for generic agents, and wraps it in bracketed paste when the CLI has turned that mode on, so that a line break does not submit early;
     - refuses any flag or fixed command argument that contains `%`, `!`, `"`, CR, or LF, with an `invalid` error that names the flag.
  6. The server binary uses the Windows GUI subsystem, so no console window appears.
- **Rationale:** portable-pty comes from WezTerm and supports all three OSes. Serves FR-006 and FR-015.
- **Alternatives considered:**
  - A Go or Node PTY library: it does not match the decided stack.
  - The `portable-pty-psmux` fork: its claims are unverified.
- **Sources:** https://docs.rs/portable-pty/latest/portable_pty/ , https://github.com/suisya-systems/renga/issues/349 , https://github.com/wezterm/wezterm/issues/4206 , https://github.com/Kc1t/alethe-agents/pull/305 , https://github.com/diazoxide/charter/issues/99
- **Risk:** the last release is from February 2025. If a fix is only on WezTerm main, T1 records a git dependency pinned to one commit.

## R3: How does the server learn a CLI's session id without writing the CLI's files?

- **Decision:** each supported CLI has an adapter with three operations: start a new session, continue a session, and find the session id.
  - **`claude`:** office-harness generates a UUID and starts `claude --session-id <uuid>`. To continue, it runs `claude -r <uuid>`.
  - **`codex`:** it continues with `codex resume <id>`. It finds the id by listing the CLI's session files before and after start, read-only.
  - **`gemini`:** it continues with `gemini --resume <uuid>`. It finds the id with `gemini --list-sessions` (read-only).
  - **`opencode`:** it continues with `opencode --session <id>`. It finds the id with the CLI's own session list command, read-only.
  - **Other commands:** they run without session continuation (FR-025).
  - **Attribution:** an id is matched by working folder, start time, and process id, with a bounded retry, because some CLIs write their session only after the first prompt. When the match is ambiguous (for example, two agents of one CLI in one folder start at once), the session gets no id and `can_continue=false` with a reason. The adapter never guesses.
  - **Startup prompt:** it is passed only when a new session starts, not on continue. A generic agent gets it as typed input after the first output.
- **Rationale:** FR-021 forbids create, change, and delete in CLI configuration folders, but not read. Only `claude` lets the caller choose the id up front. Serves FR-024, FR-025, and SC-002.
- **Alternatives considered:**
  - Pointing each CLI at a private config folder: this breaks FR-020 (the user's own configuration).
  - Keeping only "continue latest" (`-c` and `--last`): this is wrong when one folder holds two agents.
- **Sources:** https://code.claude.com/docs/en/cli-reference , https://learn.chatgpt.com/docs/developer-commands?surface=cli , https://geminicli.com/docs/cli/session-management/ , https://github.com/anomalyco/opencode/blob/dev/packages/web/src/content/docs/cli.mdx
- **Risk:** the `codex` and `opencode` flags come from secondary sources. T1 checks each CLI's `--help` on the installed version and records the result here. The Gemini CLI documentation says that Antigravity CLI replaced Gemini CLI on 2026-06-18 for some account tiers. The owner may need to decide whether `gemini` stays in SC-002 (see PB-1 in the plan).

## R4: How does the server start at login without the desktop app?

- **Decision:** on `register`, the server copies itself to a stable per-user folder, because an AppImage path is a temporary mount and a moved app bundle breaks a registered path. It then registers that copy as a login item for the current user with the `auto-launch` crate:
  - Linux: a systemd user unit.
  - macOS: a LaunchAgent (`MacOSLaunchMode::LaunchAgent`, which avoids the AppleScript permission prompt).
  - Windows: an HKCU `Run` entry.
  The desktop app bundles the server as a Tauri sidecar and asks it to register itself on first start.
  - **"Start server" from the app:** it starts the registered login item through the OS (`systemctl --user start`, `launchctl kickstart`), or else spawns the server detached (`setsid` on Unix; `DETACHED_PROCESS`, `CREATE_BREAKAWAY_FROM_JOB`, and `CREATE_NO_WINDOW` on Windows). The server must outlive the app, and Tauri ends the children it started when it exits.
  - **Unverified:** the Linux mode of `auto-launch` (systemd user unit or XDG autostart) and its behavior without systemd. T1 checks both and records them here.
- **Rationale:** `tauri-plugin-autostart` can only start the app itself, not a sidecar. A Windows service runs outside the user's login session, and A10 needs the server only while the user is logged in. Serves FR-002 and SC-004.
- **Alternatives considered:**
  - `tauri-plugin-autostart` with a background flag: this starts a webview process at every login.
  - `service-manager`: it has a Windows user-level limitation and installs system services.
- **Sources:** https://docs.rs/auto-launch , https://raw.githubusercontent.com/tauri-apps/plugins-workspace/v2/plugins/autostart/src/lib.rs , https://v2.tauri.app/develop/sidecar/

## R5: Storage, network, and owner-only files

- **Decision:**
  - SQLite through `rusqlite` with the `bundled` feature.
  - An HTTP and WebSocket server through `axum` (feature `ws`) on `tokio`.
  - Folders through the `directories` crate.
  - Owner-only access: files are created with mode `0600` and folders with `0700` on Unix. On Windows, the local application data folder inherits an access list that includes Administrators, so the server sets a protected access list on the data folder (the current user and SYSTEM only, with inheritance broken) through the `windows` crate. T4 tests this by reading the folder's access list.
- **Rationale:** bundling SQLite gives one SQLite version on all three OSes. axum provides HTTP routes, token checks, and WebSocket support in one library. Serves FR-003 and FR-009.
- **Alternatives considered:**
  - `sqlx`: its async queries and compile-time checks add little value for a single-user server.
  - `tokio-tungstenite` alone: it has no HTTP routing.
  - `windows-acl`: its maintenance is unverified. The `windows` crate is maintained by Microsoft.
- **Sources:** https://crates.io/api/v1/crates/rusqlite , https://docs.rs/crate/directories/latest , https://lib.rs/crates/windows-acl

## R7: The environment an agent CLI starts with (FR-020)

- **Decision:**
  - **Unix:** at each spawn, the server resolves the user's login environment by running `$SHELL -l -i -c` with a marker and a timeout, the way VS Code does.
  - **Windows:** it builds the environment from the user and machine environment in the registry.
  - **Both:** it removes the server's own variables (for example the AppImage `APPDIR` and `LD_LIBRARY_PATH`), applies the office additions, and looks up the command on the resolved `PATH`.
- **Rationale:** a server started by launchd or systemd gets a minimal `PATH`, so `claude` installed through Homebrew, npm, or nvm would not be found.
- **Fallback:** some interactive shell setups hang without a terminal. If the shell does not answer within 5 seconds, the server uses the shell from the user's account record (not `$SHELL`, which can be empty) without `-i`. If that also fails, it uses its own environment, and it records a warning in the agent's details.
- **Known limit:** a server that systemd starts has no `DISPLAY` or `WAYLAND_DISPLAY`, so agent CLIs may lose clipboard and browser-open features. This is recorded in the README.
- **Alternatives considered:** inheriting the server's own environment fails at login start.

## R6: Desktop app frontend

- **Decision:** Svelte with TypeScript inside Tauri v2, and `@xterm/xterm` 6 with `addon-fit` and `addon-webgl` (falling back to the default renderer).
- **Rationale:** owner decision on 2026-11-15. xterm.js is the terminal that VS Code uses.
- **Sources:** https://github.com/xtermjs/xterm.js , https://www.npmjs.com/package/@xterm/xterm

## T1 verification (2026-11-15)

Checked by running only `--version` and `--help` for the four CLIs (no session was started, no CLI folder was read), and by reading the crates.io and npm registries.

### R2: portable-pty

- **Verified on 2026-11-15, version 0.9.0:** the crates.io release is still 0.9.0 (2025-02-11).
- WezTerm `main` has one later Windows fix (`kill()`, commit `8afe0ad3`), plus a dependency migration and a typo fix. No commit title mentions the startup query stall or the missing end of output after exit.
- **Decision:** use the 0.9.0 release, with no git pin. office-harness ends processes through its own Job Object (item 4), so the `kill()` fix is not needed. The stall and the missing end of output are handled in office-harness code (items 1 and 2) and tested on the Windows CI runner.

### R3: CLI session flags

| CLI (version) | New session with a chosen id | Continue a session | Initial prompt, stays interactive | Id found by |
|---|---|---|---|---|
| `claude` 2.1.322 | `--session-id <uuid>` | `--resume <id>` | positional `"<prompt>"` | office-harness chooses it |
| `codex` 0.153.4 | none | `codex resume <SESSION_ID>` | positional `"<prompt>"` | attribution (R3) |
| `gemini` 0.42.0 | `--session-id <uuid>` | `--resume <index>`, where the index comes from `--list-sessions` | `-i "<prompt>"` | office-harness chooses it |
| `opencode` 2.2.5 | `--session <id>` ("create if it does not exist") | `--session <id>` | `--prompt "<prompt>"` | office-harness chooses it |

**Effects on the design:**
- Three of the four CLIs accept an id that office-harness chooses at start. Only `codex` needs attribution, which removes most of the ambiguity risk in R3.
- **Unverified, checked by hand in T7(g):**
  - whether `gemini --resume` accepts a UUID (otherwise the adapter maps the UUID to an index through `--list-sessions`, read-only);
  - the id format that `opencode` accepts;
  - whether `claude` and `opencode` stay interactive with an initial prompt.
- **Sandbox note:** `opencode --version` fails when its log folder is not writable, but works with temporary XDG folders. This does not affect office-harness, because office-harness runs the CLI with the user's own environment.
- **Evidence:** the help lines are quoted in the run's evidence file (local, not committed).

### R4: auto-launch

- **Verified on 2026-11-15, version 0.6.0.**
- **Linux:** the default mode is XDG autostart (a `.desktop` file in `~/.config/autostart`). Systemd mode (`LinuxLaunchMode::Systemd`) writes `~/.config/systemd/user/<name>.service` and runs `systemctl --user enable`, which does not start the server. Neither mode falls back to the other.
- **macOS:** `MacOSLaunchMode::LaunchAgent` (the default) writes `~/Library/LaunchAgents/<name>.plist`.
- **Decision:**
  - When `systemctl --user` answers, `register` uses systemd mode and then runs `systemctl --user start`.
  - Otherwise it uses XDG autostart and records in `server.log` which mode it chose.
  - XDG autostart depends on the desktop session, so the README states this limit for bare window managers.

### R8: type generation

- **Decision:** `ts-rs` 12.0.1 (stable). It exports plain serde types for the request, response, event, and channel bodies, and the desktop app keeps small `invoke` and channel wrappers in `core/api`.
- **Rejected:** `specta` and `tauri-specta`, because their Tauri v2 versions are only release candidates (2.0.0-rc.28 and rc.29), and their docs do not mention channels.

### Version pins for T2 (latest stable on 2026-11-15)

- **Rust:** `portable-pty` 0.9.0, `vt100` 0.18.1, `axum` 0.8.9, `tokio` 1.54.1, `rusqlite` 0.43.0, `directories` 6.0.0, `auto-launch` 0.6.0, `hmac` 0.13.0, `sha2` 0.11.1, `subtle` 2.6.1, `tauri` 2.12.5, `tauri-build` 2.7.4, `tauri-plugin-shell` 2.6.0, `ts-rs` 12.0.1.
- **npm:** `@xterm/xterm` 6.0.0, `@xterm/addon-fit` 0.11.0, `@xterm/addon-webgl` 0.19.0, `@tauri-apps/api` and `@tauri-apps/cli` 2.12.4, `svelte` 5.57.3, `vite` 8.3.8, `@sveltejs/vite-plugin-svelte` 7.3.1, `tailwindcss` 4.3.5, `shadcn-svelte` 1.8.1, `@playwright/test` 1.65.1, `@axe-core/playwright` 4.14.0.
- **Not pinned:** `tauri` 3 (alpha) is not used.
- **Watch items:**
  - `rusqlite` 0.43.0 was published today. If it breaks the T2 build, T2 falls back to the previous minor version and records why.
  - The xterm addons declare no peer dependency. T2 checks that they load with xterm 6.
