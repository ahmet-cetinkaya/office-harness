---
status: approved
spec: specs/changes/0009-agent-templates/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #65, main 10df6cd). 0010 modifies 0009/FR-001 and FR-003 (own worktree choice); the 0010 plan owns that MODIFIED text. r2 (2026-11-19) applies Efe's plan review r1 (office-harness-plan-0009.md: 3 HIGH, 5 MEDIUM, 4 LOW) and god's decisions: every imported env value is dropped (spec A6), one shared create undo mode with 0010, and the D6 license check and workspace pin. Needs approval through Efe's plan review. -->

# Plan: Agent and team templates (0009)

## Summary

The owner saves an agent's definition as a template and creates agents from it. He saves a lead and its members as a team template and creates whole teams from it in one action. He moves templates between offices through a plain-text file that never holds environment values. Creation is all-or-nothing, templates are copies, and only the user uses them.

**What you get:**
- A new `template` module (four crates, table `template_templates`, its own migration ledger). Templates are kept as versioned JSON bodies.
- One host bridge that creates an agent or a whole team from a template. It validates names, folders, and tree places before it creates anything, and it undoes everything on a late failure.
- Requests `template.*` and an event `template.changed`, for user clients only.
- The agent module learns "no value" for an environment variable: it is shown as such and never passed to the CLI (FR-011).
- In the desktop app:
  - a templates view (list, editor, delete);
  - "Save as template" in an agent's details;
  - "Create from template" beside the 0002 create buttons;
  - export and import with a full plain-text preview.
  The two file actions are new app commands that open the system file dialog themselves.

**Why this approach:**
- Templates need a new boundary. They are not agent settings (a closed key set) and not tree data, so `docs/ARCHITECTURE.md:75` says a new module.
- The create path must use the 0002 rules and the agent create as they are. The `RoleRequestsBridge` and `AgentCreatorBridge` already compose them in the host under the tree exclusion, so one more host bridge does the same for a batch.
- The webview never gets a raw file read or write permission. Each file command opens its own dialog and touches only the file the user picked (PB-4).

**What it will not do:** a marketplace, templates for meetings, agents using templates, updating agents from a changed template, a secret mark on variables (spec Out of scope).

**Shape of the work:** 8 tasks in 2 phase PRs into one umbrella branch `feat/0009-agent-templates`, one local commit per task. **Effort:** L.

**Main risks:**
- **All-or-nothing over many agents.** The shipped code has no multi-agent rollback. D5 validates everything first and undoes created agents and the team row in reverse order. A fault-injection test fails the batch at each agent position, on the real SQLite hierarchy repository.
- **A deadlock on the tree exclusion.** The exclusion is a plain mutex, and the shipped team writes take it again. D5 adds store variants that run under the held exclusion, as `place_agent` already does, and T4 tests the batch on the real repository, where a second take would hang.
- **A new dependency.** `tauri-plugin-dialog` `=2.8.4`, the official Tauri plugin, is used only from Rust and pinned in the workspace. No dialog permission is given to the webview. God approved it on 2026-11-19 (decision D6). Its license and the license of every crate it adds are checked in T6.
- **The shipped `team.create` bug.** The desktop sends a `lead` object that the server did not read. PR #81 fixes it. This plan does not depend on that fix: the template bridge builds its own team create.

**Decisions made for you:** the module, the storage, the body shape, "no value" and its editor syntax, the name pattern rules, the create bridge and its undo, the file commands, the protocol, and the desktop placement.

**Blockers:** none. T7 needs the 0011 `visibleText` helper on main (a gate, see Dependencies). The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0009:
- save, edit, delete, export, and import agent and team templates;
- create one agent or one team (a lead and its members) from a template in one action, all-or-nothing, within the 0002 rules;
- export no environment values, and import nothing before the user confirms a full preview.

## Context

