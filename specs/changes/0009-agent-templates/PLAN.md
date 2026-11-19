---
status: draft
spec: specs/changes/0009-agent-templates/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #65, main 10df6cd). 0010 modifies 0009/FR-001 and FR-003 (own worktree choice); this plan keeps the template body additive so 0010 adds one field. Needs approval through Efe's plan review. -->

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
- **All-or-nothing over many agents.** The shipped code has no multi-agent rollback. D5 validates everything first and undoes created agents and the team row in reverse order. A fault-injection test fails the batch at each agent position.
- **A new dependency.** `tauri-plugin-dialog`, the official Tauri plugin, is used only from Rust, with a pinned version. No dialog permission is given to the webview. God approved it on 2026-11-19 (decision D6).
- **The shipped `team.create` bug.** The desktop sends a `lead` object that the server does not read (reported to god on 2026-11-19). This plan does not depend on that fix: the template bridge builds its own team create.

**Decisions made for you:** the module, the storage, the body shape, "no value", the name pattern rules, the create bridge, the file commands, the protocol, and the desktop placement.

**Blockers:** none. The local build slot comes from god before each build or test run.

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
  - Host bridges: `H/bridges/role_requests.rs` (`RoleRequestsBridge` takes `store.tree_exclusion()` at `:54`; `AgentCreatorBridge` at `:156-197`). Wired at `H/bootstrap/hierarchy_services.rs:103-112`. The tree exclusion is a non-reentrant mutex.
  - Additions: `OfficeAdditions` (`M/agent/domain/src/value_objects/office_additions.rs:12`), `EnvEntry` (`env_entry.rs:8-42`, a `String` value), stored as JSON in `agent_office_additions` (`0003_additions_and_sessions.sql:5`).
  - Name uniqueness: a NOCASE unique index on `agent_agents.name_key`. A conflict maps to "agent name X is already used" (`application_error.rs:63`). Names are at most 64 characters (`display_name.rs:7`).
  - Gateways: `HoldGateway` (`H/server/hold_requests.rs:76`) is the pattern. Routing is `H/server/connection.rs:216-249`, the state is `server_state.rs:27-63`, and construction is `running_server.rs:83-133`. Migrations are registered in `H/bootstrap/services.rs:81-86`. The agent allowlist is `H/server/agent_call.rs:25-32`.
  - Desktop:
    - Create forms: `D/modules/agent/components/agent-form-dialog.svelte`, `D/modules/hierarchy/components/{hierarchy-agent-dialog,team-form-dialog}.svelte`.
    - Create buttons: `D/modules/hierarchy/views/hierarchy-view.svelte:494-509`.
    - Details actions: `D/modules/agent/components/agent-details.svelte:133-158`.
    - Desktop boundaries: `D/boundaries.spec.ts`.
    - Tauri commands: `src-tauri/src/commands.rs`, `build.rs:7-16` (`WEBVIEW_COMMANDS`), `capabilities/default.json`. No dialog or fs plugin exists today.
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
  - Fields are additive: 0010 adds `own_worktree: bool` (default false) to an agent body without a version bump.
- **D3 "No value"** (FR-011, Efe's plan note):
  - `EnvEntry.value` becomes `Option<String>`. `None` is "no value"; `Some("")` stays a value the user typed on purpose.
  - The additions JSON writes `null` for `None`. Older rows always have a string, so no migration is needed.
  - The launcher skips `None` entries when it builds the CLI environment, so the login value stays in effect.
  - The agent details and the template editor show "no value".
  - Export writes `null` for every value. Import keeps `null`.
- **D4 Name patterns** (FR-002, A2, edge cases):
  - `{n}` becomes the lowest positive integer that makes the name unique (case-insensitive).
  - `{team}` becomes the team name of the creation. For a direct agent creation it becomes the team of the place picked; with no team there, the creation is refused and names the pattern.
  - A pattern without `{n}` gets `-{n}` added only when the name is taken.
  - Names are chosen inside the tree exclusion, so two concurrent creations get different names.
  - Saving a template from an agent uses the A2 defaults.
- **D5 Create bridge** (FR-002, FR-004, FR-005, PB-1, PB-2, SC-005): `TemplateCreateBridge` in `H/bridges/template_create.rs`, under `tree_exclusion()` for the whole batch.
  - **Plan:** resolve each agent's name (D4), working folder (template, else the project's folder, else refused "a working folder is needed"), and role. The position sets the role in a team (A4). Then pre-check:
    - each folder exists as a directory, else it is named;
    - each name fits the 64-character rule;
    - the team name is free;
    - every place passes `check_placement` on a copy of the tree snapshot with the planned agents added in order.
    - Every failed check is collected and returned together ("name each reason").
  - **Apply:** create the lead, then the team row and the lead's place, then each member, through the existing `AgentCreator` and hierarchy store calls.
  - **Undo:** on a failure during apply, remove the created agents in reverse order with `AgentCreator::remove` (the same undo as role create) and delete the team row with `delete_team_and_unplace`. Then return the reason.
  - **Record:** each created agent is recorded by the existing create journal with the user as actor (PB-2).
  - Template changes are not recorded (A10).
