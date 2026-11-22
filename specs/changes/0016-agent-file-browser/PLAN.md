---
status: in_progress
spec: specs/changes/0016-agent-file-browser/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-23 with the spec, for one review of both. The spec review gate passed for coding at d9c12b8 (2026-11-23) with condition C1, applied in D5. T2 to T5 are built on this branch; deviations are in qa-report.md. -->

# Plan: Read-only file browser of an agent (0016)

## Summary

The desktop app gets a "Browse files" action in an agent's details. It opens a wide dialog with three tabs: Files (a tree and a read-only viewer), Changes (files that differ from HEAD and their diffs), and History (commits that touch the folder, then one commit's files and diffs). The server answers six new `agent.files.*` requests on the user connection. Every path is checked against the canonical root before any read, secret-looking files need an explicit reveal per request, content travels as base64 under a 512 KiB cap, and every git run is a read run with external programs switched off.

**What you get:**
- Six read-only requests in the agent module, routed through the existing agent gateway (contract `contracts/file-view-protocol.md`).
- A pure path and name rule set in the agent domain: path form, secret-looking names, binary test.
- A folder reader with canonical containment and a git reader on the office's hardened git runner.
- A desktop `files` module: dialog, tree, viewer, change list, diff view, history list, with keyboard access and the visible-symbol mapping of 0011.
- Rust unit and integration tests for SC-001 to SC-006 and SC-008 (server side), bun unit tests, and Playwright e2e with axe for SC-007 and SC-008 (UI side).

**Why this approach:** the agent module already owns the agent store, the project store (for the start folder), the git runner of 0010, and the `agent.*` request route, so no new module, crate, or dependency is needed. The desktop already has `visibleText` (0011), the shared `Dialog`, and the mock core for e2e.

**What it will not do:** writing of any kind, branch compare, blame, search, a live view, rendered previews, remote machines, agent access (spec Out of scope).

**Shape of the work:** 6 tasks, one PR, one commit per task. **Effort:** M.

**Main risks:**
- **Reading outside the root.** One function, `contained_path`, does every check; the reader never opens a path it did not return. Tests cover each escape of SC-001 on Unix, and the Windows forms in unit tests.
- **git running a program or writing.** The flags of D6 and a test repository that names a hook, an external diff, a textconv, a clean filter, a file system monitor, and a signature program, each writing a marker; SC-004 checks the markers and a full before and after snapshot of the folder and its git data.
- **Frame size.** 512 KiB raw is about 683 KiB in base64, under the 1 MiB frame; lists stop at a 768 KiB budget (D8).

**Decisions made for you:** request names and shapes, the limits (spec A10), base64 transport, the secret list, the UI layout, and the reuse of the 0010 git runner.

**Blockers:** none. The spec review gate passes first. Each local build or test run follows the floor's build rules.

## Goal

Deliver spec 0016: a read-only view of one agent's folder, its changes against HEAD, and its commit history, that never reads outside the root, never writes, and hides secret-looking files until a reveal.

## Context

Paths: `H` = `src/apps/office-harnessd`, `M` = `src/modules`, `D` = `src/apps/desktop`. Read on `main` at c07f21c.

- **Agent route:** `M/agent/presentation/src/handlers/agent_handlers.rs`, `route` (line 186) maps `agent.*` types; handlers parse with `Body::read(&request.fields)` and return `Success::result`. `AgentGateway` (`H/src/server/agent_requests.rs:121`) accepts `agent.` and runs handlers on the blocking pool.
- **Agent connection:** `H/src/server/agent_call.rs` `ALLOWED_REQUESTS` holds 7 types; anything else is refused and recorded. No change.
- **Start folder:** `SessionLauncher::existing_working_folder` (`M/agent/application/src/common/session_launcher.rs:381`): the working folder, else the project's folder.
- **Lookup:** `require_agent` (`M/agent/application/src/common/lookup.rs:12`) and `require_project`.
- **git runner:** `M/agent/infrastructure/src/services/git_runner.rs`. Read runs clear `GIT_*`, set `core.hooksPath` to the null device, turn off hook events and `core.fsmonitor`, set `GIT_OPTIONAL_LOCKS=0`, refuse every transport, time out at 10 s, cap stdout at 1 MiB, and return lossy text. `GitWorktrees` (`git_worktrees.rs:67`) finds git on the login `PATH` once (`RunnerSource`, `OnceLock`).
- **Frames:** `MAX_FRAME_BYTES` = 1 MiB (`src/shared/wire/src/protocol.rs:20`), on both the server and the desktop core.
- **Desktop:** `D/src/modules/agent/components/agent-details.svelte` (action row of buttons; callbacks such as `onsaveastemplate` handled in `D/src/modules/panel/views/panel-view.svelte:541`); `D/src/shared/ui/dialog.svelte` (modal, Escape, focus return; fixed 32rem width); `visibleText` in `D/src/modules/agent/models/task-text.ts:58`, exported from the agent module; `CoreClient.sendRequest`; e2e mock core `D/e2e/support/mock-core.ts` answers by request type.