- **Repository:** `main` at 037b10f.
- **Existing seams** (read at 037b10f on 2026-11-19):
  - Agent create: `CreateAgentHandler::handle` (`M/agent/application/src/commands/create_agent/create_agent_handler.rs:67-94`). It journals `AgentAction::Create` (`:92`). Folder existence is checked only at session start (`common/session_launcher.rs:385`).
  - Role create composite: `CreateRoleAgentHandler::handle` (`M/hierarchy/application/src/commands/create_role_agent/create_role_agent_handler.rs:48`) runs `check_placement`, then `creator.create`, then `store.place_agent`, and calls `creator.remove` on a failed place.
  - Tree rules: `M/hierarchy/domain/src/services/tree_rules.rs:40` (`check_placement`). The refusal text is `hierarchy_error.rs:35`.
  - Team create: `CreateTeamHandler` (`create_team_handler.rs:38`) with `team_name_taken` and `save_team_and_place_lead`. Undo: `delete_team_and_unplace` (`hierarchy_store.rs:81`).
  - Tree exclusion: `TreeExclusion` holds a plain `std::sync::Mutex` guard (`hierarchy_store.rs:15-30`), so a second take on the same thread waits forever. `place_agent` runs under the caller's held exclusion (`hierarchy_store.rs:83-98`). `save_team_and_place_lead` and `delete_team_and_unplace` take it again (`M/hierarchy/infrastructure/.../hierarchy_repository.rs:113,151`).
  - Host bridges: `H/bridges/role_requests.rs` (`RoleRequestsBridge` takes `store.tree_exclusion()` at `:54`; `AgentCreatorBridge` at `:156-197`, whose `remove` calls `DeleteAgentHandler::handle`). Wired at `H/bootstrap/hierarchy_services.rs:103-112`.
  - Agent create checks (`create_agent_handler.rs:67-75`): `DisplayName`, `AgentCommand`, `split_command`, the env names (including the reserved `OFFICE_HARNESS_` prefix), and that the project exists. The role create keeps a cleanup failure in its message (`create_role_agent_handler.rs:101-118`).
  - Additions: `OfficeAdditions` (`M/agent/domain/src/value_objects/office_additions.rs:12`), `EnvEntry` (`env_entry.rs:8-42`, a `String` value), stored as JSON in `agent_office_additions` (`0003_additions_and_sessions.sql:5`). Every path that carries an env value as text:
    - the stored JSON, in infrastructure: `M/agent/infrastructure/src/persistence/additions_document.rs:13`, a `(String, String)` pair;
    - the request reader: `M/agent/presentation/src/contracts/additions_body.rs:46-58`, which refuses `null`;
    - the response mapping, `M/agent/presentation/src/mappings/`;
    - the hierarchy port seed: `SeedAdditions.env: Vec<(String, String)>` (`M/hierarchy/application/src/common/ports/agent_creator_ports.rs:16`), mapped in `AgentCreatorBridge::create`;
    - the desktop model and editor: `D/modules/agent/services/additions-text.ts` (`parseEnvironment` refuses a bare `NAME` line at `:31`; `formatEnvironment` writes `${entry.value}` at `:44`).
  - Events: the create paths send `agent.changed` (`agent_handlers.rs:243`) and `hierarchy.changed`.
  - Migrations: `DatabaseTooNew` is per module ledger (`migrations.rs:18-36`), so an older program does not refuse a database that has an unknown `template` ledger.
  - Name uniqueness: a NOCASE unique index on `agent_agents.name_key`. A conflict maps to "agent name X is already used" (`application_error.rs:63`). Names are at most 64 characters (`display_name.rs:7`).
  - Gateways: `HoldGateway` (`H/server/hold_requests.rs:76`) is the pattern. Routing is `H/server/connection.rs:216-249`, the state is `server_state.rs:27-63`, and construction is `running_server.rs:83-133`. Migrations are registered in `H/bootstrap/services.rs:81-86`. The agent allowlist is `H/server/agent_call.rs:25-32`.
  - Desktop:
    - Create forms: `D/modules/agent/components/agent-form-dialog.svelte`, `D/modules/hierarchy/components/{hierarchy-agent-dialog,team-form-dialog}.svelte`.
    - Create buttons: `D/modules/hierarchy/views/hierarchy-view.svelte:494-509`.
    - Details actions: `D/modules/agent/components/agent-details.svelte:133-158`.
    - Desktop boundaries: `D/boundaries.spec.ts`.
    - Tauri commands: `src-tauri/src/commands.rs`, `build.rs:7-16` (`WEBVIEW_COMMANDS`), `capabilities/default.json`. No dialog or fs plugin exists today.
    - Tauri pins: the root `Cargo.toml:47-50` `[workspace.dependencies]` holds `tauri = "=2.12.5"` and `tauri-plugin-notification = "=2.8.1"`.
  - `tauri-plugin-dialog` (crates.io, read 2026-11-19): `2.8.4` is the newest 2.x, published 2026-10-28, license `Apache-2.0 OR MIT`. It requires `tauri ^2.12`, so it fits `=2.12.5`. Its normal dependencies include `rfd ^0.17` and `tauri-plugin-fs ^2.7.1`. The 3.0 line is alpha and is not used.
  - The 0011 plan adds a `visibleText` helper for control characters (`D/modules/agent/models/task-text.ts`). God's order builds 0011 before 0009, so the import preview reuses it.
