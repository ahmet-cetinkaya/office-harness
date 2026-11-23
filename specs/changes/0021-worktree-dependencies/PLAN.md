---
status: draft
spec: specs/changes/0021-worktree-dependencies/SPEC.md
commits: per-task
---

# Plan: Installed dependencies in a new worktree (0021)

## Summary

When the office makes an agent's own worktree (0010), it gives the worktree the installed Node.js packages of the base checkout. For each place that has a `node_modules` folder in the base checkout (the repository top and each folder with a tracked `package.json`), the office makes a real, empty `node_modules` folder at the same place in the worktree and puts one symbolic link per entry in it, each pointing at the same entry in the base checkout. Git ignores the real folder by the repository's own rule, so nothing of it is ever an uncommitted change. Before the office removes a worktree or undoes a create, it deletes those links without following them. A new office setting `worktree_dependency_links` (`on` by default) turns the step off. On Windows the office makes no links.

**What you get:**
- `GitWorktrees` links dependencies after a successful `worktree add` (FR-002 to FR-004, FR-007), and unlinks them before every `worktree remove` and in the undo of a create (FR-006).
- The setting `worktree_dependency_links` (`on` or `off`, FR-001), read by `CreateAgentHandler` at each create and passed in `WorktreeCreate`.
- `agent.worktree_info` answers `shares_dependencies` (setting on and not Windows), so the create form note says whether packages are shared (FR-008).
- The desktop "Worktree dependencies" dialog beside "Idle sleep", and the changed own-worktree note of the create form.

**Why this approach:**
- A link in place of the whole `node_modules` folder is not a folder for git, so the common rule `node_modules/` does not match it: `git status` lists it as untracked, the delete question would count it, and an agent could commit it. A real folder matches both `node_modules` and `node_modules/`, and git never looks inside an ignored folder, so no exclude entry and no change to the repository's configuration is needed (0010 PB-2).
- Per-entry links, limited to package entries, keep tool caches (for example `node_modules/.vite`) in the worktree's own folder, so concurrent builds never write one shared cache.
- The links are found again from the worktree itself (a link inside a dependency folder whose target is the same entry in the base checkout), so the agent row stores nothing new and no migration is needed.

**What it will not do:** link other ecosystems, junctions on Windows, refresh links after the base checkout installs more packages, or touch existing worktrees (spec Out of scope).

**Shape of the work:** 4 tasks in one PR (`feat/0021-worktree-dependencies`), one commit per task. **Effort:** S to M.

## Goal

Deliver spec 0021: an agent in a new own worktree can build and test with the base checkout's installed Node.js packages, the links are never work of the agent, the office never changes the base checkout, and the person can turn the step off.

## Context

Abbreviations: `M/` = `src/modules`, `H/` = `src/apps/office-harnessd/src`, `D/` = `src/apps/desktop/src`. Read at `main` 7d1e85d on 2026-11-23.

- **Worktrees:** `GitWorktrees` (`M/agent/infrastructure/src/services/git_worktrees.rs`): `make` runs `branch` then `worktree add`; `remove_record` runs `worktree remove` with 0, 1, or 2 `--force`; `discard` is the undo of a create; `status` counts `status --porcelain=v1 --untracked-files=all`. Port `WorktreeManager` and `WorktreeCreate` in `M/agent/application/src/common/interfaces/worktree_manager.rs`. `is_linked_worktree` in `M/agent/application/src/common/worktree_link.rs`.
- **Create:** `CreateAgentHandler::make_worktree` (`M/agent/application/src/commands/create_agent/create_agent_handler.rs:192`) builds the `WorktreeCreate`. Template creates (0010 FR-009) reach the same handler.
- **Settings:** `Settings` and `SettingKey` (`M/agent/domain/src/value_objects/settings.rs`), key and value rows; `wiring.settings` (`Arc<dyn SettingsStore>`) in `H/bootstrap/agent_services.rs`.
- **Failure log:** `FailureReporter::report` with `PortError::Storage { is_disk_failure: false }` writes to the server log (`H/bootstrap/storage_alerts.rs:71`); `GitWorktrees` already holds one.
- **Desktop:** the idle sleep dialog (`D/modules/sleep/components/idle-sleep-dialog.svelte`, model, service, controller) is the pattern for one office setting; the toolbar button sits in `D/modules/panel/views/panel-view.svelte:441`. The own-worktree note is in `D/modules/agent/components/worktree-field.svelte:95`. The e2e mock answers `settings.get` and `settings.set` in `D/../e2e/support/mock-core.ts:1613`.