## Decisions

- **D1 Requests:** `agent.files.list`, `agent.files.read`, `agent.files.changes`, `agent.files.diff`, `agent.files.history`, `agent.files.commit`, shapes in the contract. They go through the agent route, so the user connection gets them and the agent connection does not (its allow list is unchanged, PB-1).
- **D2 Layers:** domain `M/agent/domain/src/services/file_rules.rs` (pure: `RelativePath::parse`, `is_secret_looking(name)`, `is_binary(prefix)`, the limits as named constants). Application: port `FileView` in `common/interfaces/file_view.rs`, answer models in `common/models/file_view.rs`, and `queries/file_view/file_view_handler.rs`, which finds the agent, resolves its start folder by the rule of `existing_working_folder` (shared as one function, not copied), and calls the port. Infrastructure: `services/folder_reader.rs` (containment, listing, reading) and `services/git_reader.rs` (changes, diff, history, commit), joined as `LocalFileView`. Presentation: six routes and `mappings/file_view.rs` (JSON, base64, budget).
- **D3 Containment (`contained_path`):** canonicalize the root once per request; parse the relative path (D2); join; canonicalize the deepest existing ancestor and append the missing parts (which cannot be links); require `starts_with(root)` on the canonical form; refuse any part named `.git` (ASCII case-insensitive) in the result. The reader opens only the returned canonical path, with `O_NONBLOCK` on Unix (`OpenOptionsExt::custom_flags`) so that a FIFO swapped in after the check cannot block the open, checks `metadata().is_file()` from the opened handle, and reads at most 512 KiB + 1 bytes (`take`), so a file that grows after the check is still capped. A link entry's `target` comes from the same function. Residual race and hard links: spec Residual risk.
- **D4 Listing:** `read_dir` of the contained folder; `symlink_metadata` per entry for its kind; a link's target classified through D3; `.git` skipped; sort folders first, then by name (code point); first 1,000, `more` = the rest. A name that is not UTF-8 is listed lossy (contract Paths).
- **D5 Secret rule:** `is_secret_looking` holds the spec's list exactly, lower-cased once. `read` and `diff` check the requested name and the resolved name (a link named `notes` to `.env` counts) before any byte is read, and answer `secret` without opening the file unless `reveal` is true. `diff` applies the same check to `old_path`: a rename with either side secret-looking is `secret` (review r1 C1). `old_path` goes through `RelativePath::parse` and, for a diff against HEAD, through `contained_path`, exactly as `path`.
- **D6 git runs:** a new `GitRunner::read_bytes(folder, args, stdout_limit)` returns raw bytes with a caller-chosen limit; `read` stays as is (refactored over it). Every file view run adds `--literal-pathspecs`, `-c log.showSignature=false`, and for each driver name from `git config --get-regexp --name-only ^filter\.` (itself a read run) `-c filter.<name>.clean= -c filter.<name>.smudge= -c filter.<name>.process= -c filter.<name>.required=false`. Commands:
  - prefix: `rev-parse --show-prefix` (exit failure with "not a git repository" means `repository: false`).
  - changes: `status --porcelain=v2 -z --untracked-files=all --ignore-submodules=dirty -- .`, stdout limit 8 MiB; paths have the prefix removed; renames whose old path lies outside the prefix lose `old_path`.
  - diff against HEAD: `diff --no-ext-diff --no-textconv --no-color -M --relative HEAD -- <path> [<old_path>]`; untracked files (in `status --porcelain=v2 -z --untracked-files=all -- <path>`) get a diff built from the file bytes through D3, so git never reads them; a repository without HEAD diffs against the empty tree (`hash-object -t tree --stdin` with no input gives its id for SHA-1 and SHA-256).
  - history: `log --no-show-signature --no-decorate --no-color --format=%H%x1f%h%x1f%an%x1f%at%x1f%s%x1e --skip=<n> -n 51 HEAD -- .`; 51 tells `more`.
  - commit: `rev-parse --verify --quiet <id>^{commit}` after the hex check; `show -s --no-show-signature --format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B <id>`; files `diff-tree --no-ext-diff --no-textconv -r -z -M --relative --name-status <parent> <id> -- .` (`--root <id>` without a parent); one file's diff `diff --no-ext-diff --no-textconv --no-color -M --relative <parent> <id> -- <path> [<old_path>]`.
  - The diff text from the first `@@` on is sent; `Binary files ... differ` gives `binary`; output over 512 KiB, or `OutputTooLarge` at the runner's 1 MiB limit, gives `too_large`.