- **Floor rules:** one build or test at a time (ask god; `CARGO_BUILD_JOBS=4`, `--test-threads=4`, the pinned toolchain, per-crate local runs). Efe reviews at the exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Module and storage:** a new module `template`: `M/template/{domain,application,infrastructure,presentation}`.
  - Table `template_templates(id TEXT PRIMARY KEY, name TEXT NOT NULL, name_key TEXT NOT NULL UNIQUE, kind TEXT NOT NULL, body TEXT NOT NULL, updated_at INTEGER NOT NULL)`, where `name_key` is the lower-cased name and `kind` is `agent` or `team`.
  - Ledger: `template_schema_version`, script `0001_templates.sql`.
  - Rejected alternatives: agent settings (a closed key set), and the hierarchy module (templates are not tree data).
- **D2 Body shape** (FR-001, FR-003, A4, A5): JSON with `"body_version": 1`.
  - An agent body is `{name_pattern, command, working_folder?, additions: {flags, env: [{name, value: string|null}], startup_prompt?}, role?: "ceo"|"lead"|"member"}`.
  - A team body is `{lead: <agent body>, members: [{entry: <agent body>, count}]}`. Entries are copies, so editing or deleting an agent template changes no team template (A5).
  - Rules on save: the name is unique case-insensitively; the command is not empty; the pattern is not empty and at most 64 characters after expansion; each member count is at least 1; a team has at least one member entry; and 1 + the sum of the counts is at most 50 (FR-010).
  - Unknown fields: a body or a file entry with a field this build does not know is refused, and the answer names the field. A build therefore never drops a field on save or import. 0010 adds `own_worktree: bool` (default false) by its MODIFIED text, which the 0010 plan owns; a 0009 build refuses a body that carries it.
- **D3 "No value"** (FR-011, A6, Efe's plan note):
  - `EnvEntry.value` becomes `Option<String>`. `None` is "no value"; `Some("")` stays a value the user typed on purpose.
  - Every text path in Context carries the option:
    - the stored JSON (`additions_document.rs`) writes and reads `null` for `None`. Older rows always have a string, so no migration is needed;
    - the request reader (`additions_body.rs`) accepts `null`, and the response mapping writes it;
    - the hierarchy port seed becomes `SeedAdditions.env: Vec<(String, Option<String>)>`, and `AgentCreatorBridge::create` passes it through unchanged;
    - the desktop model types the value as `string | null`.
  - Editor syntax (`additions-text.ts`): a bare `NAME` line is "no value", and `NAME=` is an empty value. `formatEnvironment` writes `NAME` for `null`, so an edit round trip never turns "no value" into the text `null`.
  - The launcher skips `None` entries when it builds the CLI environment, so the login value stays in effect.
  - The agent details and the template editor show "no value".
  - Export writes `null` for every value.
  - Import drops every value: preview and confirm set each imported env value to `null`, whatever the file holds, and the preview says "values in the file are not imported". A crafted value such as `LD_PRELOAD=/tmp/x.so` is never saved (spec A6).
- **D4 Name patterns** (FR-002, A2, edge cases):
  - `{n}` becomes the lowest positive integer that makes the name unique (case-insensitive).
  - `{team}` becomes the team name of the creation. For a direct agent creation it becomes the team of the place picked; with no team there, the creation is refused and names the pattern.
  - A pattern without `{n}` gets `-{n}` added only when the name is taken.
  - Names are chosen inside the tree exclusion, so two concurrent creations get different names.
  - Saving a template from an agent uses the A2 defaults.
  - Role on a direct agent creation: the role comes from the template, and `place` gives only the parent and the team. A template with no role creates an agent with no place, and `place` must then be absent (`invalid` otherwise).
- **D5 Create bridge** (FR-002, FR-004, FR-005, PB-1, PB-2, SC-005): `TemplateCreateBridge` in `H/bridges/template_create.rs`, under `tree_exclusion()` for the whole batch.
  - **Held-exclusion store calls:** the hierarchy store gains `save_team_and_place_lead_held(&TreeExclusion, ..)` and `delete_team_and_unplace_held(&TreeExclusion, ..)`. They take the caller's guard as proof and do not lock again, as `place_agent` does. The existing methods lock and call the held variants, so their callers do not change. The bridge calls only the held variants and `place_agent`.
  - **Plan:** resolve each agent's name (D4), working folder (template, else the project's folder, else refused "a working folder is needed"), and role. The position sets the role in a team (A4). Then pre-check every planned agent and the team, and collect every failure ("name each reason", FR-005):
    - the agent create rules, through the agent module's own pure checks: `DisplayName` (which covers the 64-character rule), `AgentCommand`, `split_command`, and the env names, including the reserved `OFFICE_HARNESS_` prefix;
    - the project exists, when one is given;
    - each folder exists as a directory, else it is named;
    - `TeamName::parse` and a free team name;
    - every place passes `check_placement` on a copy of the tree snapshot with the planned agents added in order.
    - The agent rules are reached through a new `check` method on the `AgentCreator` port, which runs them without writing; `AgentCreatorBridge` implements it with the agent module's value objects.
  - **Apply:** create the lead, then the team row and the lead's place, then each member, through `AgentCreator::create`, the held team write, and `place_agent`.
  - **Undo:** on a failure during apply, remove the created agents in reverse order with `AgentCreator::remove`, then delete the team row with `delete_team_and_unplace_held`. Then return the reason.
    - If one `remove` fails, the undo goes on with the rest and still deletes the team row. The answer gives the first failure's reason and names each agent left behind, as the role create does.
    - Each rolled-back agent leaves a Create entry and a Delete entry in the action record (PB-2). The record is not rewritten.
  - **Create undo mode (shared with 0010):** `AgentCreator::remove` is the undo of a create and is used only for that (role create and this batch). It discards without asking: it deletes the agent and everything the create made for it. From 0010 on, that includes the agent's new worktree and branch; `remove` never takes the user delete's `keep` default.
  - **Events:** on success, one `agent.changed` per created agent and one `hierarchy.changed`, as the existing create paths send. On a rollback, no `agent.changed` and no `hierarchy.changed`; the `action.recorded` events of the Create and Delete entries are still sent, because those entries exist. When every write succeeded but an agent cannot be read back, the creation still answers success and sends its events (that agent has no `agent.changed`), and `storage.error` is raised.
  - **Partial state:** `snapshot` and `agent.list` do not take the tree exclusion, so another client can see an agent for a moment that is then rolled back. This is accepted: it is visible only during a failing batch, and the events above never announce it.
  - **Record:** each created agent is recorded by the existing create journal with the user as actor (PB-2).
  - Template changes are not recorded (A10).
