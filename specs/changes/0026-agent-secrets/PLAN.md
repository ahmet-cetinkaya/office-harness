---
status: approved
spec: specs/changes/0026-agent-secrets/SPEC.md
commits: per-task
---

# Plan: Secrets that reach running agents (0026)

## Summary

After this change, the person saves a secret in a Secrets dialog, grants it to agents, and a running agent reads it through `office-harnessd agent-call secret.get`. The office seals the value with a key that the OS keyring or an owner-only file protects, and replaces any stored occurrence of a value in the text it stores or sends.

**What you get:**
- **A `secret` module** (domain, application, infrastructure, presentation) that owns names, values, sealing, grants, and the store key.
- **Two key custodies.** `os_keyring` and `key_file`, decided once at the first save, shown as text.
- **Two agent requests** (`secret.available`, `secret.get`) on the agent connection, and six user requests.
- **One redaction function** that every storing point calls.
- **A Secrets dialog** in the desktop app, keyboard complete.

**Why this approach:**
- Sealing in its own module keeps the value type, the cipher, and the key custody in one place, so the rule "no value in a log or a debug print" is a property of one type, not a review habit.
- One redaction function at the storing points (output store, action record, notifications, events) is smaller and safer than scanning each caller's text.
- Grants and sealed records live in the office database next to the agents. A delete cascades with `foreign_keys` on, and `secure_delete` (0023) covers the freed bytes.

**Delivery:** one PR with the spec, 6 tasks (T1 to T6), committed per task.

## Context

- The agent connection allowlist: `src/apps/office-harnessd/src/server/agent_call.rs:30-42`; `report.send` is the model for an agent-only request (`:120`).
- Limits and settings keys: `src/modules/agent/domain/src/value_objects/settings.rs:130`, `:185`, `:215`; the bridge that reads them: `src/apps/office-harnessd/src/bridges/rate_limit_source.rs`.
- Action kinds: `src/modules/audit/domain/src/action_kind.rs:10`.
- Output store: `src/modules/terminal/infrastructure/src/services/output_store.rs:64`. Server log rule: `src/apps/office-harnessd/src/bootstrap/server_log.rs:11`. Connection pragmas, `secure_delete` on: `src/apps/office-harnessd/src/bootstrap/database.rs:12-15`.
- Private files: `src/shared/private-fs/src/lib.rs` (`open_private_file`, `replace_private_file`, `create_private_dir_all`).
- Module layout follows `docs/ARCHITECTURE.md`; `scripts/check-boundaries.sh` derives rules from manifest paths, so a new module needs no script change.
- Migrations are per module. The `secret` module starts at `0001`.
- Desktop modules sit under `src/apps/desktop/src/modules/<name>/` with `components`, `models`, `services`, `state`; the limits dialog is the pattern for the new dialog.

## Decisions

