---
status: draft
spec: specs/changes/0005-office-scene/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-17. Build starts after change 0003 (panel practicality) merges to `main`, which itself starts after 0002. The owner or god approves the plan before the build. -->

# Plan: office scene (0005)

## Summary

This change adds the optional second view of the desktop app: a flat two-dimensional picture of the office in which every agent sits at a desk with its name, its status, and its last output line, grouped into floors by project and into departments by team. The user opens the scene from the panel in one step, opens any desk's terminal in one step, and returns to the panel with everything where he left it. The app reopens the view that was open when it closed. It is webview work only: no server module, no new request, no new event, no new table.

**What you get:**
- A scene view with one desk per agent (name, status text, last output line), floors by project plus a "No project" floor, and departments by team with the team lead first where 0002 is merged.
- Open a desk by click or by keyboard: the agent's live terminal opens in the same window with keyboard focus. No second window.
- The project switcher and a search by agent name applied to the scene, with the same result as in the panel.
- Round trip panel, scene, panel with the project choice and the selected agent kept, and the open view restored on the next app start (0003/FR-008 as modified).
- Full keyboard access, a visible focus mark, status announcements to screen readers, and no moving element when the system asks to reduce motion.

**Why this approach:**
- The scene shows what the panel's agent list shows (A2), and the office store already holds it and already applies `agent.status`, `agent.changed`, `agent.deleted`, and the last line from 0003 in place. A scene that reads the same snapshot adds no second source of truth, so SC-002 and SC-003 hold by construction and tests prove them.
- `docs/ARCHITECTURE.md` already places the R7 scene: "a webview module that reads agents, roles, and statuses. It needs no server module of its own." One new folder under `src/apps/desktop/src/modules/` and small changes to the app shell and the last-view code are the whole diff.
- The panel's project switcher, terminal embed, and last-view persistence exist after 0003. Reusing them keeps PB-2 true: the scene adds no function that the panel lacks.

**What it will not do:** a 3D scene, moving avatars, or animation beyond the status text; placing, moving, or renaming desks; create, start, stop, or delete from the scene; meetings of 0004; agents on other machines (R5); search across terminal output; notifications beyond 0003; any new visual design beyond the spec's flow.

**Shape of the work:** 7 tasks in 1 phase PR into one branch `feat/0005-office-scene`, one local commit per task. **Effort:** M.

**Main risks:**
- 0003 is not merged, so its `last_view` shape, panel exports, and `last_line` field can still change before this build starts. Mitigated by the merge gate and by T1, which re-reads the merged code before any scene code is written.
- SC-004 asks for 100 desks in under 2 seconds. Plain text DOM of 100 desks is small, and the e2e run measures it; if it misses, the fallback is folding a floor's departments behind a disclosure, which the spec allows ("groups folded by the user"). Decided only if the measurement fails.
- Many statuses change at once and a screen reader announcement per desk could flood. Mitigated by one polite live region, which coalesces, and by a test with 12 desks changing in the same second.
- Switching views unmounts the embedded terminal, so the live scroll position is lost. The next attach shows the recent output (0001/FR-016), so no output is lost; accepted as the cost of one view at a time.

**Decisions made for you:** the `office-scene` webview module, the terminal embedded in the scene with the office selection untouched, the extended `last_view` JSON with a `view` field and a panel default for unknown values, an inline scene search that focuses the desk, alphabetical desk order after the lead and the CEO, and no new dependency.

**Blockers:** the build waits for 0003 to merge to `main`. Nothing else.

## Goal

Deliver spec 0005: from the panel the user opens a flat picture of the office in which every agent of his project choice sits at a desk with name, status, and last output line, grouped by project floor and team department. He opens any desk's terminal in one step, moves between panel and scene without losing his place, and the app reopens the view he left. The scene can do nothing that the panel cannot do.

## Context