- **D6 File commands** (FR-007, FR-008, PB-4), **approved by god on 2026-11-19** with conditions (a) to (e):
  - Add `tauri-plugin-dialog`, the official plugin, used from Rust only.
  - Two new app commands. Neither takes a path argument, so each touches only the path the user picked in its own dialog (condition b):
    - `export_templates_file(contents: String)` opens a save dialog and writes `contents` to the picked path. It answers `saved`, `cancelled`, or a failure ("the file was not written: ...").
    - `import_templates_file()` opens an open dialog and reads the picked file:
      - it refuses anything that is not a regular file (a FIFO, a device, a folder);
      - it reads at most 256 KiB + 1 bytes through `Read::take` and refuses the file when it got more than 256 KiB. It never trusts the size from metadata (condition c);
      - it returns the text, or a failure that names the reason.
  - The server also refuses a `text` field over 256 KiB on `template.import.preview` and `template.import.confirm`, because the webview supplies it (condition c).
  - The webview gets only `allow-export-templates-file` and `allow-import-templates-file`, with no dialog or fs permission. A Rust test reads `capabilities/default.json` and asserts that its permissions are exactly the `allow-*` entries of `WEBVIEW_COMMANDS` (`build.rs:7-16`) plus `notification:default`, with no `dialog:` or `fs:` entry (condition a).
  - Pin (condition d): `tauri-plugin-dialog = "=2.8.4"` in the root `Cargo.toml` `[workspace.dependencies]`, beside `tauri = "=2.12.5"`. `src-tauri/Cargo.toml` takes it with `workspace = true`. It fits `tauri ^2.12`.
  - License (condition e): T6 lists every crate that the dependency adds to `Cargo.lock` with its license, from `cargo metadata`, and checks each for compatibility with AGPL-3.0-or-later. The plugin itself is `Apache-2.0 OR MIT`. An incompatible crate stops T6 and goes to god. The list goes into the QA report.
  - Rejected: the plugin's JavaScript API plus a write-by-path command, which would let the webview write any path.