- **D1: Crates.** Research and pin in T1, each with an exact version, as the workspace does. Candidates: an AEAD from RustCrypto (`chacha20poly1305`, no hardware requirement, same result on the 3 OSes), `zeroize` for buffers, and `keyring` for the OS keyring (Keychain, Credential Manager, Secret Service). T1 checks the licence is compatible with AGPL-3.0-or-later, the Linux build needs (a D-Bus library at build time is a CI cost; prefer a pure Rust Secret Service client when one exists), and records the choice in the PR. The keyring access sits behind a small trait (`StoreKeyVault`) so tests use a fake and the key-file custody is a second implementation.
- **D2: Sealing.** XChaCha-style large nonce or random 96-bit nonce is T1's choice; either is a fresh random nonce per seal. Associated data is the secret's random id, its name, and its version (spec contract, review L1); a replace seals again with the new version. The sealed record is `nonce || ciphertext_with_tag`.
- **D3: Store key.** 32 random bytes from `getrandom` (already a workspace dependency). Created at the first `secret.set`. Custody tried in order: keyring (a write then a read-back to prove it works), else key file `secrets.key` in the private data folder via `replace_private_file`. A stored custody mark in the module's table (`store_meta`) records the choice, so a later start never picks again (FR-003). With the mark `os_keyring` and no entry, the state is `locked` with `the keyring entry is missing` (FR-019); nothing is recreated.
- **D4: Tables** (module migration `0001_secrets.sql`): `secret(id TEXT PK, name TEXT UNIQUE, version INTEGER, sealed BLOB, created_at, updated_at)`, `secret_grant(secret_id, agent_id, last_read_version, last_read_at, PK(secret_id, agent_id))` with `ON DELETE CASCADE` on the secret, and `store_meta(custody TEXT)`. The agent delete removes grants by an agent-side hook in the delete handler (no cross-module foreign key: the boundary rule forbids the dependency, so the grant removal is a call through an application port that the server wires).
- **D5: Reads.** `secret.get` runs in one transaction: check grant, read the sealed record, open it, update `last_read_*`, commit, then answer. The value type has no `Debug`, `Display`, `Serialize`, or `Clone`; `reveal()` is the only way out and is called once, in the presentation mapper of the answer.
- **D6: Redaction.** Covers every point of FR-012 (a) to (h): the output store append and replay, the action record writer (sweep, directive, report, refusal entries), the process identity recorder and the helper identity store (display text only; the hash is computed first), notification and Needs you writers, the 0011 task text writer, the 0012 directive queue and the typed text, the `agent.create` and temp input handlers, the finished temp record, and the event and answer mappers. A test lists the stored agent-supplied columns and the send paths and fails when one is not routed through the redactor (the contract's rule). `Redactor` in the secret application layer: `replace(text) -> text` and a streaming variant for output chunks that keeps the last `longest_value_len - 1` bytes pending between chunks, flushed at session end. It holds the open values (`Vec<(name, Zeroizing<Vec<u8>>)>`), loaded at start when the store is ready and refreshed on every change event. It is injected into: the output store append and replay (`output_store.rs`), the action record writer, the notification and Needs you writers (including the 0015 report text), and the event publisher's text fields. The live PTY stream is not changed (spec A3). Buffers are `Zeroizing`, no type has `Debug` that shows a value, and the matcher is rebuilt on every save, replace, and delete. The oracle note (spec A14) goes in the PR.
- **D7: Crash and logs.** `server_log` gains no value fields; a test greps the log after a failing read. On Unix, set `RLIMIT_CORE` to 0 at server start; on Linux also `prctl(PR_SET_DUMPABLE, 0)`, after a check that the office's own process inspection (0017) does not read the server's own `/proc` entries through a path that this blocks; macOS gets the core limit. Windows: record what the platform allows (no reliable per-process switch, spec A5); if nothing safe exists, state it in the qa-report as PARTIAL. A panic hook is not added; `Debug` of every value-holding type prints `[secret]`.
- **D8: Limits.** `limit_secret_reads_per_agent` joins `Settings` next to `limit_reports_per_agent` (default 20, range 1 to 1000). `secret.get` is an agent-made action for the 0007 holds, in the same order as other actions.
- **D9: Desktop.** A `secrets` module with a service (wire calls), a controller (state), and a dialog opened from the same place as the limits dialog. The value input is `type="password"`, `autocomplete="off"`, and is cleared on save, cancel, and close. The value is passed to the service call and never stored in a Svelte store or a log line. The Rust side of the desktop forwards the request without logging the frame (T5 checks the Tauri command and its error mapping).
- **D10: Constitution check.** The new module has no dependency on another module's domain beyond `AgentId` in the kernel. The server wires the ports. No change to 0001 files except the allowlist, the settings key, the two action kinds, and the redaction hooks. The migration is additive.

## Scope

**In:** D1 to D10, the tests, the qa-report, a README note on the custody per OS.

**Out:** everything in the spec's Out of scope; the living-spec fold, which follows after merge as in earlier changes.

## Tasks

### T1: The `secret` module core: value types, sealing, store key (D1 to D3)
- **Files:** new crates `src/modules/secret/{domain,application,infrastructure,presentation}`, workspace dependencies, `StoreKeyVault` with the key file and keyring implementations.
- **Acceptance:**
  - Unit tests: name and value validation (FR-016 boundaries, the 8-byte minimum); seal then open; two seals of one value differ; a record opened with another secret's name or id fails; a flipped byte fails; `Debug` of a value, a key, and a sealed record prints no secret text.
  - Key file custody: file created with the private-file rule; a missing file gives the locked state, not a new key.
  - Keyring custody: tested through the fake vault; a real-keyring smoke test runs where the OS offers one and is skipped, with the reason printed, where it does not.
  - `scripts/check-boundaries.sh` passes.
- **Depends on:** none.

### T2: Persistence: secrets, grants, store state (D4, D5)
- **Files:** module migration `0001_secrets.sql`, repository, application commands (set, delete, grant, revoke, reset, list, available, get), tests.
- **Acceptance:**
  - Repository tests: save, replace (version +1, grants kept), delete (grants gone), reset (all gone, key gone), the 100-secret limit, two concurrent saves of one name, a transaction killed midway leaves the old record or the new one.
  - Read tests: granted, ungranted, missing, and deleted names give the same `not_found` text; `last_read_*` updates; a damaged record gives `internal` with no value in the text.
  - Locked state: reads and saves are refused, records unchanged byte for byte, the next call after the vault returns works.
  - After a delete and a restart, the database file and its `-wal` and `-shm` hold no copy of the sealed bytes (SC-005).
- **Depends on:** T1.

### T3: Server: requests, allowlist, helper, settings, records, events (D5, D8)
- **Files:** `server/secret_requests.rs`, `server/agent_call.rs` (allowlist, dispatch), `cli/agent_call.rs` (`--value-only`), `settings.rs` and the rate limit bridge, `action_kind.rs`, `hierarchy-protocol.md` amend notes, wire types with ts-rs, tests.
- **Acceptance:**
  - A user client gets `not_found` for `secret.get` and `secret.available`; an agent gets `refused` for the six user requests (recorded).
  - Frames with extra fields, `actor`, or `caller` follow the contract.
  - `--value-only` prints the value alone and nothing else; an error prints nothing to standard output and exits non-zero.
  - Holds and the limit: read 21 within 60 seconds is refused naming the limit; a hold refuses a read first; both recorded.
  - Every read, accepted or refused, writes one `secret_read` entry; changes write `secret_change` with the person as actor; no entry holds a value.
  - `secret.changed` events carry names only.
  - Agent delete removes its grants (US4 scenario 2).
- **Depends on:** T2.

### T4: Redaction and the crash and log rules (D6, D7)
- **Files:** `Redactor`, hooks in the output store, action record writer, notification and Needs you writers, event publisher, server start (core size), tests.
- **Acceptance:**
  - A value printed whole, in two chunks, and three chunks is stored and replayed as `[secret NAME]`; an encoded form is stored as printed (documented).
  - A value in a directive (queue, typed text, records), a report, a task text, an `agent.create` and temp input, and an error text is replaced in every place of FR-012.
  - A helper process with the value in its command line for more than 10 seconds: after a kill and a restart, the stored identity and the sweep entry hold the placeholder, and the sweep still ends the process (the hash is of the original).
  - The enumeration test of D6 passes.
  - After a change event, a new value is replaced and a deleted value is not.
  - The server log after a failed read and a failed save holds no marker.
  - Linux and macOS: the core limit of the server process is 0 (test reads it back); Windows result recorded.
- **Depends on:** T2 (T3 for the event hooks).

### T5: Desktop Secrets dialog (D9)
- **Files:** `src/apps/desktop/src/modules/secrets/**`, the opener beside the limits dialog, `e2e/secrets.spec.ts`, the mock core, the Tauri command mapping if one is needed.
- **Acceptance:**
  - Unit tests for the controller: the typed value is gone after save, cancel, and close; an error text never holds the value.
  - e2e with the mock core: add, grant, replace, revoke, delete, reset with confirmation, locked state, empty state, keyboard only (SC-009), Escape closes, focus returns; status words are text.
  - The value field shows the 8-byte minimum as an error text and not a warning (FR-016).
  - Desktop lint, type check, and unit tests green.
- **Depends on:** T3.

### T6: Hands-on and byte scans (SC-001 to SC-010)
- **Files:** integration tests in `src/apps/office-harnessd/tests/agent_secrets.rs`, `specs/changes/0026-agent-secrets/qa-report.md`.
- **Acceptance:**
  - SC-001, SC-002, SC-008: byte scans over the data folder, the action record, notifications, a recording client, and an export, with unique markers; SC-002 includes the helper with the value in its command line, the kill, and the restart.
  - SC-003: 30 stand-in agents, rotate, all read version 2.
  - SC-004: 100 denied reads, one text.
  - SC-006, SC-007: custody and locked-state tests with the fake vault and, where the OS offers one, the real keyring.
  - SC-010: the limit and the hold.
  - Reduced counts are PARTIAL with the reason.
- **Depends on:** T3, T4, T5.

## Traceability

| Requirement | Task | Evidence |
|---|---|---|
| FR-001, FR-016 | T1, T2, T5 | validation tests, dialog e2e |
| FR-002, FR-003 | T1, T2, T6 | seal tests, custody tests, SC-001, SC-006 |
| FR-004 | T3, T6 | no value in any user answer, SC-002 |
| FR-005, FR-007 | T2, T3 | grant tests, one-text `not_found`, SC-004 |
| FR-006, FR-008 | T2, T3, T6 | SC-003 |
| FR-009 | T2, T3 | delete and agent delete tests, SC-005 |
| FR-010, FR-011 | T3, T6 | record tests, SC-010 |
| FR-012 | T4, T6 | redaction tests, SC-002 |
| FR-013 | T4 | log and core tests |
| FR-014 | T6 | SC-008 |
| FR-015, FR-019 | T2, T5, T6 | reset and locked tests, SC-007 |
| FR-017, FR-018 | T5 | e2e, controller tests, SC-009 |
| PB-1 to PB-7 | T3, T4 | the existing tests stay green |

## QA and evidence

- Gates, run by the lead on the final tree: `cargo fmt --all --check`, `bash scripts/check-boundaries.sh`, `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings`, `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=4`, and the desktop lint, type check, unit tests, and Playwright e2e for the Secrets dialog. Pass and fail counts are summed from the result lines.
- Heavy builds follow the floor rules: free memory, at most 3 heavy jobs, `RUSTFLAGS='-Clinker-features=-lld'` locally.
- Hands-on with a real CLI where one is signed in: save a secret, grant it, let the CLI read it inside a command, rotate it, check the stored output. Where none is signed in, stand-in CLIs carry it and the report says so.
- The security review: Efe runs it against the spec and the diff (threat table T1 to T12).

## Final verification

Run `/ac:verify` against this plan. Then CI must be green on the 3 OSes, and Efe's code gate runs at the exact head.
