---
status: approved
spec: specs/changes/0027-task-board/SPEC.md
also_spec: specs/changes/0028-safe-board-changes/SPEC.md
commits: per-task
---

# Plan: Task board, the plain board and its safe changes (0027 and 0028)

## Summary

A new `board` module keeps one board per office in the office database: cards with a title, a note, and one of four statuses. The server answers four requests (`board.list` without notes, `board.get` for one card with its note, `board.create`, `board.move`) and publishes one event (`board.card_changed`, without the note) to every open window. A note is read only when the card details open, so the list stays far below the 1 MiB frame limit of the socket. A card has a `version` that rises with each kept change, and a `seq` that orders the cards of the board; the place of a card is its rank by `seq` inside its column. A window names the version it saw; the server refuses a change when the stored version is higher, and a window sends one change per card at a time, so two windows never lose a change without a word and a window never conflicts with itself. A new card carries a key made when its form opened, so a repeated save keeps one card. The desktop app gets a Board view (a new `board` webview module) opened from the panel and from the scene, with a New card form, a card details dialog, an action list for moves, pointer drag and drop, a live region that announces each kept or refused change, and a 2 second not-confirmed path.

**What you get:**
- Crates `src/modules/board/{domain,application,infrastructure,presentation}`, the table `board_cards`, the gateway `BoardGateway`, and the request prefix `board.`.
- A Board view in the desktop app on the plain shadcn template, with no new visual style (the look is chosen later, so the view keeps its markup and classes in a few components that a restyle can replace).
- Tests: Rust unit, persistence, host integration (two connections, restart, race), webview unit, and Playwright e2e on the mock core.

**Why this approach:**
- `schedule` is the closest model for the server path (per-row event, gateway, migration ledger); `template` is the closest model for a full-page view. The board copies both so no new pattern enters the codebase.
- Versions and keys make the two safety rules of 0028 plain data checks done inside one lock, so no per-field merge and no timing rule is needed.
- Places are derived from `seq`, so a move never rewrites other cards and an event carries one card.

**What it will not do:** editing, project, assignee, delete (0029), agent access (0030), Undo, in-column ordering, last-change time.

**Decisions made for the owner:** phased delivery (server, then desktop) into one umbrella branch (chosen); the board view stays on the plain template so a later restyle is cheap; Alt+Left and Alt+Right move a focused card to the previous or next column, in addition to the Actions list.

**Main risk:** the desktop behaviour on a lost server, because the not-reachable view of 0001 replaces the board view; the notice and its announcement live at app level for that reason, and the real-app run in T10 proves it.

**Shape of the work:** 11 tasks in 2 phases, each a PR into one umbrella branch (see Delivery). **Effort:** Large.

## Goal

Ship specs 0027 and 0028 together: a person can put cards on a four-column board, read them, and move them by pointer or keyboard; the board survives restarts and shows each kept change in every window within 2 seconds; a change that the office does not keep, or a stale or repeated change, never leaves a false board.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 22de3fe on 2026-11-28.