- **D7 Protocol** (user clients only; not in the agent allowlist; the allowlist comment is updated):
  - `template.list` and `template.get {id}`.
  - `template.save {id?, name, kind, body}`. Without an id it creates; with one it replaces.
  - `template.delete {id}`.
  - `template.from_agent {agent_id}` returns an unsaved agent body with the A2 defaults, which the editor then saves.
  - `template.create_agent {template_id, place?, project_id?}`. `place` gives the parent and the team for the template's role (D4); it is absent for a template with no role.
  - `template.create_team {template_id, team_name, parent_agent_id, project_id?}`.
  - `template.export {ids?}` returns `{text}`: the file text `{"format":"office-harness-templates","version":1,"templates":[...]}` with every env value `null`.
  - `template.import.preview {text}` returns every template with its save name (A3) or the FR-009 refusal, with every env value `null` (D3). It saves nothing.
  - `template.import.confirm {text}` validates again, sets every env value to `null`, and saves all templates in one transaction.
  - Both import requests refuse a `text` over 256 KiB with `invalid` (D6).
  - Events: `template.changed {templates}`; a creation from a template also sends `agent.changed` and `hierarchy.changed` (D5).
  - Codes: `invalid` (rule failures, with each reason), `conflict` (name taken on save), `not_found`, `refused` (tree rules, the 0002 text).
- **D8 Desktop:**
  - A `D/modules/template/` module: state, service, a `templates-view.svelte` reached from the panel header, an agent and team editor dialog, a create dialog (place or parent, team name, project), and an import preview dialog.
  - The preview lists every template's commands, folders, flags, env names (with "no value"), startup prompts, and save name, with control characters shown visibly through `visibleText`. It says that values in the file are not imported.
  - The export notice says before writing that flags and startup prompts are written as they are.
  - "Save as template" sits in agent details. "Create from template" sits beside the hierarchy create buttons.
  - An empty view says so and offers save and import.
  - Everything works by keyboard. No new visuals beyond these dialogs.
- **Constitution check:**
  - The new module depends on no other module. Cross-module work is in `H/bridges/`. `scripts/check-boundaries.sh` passes and gains the new crates.
  - No CLI configuration is touched (PB-3).
  - No data leaves the machine; export writes only the picked file (PB-4).
  - Keyboard access is kept (PB-5).
  - No other project's name is in the repository.

## Scope

**In:** FR-001 to FR-011, PB-1 to PB-5, A1 to A10, SC-001 to SC-005, the spec's edge cases, the contract and README updates, a QA report.

**Out:** everything in the spec's Out of scope; the own worktree choice (0010); the shipped `team.create` lead-object bug (its own fix).

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Server | No value, template module, create bridge, protocol | T1 to T5 | `feat/0009-p1-server` (umbrella) | p1 → umbrella |
| 2 Desktop and ship | File commands, UI, docs, QA | T6 to T8 | `feat/0009-p2-desktop` (umbrella after p1) | p2 → umbrella, then umbrella → `main` |

## Tasks

Paths: `M/` = `src/modules/`, `D/` = `src/apps/desktop/src/`, `H/` = `src/apps/office-harnessd/src/`.

### T1: Design artifacts (no product code)

- **Files:** `specs/changes/0009-agent-templates/{data-model.md,contracts/template-protocol.md}`.
- **Work:** the D1 table, the D2 body and file format with examples, the D3 `null` rule, and the D7 requests, codes, and event.
- **Depends on:** none.
- **Acceptance:** every FR names its message, field, or rule; style as `0007-hold-and-limits/contracts/hold-protocol.md`.

### T2: "No value" in the agent module (FR-011)

- **Files:**
  - `M/agent/domain/src/value_objects/{env_entry.rs,office_additions.rs}` (+ tests);
  - `M/agent/application/src/common/{additions_input.rs,session_launcher.rs}`;
  - `M/agent/infrastructure/src/persistence/additions_document.rs` (+ tests);
  - `M/agent/presentation/src/contracts/additions_body.rs` (+ tests) and `M/agent/presentation/src/mappings/`;
  - `M/hierarchy/application/src/common/ports/agent_creator_ports.rs` (`SeedAdditions.env`) and the hierarchy test fakes that build it;
  - `H/bridges/role_requests.rs` (`AgentCreatorBridge::create` mapping);
  - `D/modules/agent/` (`models/`, `services/additions-text.ts` + spec, and the additions display "no value").
- **Work:** D3, except the import rule (T3).
- **Acceptance:**
  - Unit tests: `None` and `Some("")` round-trip as different values through the domain and the stored JSON; an old additions row with strings loads unchanged.
  - The request reader accepts `null` and the response writes it.
  - `additions-text` tests: a bare `NAME` line parses to `null`, `NAME=` to `""`, and format then parse keeps `null` (never the text `null`).
  - Launcher test: a `None` entry is not in the child environment, and the login value of that name stays. A `Some("")` entry is set to empty, as today.
  - fmt, clippy, per-crate tests for the agent and hierarchy crates and `office-harnessd`; `bun test`.