- **Repository:** 0001 is on `main` (merge 4425166). 0002 and 0003 are approved and wait for their builds. This plan branches `feat/0005-office-scene` from `main` after 0003 merges; until then only the spec and this plan exist. The scene depends on 0003 (the panel it opens from) and not on 0004; it reads role and team where 0002 is present, which is always the case after the gate.
- **Existing seams this change touches** (verified in the repository at `origin/main` 785fdbc on 2026-11-17):
  - App shell: `src/apps/desktop/src/app.svelte` renders the 0001 connection states and then one office view, with no router. The `OfficeStore` is created once here (`modules/agent/state/office.store.svelte.ts`), so its snapshot survives a view switch unchanged. This is what makes FR-006 hold.
  - Office data: `modules/agent/state/office-controller.ts` applies `agent.status`, `agent.changed`, `agent.deleted`, additions, action-record, resync, and storage events to one snapshot and keeps the selection; `state/office-events.ts` reads the event bodies; `models/agent.ts` holds `Agent`, `Project`, `shownStatusText` (which shows "Stopping" while a stop request is open), and `agentEntryName`. The 0003 plan adds `agent.line` and the snapshot field `last_line` (its T2); the scene reads that field once merged.
  - Terminal: `modules/terminal/index.ts` exports `TerminalView` with props `client`, `agentId`, `agentName`, `isRunning`; `views/terminal-view.svelte` sends every key to the CLI except the focus shortcut and shows a stopped agent's recent output (`isRunning` false); `core/shortcuts/shortcut.ts` parses and matches shortcuts.
  - Panel (from the 0003 plan, draft): a `modules/panel/` folder with the agent rows, the project switcher (`components/project-switcher.svelte`), the switcher dialog, and the last-view code under `modules/panel/state/`, which writes one `agent_settings` key `last_view` holding JSON `{ "project", "agent", "terminal_open" }` at most once per second and on close, and restores it on start. Not on `main` yet; every name in this list is confirmed in T1 before any scene code is written.
  - Accessibility today: rows are buttons with a hand-set accessible name, `focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700` for the focus mark, headings with `tabindex="-1"` as focus targets, and `role="status"` for loading text (`views/office-view.svelte`, `components/agent-list.svelte`).
  - Tests today: `bun test src` beside the `.ts` files; `boundaries.spec.ts` enforces module imports through `index.ts` only; Playwright with `@axe-core/playwright` against the Vite build with `e2e/support/mock-core.ts`, which seeds agents and projects, pushes events and terminal frames, records every invoke call, and answers `settings.get` with the focus shortcut. Project `chromium` only.
  - What does not change: no server crate, no wire type, no Tauri command, no new npm dependency, so `scripts/check-boundaries.sh`, the Cargo graph, `src-tauri/`, and `package.json` dependencies are untouched.
- **Floor rules:** one build or test at a time on the floor; ask god for the build slot before each local build or test run (bun, at most 30 minutes). Efe reviews each PR at its exact head sha; god merges. Nothing is pushed to `main`. No other project's name appears in any repository file.
- **Toolchain:** as 0001 to 0003 (cargo 1.100, bun 1.4.4, Node 24, Tauri v2, Svelte 5 with runes, Tailwind CSS 4, TypeScript).

## Decisions

- **D1 Module layout:** one new webview module `src/apps/desktop/src/modules/office-scene/` with `index.ts` (public face), `views/scene-view.svelte`, `components/floor.svelte`, `components/desk.svelte`, and `state/floor-groups.ts` with its `floor-groups.spec.ts`. No server module and no new server data (A2, PB-5), exactly as `docs/ARCHITECTURE.md` places the scene.
  - Simpler alternative rejected: put the scene inside the `agent` module folder. The scene is a view of agent data, but the architecture doc keeps one webview module per concern, and a second module folder keeps `boundaries.spec.ts` meaningful.