- **D6 File commands** (FR-007, FR-008, PB-4), **approved by god on 2026-11-19:**
  - Add `tauri-plugin-dialog` (the official plugin, used from Rust only).
  - Two new app commands:
    - `export_templates_file(contents: String)` opens a save dialog and writes `contents` to the picked path. It answers `saved`, `cancelled`, or a failure ("the file was not written: ...").
    - `import_templates_file()` opens an open dialog and reads the picked file. It refuses more than 256 KiB before reading the rest, and returns the text.
  - The webview gets only `allow-export-templates-file` and `allow-import-templates-file`, with no dialog or fs permission.
  - The plugin version is pinned exactly in `Cargo.toml`. A capability test proves the webview cannot call any `dialog:` or `fs:` command and can call only the two app commands.
  - Rejected: the plugin's JavaScript API plus a write-by-path command, which would let the webview write any path.
- **D7 Protocol** (user clients only; not in the agent allowlist; the allowlist comment is updated):
  - `template.list` and `template.get {id}`.
  - `template.save {id?, name, kind, body}`. Without an id it creates; with one it replaces.
  - `template.delete {id}`.
  - `template.from_agent {agent_id}` returns an unsaved agent body with the A2 defaults, which the editor then saves.
  - `template.create_agent {template_id, place, project_id?}`. `place` is a 0002 role placement object.
  - `template.create_team {template_id, team_name, parent_agent_id, project_id?}`.
  - `template.export {ids?}` returns `{text}`: the file text `{"format":"office-harness-templates","version":1,"templates":[...]}` with every env value `null`.
  - `template.import.preview {text}` returns every template with its save name (A3) or the FR-009 refusal. It saves nothing.
  - `template.import.confirm {text}` validates again and saves all templates in one transaction.
  - Event: `template.changed {templates}`.
  - Codes: `invalid` (rule failures, with each reason), `conflict` (name taken on save), `not_found`, `refused` (tree rules, the 0002 text).
- **D8 Desktop:**
  - A `D/modules/template/` module: state, service, a `templates-view.svelte` reached from the panel header, an agent and team editor dialog, a create dialog (place or parent, team name, project), and an import preview dialog.
  - The preview lists every template's commands, folders, flags, env names (with "no value"), startup prompts, and save name, with control characters shown visibly through `visibleText`.
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

- **Files:** `M/agent/domain/src/value_objects/{env_entry.rs,office_additions.rs}` (+ tests), `M/agent/application/src/common/{additions_input.rs,session_launcher.rs}`, the additions JSON mapping in `M/agent/presentation/`, `D/modules/agent/` (additions display "no value").
- **Work:** D3.
- **Acceptance:**
  - Unit tests: `None` and `Some("")` round-trip as different values; an old additions row with strings loads unchanged.
  - Launcher test: a `None` entry is not in the child environment, and the login value of that name stays. A `Some("")` entry is set to empty, as today.
  - `bun test` for the label.
  - fmt, clippy, per-crate tests.

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
  - Editing or deleting a template leaves a copy in a team template intact (A5).
  - Repository: templates survive closing and reopening the database (A8).
  - fmt, clippy, per-crate tests, boundaries.

### T4: Create bridge (FR-002, FR-004, FR-005, PB-1, PB-2, SC-005)

- **Files:** `H/bridges/template_create.rs` (+ tests), `H/bootstrap/{services.rs,hierarchy_services.rs}`, and the hierarchy store port if a read of the snapshot with planned rows needs a helper (pure, in `M/hierarchy/domain`).
- **Work:** D5.
- **Depends on:** T3.
- **Acceptance:**
  - Bridge tests with fault injection: a team creation that fails at the lead, at member 1, and at the last member leaves 0 new agents and 0 new teams (SC-005).
  - A missing working folder refuses before any create and names the folder.
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
  - An agent connection asking for `template.list` is refused as not allowed.
  - fmt, clippy, per-crate tests, boundaries.

### T6: File commands (FR-007, FR-008, FR-009 size, PB-4)