- **Module pattern:** `M/schedule/{domain,application,infrastructure,presentation}`; the root `Cargo.toml:4-10` globs module crates, but `src/apps/office-harnessd/Cargo.toml` must list the new crates. Port `M/schedule/application/src/common/interfaces/schedule_store.rs:10`, error enum `.../errors/schedule_application_error.rs:29` (`Invalid`, `CapReached`, `NotFound`, `Storage{message, is_disk_failure}`), SQLite store `M/schedule/infrastructure/src/persistence/schedule_repository.rs:20` (`Arc<Mutex<Connection>>`, `storage_failure` at `:45`).
- **Migrations:** `M/<module>/infrastructure/src/persistence/migrations/NNNN_name.sql`, listed in `migrations/mod.rs` (`LEDGER_TABLE`, `SCRIPTS`); shipped scripts never change. The runner is `H/bootstrap/migrations.rs:21-66`; each module is called from `H/bootstrap/services.rs:93-106`. Migration tests: `H/../tests/migrations.rs`.
- **Wire:** request names are string constants in `M/<module>/presentation/src/contracts/*_requests.rs`; events are built in `.../mappings/*_json.rs` (`schedule_changed_event`). Gateway `H/server/schedule_requests.rs:32` with `accepts(kind)` by prefix (`:73`) and `run` (`:91`). The router is the `if`-chain in `H/server/connection.rs:251-326`. Gateways are built in `H/server/running_server.rs:142-205` and held in `H/server/server_state.rs`. Wire error codes: `Invalid`, `Conflict`, `NotFound`, `Refused`, `Internal` (`src/shared/wire/src/envelope.rs:25`). A disk failure goes to a `storage.error` event (`H/server/template_requests.rs:106-108`).
- **Auth and agents:** a client must pass `handshake.rs:46-98` before any request; `Authenticated::Agent` requests pass the allowlist `H/server/agent_call.rs:31-42` (10 entries, none under `board.`), so a board request from an agent is already refused (0027/PB-4). The record is verified by host test (g) in T4.
- **Live updates:** `H/server/event_hub.rs` (`publish`, `subscribe`, backlog 256); every connection forwards every event (`connection.rs:132-146`). A lagging connection is closed and the desktop resyncs (`src-tauri/src/channels.rs:27-60`). The webview reloads its lists on each connection (`D/app.svelte:121-134`). There is no snapshot on connect, so the board view calls `board.list` on connect.
- **Desktop:** `D/modules/schedule/` (service `services/schedule-service.ts:26`, controller `state/schedules-controller.ts:71` upserting one row by id) and `D/modules/template/` (`views/templates-view.svelte`, `index.ts:3-10`). Controllers are created in `D/modules/panel/state/panel-store.svelte.ts:46,58`; views switch in `D/app.svelte:200-234` with `openView` held in `panel-store.svelte.ts:158-164`; the panel header buttons are in `D/modules/panel/views/panel-view.svelte:436-497`. Shared UI in `D/shared/ui/`: `dialog.svelte`, `announcer.svelte` (new node per text), `button`, `text-field`, `select-field`, `failure-message`, `status-panel`. There is no dropdown-menu component and no drag and drop precedent. Boundary rules: `D/boundaries.spec.ts` (only `$shared`; cross-module imports through `index.ts`; no cycles).
- **Tests:** Rust unit tests in `*.tests.rs` beside code; host tests in `H/../tests/*.rs`; webview `*.spec.ts` run by `bun test src`; Playwright `D/../e2e/*.spec.ts` with `installMockCore` (`e2e/support/mock-core.ts:317`, one handler per request kind); axe is used in `e2e/panel-scale.spec.ts`. Commands: `just test-rust`, `just test-web`, `just e2e`, `just lint-rust`, `just boundaries`, `just lint-web`, `just check`. CI path filters are in `.github/workflows/ci.yml:62-63`.
- **Text rules:** 0025/FR-001 refuses invisible and direction-changing format characters in a schedule name. The check (`is_format_char`, `FORMAT_RANGES`) is private to `M/schedule/domain/src/validation.rs:132`; T1 moves it to the kernel and both domains use it (D7).
- **Frame limit:** `MAX_FRAME_BYTES` is 1 MiB (`src/shared/wire/src/protocol.rs:20`, set on the server socket in `H/server/router.rs:68` and on the desktop socket). 200 cards with 8,192-character notes are about 1.6 MB, so a list with notes cannot pass; D6 keeps notes out of the list.
- **Agent refusal:** a `board.` request is not in `ALLOWED_REQUESTS` (`H/server/agent_call.rs:31-42`), takes the refusal branch, and `journal_refusal` writes the action record for every refusal, so 0027/PB-4 holds with no change.
- **Design artifacts:** D2 (the table) and D6 (the wire shapes) are the data model and the contract of this change; no separate `data-model.md` or `contracts/` file is written.
- **Docs:** `docs/ARCHITECTURE.md` lists modules, request prefixes, and the migration ledgers. QA reports are `specs/changes/archive/<change>/qa-report.md`; a shipped change moves under `specs/changes/archive/` and its requirements merge into `specs/task-board/SPEC.md`.