- **D2 One snapshot source (FR-002, FR-004, SC-003):** `SceneView` receives the same `OfficeStore` prop that the office view receives today and derives everything from `store.snapshot`: agents, projects, `statusDetails`, `stoppingAgentIds`, and the last line as merged from 0003. No second fetch, no polling, no scene store of its own. Updates ride the events the controller already applies in place, so a desk shows a new status or line as fast as the panel does (A6: measured from the moment the app reports the change).
- **D3 Grouping (FR-003):** pure functions in `floor-groups.ts` turn agents and projects into ordered floors. Floors are the projects of the current choice sorted by project name, plus one floor named "No project" last when any agent of the choice lacks a project. Each floor shows its project name and its agent count. Within a floor, the group without a department name comes first, then departments sorted by team name. A department lists its team lead first, then the rest by agent name. The group without a name lists the CEO first when present (the spec's open question), then the rest by agent name. A department appears on every floor where one of its agents sits, and its lead is first only on the floor where the lead sits (the spec's open question). Where the 0002 role and team fields are absent or empty, the grouping is floors only, and an agent with a project but no team sits in the group without a name.
- **D4 Desk (FR-002, PB-3):** a desk is a button. Its accessible name is the agent's name, its project's name, and its status text, as the spec's accessibility bullet demands. Its visible text is the agent name (cut with an ellipsis when long, the accessible name always holding the full name), the status as text from `shownStatusText`, and the last output line cut with an ellipsis at the desk's width, empty when the agent has no output yet. Status is text, never color alone.
- **D5 Open a desk (FR-005, PB-1, PB-4):** a click or Enter on a desk embeds the terminal module's `TerminalView` in the scene view with keyboard focus, exactly as the panel embeds it in its detail area, and no second window opens. The focus-leave shortcut returns focus from the terminal to the desk, by the same mechanism the panel uses (confirmed in T1 R2). A desk of an agent with status `exited`, `failed`, or `stopped` opens with the recent output, which `TerminalView` already does for a non-running agent (0001/FR-016). The office selection (`selectedAgentId`) is never touched by the scene, so FR-006 is exact: returning shows the panel with the selection it had before the scene opened. The scene exposes no other action; it renders no create, start, stop, or delete control (PB-1), and switching views sends no agent request (PB-4).
- **D6 View switch and last view (FR-001, FR-006, MODIFIED 0003/FR-008):** `app.svelte` owns `openView: "panel" | "scene"` and renders one view at a time, with the two actions (to the scene, to the panel) in the shell above the view, so both views offer the other in one step. The `last_view` value gains `"view": "panel" | "scene"`, written by the same throttle and on the same moments as 0003 (at most once per second and on close). On start the app opens the stored view; an unknown or missing `view` opens the panel, so a value written before this change still works and the settings validator never rejects a value because of this field. The scene is optional: an office that never opens it behaves exactly as 0003 built it (A4, PB-2).
- **D7 Project switcher and search (FR-007):** the scene reuses the panel's project switcher component through `modules/panel/index.ts` (the public face; `boundaries.spec.ts` allows an import through another module's index), so both views switch in one step with the same result: one project shows only that project's floor, "All projects" shows every floor, and the "No project" floor shows only under "All projects". The scene's search is an inline field of its own: it reuses the panel switcher's name match function (recorded in T1 R2), so both views return the same matches, keyboard focus moves to the first matching desk, spaces-only text counts as no search, and zero matches shows the "no desk matches" text. The panel's switcher dialog is not reused for this, because its result is an opened terminal, which is a different result from the spec's "shows which desk it is and focuses it".
- **D8 Empty and failure states:** the office with no agent shows the office-empty text and a button that returns to the panel, where the create action of 0001 lives; the scene does not open the create dialog itself (PB-1, and the Clarification keeps create in the panel). A project floor without agents shows its name and the text that it has no agents. The 0001 connection states stay in front of both views, and an office load failure shows the same failure message and retry as the panel (PB-2). A stop request in flight shows "Stopping" on the desk, as the panel row does.
- **D9 Accessibility (FR-008):** desks are buttons in reading order: floor by floor, department by department, desk by desk, which is the DOM order. Tab moves between desks in that order; the arrow keys additionally move within one floor; Enter opens. The focus mark is the shared `focus-visible` outline, always visible. One polite live region announces "agent name, new status" on each desk status change and announces the focus move when the focused desk's agent is deleted; then focus moves to the next desk in reading order, to the previous desk when none follows, and to the empty-state heading when no desk is left. Text contrast comes from the shared theme (AA), and the 200% zoom check is a hands-on row in the QA report.
- **D10 Reduced motion (FR-009, SC-007):** the scene ships no animation, no transition, and no moving element of its own. Any transition it inherits from the shared UI already honors `prefers-reduced-motion` or gains a guard in this change. The SC-007 scan runs Playwright with `reducedMotion: "reduce"` while 12 statuses change and asserts zero moving or flashing elements.
- **D11 Scale (SC-004):** every desk renders as plain DOM at a fixed readable size and the page scrolls; nothing shrinks or virtualizes at 100 agents. The 2-second open and the 15-second find-and-open are measured in Playwright with 100 agents in 10 projects seeded in the mock core; the find-and-open measurement uses the scene search to locate the named desk, then opens it.
- **D12 Tests:** `bun test` for `floor-groups.ts`; Playwright with the mock core and axe for the views at 1280 and 360 widths, as 0001 and 0003 did; the timing measurements for SC-002, SC-003, SC-004, and SC-006 and the keyboard-only run for SC-005 in the same e2e suite; `e2e/support/mock-core.ts` gains role and team fields on seeded agents and a `settings.set` recorder. The screen reader pass and the 200% zoom check are hands-on rows.
- **D13 Lint and boundaries:** unchanged. The new folder passes `boundaries.spec.ts` as any module does; `scripts/check-boundaries.sh` is untouched because no Rust crate changes.
- **D14 Delivery and commits:** one phase PR into `feat/0005-office-scene`, per-task local commits, Efe gates the PR at its head sha, god merges (the 0003 procedure).
- **Constitution check:** the repository has no instruction file; `docs/ARCHITECTURE.md` is binding, and this plan adds one webview module folder that its Frontend Folder Structure already anticipates. The owner's global rules apply: clean code, no secrets, explicit approval for remote actions.