## Decisions

- **D1 Link shape.** At each place, `<worktree>/<place>/node_modules` is a real folder that the office makes with `create_dir` (never `create_dir_all`: the parent must already exist in the checkout). Inside it, one symbolic link per package entry of `<repository>/<place>/node_modules` (spec Terms): a name without a leading dot, or one of `.bin`, `.pnpm`, `.modules.yaml`, `.package-lock.json`, `.yarn-state.yml`, `.bun`. Each link has the absolute target `<repository>/<place>/node_modules/<entry>`. Tool caches such as `.vite` and `.cache` get no link, so each worktree's tools write their own cache (review r1 M1).
- **D2 Places.** `git ls-files -z -- ':(glob)**/package.json'` in the new worktree (reads the index; runs no filter or hook). A place is the folder of each path, plus the top. A place with a `node_modules` part is dropped (a package inside a tracked dependency folder). The list is deduplicated and sorted.
- **D3 Checks per place** (FR-003), in this order; a failed check skips the place and changes nothing:
  1. `<worktree>/<place>/node_modules` does not exist, not even as a broken link (`symlink_metadata` is `NotFound`) (f).
  2. `<repository>/<place>/node_modules` exists (`symlink_metadata`) (c).
  3. `git check-ignore -q -- <place>/node_modules` in the repository ignores it (exit 0; any failure reads as not ignored) (d).
  4. Its canonical path is inside the canonical repository folder; otherwise the refusal is logged (FR-004). Then it must be a folder.
  5. After `create_dir`, the same `check-ignore` in the worktree ignores it; otherwise the folder is removed and the place is skipped (e).
- **D4 Refuse outside targets.** An entry whose canonical path is outside the canonical repository folder gets no link (for example a package made with a link to another folder). Other entries still get theirs.
- **D5 Failure and refusal log.** An I/O error at a place removes the links already made there and the folder, and reports `the dependency links of <place>/node_modules were not made: <error>` (FR-007). A refused link reports `the dependency link <path> was not made: it leads outside the base checkout or to nothing` (FR-004, A11). Both go through the `FailureReporter` as a storage error that is not a disk failure, which the server writes to its log only. The create never fails because of a link.
- **D6 Recognizing an office link.** A link inside `<worktree>/<place>/node_modules` whose `read_link` is exactly `<repository>/<place>/node_modules/<entry>`. Anything else (a link the agent made, a real file or folder) is not the office's link.
- **D7 Unlink before removal.** `remove_record` (every force level) and `discard` first call `unlink_dependencies`: it requires `is_owned(folder)` and `is_linked_worktree(folder, repository)` (else it does nothing: git would read another repository), finds the places as D2, removes each office link with `remove_file` (removes the link, never its target), and then `remove_dir` on the folder, which fails harmlessly when the agent put real files there; those are ignored content that the 0010 removal deletes (0010 A9). A place that D2 no longer finds (the agent deleted its `package.json`) keeps its links until git removes the worktree; git's removal and `remove_dir_all` delete a link without following it, so the base checkout is still safe.
- **D8 Not before the status read.** `status` stays as it is: the real folder is ignored, so the links are never counted, and the agent keeps its packages while the delete question is open.
- **D9 Windows.** The link step is `#[cfg(unix)]`; on Windows `link_dependencies` returns at once. `unlink_dependencies` is the same on every platform and finds nothing on Windows.
- **D10 Setting.** `SettingKey::WorktreeDependencyLinks` = `worktree_dependency_links`, values `on` and `off`, default `on`. Another value is refused with `worktree_dependency_links is on or off` (new `AgentError::InvalidSwitch { field }`, mapped to `invalid` like the other setting errors). A change is not recorded in the action record (spec A, as 0018 A11).
- **D11 Who reads the setting.** `CreateAgentHandler::with_settings` and `WorktreeInfoHandler::with_settings` take the settings store (bootstrap wires both; without one they use the default settings). `make_worktree` sets `WorktreeCreate::link_dependencies`; the info handler sets `WorktreeInfo::shares_dependencies` to the setting and `DEPENDENCY_LINKS_SUPPORTED` (`cfg!(unix)`, one constant for the server). A failed settings load fails the request with the storage error, as other handlers do.
- **D12 Desktop.** Module `D/modules/worktree-dependencies` (model, service, controller, dialog), the same shape as the idle sleep module: one `role="switch"` checkbox labelled "Worktree dependencies" with the FR-001 help text linked by `aria-describedby`, Save and Cancel, the loading, load-failed, and not-saved states. The toolbar button "Worktree dependencies" sits beside "Idle sleep". The own-worktree note keeps the 0010 sentence and adds, from `shares_dependencies`: "Installed Node.js packages are shared from this checkout; an install, update, or removal of packages in the worktree changes them for the checkout too." or "Installed packages are not shared, so the agent installs them." (FR-008).