### T3: Template module (FR-001, FR-003, FR-006, FR-009 rules, FR-010)

- **Files:** `M/template/{domain,application,infrastructure,presentation}/` (new crates), `Cargo.toml` (workspace members), `scripts/check-boundaries.sh` (planted cases if needed).
- **Work:**
  - Domain: the D2 body types and save rules, the D4 pattern expansion as pure functions, and the file format parse with the FR-009 checks: version, shape, 50 agents, the FR-001 and FR-003 rules naming the template, and A3 save names.
  - Application: `TemplateService` (save, delete, list, from-agent body through a port, export, preview, confirm).
  - Infrastructure: the repository and migration.
- **Depends on:** T1, T2.
- **Acceptance:**
  - Unit tests for every save rule, including 50 agents accepted and 51 refused.
  - `{n}` picks the lowest free number. A pattern without `{n}` gets `-{n}` only when the name is taken. `{team}` is replaced.
  - Import of an unknown version, unreadable text, 51 agents, an empty command, and a member count of 0 each name the reason and the template, and save nothing (SC-003).
  - A name clash on import gets ` (2)`, then ` (3)`.
  - A file whose env entry has a value imports with "no value", in preview and after confirm (D3).
  - A body or file entry with an unknown field is refused and names the field (D2).
  - Template saves and deletes add no action-record entry (A10).
  - Editing or deleting a template leaves a copy in a team template intact (A5).
  - Repository: templates survive closing and reopening the database (A8).
  - fmt, clippy, per-crate tests, boundaries.

### T4: Create bridge (FR-002, FR-004, FR-005, PB-1, PB-2, SC-005)

- **Files:**
  - `H/bridges/template_create.rs` (+ tests);
  - `H/bootstrap/{services.rs,hierarchy_services.rs}`;
  - `M/hierarchy/application/` (the held-exclusion store variants, the `AgentCreator::check` port method, and the test fakes);
  - `M/hierarchy/infrastructure/` (the held variants in `hierarchy_repository.rs` + tests);
  - `M/hierarchy/domain/` if a read of the snapshot with planned rows needs a pure helper;
  - `H/bridges/role_requests.rs` (`AgentCreatorBridge::check`).
- **Work:** D5.
- **Depends on:** T3.
- **Acceptance:**
  - The bridge tests run against the real `HierarchyRepository` on SQLite, not a mock store, so a second take of the exclusion would hang the test.
  - Fault injection: a team creation that fails at the lead, at member 1, and at the last member leaves 0 new agents and 0 new teams (SC-005).
  - A `remove` that fails during the undo: the other agents are still removed, the team row is deleted, and the answer names the first reason and the agent left behind.
  - Each rolled-back agent has a Create and a Delete entry.
  - Pre-check cases, each collected with the others in one answer: a bad name, an empty command, a command that cannot be split, an env name with the reserved prefix, an unknown project, a bad team name, a taken team name.
  - A missing working folder refuses before any create and names the folder.
  - No folder and no project answers "a working folder is needed".
  - A `{team}` pattern on a direct creation with no team is refused and names the pattern.
  - A template with a value-less env entry creates an agent whose stored additions keep "no value", through the port, and the launcher does not pass it (FR-011).
  - A CEO template while a CEO exists is refused with the 0002 text (PB-1).
  - A team-template lead entry with role CEO is created as a lead (A4).
  - Two concurrent creations from one template get different names.
  - Each created agent has a `create` record with the user as actor (PB-2).
  - fmt, clippy, tests, boundaries.

### T5: Protocol and integration (FR-001 to FR-010, SC-002 to SC-005)

- **Files:** `M/template/presentation/src/handlers/`, `H/server/{template_requests.rs,server_state.rs,running_server.rs,connection.rs,agent_call.rs}` (comment), `src/apps/office-harnessd/tests/templates.rs`.
- **Work:** the D7 gateway and event.
- **Depends on:** T4.
- **Acceptance:** integration tests through a real client:
  - Save an agent template from an agent. Create `reviewer-1` and `reviewer-2`.
  - Create two teams from one team template under the CEO, 1 + 6 each, with the names from the patterns.
  - A taken team name creates nothing.
  - Export, then import into a fresh office through preview and confirm, reproduces every value except env values, and the text holds no env value (SC-002).
  - Preview saves nothing. A cancel (no confirm) saves nothing.
  - Editing and deleting a template change no agent or team (SC-004).
  - A second client receives one `agent.changed` per created agent and one `hierarchy.changed`, and no `agent.changed` or `hierarchy.changed` for a failed creation (its `action.recorded` events are sent).
  - An import `text` over 256 KiB is refused with `invalid`.
  - An import of a file whose env entry has a value saves "no value".
  - An agent connection asking for `template.list` is refused as not allowed.
  - fmt, clippy, per-crate tests, boundaries.