## Scope

**In:**
- FR-001 to FR-009, the modified 0003/FR-008, and SC-001 to SC-007 of spec 0005.
- Desktop webview: the `office-scene` module (model, floors, desks, search, empty states), the view switch in `app.svelte` with the two actions, the embedded terminal on desk open, and the `view` field in the `last_view` value.
- Reuse of the panel's project switcher, terminal embed, and last-view code.
- `e2e/support/mock-core.ts` extensions for the scene fixtures.
- README and `docs/ARCHITECTURE.md` additions for the scene.

**Out (guards against unrequested additions):**
- A 3D scene, avatars, and any animation beyond the status text (A1).
- Placing, moving, renaming, folding, or saving desks, floors, or departments (A3); the fold fallback of D11 exists only if the SC-004 measurement fails.
- Create, start, stop, or delete controls in the scene (PB-1).
- Meetings of 0004 (A5), machines of R5, search across terminal output, notifications beyond 0003.
- Any second terminal at once, any new request, event, wire type, table, or migration (A2, PB-5).
- Any new visual design beyond the spec's flow (the spec says none exists).

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Office scene | The scene view, desk terminals, view switching and restore, keyboard and motion work, docs, QA | T1 to T7 | `feat/0005-office-scene` (`main` after 0003 merges) | `feat/0005-office-scene` → `main` |

The branch is created from `main` after 0003 merges and holds the approved spec and this plan (committed by the docs PR). The PR runs CI on three OSes and gets Efe's review at its head sha.

## Tasks

Paths are relative to `src/apps/desktop/src/` unless they start at the repository root. `M/` stands for `modules/`.

### T1: Research against the merged code (no product code)