## Tasks

### T1: Setting and links in the server (commit 1)

- `settings.rs`: key, default, `value_of`, `set_from_text`, `InvalidSwitch`; tests for default, on, off, refusal, stored-row read.
- `worktree_manager.rs`: `WorktreeCreate::link_dependencies`, `WorktreeInfo::shares_dependencies`, `DEPENDENCY_LINKS_SUPPORTED`.
- `create_agent_handler.rs`, `worktree_info_handler.rs`: settings store (D11); the `agent.worktree_info` mapping adds `shares_dependencies`; bootstrap wiring.
- `dependency_links.rs` (new): places (D2), checks (D3, D4), link and unlink (D1, D6, D7), log reasons (D5); `git_worktrees.rs` calls `link_dependencies` after the subfolder check in `make` and `unlink_dependencies` at the start of `remove_record` (which `discard` runs).
- `git_worktrees_dependencies.tests.rs` (real git in a temporary folder, Unix only where links are made): top and nested places linked for the rules `node_modules/` and `node_modules`; `.bin` linked, the tool caches `.vite` and `.cache` not linked; `.vite` and `.cache` not linked, `.bin` linked; `git status` empty and `status().changed_files == 0`; `git add --all` stages nothing; setting off; base folder not ignored; tracked file at the place; base folder and single packages that lead outside or to nothing are refused and logged; an unreadable base folder is logged and the create goes on; remove and discard leave the base packages unchanged; an agent's own link to an outside folder is never followed; Windows makes no link (`cfg(windows)` test).
- **Acceptance:** `cargo test -p agent-infrastructure -p agent-application -p agent-domain` and clippy clean.

### T2: Wire test (commit 2)

- `H/../tests/agent_worktree.rs`: a repository with an ignored, installed `node_modules` at the top and in `pkg/`; `agent.create` with own worktree: the worktree has both places linked, a Node-free check reads a file through each link, `worktree status` counts 0, `agent.delete` with remove succeeds without a second confirmation and the base files are unchanged. `settings.set worktree_dependency_links off`: the next create makes no folder. An invalid value is refused with the rule.
- **Acceptance:** the test file passes on Linux and macOS; on Windows the create test asserts that no link is made.

### T3: Desktop (commit 3)

- Module from D12, button in the panel toolbar, note sentence in `worktree-field.svelte`, `mock-core.ts` answers the key (default `on`, refuses other values).
- Unit tests for the model and controller; e2e `worktree-dependencies.spec.ts`: open, default on, turn off and save sends `off`, reopen shows off, a refusal shows the reason and keeps the dialog, Escape and Cancel close without saving, keyboard only, axe has no serious violation, the create form note shows the sentence.
- **Acceptance:** `bun run check`, `bun run lint`, `bun run format:check`, `bun test`, and the Playwright suite on a private port.

### T4: Docs and QA (commit 4)

- README "Own worktree per agent": a short paragraph on shared packages, the setting, and Windows.
- `specs/changes/0021-worktree-dependencies/qa-report.md` with one row per FR and SC, and the hands-on rows: a real agent in a worktree of this repository runs `bun test` in the desktop folder without an install, and a Vite build in that worktree leaves the checkout's `node_modules/.vite` unchanged (review r1 N1).
- SPEC Deviations for anything shipped differently.
- **Acceptance:** links in the docs resolve; no other product is named; no zero-width character in added lines.

## Verification

- Server: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (with `CARGO_BUILD_JOBS=4`, `--test-threads=4`).
- Desktop: the T3 commands.
- CI green on Linux, macOS, and Windows.

## Rollback

Revert the PR. No stored data changes shape: the setting is one more key row, which an older server ignores. Worktrees made with links keep real `node_modules` folders with links; an older server removes them through git, which deletes links without following them.

## Risks

- An install in the worktree writes through a link into the base checkout's package. The help text and the note say so, and the setting turns linking off (spec A).
- A repository whose base checkout has thousands of packages gets thousands of links per create; each is one system call, well under the time of `worktree add`.