## Decisions

- **D1 Module and names.** Crates `office-harness-board-{domain,application,infrastructure,presentation}`; the request prefix is `board.`. The agent allowlist is not changed (0030 does that).
- **D2 Data.** Table `board_cards(id TEXT PRIMARY KEY, title TEXT NOT NULL, note TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('todo','doing','blocked','done')), seq INTEGER NOT NULL UNIQUE, version INTEGER NOT NULL, request_key TEXT UNIQUE)`; ledger `board_schema_version`; script `0001_board.sql`. The card cap is 200, a constant `CARD_CAP` in the domain. Ids are random UUID text made by the server.
- **D3 Order and place.** `seq` comes from one counter: the highest stored `seq` plus 1, read inside the store lock. A new card and a moved card both take the next `seq`, so each lands at the last place of its column (A4). The place of a card is its 1-based rank by `seq` among the cards with its status; the window computes it.
- **D4 Version and conflict.** `version` starts at 1 and rises by 1 with each kept change to that card. `board.move` first checks the card exists, then the own-column case (D5, nothing stored, no version check), then `expected_version`: if the stored version is higher, the answer is `Conflict` with the reason "the card changed in another window" and the current card. A move of another card does not touch this card's version. `board.create` carries a `request_key`: if a card with the key exists, the answer is the stored card with `repeated: true` whatever the new payload says, and nothing is stored again (0028/FR-005); the form makes a new key after an edit. Mutations run under the store mutex, so two changes are applied one at a time in arrival order (0028/FR-001).
- **D5 Own-column move.** A `board.move` to the card's own status stores nothing, ignores `expected_version`, and answers `changed: false`; the window says that the card is already there. A keyboard move past the first or last column is checked in the window and sends nothing.
- **D6 Requests.** `board.list` returns `{cards: [...]}` with `id, title, status, seq, version` and no note (200 titles of 200 characters stay near 160 KB); `board.get {id}` returns `{card}` with the note; `board.create {title, note, status, request_key}` returns `{card, repeated}` and `board.move {id, status, expected_version}` returns `{card, changed}`, both without the note. Errors: `Invalid` with `{field, rule}` entries (a title or note rule) or `{cap}`; `Conflict`; `NotFound` (the card is gone; only possible after 0029, kept for completeness); `Internal` with the disk-failure text. The event `board.card_changed {card}` is published after every kept change and carries the card without the note. A disk failure answers `Internal` and publishes `storage.error`.
- **D7 Text rules** (0027/FR-006): title trimmed of white space, at most 200 Unicode code points, not blank, no Cc characters, no zero-width or bidi format characters; note with CR and CRLF changed to LF, at most 8,192 code points, no Cc other than LF and tab. Checked in the domain; the application handler returns every failing field.
- **D8 Client state.** One `BoardController` in `D/modules/board/state/` holds `cards` (map by id; a received card replaces the stored one only if its `version` is not lower), `loaded`, `notice` (text), `announcement` (for the `Announcer`), and the pending changes. After a change the window shows the answer; a late event for the same card is ignored by the version check. The window names the version it last displayed for the card as `expected_version`. The controller sends one change per card at a time: a second move of a card that has a change in flight waits in a queue, and is sent with the version from the answer of the first (or is dropped if the first one was refused), so a window never conflicts with itself.
- **D9 Not confirmed and where it shows** (0028/FR-002, FR-004). The service races each change against a 2 second timer. On timeout or a transport error the controller drops the optimistic state (a moved card returns to its old column and place; a new card never appears before the answer), keeps the form text, sets the notice "not confirmed" with the reason, announces it, and calls `board.list` when the connection is back. The webview already reloads on connect; the board view adds `board.list` to that reload. A move is shown optimistically (so the card is not stuck waiting); a new card is shown only after the answer. The controller state lives in the panel store, and the notice is an app-level element in `app.svelte` outside the board view, so it survives the swap to the 0001 not-reachable view: while the connection is lost the 0001 view is what the person sees, and the notice shows when the app is connected again, with the card back where it was.
- **D10 Announcements** (0027/FR-010, 0028/FR-006). Only the response to a change that this window made announces: move "title, moved to Column, n of m"; new card "title, added to Column"; already there "title is already in Column" (or "already first column/last column" for a key move); refusal "change, reason". Events from other windows announce nothing. An app-level shared `Announcer` with `role="status"` in `app.svelte` speaks the board announcements. It is mounted only while the board view is open or a board notice is pending, so the existing specs that select the one `p.sr-only[aria-live='polite']` region stay unchanged, and an announcement made while the board view unmounts is still spoken. The app shortcuts (Ctrl+Shift+F-keys) are not active in the board view, as in the scene, because their listener lives in the panel view.
- **D11 View and menus.** The view is `D/modules/board/views/board-view.svelte`: a header with Back to the panel and New card; four `<section aria-labelledby>` columns with the name and count; cards as list items. A card is one `<li>` with a title button (opens details on Enter or click), an "Actions" disclosure button that lists Open, Move to previous column, Move to next column, and Move to each other column as buttons, and `draggable="true"` for the pointer drop on a column. Keys on a focused card: Alt+Left moves it to the previous column and Alt+Right to the next (the handler calls `preventDefault` so the web view does not navigate back); T7 checks these keys against the panel and terminal shortcuts and records the result. The disclosure is built from shared `Button`; no new shared component is added. The accessible name of a card is its title and its description is "Column, n of m". Plain existing classes only (A10).
- **D12 Entry and state.** The panel header gets a "Board" button and the office scene header gets one too; both call `openView = "board"`. The board view is never written as the last view (PB-7, A8): the code that stores the last view ignores `"board"`. Leaving the view returns to the panel.
- **D13 Detail dialog.** Card details use the shared `Dialog`; Escape closes it and focus returns to the card.
- **D14 Layout.** Columns are a CSS grid of four on wide screens and one column at 360 px; titles use `overflow-wrap: anywhere`; the list scrolls only vertically (FR-004).
- **D15 Mock core.** `mock-core.ts` gains `board.*` handlers that keep cards in memory, honor `expected_version`, `request_key`, the cap, the text rules, and push `board.card_changed` events; helpers can make the next change fail or hang, to cover not-confirmed.
- **D16 No agent change.** The agent allowlist stays as is; host test (g) proves the refusal and its record.
- **D17 Windows and Linux.** The Rust code is OS-neutral. Linux and CI are enough for proof (owner decision of 2026-11-28; macOS and Windows hand proof stays waived).
- **D18 Live-region proof on Linux.** The platform web view gives no screen reader event for live-region text (0027, A13), so SC-003 is proved by an e2e assertion on the live-region text and the QA report states the residual.