- **Files:** `specs/changes/0005-office-scene/research.md`.
- **Work:**
  - R1: the merged 0003 last-view code: the exact files under `M/panel/state/`, the `last_view` JSON shape and its validator rules, and the write throttle. Record the `view` field extension of D6 with the exact key order and size impact. Verify that the validator ignores an unknown key in the stored view, or record the rollback limit in "Rollback".
  - R2: the merged 0003 panel: the project switcher component path and its public-face export, the switcher's name match function (which the scene search reuses for FR-007), how the panel embeds `TerminalView`, and how it receives the focus-leave shortcut from the terminal.
  - R3: the merged 0002 wire fields for role, team, team name, and the team lead on the agent snapshot; the exact field names that `floor-groups.ts` reads (D3).
  - R4: WAI-ARIA Authoring Practices for the desk pattern (a list of buttons with arrow-key movement), the polite live region, and `prefers-reduced-motion`; cite each.
  - R5: the Playwright reduced-motion emulation and the method of the SC-007 scan (which elements, which computed properties); cite the API. Decide the desk cut-off rule for very long lines (CSS `text-overflow: ellipsis`).
- **Depends on:** none, but every item reads merged code, so T1 starts only after the gate.
- **Acceptance:** every item has a "verified <date>, source <file or link>" line; the file names the exact files, exports, and field names that T2 to T5 use, and nothing else.

### T2: Scene model: floors, departments, search

- **Files:** `M/office-scene/state/floor-groups.ts`, `M/office-scene/state/floor-groups.spec.ts`, `M/office-scene/index.ts`.
- **Work:**
  - (a) Floors from the project choice sorted by name, plus "No project" last when any chosen agent lacks a project; each floor with its name and agent count (D3).
  - (b) Departments where the 0002 fields are present, with the lead first, cross-floor departments, the CEO first in the unnamed group, and floors-only grouping when the fields are absent (D3).
  - (c) Search matching by name: trims the text, treats spaces-only as no search, returns the matches in reading order (D7).
  - (d) Desk text inputs: status from `shownStatusText`, the last line, and the empty line when there is none (D4).
- **Depends on:** T1 (R3 field names).
- **Acceptance:**
  - (a) `bun test`: 12 agents in 3 projects produce 3 floors with 12 desks (US1 scenario 1); a no-project agent sits on "No project"; a project without agents produces its floor with count 0; 100 agents in 10 projects keep a stable order.
  - (b) Fixture tests with and without role and team: lead first in its department, a lead whose project differs from its team's project first only on its own floor, the CEO first in the unnamed group, an agent with a project but no team in the unnamed group.
  - (c) Search tests: one match, several matches in reading order, no match, spaces-only text treated as no search.
  - `bun test`, `svelte-check`, `eslint`, `prettier --check`.

### T3: Scene view: floors, desks, switcher, search, states

- **Files:** `M/office-scene/views/scene-view.svelte`, `M/office-scene/components/floor.svelte`, `M/office-scene/components/desk.svelte`, `M/office-scene/index.ts`, `app.svelte`, `M/panel/index.ts` (export the switcher), `e2e/support/mock-core.ts` (role, team, and settings seeds).
- **Work:**
  - (a) `SceneView` receives the `OfficeStore` and renders floors with name and count and desks with name, status text, and last line, all derived from the snapshot, so a change updates the desk in place without a user action (FR-002, FR-003, FR-004, D2, D4).
  - (b) The project switcher filters the floors; "All projects" shows every floor including "No project"; one project shows only its floor (FR-007, D7).
  - (c) The search field filters desks and focuses the first match; no match shows its text (FR-007, D7).
  - (d) `app.svelte` owns `openView` and renders one view at a time with the two shell actions (FR-001, D6).
  - (e) Empty and failure states (D8): office empty with a button that returns to the panel (the create lives there), empty floor text, no-match text, and the shared failure message.
  - (f) The scene renders no create, start, stop, or delete control (PB-1).