### T6: File commands (FR-007, FR-008, FR-009 size, PB-4)

- **Files:** the root `Cargo.toml` (`[workspace.dependencies]` pin), `Cargo.lock`, `src/apps/desktop/src-tauri/{Cargo.toml,src/commands.rs,src/main.rs,build.rs,capabilities/default.json}` and the permission files, `src/apps/desktop/e2e/support/mock-core.ts` (the two invokes), `specs/changes/0009-agent-templates/qa-report.md` (the license list).
- **Work:** D6.
- **Depends on:** T5.
- **Acceptance:**
  - Rust unit tests behind the dialog seam, with an injected path:
    - a file of exactly 256 KiB is read; 256 KiB + 1 is refused;
    - a FIFO (unix) and a folder are refused as not regular files;
    - the write failure text.
  - A Rust test reads `capabilities/default.json` and asserts that its permissions equal the `allow-*` entries of `WEBVIEW_COMMANDS` plus `notification:default`, with no `dialog:` or `fs:` entry.
  - `tauri-plugin-dialog` is `=2.8.4` in the root `[workspace.dependencies]`, and `cargo tree -i tauri` shows one tauri version.
  - The crate facts in Context are proven with the tools, not taken from this plan: `cargo metadata --format-version 1` gives the version `2.8.4` and the license `Apache-2.0 OR MIT` of `tauri-plugin-dialog`, and its `tauri` requirement (`^2.12`); `cargo tree -p tauri-plugin-dialog` lists what it adds. The output is saved in the QA report. A fact that differs stops T6 and goes to god.
  - The license list of every crate the change adds to `Cargo.lock` is in the QA report, each compatible with AGPL-3.0-or-later.
  - fmt, clippy, `cargo test -p office-harness-desktop` (CI covers it; locally only if `libdbus` builds).

### T7: Desktop templates UI (FR-001 to FR-011, PB-5, SC-001)

- **Files:** `D/modules/template/` (new), `D/modules/agent/components/agent-details.svelte`, `D/modules/hierarchy/views/hierarchy-view.svelte`, `D/modules/panel/views/panel-view.svelte` (view entry), `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/templates.spec.ts`.
- **Work:** D8.
- **Depends on:** T6.
- **Acceptance:**
  - `bun test` for the state and service.
  - Playwright with axe on the mock core, all by keyboard:
    - save an agent from details;
    - build a team template;
    - create a CEO, then two teams of 1 + 6 from the templates;
    - the import preview shows env names with "no value", the "values are not imported" line, and an ESC as a visible symbol, and confirm saves while cancel saves nothing;
    - the create dialog shows "no value" for a value-less env entry (FR-011);
    - the export notice shows before writing;
    - an invalid import shows its reason;
    - the empty view offers save and import.
  - A timed scripted run of the SC-001 flow is recorded as machine evidence. The human timing stays a hands-on row.
  - `svelte-check`, `eslint`, `prettier --check`.

### T8: Docs and QA (all criteria)

- **Files:** `README.md` (templates, export and import, "no value"), `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md` (a cross-reference), `specs/changes/0009-agent-templates/qa-report.md`.
- **Work:**
  - Document the flows.
  - QA rows for SC-001 to SC-005 and the edge cases.
  - Hands-on rows are recorded as BLOCKED until run: SC-001's 3-minute human run, the real file dialogs on KDE, macOS, and Windows, the screen reader pass, and 200 percent zoom.
- **Depends on:** T7.
- **Acceptance:** every Traceability row has evidence or a recorded gap; `rg` finds no other project's name.

## Dependencies

T1 → T3; T2 → T3 → T4 → T5 → T6 → T7 → T8. T1 and T2 can run together. `Cargo.lock`, the migration ledgers, and the e2e mock core belong to one running task at a time.

