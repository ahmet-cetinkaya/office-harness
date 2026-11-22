---
status: approved (god, 2026-11-19; Efe plan review PASS r2 at 1e1af70)
spec: specs/changes/0008-needs-you-queue/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-19 from the spec approved by the owner on 2026-11-19 (through Efe's review gate, PR #64, main dfd4611). Approved by god on 2026-11-19 after Efe's plan review PASS (r2 at 1e1af70); Efe gates each PR at its head sha. -->

# Plan: Needs-you queue (0008)

## Summary

This change gives the owner one standing list, "Needs you", with one item per agent that waits for him: failed agents first, then agents that wait for input, rang the bell, or were held automatically, then exited agents, longest wait first inside each group. One key, `Ctrl+Shift+F11`, opens the agent at the top, in the panel or in the office scene. The window title shows the count.

**What you get:**
- A server-kept item per agent in the `notification` module (new table, migration `0003_needs_you.sql`), so items survive app closure and a second app sees the same list (A6).
- The existing `NotificationBridge` feeds the items from the same events that create 0003 notifications. It also clears them on `starting`, `working`, `stopped`, delete, and the release of an automatic hold.
- Requests `needs_you.list` and `needs_you.dismiss`, and the event `needs_you.changed`.
- In the desktop app: a Needs you section above the agent list, the jump shortcut in the panel and the scene, the count in the window title in both views, and the count change announced in the live region.
- ACF-160 rides along in the desktop task: the always-true `isTerminalShortcut` field is removed.

**Why this approach:**
- The bridge already sees every attention event and every status frame (`H/bridges/notification_bridge.rs`). Hooking the items there adds no new detector (spec Context, PB-2).
- The `notification` module already owns the read rule and the per-agent unread state. Keeping items beside it needs no cross-module dependency. Status, delete, and hold-release signals reach it through host bridges, as holds do today.

**What it will not do:** read CLI prompts, answer agents, add alerts or reminders, or show the task line (spec Out of scope).

**Shape of the work:** 7 tasks in 2 phase PRs into one umbrella branch `feat/0008-needs-you-queue`, one local commit per task. **Effort:** M.

**Main risks:**
- **Read-on-arrival is decided by the client.** The server never learns which agent is on screen (`D/modules/notification/state/notification-store.ts:96-118`). D3, D4, and D8 make the desktop drop an item whose origin notification it has just marked read on arrival, so the item never shows, counts, changes the title, or is announced. The server removes it on the read (D2).
- **A stored focus shortcut equal to `Ctrl+Shift+F11`.** Today a stored focus shortcut that hits the reserved table falls back to the default, on both the server (`settings_repository.rs:35-41`) and the desktop (`focus-shortcut.ts:45`). The spec wants the stored key kept and the jump unbound. D6 changes both load paths for this one case.
- **The window title needs a new Tauri permission** (`capabilities/default.json` has no window permission). D7 adds one app command instead of a broad permission.

**Decisions made for you:** item storage in `notification`, the read and clear hooks, the protocol names, the ordering in the domain, the stored detail per event, the jump handler placement, the collision rule, and the title command.

**Blockers:** none. The local build slot comes from god before each build or test run.

## Goal

Deliver spec 0008. Needs you lists exactly the agents that wait for the user, in the FR-003 order, with the event as the office saw it and the wait time. One key opens the top agent's terminal, or its desk in the scene. Items survive app closure. The count shows next to Needs you and in the window title. 0003 notifications are unchanged.

## Context

- **Repository:** `main` at 037b10f (0001, 0002, 0003, 0005, 0007 shipped; D9 = b merged in #80; specs 0008 to 0011 merged).
- **Existing seams** (read at 037b10f on 2026-11-19):
  - Notification module: `M/notification/{domain,application,infrastructure,presentation}`. `NotificationEvent { Exited, Failed, WaitingForInput, Bell, AutoHold }` (`domain/src/enums/notification_event.rs:13`). The store port is `application/src/common/interfaces/notification_store.rs`. The repository is `infrastructure/src/persistence/notification_repository.rs`. Migrations end at `0002_detail.sql`; the ledger is `migrations/mod.rs:9`, registered at `H/bootstrap/services.rs:85`.
  - Bell coalescing: `create_notification_handler.rs:44` drops a bell while the agent has an unread bell. Spec FR-001 says a coalesced bell still counts for Needs you.
  - Bridge: `H/bridges/notification_bridge.rs`. `spawn` (66) listens to `agent.status` frames and bells. `notification_of_frame` (216) maps `exited`, `failed`, `waiting_for_input`. `record_auto_hold` (133) and `record` (141) publish `notification.created`. Wired at `H/server/running_server.rs:79`.
  - Read rule: `notification.read_agent` (`read_agent_notifications_handler.rs:29`) is sent on every route that opens a terminal: `panel-store.svelte.ts:72,82,95,151` and `app.svelte:177`. `notification.read` marks one row. The routing is `notification_handlers.rs:56`, and the gateway accepts the `notification.` prefix (`H/server/notification_requests.rs:33`).
  - Status frames: `agent_status_event` (`M/agent/presentation/src/mappings/events.rs:14`) carries `status` and an optional `detail` (the text of the status machine for `exited` with a failure and for `failed`, for example "the command ended with exit code 2"; none for exit code 0). Delete cleanup precedent: `H/server/agent_requests.rs:145-158` (`holds.forget_deleted_agents()`).
  - Hold release: `HoldService::release_hold` (`M/hierarchy/application/src/commands/hold/hold_service.rs:156`) calls `HoldJournal::released`, bridged at `H/bridges/hold_bridges.rs:69`.
  - Settings: `quiet_period_ms` (`M/agent/domain/src/value_objects/settings.rs:90`); `RESERVED_SHORTCUTS` has 11 entries (`:28`); `normalized_shortcut` (`:61`).
  - Desktop: `D/core/shortcuts/app-shortcuts.ts` (table at 48, kinds at 15); terminal passthrough `D/modules/terminal/views/terminal-view.svelte:143`; the only app key handler `D/modules/panel/views/panel-view.svelte:195`; the live region `:270`; the left column `:339`. The scene (`D/modules/office-scene/views/scene-view.svelte`) has no app key handler and is mounted instead of the panel (`D/app.svelte:158-179`); `openDesk` is at `:275`. The window title is fixed (`src-tauri/tauri.conf.json:15`); `src-tauri/capabilities/default.json` has no window permission. The e2e mock core is `src/apps/desktop/e2e/support/mock-core.ts` (`send_request` switch, invokes at 821-865).
  - Tests that use `Ctrl+Shift+F11` as a free key: `e2e/panel.spec.ts:412-430`, `core/shortcuts/app-shortcuts.spec.ts:53,84,133,148`, `modules/terminal/services/focus-shortcut.spec.ts:26`.
- **Floor rules:** one build or test at a time on the floor (ask god; `CARGO_BUILD_JOBS=4`, `--test-threads=4`, the pinned 1.100.0 toolchain first in `PATH`, per-crate local Rust runs). Efe reviews each PR at its exact head sha; god merges. Nothing is pushed to `main`.

## Decisions

- **D1 Placement and model:** items live in the `notification` module.
  - Domain: `NeedsYouItem { agent_id, first_event_at, latest: AttentionEvent { kind, detail: Option<String>, at }, origin: Option<NotificationId> }`, where `kind` is `Failed | WaitingForInput | Bell | AutoHold | Exited`.
  - Pure functions: `order(items) -> Vec<NeedsYouItem>` (group rank failed 0, waiting/bell/auto_hold 1, exited 2; then `first_event_at` ascending; then agent name, passed in as a lookup); `record_event(item?, event) -> NeedsYouItem` (a new item, or the existing one with a new `latest`, keeping `first_event_at` and `origin`).
  - Application: `NeedsYouService` with `record`, `clear(agent, reason)`, `clear_on_read(notification)`, `dismiss`, `list`, `forget_missing(agents)`. Infrastructure: the repository and migration `0003_needs_you.sql`.
  - Rejected: a new module. It would need the read rule from `notification` and add four crates for one table.
- **D2 Read and clear rules:**
  - `notification.read_agent` clears the agent's item. That is FR-004, "opens the terminal by any route": every open route sends it.
  - `notification.read` of the notification that created the item (`origin`) clears it. That covers the read-on-arrival rule of 0003/FR-006(b) (FR-001, edge case "terminal open in the focused window"): the client marks that row read at once, so the item lives for one round trip.
  - A dismiss clears the item only and marks nothing read (A4).
  - Status `starting`, `working`, or `stopped`, an agent delete, and the release of an agent hold when `latest.kind` is `AutoHold` clear it (FR-005).
  - `interrupted` neither adds nor clears (PB-3, edge case "server restarts").
- **D3 Recording:** the bridge calls `NeedsYouService::record` for every attention event, even one that the 0003 coalescing check turns into no notification, so a coalesced bell still updates the item (FR-001, LOW-2). It stores `detail` as follows:
  - `waiting_for_input`: the quiet period in ms that the terminal runtime started with, shown as "quiet for 5 s".
  - `exited` and `failed`: the status frame's `detail` text exactly as the server writes it, for example "the command ended with exit code 2". It is none for exit code 0 and the launcher's error text for a start failure.
  - `auto_hold`: the limit name.
  - `bell`: none.

  The bridge creates the 0003 notification without publishing it, then stores the item with `origin` (the new notification id, or none for a coalesced bell), then publishes `notification.created`, then `needs_you.changed`. A client that reads the origin notification on arrival thus always finds the item to clear. The bell check reads the unread state, not the item, so FR-001 still holds. An agent that is already deleted keeps its notification and gets no item.
- **D4 Protocol** (user clients only; the agent connection never answers `needs_you.*`):
  - `needs_you.list` returns `{items: [{agent_id, kind, detail, first_event_at, latest_at, origin_notification_id}]}` in the D1 order. `origin_notification_id` is text or null.
  - `needs_you.dismiss {agent_id}` returns the list. An agent without an item answers `not_found`.
  - Event `needs_you.changed {items}` carries the full ordered list after every change, as `hold.changed` does.
  - The desktop computes the wait time from `first_event_at`. T1 writes the contract.
- **D5 Jump shortcut:** a 12th table entry `Ctrl+Shift+F11`, kind `jump`. With ACF-160 every entry leaves the terminal, so the `isTerminalShortcut` field and the `pressedTerminalShortcut` filter are removed, and the terminal passes every table key out (0001/FR-017 as modified by 0008).
  - Panel: `handleAppKeys` handles `jump` by opening the top item's agent through the same `openAgent` path as the 0003 switcher (A2).
  - Scene: a window keydown listener in `scene-view.svelte` handles only `jump`, by calling `openDesk(topAgentId)` (FR-010). Other app shortcuts in the scene stay as today.
  - Focus (clarification by Efe, 2026-11-22, owner away): focus lands in the terminal when the agent runs, else on the terminal view heading, whose accessible name is "Terminal, <agent name>". A read-only terminal cannot take focus (0001).
  - Empty list: the view stays, and the live region says "No agent needs you" (FR-008).
  - Fast presses (US2: "Pressing it again goes to the next one"): after a jump the desktop removes the opened agent's item from its local list at once, so a second press before `needs_you.changed` comes back opens the next item.
  - The jump's per-OS fallback is not shipped yet, as `README.md:41` says for F1 to F10. A later release can move it into the 0003 fallback family with the agent shortcuts (A1).
- **D6 Collision with a stored focus shortcut** (edge cases, A1):
  - `RESERVED_SHORTCUTS` gains `Ctrl+Shift+F11`. A new focus shortcut equal to it is refused, as 0003 does.
  - On load, a stored focus shortcut equal to `Ctrl+Shift+F11` is kept, on the server (`settings_repository.rs` keeps it instead of falling back) and on the desktop (`focus-shortcut.ts`).
  - Server: the stored value is compared in the normalized form (`normalized_shortcut`), so `Shift+Ctrl+F11` is kept too. Only the jump key is exempt: a stored F1 to F10 or `Ctrl+Shift+H` still falls back to the default.
  - Desktop: the desktop loads the focus shortcut once into one shared state. The terminal, the panel handler, the scene listener, and the Needs you section read that state. While the focus shortcut equals the jump key, `pressedAppShortcut` does not match the jump entry. The focus-shortcut load does not count the jump entry as a table key, so a stored `Ctrl+Shift+F11` is kept.
  - The app has no screen for the focus shortcut; it is the `focus_shortcut` setting, changed through `settings.set` (`README.md:39`), and no settings event exists. The Needs you section therefore shows one line: "The jump shortcut is off because the focus shortcut uses Ctrl+Shift+F11. Change the focus_shortcut setting (see the README) and reopen the app to turn the jump on." The jump is bound again at the next app start after the setting changes, when the shared state loads.
  - Any other stored focus shortcut behaves as today. A test proves that no stored focus shortcut is lost by the new reservation (A1).
- **D7 Window title:** a new app command `set_window_title(count: u32)` in `src-tauri/src/commands.rs`, with its `allow-set-window-title` permission in `capabilities/default.json`. It sets `office-harness (N)`, or `office-harness` for 0.
  - The shell (`app.svelte`) calls it from an effect on the item count, so it runs in both views (FR-009).
  - Rejected: `core:window:allow-set-title`, which grants more than one string.
- **D8 Desktop list:** a `D/modules/needs-you/` module holds the state (list, `watchEvents` for `needs_you.changed`, a 1 s tick for the wait time), the service, and a `needs-you-section.svelte` component above the `Agents` heading.
  - Each row shows name, project, event text, and wait time as text. Its accessible name holds all four. Dismiss is a button.
  - An empty list shows "No agent needs you". The count is text next to the heading.
  - A count change is announced in the panel's live region and in the scene's live region. A new item is not announced separately (spec Accessibility).
  - The desktop does not show, count, title, or announce an item whose origin notification it has just marked read on arrival. The server removes that item on the read (D2).
  - No new visuals beyond these controls (spec Design rule).
- **D9 Accepted deviations (shipped; recorded 2026-11-22 from qa-report "Plan deviations"):**
  - `AgentNameLookup` is a port in `M/notification/application/src/common/interfaces/agent_name_lookup.rs`, so the notification module reads an agent's name without a dependency on the agent module. The server adapter `AgentNames` lives in `H/bridges/needs_you_bridge.rs`. Accepted by Efe.
  - T3 and T4 landed in one commit. Accepted by Efe.
  - The `detail` of a `failed` item is the status text of the server, not a text built from the exit code in the desktop. Decided by god; the contract says so.
  - The free key of the e2e test is `Control+Alt+F11`, not `Ctrl+Shift+F12` as T5 says: `Ctrl+Shift+F12` is the default focus shortcut, so it is not free (`D/e2e/panel.spec.ts`, `a key that is no table combination reaches the CLI`).
  - `focus-shortcut.ts` moved from `D/modules/terminal/services/` to `D/core/shortcuts/`, beside a new `focus-shortcut-state.ts`, so the shared state loads the focus shortcut once at app start. A change of the setting applies at the next app start. Every file name `D/modules/terminal/services/focus-shortcut*` in the Context and in T5 means the `D/core/shortcuts/` file.
  - A dismiss that answers `not_found` is a success in the desktop, because the item is already gone.
  - The jump focus rule of D5 is the clarification of Efe of 2026-11-22 (focus on the terminal view heading for an agent that does not run).
- **Constitution check:**
  - Module boundaries hold: `notification` gains no dependency, bridges live in `H/bridges/`, and `scripts/check-boundaries.sh` passes.
  - No CLI configuration is touched (PB-2).
  - Every new action works from the keyboard with visible focus (PB-4).
  - No data leaves the machine (PB-5). No other project's name is in the repository.

## Scope

**In:** FR-001 to FR-010, MODIFIED 0001/FR-017, PB-1 to PB-5, A1 to A6, SC-001 to SC-005, the spec's edge cases, ACF-160, the contract and README updates, a QA report.

**Out:** everything in the spec's Out of scope, other app shortcuts in the scene, and the task line (0011).

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Server | Items, hooks, protocol | T1 to T4 | `feat/0008-p1-server` (umbrella) | p1 → umbrella |
| 2 Desktop and ship | Section, jump, title, collision, docs, QA | T5 to T7 | `feat/0008-p2-desktop` (umbrella after p1) | p2 → umbrella, then umbrella → `main` |

The umbrella `feat/0008-needs-you-queue` is created from `main`. A single PR is the fallback if god prefers it; the tasks do not change.

## Tasks

Paths: `M/` = `src/modules/`, `D/` = `src/apps/desktop/src/`, `H/` = `src/apps/office-harnessd/src/`.

### T1: Design artifacts (no product code)

- **Files:** `specs/changes/0008-needs-you-queue/{data-model.md,contracts/needs-you-protocol.md}`.
- **Work:** (a) the data model: the item table, the stored `detail` per kind (D3), the clear reasons (D2). (b) The contract: `needs_you.list`, `needs_you.dismiss`, `needs_you.changed`, codes and shapes in the style of `0007-hold-and-limits/contracts/hold-protocol.md`, and the 12th reserved shortcut.
- **Depends on:** none.
- **Acceptance:** every FR that touches the server names its message or column; Efe reviews it with the p1 PR.

### T2: Item domain, store, service (FR-001 to FR-006 rules)

- **Files:** `M/notification/domain/src/` (new `needs_you_item.rs`, `attention_event.rs`, with `.tests.rs`), `M/notification/application/src/` (new `commands/needs_you/` service, store port), `M/notification/infrastructure/src/persistence/` (migration `0003_needs_you.sql`, repository, ledger entry).
- **Work:** D1 types and pure functions; `NeedsYouService` with the D2 clear reasons; the repository (`agent_id` primary key; one row per agent).
- **Depends on:** T1.
- **Acceptance:**
  - Unit tests for the order: failed above waiting above exited; waiting, bell, and auto_hold in one group; the longest wait first; ties broken by name.
  - A second event keeps the wait time and changes the latest event. A waiting agent that then fails moves to the failed group and keeps its wait time (A3).
  - A dismissed agent's next event starts a new item at the new time.
  - Repository: items survive closing and reopening the database (FR-006, SC-004).
  - Migration test: a database at notification `0002` upgrades with every notification row intact.
  - fmt, clippy, per-crate tests.

### T3: Bridge hooks (FR-001, FR-004, FR-005, PB-1, PB-3)

- **Files:** `H/bridges/notification_bridge.rs` (+ tests), `H/bridges/hold_bridges.rs`, `H/server/agent_requests.rs` (delete), `H/bootstrap/{notification_services.rs,services.rs}`, `M/notification/application/src/commands/{read_agent_notifications,read_notification}/` (clear calls).
- **Work:**
  - (a) Record every attention event, coalesced or not, with the D3 detail. The quiet period is the value the terminal runtime started with.
  - (b) Clear on `starting`, `working`, `stopped`. Ignore `interrupted`.
  - (c) Clear on `notification.read_agent` and on `notification.read` of the origin.
  - (d) Clear the items of deleted agents in the delete path. Drop items of missing agents at startup.
  - (e) Clear on the release of an agent hold whose item's latest event is `auto_hold`.
- **Depends on:** T2.
- **Acceptance:**
  - Bridge tests: a coalesced bell updates the item and creates no second notification (PB-1); `stopped` and `interrupted` add no item (PB-3); `working` after `waiting_for_input` clears it.
  - The office release does not clear an automatic agent hold's item.
  - The 0003 notification tests pass unchanged (PB-1).
  - fmt, clippy, per-crate tests, `scripts/check-boundaries.sh`.

### T4: Protocol and integration (FR-001 to FR-006, SC-001, SC-003, SC-004, SC-005)

- **Files:** `M/notification/presentation/src/{handlers,contracts,mappings}/`, `H/server/notification_requests.rs` (or a new `needs_you_requests.rs` gateway), `H/server/{router.rs,running_server.rs,server_state.rs}`, `src/apps/office-harnessd/tests/needs_you.rs`.
- **Work:** `needs_you.list`, `needs_you.dismiss`, and the `needs_you.changed` event published after every change (D4). The agent connection refuses `needs_you.*`.
- **Depends on:** T3.
- **Acceptance:** integration tests through a real client:
  - 30 agents, 5 attention events of mixed kinds: the list holds exactly those 5 in the FR-003 order (SC-001).
  - `needs_you.changed` arrives within 2 s of the event, and after the quiet period for waiting (SC-003).
  - The client disconnects, 3 events occur, and a new client's `needs_you.list` returns all 3 with their first event times (SC-004, FR-006).
  - An agent that writes output more often than the quiet period, rings no bell, and keeps running gets no item (SC-005).
  - An item is cleared by `read_agent`, by dismiss without marking notifications read (A4), by `working`, and by delete.
  - A server restart keeps items, and `interrupted` keeps them.
  - An agent connection asking for `needs_you.list` is refused as not allowed.
  - fmt, clippy, per-crate tests, boundaries.

### T5: Shortcut table, collision, ACF-160 (FR-007, MODIFIED 0001/FR-017, edge cases, A1)

- **Files:** `M/agent/domain/src/value_objects/settings.rs` (+ tests), `M/agent/infrastructure/src/persistence/repositories/settings_repository.rs`, `M/agent/infrastructure/tests/persistence.rs`, `D/core/shortcuts/{app-shortcuts.ts,app-shortcuts.spec.ts,index.ts}`, `D/modules/terminal/views/terminal-view.svelte`, `D/core/shortcuts/{focus-shortcut.ts,focus-shortcut.spec.ts}` (moved from `D/modules/terminal/services/`, D9), `src/apps/desktop/e2e/panel.spec.ts`.
- **Work:**
  - The 12th reserved shortcut and the 12th table entry (D5).
  - Remove `isTerminalShortcut` and `pressedTerminalShortcut`; the terminal checks `pressedAppShortcut` (ACF-160).
  - The D6 load rule on the server and the desktop.
  - The free-key e2e test moves from `Ctrl+Shift+F11` to a free key. Shipped: `Control+Alt+F11`, because `Ctrl+Shift+F12` is the default focus shortcut (D9).
- **Depends on:** T4 (lands in p2).
- **Acceptance:**
  - Settings tests: setting the focus shortcut to `Ctrl+Shift+F11` is refused with the conflict named, and a stored `Ctrl+Shift+F11` and a stored `Shift+Ctrl+F11` load unchanged. Every other stored value loads as before: a table of stored values, each kept or falling back exactly as at 037b10f (A1).
  - `bun test`: the table has 12 entries; the jump key passes out of a focused terminal; the jump entry is unbound when the focus shortcut is `Ctrl+Shift+F11`.
  - fmt, clippy, tests, `svelte-check`, `eslint`, `prettier --check`.

### T6: Desktop section, jump, title (FR-002, FR-003, FR-004, FR-007 to FR-010, PB-4, SC-001 to SC-003)

- **Files:** `D/modules/needs-you/` (new: state, service, `needs-you-section.svelte`, specs), `D/modules/panel/views/panel-view.svelte`, `D/modules/office-scene/views/scene-view.svelte`, `D/modules/notification/state/notification-store.ts`, `D/core/shortcuts/` (shared focus-shortcut state), `D/app.svelte`, `src-tauri/src/{commands.rs,main.rs}`, `src-tauri/build.rs` (`WEBVIEW_COMMANDS`, which generates `allow-set-window-title`), `src-tauri/capabilities/default.json`, `src/apps/desktop/e2e/support/mock-core.ts`, `src/apps/desktop/e2e/needs-you.spec.ts`.
- **Work:** the D8 section, the D5 handlers in the panel and the scene, the D7 title command and effect, and the mock core `needs_you.*` cases with the title invoke recorded.
- **Depends on:** T5.
- **Acceptance:**
  - `bun test` for the state: order kept from the server, wait time text, count.
  - Playwright with axe on the mock core:
    - Items show name, project, event text ("quiet for 5 s", "failed: the command ended with exit code 2"), and wait time.
    - 5 jump presses with a terminal focused open the 5 terminals in order and empty the list (SC-002), and the key never reaches the CLI.
    - Jump with an empty list keeps the view and announces "No agent needs you".
    - Jump in the scene opens the top desk with focus and leaves the panel's project and selection unchanged (FR-010).
    - Dismiss by keyboard removes the item and leaves the agent's status.
    - The title invoke gets 3, then 2, then 0, in the panel and in the scene.
    - A count change is announced once, and a new item is not announced twice.
    - An event for the on-screen agent in the focused window leaves no item, causes no title invoke, and announces no count change.
    - The collision line shows when the stored focus shortcut is `Ctrl+Shift+F11`. With that stored value, the key in a focused terminal moves focus out as the focus shortcut does and opens no agent, and the key in the panel opens no agent. With the default focus shortcut, `Ctrl+Shift+F11` in a focused terminal jumps.
    - The top item belongs to a project that the switcher hides: the jump opens it the way the 0003 switcher does (US2 scenario 4).
    - The top agent is deleted at the press: the jump opens the next item, or says "No agent needs you".
    - More items than fit: the list scrolls, and the title shows the full count.
    - The jump shows the top agent's terminal with focus within 1 s (SC-001).
    - Two fast presses with a delayed mock answer open the first and then the second item.
  - `svelte-check`, `eslint`, `prettier --check`.

### T7: Docs and QA (all criteria)

- **Files:** `README.md` (Needs you, the jump shortcut, the collision note; update line 39, which says the app reads the focus shortcut when a terminal opens (with the shared state of D6 a change applies at the next app start); correct line 61, which still says `Ctrl+Shift+H` does not work in a terminal (stale since #80); add the jump key to line 41's list and to its fallback note), `specs/changes/0003-panel-practicality/contracts/panel-protocol.md` (a cross-reference to the new requests), `specs/changes/0008-needs-you-queue/qa-report.md`.
- **Work:** document the flow; record the QA rows for SC-001 to SC-005 and the edge cases with evidence. Hands-on rows (screen reader pass, 200 percent zoom, the window title on KDE, macOS, and Windows, the jump key on KDE and macOS with Fn) are recorded as BLOCKED until a person or an agent-run pass runs them, never PASS.
- **Depends on:** T6.
- **Acceptance:** every Traceability row has evidence or a recorded gap; `rg` finds no other project's name.

## Dependencies

T1 → T2 → T3 → T4 → T5 → T6 → T7.

- **Parallel:** T5's server part (settings) can run beside T4, because they touch different crates. The desktop part waits for T4's contract.
- **Exclusive:** `Cargo.lock`, the migration ledgers, `src/shared/wire`, and the e2e mock core belong to one running task at a time.
- **Gate:** this plan approved by god.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; sandbox dotfiles are never staged). After each commit, `git status --porcelain` lists only untracked sandbox files.

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 1 | T1 | `specs/changes/0008-needs-you-queue/{data-model.md,contracts/}` | `docs(spec-0008): add the needs-you data model and protocol` |
| 2 | T2 | `M/notification/` | `feat(notification): keep one needs-you item per agent` |
| 3 | T3 | `H/bridges/`, `H/server/agent_requests.rs`, `H/bootstrap/`, `M/notification/application/` | `feat(server): feed and clear needs-you items from office events` |
| 4 | T4 | `M/notification/presentation/`, `H/server/`, `src/apps/office-harnessd/tests/` | `feat(server): answer needs_you requests and publish changes` |
| 5 | T5 | `M/agent/`, `D/core/shortcuts/`, `D/modules/terminal/`, `src/apps/desktop/e2e/panel.spec.ts` | `feat(desktop): reserve the jump shortcut and drop the terminal flag` |
| 6 | T6 | `D/`, `src/apps/desktop/src-tauri/` (including `build.rs`), `src/apps/desktop/e2e/` | `feat(desktop): show Needs you and jump to the top agent` |
| 7 | T7 | `README.md`, `specs/changes/0003-panel-practicality/contracts/panel-protocol.md`, `specs/changes/0008-needs-you-queue/qa-report.md` | `docs(spec-0008): document Needs you and record QA` |

## QA and evidence

- **Per task:** fmt, clippy, per-crate `cargo test`, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, and Playwright with axe, as far as the task touches them. Ask god for the build slot first.
- **Per phase PR:** CI green on the matrix, and Efe's review at the head sha.
- **Security pass** (`secure-engineering`, light; one new trust-boundary input):
  - `needs_you.dismiss` validates `agent_id` as an id.
  - The agent connection cannot reach `needs_you.*`.
  - The new Tauri command takes only a number.
- **Real surface:** the window title and the jump key on a real desktop go to the QA report as hands-on rows.
- **Rollback:** each phase PR is reverted in the umbrella. The notification migration is forward-only: a reverted binary refuses the upgraded database (`DatabaseTooNew`). A rollback on a user's machine is the reverted binary plus the `office.db` copy taken before the upgrade, as the README already says.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-005 and the edge cases.
2. **Architecture:** `scripts/check-boundaries.sh` passes; no module crate depends on another.
3. **Code quality:** lints are clean; `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office created before this change opens after the upgrade with its notifications intact (T2 test, CI on three OSes).
5. **Scope fidelity:** nothing from Out is in the diff; 0003 notification tests pass unchanged.
6. **Readiness:** `/ac:verify` passes on the umbrella head before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T2, T3, T6 | T3 record before coalescing; T6 on-screen agent leaves no item |
| FR-002 | T3, T6 | T3 detail per kind; T6 row text |
| FR-003 | T2, T4 | T2 order tests; T4 SC-001 run |
| FR-004 | T3, T4, T6 | `read_agent` and dismiss clears; T6 keyboard dismiss |
| FR-005 | T3, T4 | status, delete, auto-hold release clears |
| FR-006 | T2, T4 | reopen test; disconnected-client run |
| FR-007 | T5, T6 | table entry; 5-press run |
| FR-008 | T6 | empty-list jump run |
| FR-009 | T6 | section count; title invokes in both views |
| FR-010 | T6 | scene jump run |
| 0001/FR-017 (modified) | T5, T6 | jump key never reaches the CLI |
| PB-1 | T3, T4 | 0003 notification tests unchanged; coalesced bell adds no notification |
| PB-2 | T3 | no new detector; no CLI configuration path written |
| PB-3 | T3, T4 | `stopped` and `interrupted` add no item |
| PB-4 | T6 | keyboard Playwright with axe |
| PB-5 | T4 | no outbound connection (existing guards) |
| A1, edge collision rows | T5, T6 | stored-value table; collision line |
| SC-001 | T4, T6 | 30-agent integration run; Playwright order |
| SC-002 | T6 | 5-press Playwright run |
| SC-003 | T4, T6 | `needs_you.changed` within 2 s; title update |
| SC-004 | T4 | disconnected-client run |
| SC-005 | T4 | busy-agent run |
| US1, US2, US3 | T2 to T6 | the T4 integration runs and the T6 Playwright runs |
| ACF-160 | T5 | field removed; `bun test`, `svelte-check` |
