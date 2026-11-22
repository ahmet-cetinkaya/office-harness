# Research: 0002 agent hierarchy

Date: 2026-11-16. Verified against the 0001 sources on `main` (merge `4425166`).

## R1: One directive is one unbroken terminal input

**Question:** can a directive ride the existing terminal input path so it never interleaves with user keys (FR-010, SC-004)?

**Findings (source: repository code):**
- `TerminalSession` owns one `writer: Mutex<Box<dyn Write + Send>>` per terminal (`src/modules/terminal/infrastructure/src/services/terminal_session.rs`, line 119).
- `write_input` locks that mutex and does one `write_all` + `flush` (same file, lines 213 to 222). Two callers never write between each other's bytes; the OS pipe write is a single call.
- The presentation layer exposes `TerminalRuntime::write_input(agent_id, bytes)` (`src/modules/terminal/application/src/common/interfaces/terminal_runtime.rs`, line 44), so the hierarchy application can reach the write without a new port.
- `paste(text)` wraps text in bracketed-paste markers when the child enabled that mode (same file, lines 226 to 233). A directive must NOT use `paste`: the marker is a mode the child turns on and off, and a CLI may not treat pasted text as a submitted prompt. The server appends `\r` itself, matching the Ctrl+C path that already sends one key with meaning.

**Decision:** a directive is exactly one `write_input(text.as_bytes() + b"\r")` call under the existing writer mutex. No new serialization is needed. Verified 2026-11-16, source `terminal_session.rs`.

## R2: An inherited environment variable reaches every CLI without configuration writes

**Question:** can the server hand each child its agent-call token through the environment (PB-3: no CLI configuration writes)?

**Findings (source: repository code):**
- The child environment is assembled in one place: the terminal-host bridge builds `PrepareLaunch { environment_additions, .. }` (`src/apps/office-harnessd/src/bridges/terminal_host.rs`, line 58), and `login_environment.rs` applies additions over the resolved login environment (`without_server_variables`, then additions; `src/modules/agent/infrastructure/src/services/login_environment.rs`, lines 218 to 236). The additions path already carries `OFFICE_KEY`-style variables in tests (`start_agent_handler.tests.rs`, lines 207 and 224), so a server-set variable rides the same channel.
- The server removes its own variables from the child environment (`without_server_variables`, line 218), so the office token variable is already excluded; the agent-call token is added as a normal addition, which proves it is visible to the child and its subprocesses.
- `PreparedLaunch.is_batch_script` (start_terminal.rs) already flags `.cmd`/`.bat` targets on Windows. A helper subcommand of `office-harnessd` is an `.exe`; when a CLI shim resolves it through `npm_shim.rs` the resolved target is the script's interpreter, not `cmd.exe`, so argv quoting follows the normal rules. No new argv channel is needed; the helper binary path is exported as an environment variable, not written into any CLI file.
- All four CLIs inherit the process environment for their children; none of the four adapters writes configuration (verified in 0001 T7: `create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration`).

**Decision:** `OFFICE_HARNESS_AGENT_TOKEN` (the per-agent call token) and `OFFICE_HARNESS_CLI` (absolute path of the running `office-harnessd`) ride the environment additions. The helper is `office-harnessd agent-call <method> <json>`; it reads the token from its own inherited environment. No file outside office-harness's data folder is written. Verified 2026-11-16, sources `bridges/terminal_host.rs`, `login_environment.rs`, `start_terminal.rs`.

**Open point for T4:** grandchildren inherit both variables (normal OS semantics). The advisor security pass decides whether the token must be scrubbed when an agent starts its own child agents through the office (the office controls that spawn, so it can strip `OFFICE_HARNESS_AGENT_TOKEN` from a grandchild's additions).

## R3: The webview tree follows the WAI-ARIA tree pattern

**Question:** what must the hierarchy tree do for keyboard and screen-reader use (PB-5, the accessibility section)?

**Findings (source: WAI-ARIA Authoring Practices, aria `tree` pattern):**
- Container `role="tree"`; each node `role="treeitem"`; nested groups `role="group"` inside the parent item.
- Keyboard: Arrow Up/Down move between visible items, Arrow Right expands or moves to the first child, Arrow Left collapses or moves to the parent, Enter opens (attach/detail), Home/End first/last item. Escape closes any open dialog with focus restored.
- Each `treeitem` needs an accessible name that carries the agent's name, role, and status as text (`aria-label` or visible text), `aria-expanded` on parents, and `aria-level` (or DOM nesting) for depth. Indentation may reinforce but never replaces level information.
- The 0001 webview already runs Playwright with `@axe-core/playwright` and keyboard-only e2e flows (`src/apps/desktop/e2e/office.spec.ts`), so the pattern is testable with the existing harness.

**Decision:** build the tree with `role="tree"`/`treeitem`/`group`, full arrow keyboard model, visible text for role and status, and axe checks in every tree e2e test. Verified 2026-11-16, source WAI-ARIA Authoring Practices Guide, Tree View pattern (`https://www.w3.org/WAI/ARIA/apg/patterns/treeview/`).