Gates:
- this plan approved (Efe plan review PASS);
- T7 starts only when the 0011 `visibleText` helper (`D/modules/agent/models/task-text.ts`) is on main. If it is not, T7 stops and reports to god; it does not copy the helper.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged).

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0009-agent-templates/{data-model.md,contracts/}` | `docs(spec-0009): add the template data model and protocol` |
| 2 | T2 | `M/agent/`, `M/hierarchy/application/`, `H/bridges/role_requests.rs`, `D/modules/agent/` | `feat(agent): keep environment variables without a value` |
| 3 | T3 | `M/template/`, `Cargo.toml`, `Cargo.lock`, `scripts/check-boundaries.sh` | `feat(template): add the template module` |
| 4 | T4 | `H/bridges/`, `H/bootstrap/`, `M/hierarchy/` | `feat(server): create agents and teams from templates` |
| 5 | T5 | `M/template/presentation/`, `H/server/`, `src/apps/office-harnessd/tests/` | `feat(server): answer template requests` |
| 6 | T6 | `Cargo.toml`, `Cargo.lock`, `src/apps/desktop/src-tauri/`, `src/apps/desktop/e2e/support/mock-core.ts`, `specs/changes/0009-agent-templates/qa-report.md` | `feat(desktop): export and import template files` |
| 7 | T7 | `D/`, `src/apps/desktop/e2e/` | `feat(desktop): manage templates and create from them` |
| 8 | T8 | `README.md`, `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md`, `specs/changes/0009-agent-templates/qa-report.md` | `docs(spec-0009): document templates and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, and Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **Per phase PR:** CI green on the matrix; Efe's review at the head sha.
- **Security pass** (`secure-engineering`, full: the import is untrusted input, and file I/O is new):
  - The server parses the import with size (256 KiB) and shape limits, and refuses unknown fields.
  - Nothing is saved before confirm, and no imported env value is ever saved.
  - The tree exclusion is never taken twice on one thread (held variants, real-repository tests).
  - The preview never applies control codes.
  - The file commands take no path, touch only the dialog-picked path, read only regular files, and count the bytes they read.
  - Export holds no env values, and the notice warns about flags and prompts.
  - The agent connection cannot reach `template.*`.
  - The new dependency is pinned in the workspace, comes from the official Tauri org, and every added crate's license is listed and compatible.
- **Rollback:** each phase PR is reverted in the umbrella. The new module's migration is forward-only. `DatabaseTooNew` is per module ledger, so an older program does not know the `template` ledger and opens the database, ignoring the `template_*` tables. After a downgrade, an agent whose additions hold a "no value" variable fails to load them, because the older reader expects a string. The only safe rollback on a user's machine is the reverted binary plus the `office.db` copy taken before the upgrade.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-005 and the edge cases.
2. **Architecture:** `scripts/check-boundaries.sh` passes with the new crates.
3. **Code quality:** lints are clean; `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office from before the change opens after the upgrade (T3 migration test, CI on three OSes).
5. **Scope fidelity:** no own-worktree field, no agent use of templates.
6. **Readiness:** `/ac:verify` passes on the umbrella head before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T3, T5, T7 | save rules; from-agent run; details "Save as template" (delivered at the 0009 text; the 0010 plan owns the MODIFIED text) |
| FR-002 | T3, T4, T5 | pattern tests; `reviewer-1`/`reviewer-2` run |
| FR-003 | T3, T7 | team body rules; team editor (delivered at the 0009 text; the 0010 plan owns the MODIFIED text) |
| FR-004 | T4, T5 | 1 + 6 team run |
| FR-005 | T4 | fault injection; collected pre-check reasons; failed `remove` during undo |
| FR-006 | T3, T5 | copy semantics; SC-004 run |
| FR-007 | T3, T5, T6, T7 | env `null` export; file write; notice |
| FR-008 | T3, T5, T7 | preview saves nothing; preview dialog |
| FR-009 | T3, T6 | each refusal kind; 256 KiB cap |
| FR-010 | T3 | 50 and 51 agents |
| FR-011 | T2, T3, T4, T7 | every text path keeps `None`; editor round trip; import drops values; launcher skips `None`; "no value" shown |
| PB-1 | T4 | 0002 refusal text |
| PB-2 | T4 | create records with the user as actor; Create and Delete entries after a rollback |
| PB-3 | T2, T4 | no CLI configuration path written |
| PB-4 | T6 | no path argument; regular files only; capability equality test |
| PB-5 | T7 | keyboard Playwright with axe |
| A1 to A10 | T2 to T7 | per decision above |
| SC-001 | T7, T8 | scripted run; human timing BLOCKED until run |
| SC-002 | T5 | export and import round trip |
| SC-003 | T3 | each invalid file kind |
| SC-004 | T5 | edit and delete run |
| SC-005 | T4 | fault injection at each position |
| US1, US2, US3 | T2 to T7 | the T5 runs and the T7 Playwright runs |