- **Depends on:** T2.
- **Acceptance:**
  - (a) Playwright with axe at 1280 and 360 widths: 3 floors and 12 desks from the US1 fixture; each desk's status text equals the panel row's text for the same agent (SC-003 shape); a pushed status change and a pushed line change update the desk in place and keep the search text and focus (SC-002 shape).
  - (b) Playwright: switcher to one project and back to "All projects"; the "No project" floor only under "All projects"; an empty floor shows its text.
  - (c) Playwright: no-match text, spaces-only search shows every desk again, the scene search and the panel switcher return the same matches for a mixed-case query, and the scene exposes no stop or delete control (PB-1). The empty-scene button returns to the panel, whose create control is present.
  - (d) Playwright: the scene action and the panel action each switch in one step, and switching sends no agent request (PB-4, asserted from the mock core's recorded calls).
  - `svelte-check`, `eslint`, `prettier --check`.

### T4: Open a desk, return to the panel, restore the view

- **Files:** `M/office-scene/views/scene-view.svelte`, `M/office-scene/components/desk.svelte`, `M/panel/state/` (the merged 0003 last-view files, names from T1 R1), `app.svelte`.
- **Work:**
  - (a) A desk click or Enter embeds `TerminalView` with keyboard focus; the focus-leave shortcut returns focus to the desk; a desk of an `exited`, `failed`, or `stopped` agent opens with the recent output (FR-005, D5).
  - (b) The panel action returns to the panel with the project choice and the selected agent untouched (FR-006, D5).
  - (c) The `last_view` write gains `view`, written by the 0003 throttle on switch and on close; the start restore opens the stored view; an unknown or missing `view` opens the panel (MODIFIED 0003/FR-008, D6).
- **Depends on:** T3.
- **Acceptance:**
  - (a) Playwright: a click and an Enter each open the terminal in the same window with focus and open no second window (US1 independent test); the focus-leave shortcut returns to the desk; a stopped agent's desk shows the recent output.
  - (b) Playwright: the round trip keeps the project choice and the selected agent in 20 trials (SC-006).
  - (c) Playwright over the mock core's recorded calls: `settings.set` carries `view: "scene"` while the scene is open and `view: "panel"` after the return; a start with a stored `last_view` without `view` opens the panel; a start with `view: "scene"` opens the scene; the recorded calls show no write other than `settings.set` of `last_view` (PB-6: no CLI configuration file and no project file).
  - `svelte-check`, `eslint`, `prettier --check`.

### T5: Keyboard, announcements, reduced motion, scale

- **Files:** `M/office-scene/views/scene-view.svelte`, `M/office-scene/components/floor.svelte`, `M/office-scene/components/desk.svelte`.
- **Work:**
  - (a) Tab moves between desks in reading order, floor by floor; the arrow keys move within one floor; the focus mark is always visible (FR-008, D9).
  - (b) One polite live region announces "agent name, new status" and the focus move after a deletion; deleting the focused desk's agent moves focus to the next desk in reading order, to the previous desk when none follows, and to the empty-state control when no desk is left (FR-008, D9, the concurrent edge cases).
  - (c) No animation or transition of its own; inherited ones honor `prefers-reduced-motion` (FR-009, D10).
  - (d) A long name and a long line are cut with a visible ellipsis; the accessible name holds the full name and the agent's name, project, and status (D4, edge cases).
- **Depends on:** T4.
- **Acceptance:**
  - (a) Playwright, keyboard only: open the scene, move over all 12 desks in reading order, open one with Enter, and return to the panel with 0 pointer actions (SC-005, US3 independent test); the focus mark is asserted on the focused desk.
  - (b) Playwright: a pushed status change adds "name, status" to the live region; deleting the focused agent moves focus to the next desk, to the previous desk when none follows, and to the empty-state control when the last desk is deleted, each with its announcement.
  - (c) Playwright with `reducedMotion: "reduce"` while 12 statuses change: the scan finds 0 moving or flashing elements (SC-007).
  - (d) Playwright with 100 agents in 10 projects at 360 width: every desk keeps a readable size, the page scrolls, the scene opens in under 2 seconds, and the user finds a named agent through the search and opens its desk in under 15 seconds (SC-004 measurement); a long name and a long line are cut with an ellipsis and the accessible names hold the full text.
  - `svelte-check`, `eslint`, `prettier --check`.

### T6: Documentation

- **Files:** `README.md`, `docs/ARCHITECTURE.md`.
- **Work:** README: the scene, how to open and leave it, what a desk, floor, and department show, that the scene can only open a terminal. `docs/ARCHITECTURE.md`: the `office-scene` folder in the Frontend Folder Structure tree and the phases status line for 0005.
- **Depends on:** T5.
- **Acceptance:** every behavior in the README matches the code and the spec; `rg` finds no other project's name or outside path in the changed docs.

### T7: QA report and final verification

- **Files:** `specs/changes/0005-office-scene/qa-report.md`.
- **Work:** run the checks in "Final verification", write one verdict per SC (SC-001 to SC-007) with the evidence, and list the hands-on rows.
- **Depends on:** T6.
- **Acceptance:** SC-002 (50 status changes and 50 line changes within 2 seconds), SC-003 (50 samples of both views), SC-004 (with 100 agents in 10 projects the scene opens in under 2 seconds and a user finds and opens a named agent within 15 seconds), SC-005 (0 pointer actions), SC-006 (20 trials), 0003/SC-006 (the panel shows the last view within 3 seconds for both views), and SC-007 (0 moving elements) are measured by the automated e2e runs against the mock core. SC-001 (a named desk found and opened in under 10 seconds with 12 agents) is the same e2e measurement plus one hands-on row against a real server. The hands-on rows (real server, screen reader pass, 200% zoom) are recorded as BLOCKED until a person runs them, never as PASS.

## Dependencies

T1 → T2 → T3 → T4 → T5 → T6 → T7.

- **Sequential:** T3 to T5 change the same scene folder and `app.svelte`; T4 changes the last-view files that T3's shell change touches.
- **Parallel:** none; the shared files make parallel tasks collide.
- **Exclusive:** `app.svelte` and `M/panel/` are owned by one running task at a time.
- **Gate before everything:** 0003 (and therefore 0002) is merged to `main`; the branch `feat/0005-office-scene` is created from that `main`; T1 starts only then.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; the sandbox dotfiles are never staged). After each commit, `git status --porcelain` lists only the sandbox dotfiles and the other draft specs.

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 0 | spec | the docs PR | `docs(spec-0005): approve office scene` |
| 1 | T1 | `specs/changes/0005-office-scene/research.md` | `docs(spec-0005): verify scene research against the merged panel` |
| 2 | T2 | `src/apps/desktop/src/modules/office-scene/` | `feat(desktop): model floors and departments for the scene` |
| 3 | T3 | `src/apps/desktop/src/modules/office-scene/`, `src/apps/desktop/src/app.svelte`, `src/apps/desktop/src/modules/panel/index.ts`, `src/apps/desktop/e2e/support/mock-core.ts` | `feat(desktop): add the office scene view` |
| 4 | T4 | `src/apps/desktop/src/modules/office-scene/`, `src/apps/desktop/src/modules/panel/state/`, `src/apps/desktop/src/app.svelte` | `feat(desktop): open terminals from desks and restore the scene` |
| 5 | T5 | `src/apps/desktop/src/modules/office-scene/` | `feat(desktop): make the scene keyboard and reader complete` |
| 6 | T6 | `README.md`, `docs/ARCHITECTURE.md` | `docs: describe the office scene` |
| 7 | T7 | `specs/changes/0005-office-scene/qa-report.md` | `test(spec-0005): record QA results` |