- **D7 No writes:** all runs use the read mode of the runner (no optional locks, no transport). The runner's `-c core.fsmonitor=false` is the control for the file system monitor of SC-004; `core.hooksPath` to the null device and the disabled hook events are the control for its hook. Nothing in the reader opens a file for writing. A test snapshots the folder and the git data (path, bytes, modification time) before and after a full session (SC-004).
- **D8 Answer budget:** the mapping adds list items while the running JSON size stays under 768 KiB and counts the rest in `more`. `read` and `diff` encode with the standard base64 alphabet; the encoder is a small function in the mapping (no new dependency) with unit tests against known vectors.
- **D9 git runner sharing:** the `RunnerSource` and `OnceLock` of `GitWorktrees` move into `LazyGitRunner` in `git_runner.rs`; `GitWorktrees` and `LocalFileView` share one `Arc<LazyGitRunner>`, built in `H/src/bootstrap/terminal_services.rs`. Without git, the git requests answer `internal` "git is not available" and `agent.files.list` and `agent.files.read` still work.
- **D10 Desktop module:** new `D/src/modules/files/` with `models/file-view.ts` (types, base64 to UTF-8 with replacement, unified diff parsing into numbered lines, `visibleLine` that keeps tabs and maps other control and bidirectional characters through `visibleText`), `services/file-view-service.ts` (the six requests), `state/file-view.svelte.ts` (per tab: phase `loading`, `ready`, `empty`, `failed`; `read_at`; reveal state cleared on every open, tab switch, and refresh), and components `file-view-dialog.svelte`, `file-tree.svelte`, `file-content.svelte`, `change-list.svelte`, `diff-view.svelte`, `history-list.svelte`. The panel view owns `filesAgentId` and renders the dialog; `agent-details.svelte` gets `onbrowsefiles` and a secondary "Browse files" button.
- **D11 Dialog width:** `Dialog` gains an optional `size` prop, `"normal"` (today's width) or `"wide"` (`min(72rem, 100vw - 2rem)`, full height minus margins). Other dialogs are unchanged (PB-5).
- **D12 Keyboard and screen reader:** tabs follow the WAI-ARIA tabs pattern (Left and Right move, Home and End); the tree follows the tree pattern (`role="tree"`, `treeitem`, `aria-expanded`, Up, Down, Left, Right, Home, End, Enter); lists are buttons in order; the content and diff areas are focusable regions with `tabindex="0"` so they scroll by keyboard; loading uses `role="status"` and failures `role="alert"`; diff lines carry a sign column and a visually hidden "added" or "removed" text. Content shows in `<pre>` text nodes only: no `{@html}`, no images, no links.
- **D13 Text safety:** names, paths, content, diffs, and commit texts go through `visibleLine` before display (FR-009); a line feed splits lines and a trailing carriage return is dropped from the line, not shown.
- **D14 Constitution check:** AGPL header on new files; no new dependency on either side; module boundaries kept (the files module imports the agent module only through its index; the panel composes them); no code from any other project.

## Scope

- **In:** D1 to D14, tests, README section, `qa-report.md`, the spec report page.
- **Out:** the spec's Out of scope list.

## Tasks

### T1: Spec, contract, plan (no product code)

SPEC.md, `checklists/requirements.md`, `contracts/file-view-protocol.md`, this plan. **Accept:** the spec review gate passes; no product file changes.

### T2: Rules and containment (D2 domain, D3, D4, D5)

`file_rules.rs` with unit tests: every refused path form of the contract (also the Windows forms behind `cfg`), every name of the secret list in mixed case plus near misses (`environment.txt`, `keys.md`), the binary test at 7,999 and 8,000 bytes. `folder_reader.rs` with tests on a temp folder: listing order, `.git` hidden, 1,001 entries give `more` 1, link targets (`folder`, `file`, `outside`, `broken` for a missing target and a loop), the SC-001 escapes (parent parts, absolute, link to a file outside, link to a folder outside, a link inside a linked folder, a chain that leaves), `.git` reached through a link, a FIFO listed as `other` and refused on read without blocking, size cap at 524,288 and 524,289 bytes, `secret` before any read. **Accept:** `cargo test -p agent-domain -p agent-infrastructure` pass; clippy clean.

### T3: git reader (D6, D7, D9)

`LazyGitRunner`, `read_bytes`, `git_reader.rs`. Tests on temp repositories (skipped when git is missing, as the 0010 tests do): the SC-005 set with an outside change; a staged rename of `.env` to `notes.txt` and the same rename in a commit, both `secret` without reveal (review r1 C1); a subfolder root; no HEAD; a staged rename; an untracked file diff built from bytes; a binary change; a diff over 512 KiB; history of 120 commits in the root and 10 outside (SC-006) with `skip`; a merge commit against its first parent; a root commit; commit id forms refused (`HEAD`, a short id, `--output=x`, a tree id); the SC-004 marker repository (hook, `diff.external`, a textconv driver, a clean filter marked required, `core.fsmonitor`, `log.showSignature` with `gpg.program`) and a before and after snapshot. **Accept:** the tests pass three times in a row; the existing `git_worktrees` tests stay green.

### T4: Requests (D1, D2 application and presentation, D8)

Port, models, handler, routes, mappings, wiring. Unit tests: base64 vectors; the budget cut with names full of control characters. Integration test `H/tests/file_view.rs` through the real server and a user client: each request's happy path and errors (`not_found`, `invalid`, `refused`), SC-001 over the wire with 0 outside bytes in any answer, SC-003 (10 secret-looking changed files: 0 content bytes without reveal, exactly one with), the agent connection refusing `agent.files.list` and recording it (PB-1), the action record unchanged (PB-4), SC-009 timings on a generated folder. **Accept:** `cargo test -p office-harnessd --test file_view` passes three times in a row; `agent_call`, `agent_directive`, `agent_subtree_status` stay green.

### T5: Desktop (D10 to D13)

The files module, the dialog size, the agent details button, the panel wiring, and mock-core answers for the six requests. bun tests: base64 decoding with invalid UTF-8, the diff parser (line numbers, `\ No newline at end of file`, hunk headers), `visibleLine` (tab kept, ESC, DEL, C1, U+202E, CR at line end). Playwright with axe on the mock core: open from details by keyboard; tree navigation; each Visible states row (SC-007); reveal and hide again (SC-003, UI side); a file and a commit message with `<script>`, `<img>`, ESC, and U+202E (SC-008: no script runs, no image request, visible symbols); the diff signs and hidden text; Escape closes and focus returns; 200% zoom without sideways page scroll. **Accept:** `bun test`, `svelte-check`, `eslint`, `prettier --check`, boundaries, and the new Playwright file pass.

### T6: Docs and QA

README: a short "Browse an agent's files" section (what it shows, the limits, the reveal, read-only). `qa-report.md` with a row per FR, SC, and PB and its evidence. The spec report page under `.agents/work/spec-reports/` (outside the repo) and its index row. SPEC status after merge is the owner's call. **Accept:** every row has evidence or a stated gap; the diff names no other project.

## Dependencies

T2 before T3 before T4; T5 needs only the contract, so it can run beside T3 and T4; T6 last. T1 is the spec PR's first commit.

## Commit strategy

`per-task`: one signed commit for each of T1 to T6 on `feat/0016-agent-file-browser`, one PR to `main`.

## QA and evidence

Local gates per run: `cargo fmt --check`, clippy on the touched crates, the tests of T2 to T4, `bun test`, `svelte-check`, `eslint`, `prettier --check`, the boundaries checks, and the T5 Playwright file. CI covers three OSes. Each local run follows the floor's build rules (memory and job count checked first, `-j 4`).

## Final verification

All tasks accepted; CI green on three OSes; `qa-report.md` complete; the diff names no other project; the spec review gate's verdict at the final head.

## Traceability

| Criterion | Task | Evidence |
|---|---|---|
| FR-001 | T2, T4, T5 | listing tests; e2e tree |
| FR-002 | T2, T4, T5 | read tests; e2e states |
| FR-003 | T2, T4 | containment tests; SC-001 over the wire |
| FR-004 | T2, T4, T5 | secret tests; SC-003 both sides |
| FR-005 | T3, T4, T5 | SC-005 repository; e2e changes |
| FR-006 | T3, T4, T5 | SC-006 repository; e2e history |
| FR-007 | T3 | SC-004 marker repository and snapshot |
| FR-008 | T5 | e2e Visible states (SC-007) |
| FR-009 | T5 | bun `visibleLine`; e2e SC-008 |
| SC-001 to SC-006 | T2 to T4 | Rust tests named above |
| SC-007, SC-008 | T5 | Playwright with axe |
| SC-009 | T4 | timing test on a generated folder |
| PB-1 | T4 | agent connection refusal test |
| PB-2 | T3 | SC-004 snapshot |
| PB-3 | T4 | the requests sit behind the authenticated user connection (existing auth tests) |
| PB-4 | T4 | action record and status unchanged |
| PB-5 | T5 | existing Playwright files stay green |