## Delivery

Phased delivery into one umbrella branch (chosen): the server (4 crates, a migration, a gateway, host tests) and the desktop (a module, a view, e2e) are each a full review and fail in different ways. The umbrella `feat/0027-board` merges to `main` only after phase 2, so 0027 and 0028 release together (0027 alone has no not-confirmed handling). The final umbrella PR repeats a diff that the two phase PRs already reviewed, so it may get a lighter review; the phase PRs carry the review.

**Phase 1: server.**
- Outcome: the board server module works over the socket and passes its host tests; nothing in the desktop app calls it yet.
- Scope: T1 to T5. Exclusions: any webview file; any change to the agent allowlist.
- Dependencies and order: T1, T2, T3, T4, T5 in this order; the schedule domain is touched only in T1 (the moved check).
- Acceptance and evidence: each task's acceptance; `just test-rust`, `just lint-rust`, `just boundaries` green; the host tests (a) to (j) listed in T4.
- Branch: `feat/0027-p1-server`, created from `origin/main` (22de3fe or newer) with `git branch --no-track`; PR base: `feat/0027-board`, which is created from the same `origin/main` commit. The baseline is the green `main`.

**Phase 2: desktop and QA.**
- Outcome: the Board view works in the app on the plain template, the e2e and the real run pass, the QA report is written, and both specs are shipped.
- Scope: T6 to T11. Exclusions: any server change except a fix that phase 1 missed (then it goes to a new commit in phase 1's branch first).
- Dependencies and order: T6 starts once phase 1 is in the umbrella; then T7, T8, T9, T10, T11. T10 starts only when phase 1 is in the umbrella.
- Acceptance and evidence: each task's acceptance; `just test-web`, `just e2e`, `just lint-web`, `just check` green; the QA report and its evidence folder.
- Branch: `feat/0027-p2-desktop`, created from the umbrella after phase 1 merged; PR base: `feat/0027-board`.

**Final PR:** `feat/0027-board` into `main`, after phase 2, with `just check` green on its head. A single PR is the fallback if the owner changes the shape.

## Commit strategy

`commits: per-task`. One commit per task, each with a Conventional Commit title and no tracker or process words, signed.

| Task | Files | Proposed message |
|---|---|---|
| T1 | `src/kernel` (moved check), `M/schedule/domain/src/validation.rs`, `M/board/domain/**` | `feat(board): add the board domain and share the invisible character check` |
| T2 | `M/board/application/**` | `feat(board): add the board handlers` |
| T3 | `M/board/infrastructure/**`, `H/bootstrap/migrations.rs`, `H/bootstrap/services.rs`, `src/apps/office-harnessd/Cargo.toml` | `feat(board): keep the board in the office database` |
| T4 | `M/board/presentation/**`, `H/server/board_requests.rs`, `H/server/connection.rs`, `H/server/server_state.rs`, `H/server/running_server.rs`, `H/../tests/board.rs` | `feat(board): serve the board requests and publish card changes` |
| T5 | `docs/ARCHITECTURE.md` | `docs(architecture): add the board module` |
| T6 | `D/modules/board/{models,services,state}/**`, `D/modules/board/index.ts` | `feat(desktop): add the board controller with safe changes` |
| T7 | `D/modules/board/{views,components}/**` | `feat(desktop): add the board view` |
| T8 | `D/app.svelte`, `D/modules/panel/**`, scene header files, `D/../e2e/support/mock-core.ts` | `feat(desktop): open the board from the panel and the scene` |
| T9 | `D/../e2e/board.spec.ts` | `test(desktop): cover the board in the browser` |
| T10 | `specs/changes/0027-task-board/qa-report.md` | `docs(specs): add the board QA report` |
| T11 | `specs/**` (living spec, archive moves) | `docs(specs): ship the task board specs` |

## Scope

In: specs 0027 and 0028 as approved. Out: everything in their Out of scope lists (0029, 0030, Undo, in-column order, last-change time, project filter, and every item there).

## Tasks

### Phase 1

**T1 Domain crate** (deps: none). Create `M/board/domain` with: `CardId`, `CardStatus` (four values, text form, order), `CardTitle` and `CardNote` value types that apply D7, the invisible-character check moved from `M/schedule/domain/src/validation.rs` to the kernel (the schedule domain uses it, and its tests stay unchanged), `Card` (id, title, note, status, seq, version), `BoardRule` (field and rule text), `CARD_CAP`, and the place function. Write the invalid-title table test first (blank, spaces, 201, control, only zero-width, bidi, exactly 200, emoji joined by ZWJ refused in a title) and the note table (8,193, escape, LF and tab kept, CRLF to LF).
- Acceptance: unit tests for SC-006 of 0027 inputs pass, the schedule tests pass unchanged; no panic path (`unwrap_used` denied); `just lint-rust` clean on the crate.

**T2 Application crate** (deps: T1). `M/board/application`: port `BoardStore` (`list`, `get`, `find_by_key`, `insert`, `update_status`, `count`, all returning `BoardApplicationError`), the error enum (`Invalid(Vec<BoardRule>)`, `CapReached{cap}`, `Conflict{current}`, `NotFound`, `Storage{message,is_disk_failure}`), and handlers `ListBoard` (no notes), `GetCard`, `CreateCard`, `MoveCard`. `CreateCard`: validate, repeated key returns the stored card, cap check, next seq, insert. `MoveCard`: not found, own status returns `changed:false` (no version check), version check, else next seq and version + 1; plus `GetCard` (with the note). All mutations hold one `BoardLock` (a mutex the host passes in) so order is arrival order.
- Acceptance: unit tests with a fake store cover create, repeat key (one card), cap at 200, stale version refused with the current card, move of another card leaves this version unchanged, own-column move with an old version answers no change, order of two moves into one column, a repeated key with a different payload returns the stored card.

**T3 Infrastructure crate** (deps: T2). `M/board/infrastructure`: `0001_board.sql`, `migrations/mod.rs` (`LEDGER_TABLE = "board_schema_version"`), `SqliteBoardStore` over `Arc<Mutex<Connection>>`, `storage_failure` for disk errors as in schedule. Register the ledger in `H/bootstrap/migrations.rs` and `services.rs`; add the crates to `src/apps/office-harnessd/Cargo.toml`.
- Acceptance: persistence tests (reopen the database and read 200 cards with 8,192-character notes with the same fields and order; unique key conflict; the ledger name and script count); `H/../tests/migrations.rs` updated and green; `just boundaries` clean.

**T4 Presentation and gateway** (deps: T3). `M/board/presentation` (requests `board.list`, `board.get`, `board.create`, `board.move`): `contracts/board_requests.rs` (the three kinds and the event name), `mappings/board_json.rs`, handlers with the error mapping of D6. In the host: `H/server/board_requests.rs` (`BoardGateway` with `accepts` and `run`, unknown-field refusal, publish after each kept change, disk failure to `storage.error`), the router arm in `connection.rs`, the field in `server_state.rs`, the build in `running_server.rs`.
- Acceptance: wire and handler tests; host tests in `H/../tests/board.rs` with real sockets: (a) two connections, a move in one reaches the other as an event within 2 s; (b) 20 trials of two connections moving the same card to two columns at once end with one kept move, one `Conflict`, the same card state for both; (c) a repeated `board.create` with one key keeps one card; (d) 201st card is refused with the cap; (e) five server restarts with 200 cards keep every field and order; (f) a client without a valid token gets nothing; (g) an agent connection's `board.list` is refused and the refusal is in the action record; (h) the database file mode is owner-only on Unix; (i) a store failure answers `Internal`, publishes `storage.error`, and leaves the board unchanged; (j) at the cap, 200 cards with 8,192-character notes: `board.list` answers within the frame limit and `board.get` answers one 8,192-character note over a real socket.

**T5 Architecture doc** (deps: T4). Update `docs/ARCHITECTURE.md`: the module list and count, the `board.` request prefix, the `board_cards` table and the ledger.
- Acceptance: link check clean; every claim matches the code on the branch.

### Phase 2

**T6 Webview model, service, controller** (deps: phase 1 in the umbrella). `D/modules/board/{models,services,state}` and `index.ts`: types, `createBoardService(client)` with a 2 second race (D9), `BoardController` with the rules of D8 to D10, and `board.list` on load and reconnect.
- Acceptance: `bun test` specs: two moves of one card at once send the second only after the first answer, with the new version, and cause no conflict; order and place by `seq`; ignoring an older version; a move shown at once and reverted on refusal or timeout with the old place; a new card not shown before the answer; announcements only for own changes; the form text kept; `svelte-check` and eslint clean.

**T7 Components and view** (deps: T6). `board-view.svelte`, `board-column.svelte`, `board-card.svelte`, `card-form.svelte` (title, note, status; makes `request_key` when opened, makes a new one after the title or note is edited following a failed save), `card-details.svelte` (loads the note with `board.get` when it opens), the action disclosure, pointer drag and drop (HTML5 events; a drop on the card's own column does nothing), the keys Alt+Left and Alt+Right (D11), focus kept on the moved card after the re-render, layout of D14.
- Acceptance: component specs for the focus rule and the own-column drop; no new shared UI element; axe clean on the view.

**T8 Wiring and mock core** (deps: T7). The app-level notice and `Announcer` in `app.svelte` (D9, D10), panel header "Board" button, scene "Board" button, `openView = "board"` in `app.svelte` and the panel store, the last-view writer ignoring `"board"`, the reload on connect, and the `board.*` handlers of D15 in `mock-core.ts`.
- Acceptance: existing panel and scene e2e specs pass unchanged; a new spec proves the app still opens the panel or scene after the board view was open (PB-7).

**T9 e2e** (deps: T8). `D/../e2e/board.spec.ts` on the mock core, with one test per row: open from panel and scene with 4 empty columns and no count while loading (SC-001); keyboard-only make, open and close details, move through all columns, already-there, pointer drop of 5 cards (SC-002); live-region text for 10 moves, 3 refusals, 0 announcements for a second page's change (SC-003, 0028/SC-004); 200 cards with 200-character titles and 8,192-character notes load within 3 s and 20 moves show in a second page within 2 s (SC-004); refused titles and notes, kept 200 and 8,192, markup and bidi text inert (SC-006); the cap message (SC-007); the server not reachable: 5 moves return within 2 s with the reason (0028/SC-001); two pages moving one card (0028/SC-002); two quick moves of one card in one page cause no conflict; keyboard moves with Alt+Left and Alt+Right (the 10 moves of SC-003); repeat save keeps one card (0028/SC-003); axe and 360 px and 200% zoom with no sideways scroll (SC-008, 0027/FR-004).
- Acceptance: all green on the plain Linux run; flaky-free in 10 repeats with the usual worker count.

**T10 Real run and QA report** (deps: T9 and phase 1 merged into the umbrella). Run the real server and the desktop app on Linux: make, move, and restart cards; five server restarts and one cold machine start with 200 cards (the machine start is by hand; if it is not run, the report marks it a gap); two real clients (the app and a socket client) for the race and the 2 second live update; and a real-app row: stop the real server with a move in flight, record what the person sees and hears (the 0001 not-reachable view, then the notice and its announcement when the app is connected again), and the card after the server restarts (0028/SC-001, FR-002, FR-004); and a line for Alt+Left and Alt+Right in the real app (the web view must not navigate back). Write `specs/changes/0027-task-board/qa-report.md` (it covers 0028 too) with one row per SC and PB of both specs, the commands, the evidence folder, the Linux live-region residual, and the waived macOS and Windows rows.
- Acceptance: every SC has a PASS with evidence or a stated GAP; none is marked PASS unrun.

**T11 Verify and ship the specs** (deps: T10). `/ac:verify` against both specs; then set both specs `shipped`, merge their requirements into `specs/task-board/SPEC.md` (ids with the change number as prefix), and move both change folders to `specs/changes/archive/`.
- Acceptance: the verify verdict is READY; the living spec holds every requirement of 0027 and 0028; link check clean.

## Traceability

| Requirement | Task(s) | Evidence |
|---|---|---|
| 0027/FR-001 board, 4 columns, view, open and leave | T7, T8 | e2e SC-001, component specs |
| 0027/FR-002 loading, counts, empty texts | T6, T7 | bun specs, e2e SC-001 |
| 0027/FR-003 inert plain text, direction | T7 | e2e SC-006 |
| 0027/FR-004 card order, wrap, no sideways scroll | T7 | e2e SC-008 |
| 0027/FR-005 card details | T7 | e2e SC-002, component spec |
| 0027/FR-006 create, rules, cap | T1, T2, T4, T7 | unit tables, host test (d), e2e SC-006, SC-007 |
| 0027/FR-007 move, focus, already-there | T2, T6, T7 | unit, bun, e2e SC-002 |
| 0027/FR-008 keep, owner-only, token | T3, T4 | persistence, host tests (e), (f), (h) |
| 0027/FR-009 live in every window | T4, T6 | host test (a), e2e SC-004 |
| 0027/FR-010 announce own changes | T6, T9 | bun, e2e SC-003 |
| 0028/FR-001 one at a time, stale refused | T2, T4 | unit, host test (b) |
| 0028/FR-002 not kept: revert, say why | T6, T9 | bun, e2e 0028/SC-001 |
| 0028/FR-003 keep typed text | T6, T7 | bun, e2e |
| 0028/FR-004 refresh when back | T6 | bun, T10 real run |
| 0028/FR-005 repeat save, one card | T2, T4, T7 | unit, host test (c), e2e 0028/SC-003 |
| 0028/FR-006 announce refusals | T6, T9 | bun, e2e |
| 0027/SC-001 to SC-008 | T9, T10 | e2e rows; SC-005 host test (e) and T10 |
| 0028/SC-001 to SC-005 | T4, T9, T10 | host tests, e2e, real run |
| 0027/PB-1, PB-2, PB-3 | T8, T9 | existing panel, needs-you, schedule specs unchanged; no terminal input in e2e |
| 0027/PB-4 and 0028/PB-3 agent request refused | T4 | host test (g) |
| 0027/PB-5 no data off the machine | T4 | no new network code; checked in review |
| 0027/PB-6 keyboard, text status | T7, T9 | e2e SC-002 |
| 0027/PB-7 last view | T8 | e2e |
| 0028/PB-1, PB-2, PB-4 | T8, T9 | e2e |

## QA and evidence

- Evidence for the real run goes in `qa/evidence/b-0027-board/` (outside the repo) and is quoted in the QA report.
- Rows that cannot run on Linux (macOS, Windows hand proof) are marked WAIVED by the owner decision, not PASS. The screen reader check on Linux is a stated residual (D18).

## Final verification

1. `just check` (sidecar, lint, tests, boundaries, web lint and tests, e2e, build) passes on the delivered umbrella tree.
2. The real run of T10 is done and its rows are in the QA report.
3. `/ac:verify` reports READY with a criterion table for every FR, SC, and PB of both specs.
4. Public-repo hygiene: Conventional Commit titles and PR bodies, no office words, no em dash or zero-width character in added text; every commit signed.
5. The specs ship (T11): both set to `shipped`, merged into `specs/task-board/SPEC.md`, and moved to the archive.

## Risks

- **Drag and drop on the platform web view:** HTML5 drag may behave differently in the Tauri web view. The action list and the keys are the full path (WCAG 2.5.7), so a drag problem does not block the keyboard and menu paths; T10 tries it in the real app and records the result.
- **Alt+Arrow keys:** the web view may treat them as history navigation or the system may use them; T7 checks and prevents the default, and the Actions list stays the fallback.
- **Lost server in the real app:** the notice and the announcement depend on the app-level placement of D9 and D10; the real-app row of T10 proves or disproves it.

## Owner decisions

None open. The delivery shape is chosen (phased, into the umbrella branch); the fallback is one PR.