## QA and evidence

- **Per task:** `bun test`, `svelte-check`, `eslint`, `prettier --check`, Playwright with axe, each as far as the task touches it. Ask god for the build slot before each local build or test run. No Rust crate changes, so the cargo checks are unaffected; `cargo test --workspace` and `scripts/check-boundaries.sh` run once at T7 to prove it.
- **Phase PR:** CI green on all three OSes, Efe's review at the head sha, evidence in the PR body.
- **Security pass:** `secure-engineering` on T4: the `last_view` value is validated JSON, never evaluated; the scene adds no request type, no event, and no outbound call (PB-5); it writes no file (PB-6).
- **Real surface:** SC-001 against a real server with 12 agents, a screen reader pass over the scene, and the 200% zoom check are hands-on rows in the QA report; under the overnight directive they are recorded as BLOCKED (need a person), never PASS.
- **Rollback:** one PR into `feat/0005-office-scene`; a failed change is reverted there. On a user's machine the change adds one JSON field to an existing settings value; a value written after this change restores on an app built before it if the 0003 validator ignores the unknown `view` key, which T1 R1 verifies (otherwise the rollback limit is that the stored view is dropped and the panel opens). No table, no migration, no CLI file.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-007.
2. **Architecture compliance:** `boundaries.spec.ts` passes; the scene imports only `core/`, `shared/`, its own files, and other modules' `index.ts`; `scripts/check-boundaries.sh` and `cargo test --workspace` pass unchanged.
3. **Code quality:** all lints clean; a `/ac:review` of the PR diff has no open blocking finding.
4. **Migration safety:** an office and a `last_view` written by 0003 restore after the upgrade, with and without the `view` field (e2e runs, both branches of D6).
5. **Scope fidelity:** nothing from the Out list is in the diff; the scene has no control that acts on an agent beyond opening a terminal (PB-1); `rg` finds no other project's name, outside path, or person in docs or code.
6. **Readiness:** `/ac:verify` passes on the PR head before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T3 | T3(d) the shell actions and one view at a time; no second window in the T4(a) run |
| FR-002 | T2, T3 | T2(d) desk text; T3(a) desk content and in-place update |
| FR-003 | T2, T3 | T2(b) grouping fixtures; T3(a) floors and departments rendered |
| FR-004 | T3 | T3(a) in-place update keeps search and focus; SC-002 measurement at T7 |
| FR-005 | T4 | T4(a) terminal in the same window with focus, no second window |
| FR-006 | T4 | T4(b) round trip with the choice and the selection kept |
| FR-007 | T2, T3 | T2(c) search model with the panel switcher's match function; T3(b) switcher, T3(c) focus the first match and the same matches as the panel |
| FR-008 | T5 | T5(a) keyboard order and focus mark; T5(b) announcements and deletion focus |
| FR-009 | T5 | T5(c) SC-007 scan with reduced motion |
| 0003/FR-008 (modified) | T4 | T4(c) the restore of both views; 0003/SC-006 now measures the open view at T7 (both views within 3 seconds) |
| SC-001 | T7 | e2e find-and-open with 12 agents; hands-on row against a real server |
| SC-002 | T3, T7 | T3(a) same-event updates; T7 50 changes of each kind within 2 seconds |
| SC-003 | T3, T7 | T3(a) one snapshot for both views; T7 50 samples comparing the texts |
| SC-004 | T5, T7 | T5(d) 100 agents in 10 projects, readable desks, scroll, 2-second open, and find-and-open of a named agent within 15 seconds |
| SC-005 | T5 | T5(a) keyboard-only run with 0 pointer actions |
| SC-006 | T4, T7 | T4(b) 20-trial round trip |
| SC-007 | T5, T7 | T5(c) scan while 12 statuses change |
| PB-1 to PB-6 | T3, T4, T7 | PB-1: no action control in T3(f); PB-2: the scene adds no function and shares the failure states in T3(e); PB-3: text status in T3(a), keyboard in T5(a); PB-4: no agent request on a view switch in T3(d); PB-5: no new request, event, or outbound call in T4(c); PB-6: no write beyond `settings.set` of `last_view`, asserted from the recorded calls in T4(c) |