- **Files:** `src/apps/desktop/src-tauri/{Cargo.toml,src/commands.rs,src/main.rs,build.rs,capabilities/default.json}` and the permission files, `src/apps/desktop/e2e/support/mock-core.ts` (the two invokes).
- **Work:** D6.
- **Depends on:** T5.
- **Acceptance:**
  - Rust unit tests for the read cap (256 KiB read; 256 KiB + 1 refused before reading the rest) and for the write failure text, using a path injected behind the dialog seam.
  - The capability file grants only the two commands. A capability test shows that a `dialog:` or `fs:` call from the webview is refused.
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
    - the import preview shows env names with "no value" and an ESC as a visible symbol, and confirm saves while cancel saves nothing;
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

T1 → T3; T2 → T3 → T4 → T5 → T6 → T7 → T8. T1 and T2 can run together. `Cargo.lock`, the migration ledgers, and the e2e mock core belong to one running task at a time. Gate: this plan approved (Efe plan review PASS).

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged).

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0009-agent-templates/{data-model.md,contracts/}` | `docs(spec-0009): add the template data model and protocol` |
| 2 | T2 | `M/agent/`, `D/modules/agent/` | `feat(agent): keep environment variables without a value` |
| 3 | T3 | `M/template/`, `Cargo.toml`, `Cargo.lock`, `scripts/check-boundaries.sh` | `feat(template): add the template module` |
| 4 | T4 | `H/bridges/`, `H/bootstrap/`, `M/hierarchy/domain/` (if needed) | `feat(server): create agents and teams from templates` |
| 5 | T5 | `M/template/presentation/`, `H/server/`, `src/apps/office-harnessd/tests/` | `feat(server): answer template requests` |
| 6 | T6 | `src/apps/desktop/src-tauri/`, `Cargo.lock`, `src/apps/desktop/e2e/support/mock-core.ts` | `feat(desktop): export and import template files` |
| 7 | T7 | `D/`, `src/apps/desktop/e2e/` | `feat(desktop): manage templates and create from them` |
| 8 | T8 | `README.md`, `specs/changes/0002-agent-hierarchy/contracts/hierarchy-protocol.md`, `specs/changes/0009-agent-templates/qa-report.md` | `docs(spec-0009): document templates and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, and Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **Per phase PR:** CI green on the matrix; Efe's review at the head sha.
- **Security pass** (`secure-engineering`, full: the import is untrusted input, and file I/O is new):
  - The server parses the import with size and shape limits.
  - Nothing is saved before confirm.
  - The preview never applies control codes.
  - The file commands touch only the dialog-picked path.
  - Export holds no env values, and the notice warns about flags and prompts.
  - The agent connection cannot reach `template.*`.
  - The new dependency is pinned and comes from the official Tauri org.
- **Rollback:** each phase PR is reverted in the umbrella. The new module's migration is forward-only (`DatabaseTooNew`). An older program ignores the unknown `template_*` tables, but refuses the database through the ledger check. A rollback on a user's machine is the reverted binary plus the `office.db` copy taken before the upgrade.

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
| FR-001 | T3, T5, T7 | save rules; from-agent run; details "Save as template" |
| FR-002 | T3, T4, T5 | pattern tests; `reviewer-1`/`reviewer-2` run |
| FR-003 | T3, T7 | team body rules; team editor |
| FR-004 | T4, T5 | 1 + 6 team run |
| FR-005 | T4 | fault injection; missing folder refusal |
| FR-006 | T3, T5 | copy semantics; SC-004 run |
| FR-007 | T3, T5, T6, T7 | env `null` export; file write; notice |
| FR-008 | T3, T5, T7 | preview saves nothing; preview dialog |
| FR-009 | T3, T6 | each refusal kind; 256 KiB cap |
| FR-010 | T3 | 50 and 51 agents |
| FR-011 | T2, T7 | launcher skips `None`; "no value" shown |
| PB-1 | T4 | 0002 refusal text |
| PB-2 | T4 | create records with the user as actor |
| PB-3 | T2, T4 | no CLI configuration path written |
| PB-4 | T6 | dialog-picked path only |
| PB-5 | T7 | keyboard Playwright with axe |
| A1 to A10 | T2 to T7 | per decision above |
| SC-001 | T7, T8 | scripted run; human timing BLOCKED until run |
| SC-002 | T5 | export and import round trip |
| SC-003 | T3 | each invalid file kind |
| SC-004 | T5 | edit and delete run |
| SC-005 | T4 | fault injection at each position |
| US1, US2, US3 | T2 to T7 | the T5 runs and the T7 Playwright runs |
