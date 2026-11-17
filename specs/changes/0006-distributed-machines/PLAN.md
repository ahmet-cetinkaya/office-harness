---
status: draft
spec: specs/changes/0006-distributed-machines/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-17. Status: draft. Build starts after the owner approves spec 0006; the spec is a draft awaiting owner approval. The build also waits for 0003 to merge to `main`, which itself starts after 0002. The owner or god approves the plan before the build. A security review of this plan passes before the build starts. -->

# Plan: distributed machines (0006)

## Summary

This change opens the office to the network for the first time. The office server gains a TLS listener that is off until the user turns it on. A second machine joins with a one-time join code that carries the server fingerprint, receives a machine credential, and then runs agents that appear in the one panel. The user creates an agent on a chosen machine, watches its terminal, sees `unreachable` when the machine is offline, and removes a machine so that its credential stops working. Every join, credential, and transport rule of the spec's Threat model is a separate task with its own tests, and security reviews gate the start and the merge.

**What you get:**
- A machines view in the panel: the home machine, joined machines with their state as text and last-seen time, the listener state with its on and off actions, a join code shown once with expiry and fingerprint, and a remove flow that names the affected agents.
- A `machine` server module: join codes (at least 64 bits from a secure random source, one use, 10 minutes), machine credentials, the per-source lockout, and the machine registry.
- A TLS listener with a server-created identity; the full fingerprint hash is pinned at join and compared in full on every connection, with the handshake signature checked by the crypto provider; a changed identity is refused until a new join.
- The same office-harnessd binary in a join mode on the joined machine: it runs the agents there with the user's own environment and relays terminals, statuses, and output to the office server.
- One agent list across machines, `unreachable` as text with the last output the office server holds, and a create-agent dialog with a machine picker and per-machine project folders.

**Why this approach:**
- `docs/ARCHITECTURE.md` already reserves the `machine` module for R5 and a listener next to the loopback one, and each agent already carries `machine_id` (`agent_machines` table, migration 0001), so the data model needs no new agent column.
- `access` already draws bytes from the operating system's secure random source (`random_bytes.rs`), so the join code and the credential reuse a proven random source. The credential is checked by a constant-time comparison of its hash over the pinned link; the proof scheme in `src/shared/wire/src/proof.rs` is not reused, because its verifier must hold the credential itself as the HMAC key and the server stores only the hash.
- The joined machine runs the same binary that already owns PTYs, the screen emulator, the output store, the CLI adapters, and the login environment, so a remote agent behaves like a local one on its own machine (0001/FR-020, PB-3) without a second terminal implementation.

**What it will not do:** a relay or tunnel service, machine discovery, routes through routers or firewalls; desktop apps or web clients on other machines; moving agents between machines; peer-to-peer offices; installing office-harness from the panel or the Docker Compose install; syncing project files between machines; protection against a compromised home machine or an agent that reads its own machine's credential (Threat model).

**Shape of the work:** 13 tasks in 3 phase PRs into one umbrella branch `feat/0006-distributed-machines`, one local commit per task. **Effort:** XL.

**Main risks:**
- The automated suite runs on one machine; a true two-machine join over a real network is a hands-on row. Mitigated by loopback integration tests with two processes (the office server and a joined daemon) that exercise the full protocol, and by the hands-on rows in the QA report recorded as BLOCKED until a person runs them.
- The lockout can lock a shared NAT address that several machines use. Mitigated by the spec's own answer: the lockout blocks joins only, clears when the listener turns off and on, and the panel says that a source is locked out (FR-005, edge cases).
- Terminal relay latency over a slow link (SC-011). Mitigated by forwarding frames unchanged, keeping no per-frame server processing on the relay path, and measuring with a delay-inserting proxy; the 250 ms limit at 50 ms round trip is the spec's accepted figure.
- 0003 is not merged, so the panel code this change extends does not exist on `main` yet. Mitigated by the merge gate; T10 and T11 re-read the merged panel code before any desktop change.
- A TLS or key-handling mistake is hard to see in review. Mitigated by one identity module with redaction tests, the full-hash comparison on every connection, and the security-review gates before the start and before the merge.

**Decisions made for you:** the `machine` module layout, a server-created TLS identity checked by fingerprint, the code and credential formats and their hashed storage, the constant-time credential check over the pinned link, the in-memory per-source lockout, the join mode of the existing server binary, the heartbeat figures (5 second ping, 15 second offline), `unreachable` as a derived display state rather than a new status-machine state, per-machine project folders as one new agent migration, and rustls with rcgen as the TLS stack (T1 confirms versions).

**Blockers:** the owner's approval of spec 0006, 0003 merged to `main`, and a security review of this plan passed (advisor or `/security-review`). Nothing else.

## Goal

Deliver spec 0006: the user turns on the listener, hands one join code to a second machine, and that machine appears in the panel. He creates an agent on it, uses its terminal as any other, sees `unreachable` when it is offline, and removes it so that its credential stops working. Every connection is encrypted, pinned to the server identity, and carries a machine credential that works for that machine's agents only.

## Context

- **Repository:** 0001 is on `main` (merge 4425166). 0002 is approved and its build branch exists; 0003 is approved with a drafted plan. This plan branches from `main` after 0003 merges. Spec 0006 itself is a draft awaiting owner approval; the build starts only after that approval.
- **Existing seams this change touches** (verified at `origin/main` 7750602 on 2026-11-17):
  - Server socket: `src/apps/office-harnessd/src/server/` binds `127.0.0.1` on a free port in `running_server.rs`, refuses non-loopback peers and wrong `Host` or `Origin` in `upgrade_check.rs`, and runs the token handshake in `handshake.rs` and `connection.rs`. The listener is a second server beside this one, not a change to it.
  - Auth: `src/shared/wire/src/proof.rs` computes HMAC proofs over `nonce || port || role`; `src/modules/access/infrastructure/src/services/` has `random_bytes.rs` (getrandom), `hmac_proof_scheme.rs`, and `file_token_store.rs` (owner-only file through `private-fs`).
  - Machines today: `src/modules/agent/domain/src/entities/machine.rs` (id and name only), table `agent_machines` in `src/modules/agent/infrastructure/src/persistence/migrations/0001_machines_projects_agents.sql`, `machine_id` on every agent row. One home machine row exists.
  - Statuses: `src/modules/agent/domain/src/enums/agent_status.rs` and `services/status_machine.rs` hold the seven statuses; the desktop shows them through `statusText` in `src/apps/desktop/src/modules/agent/models/agent.ts`.
  - Action record: `src/modules/audit/domain/src/action_kind.rs` holds six kinds; each writing module declares its own `ActionRecorder` port and the host bridges to `audit-application`.
  - Composition: `src/apps/office-harnessd/src/bootstrap/` builds services per module and runs per-module migrations (`migrations.rs`); `src/apps/office-harnessd/src/bridges/` holds the cross-module adapters. `scripts/check-boundaries.sh` reads crate roles from manifest paths, so a new module needs no script change.
  - Desktop: `app.svelte` renders `OfficeView` after the connection states; the create flow is `modules/agent/components/agent-form-dialog.svelte`; requests go through `modules/agent/services/agent-service.ts`; state lives in `modules/agent/state/office-controller.ts`; the e2e mock core is `src/apps/desktop/e2e/support/mock-core.ts`.
  - Protocol surface: `specs/changes/0001-office-server-and-terminal/contracts/client-protocol.md` fixes the message shapes, error codes, and close codes that this change extends.
- **Floor rules:** one build or test at a time on the floor; ask god for the build slot before each local build or test run (`CARGO_BUILD_JOBS=4`, `--test-threads=4`, at most 30 minutes). Efe reviews each PR at its exact head sha; god merges. Nothing is pushed to `main`. No other project's name appears in any repository file.
- **Toolchain:** as 0001 to 0003 (cargo 1.100, bun 1.4.4, Node 24, Tauri v2, Svelte 5 with runes, Tailwind CSS 4, TypeScript). TLS rides rustls; no platform TLS API, so the three OSes share one code path.

## Decisions

- **D1 Module layout:** one new server module `src/modules/machine/{domain,application,infrastructure,presentation}` with crates `machine-<layer>`, tables prefixed `machine_`, its own migration ledger. It owns the join codes, the machine registry, the credential hashes, the listener state, the lockout, and the machine links. The listener socket itself lives in the host app (`src/apps/office-harnessd/src/server/`), like the loopback server, because sockets are host concerns.
  - Simpler alternative rejected: keep join state in the `access` module. Access owns the office token and its proof; machines are a new boundary with their own lifecycle, and `docs/ARCHITECTURE.md` already reserves the module.
- **D2 Identity and fingerprint (FR-004, security c):** the office server creates its TLS identity once at the first listener start: a self-signed certificate (rcgen, T1 confirms the version), private key written through `private-fs` so only the owner reads it. The fingerprint is SHA-256 over the certificate DER, 256 bits, far above the 128-bit floor. The certificate is valid for 10 years. At each listener start the server renews an identity that is near expiry; renewal replaces the certificate and key as one step. The pin compares the fingerprint only: an expired certificate still matches its fingerprint and does not end the pin. A renewed identity has a new fingerprint, so machines joined under the old one rejoin with a new code, as any identity change does. The join code carries the full fingerprint. The joining machine pins it and every later connection compares the full hash of the reached server's certificate against the pin before any credential byte is sent; a mismatch refuses the connection and the machine says that the identity changed and a new join is needed. The pin verifier also verifies the TLS 1.3 handshake signature through the crypto provider (`rustls::crypto::verify_tls13_signature`), so a replayed certificate with a different key is refused; plain connections are refused (threat row 1). The panel may additionally show a short grouped form for human comparison; only the full value is ever compared (SC-009's forged short form fails this check).
- **D3 Join code (FR-002, security a):** 16 bytes (128 bits) from getrandom, text-encoded base32 without padding, one use, 10 minutes from creation, expiry counted on the office server's clock only. Stored only as SHA-256 hash with expiry and use time; the plaintext exists in the creating handler's memory and in the one response to the desktop app, is shown once, and is never stored, logged, or shown again. Several live codes may exist at once (edge case); a used or expired code is refused and the refusal is recorded.
- **D4 Machine credential (FR-003, FR-008, security e):** at the first successful join the server issues 32 bytes (256 bits) from getrandom, stores only its hash beside the machine row, and sends it once over the fresh TLS link. The machine keeps it in its own data folder owner-only. One credential per machine; removal invalidates the hash, closes the live link, and deletes the machine's agents through the agent module; a new join issues a new credential. Scoping: the link authenticates to one machine id, and the dispatch refuses every request whose target agent does not belong to that machine (a joined machine reads and controls only its own agents; it never sees the office token).
- **D5 Listener (FR-001, security b):** the listener binds every interface on a port chosen once at the first start and stored; `server.json` stays loopback-only and the machines view shows the reachable address for the user to hand out (A3). It is off until the user turns it on, and a stored option "start the listener after a restart" (off by default) decides its state after a server restart; otherwise it is off again. Turning it off closes every machine link; the loopback server and the desktop app are untouched. A port already in use leaves the listener off and the panel names the reason. With the listener off, no machine connection is accepted (PB-1).
- **D6 Lockout (FR-005, security d):** in-memory per source address: 5 refused join attempts within a sliding 1 minute lock that source out of join attempts for 5 minutes. The map is capped at 10,000 entries; a full map evicts the entry idle the longest, so IPv6 address spread cannot grow the memory. Only join attempts count and only joins are blocked; connected machines, their credentials, and stored codes are untouched. Turning the listener off and on clears it (edge case). The state is memory by design: a restart clears it, which matches the off-by-default listener.
- **D7 Machine link and relay (FR-006, FR-007):** the joined machine runs the same `office-harnessd` binary in a join mode (`office-harnessd join`): it connects to the office server over TLS with the pinned fingerprint, and sends the machine credential as the first link message over that encrypted and pinned link. The server hashes the received credential and compares it in constant time against the stored hash; only this check authenticates the link. The stored hash alone never authenticates anything (a stolen database cannot join a machine). The office server routes: agent and terminal requests for an agent of machine M go down M's link; the machine spawns the CLI in its own PTY with its own login environment and CLI configuration (PB-3), streams the same binary terminal frames back, and reports statuses, session starts and exits, and output. Every message the machine reports up is checked against that machine's own agents: a frame, status, or output for an agent it does not own is dropped, recorded, and refused. The machine link is never a hub subscriber, so no broadcast event reaches it. The office server relays frames and events to desktop clients, records the sessions in its own database (A1: the home machine holds the records), and keeps a capped copy of each joined agent's recent output so the offline view works (A6). The relay adds no transformation to frames, so SC-007 and SC-011 measure little more than the network.
- **D8 Unreachable and reconnect (FR-007, US3):** the link carries a heartbeat every 5 seconds; 15 seconds without traffic marks the machine offline and its agents `unreachable` (SC-006's 15 seconds). `unreachable` is a derived display state from the machine's state, not a new status-machine state, so `status_machine.rs` and the seven statuses stay intact; the agent snapshot carries the machine's state and the desktop shows `unreachable` as text with the last-seen time. On return, the machine reports every agent's real status and the office server relays it within 10 seconds. A machine restart shows its agents `interrupted` and none restarts on its own (0001/FR-004). After a server restart, joined machines reconnect with their credentials once the listener is on (edge case: the stored option decides).
- **D9 Create across machines and per-machine folders (FR-006, A5):** `agent.create` gains `machine_id`; a create for an offline machine is refused with that reason and no agent exists (edge case). Project folders become per machine: one new agent migration adds a project-folder-per-machine table (`project_id`, `machine_id`, `folder`), the home row keeps today's value, and an agent whose project has no folder on its machine cannot start until the user sets one. The agent list and details show the machine name.
- **D10 Registry sync (A1):** `agent_machines` (agent module) keeps the name and agent linkage; `machine_machines` (machine module) keeps join state, credential hash, state, and last seen. The host bridge creates the agent machine row at registration and, on removal, deletes the machine's agents through the agent module's delete path, so no module depends on another (the architecture's bridge rule).
- **D11 Wire surface (D9):** desktop clients gain `machine.list`, `machine.join_code.create`, `machine.listener.on`, `machine.listener.off`, `machine.remove`, `machine.folder.set`, and events `machine.status`, `machine.joined`, `machine.removed`, `listener.state`; the agent snapshot gains the machine name and the machine state; the action record gains `machine joined`, `join refused`, and `machine removed`. The machine link speaks its own message set fixed in `contracts/machine-protocol.md`. Error codes and close codes stay the 0001 contract.
- **D12 Desktop surface:** one new webview folder `src/apps/desktop/src/modules/machines/` with the machines view, the join code dialog (code, expiry, fingerprint, once), the listener toggle with its failure reasons, and the remove confirmation naming the affected agent count. The agent form dialog gains the machine picker (home machine or `online` machines) and the per-machine project folder field; the agent list and details show the machine name; `unreachable` rows open on the stored output and refuse key delivery (edge cases). All of it keyboard reachable, text status, focus visible (PB-5).
- **D13 Tests:** Rust unit tests beside the code and integration tests in `tests/`; the join and link run on one machine as two processes over loopback TLS. Playwright with the mock core and axe for the desktop views at 1280 and 360 widths, as 0001 to 0005 did. The threat tests of T13 (traffic capture, forged fingerprint, 1000-address guessing, revoked credential, cross-machine access) are integration tests, not hands-on rows. Redaction tests live with the code they guard (T7).
- **D14 Lint and boundaries:** `scripts/check-boundaries.sh` needs no change (it classifies by manifest path, so `machine-*` crates are covered at once); `boundaries.spec.ts` covers the new `machines` folder as any module. `unsafe_code` stays forbidden; rustls and rcgen need none.
- **D15 Delivery and commits:** one umbrella branch `feat/0006-distributed-machines` created from `main` after the gates, three phase PRs, one local commit per verified task, Efe reviews each PR at its head sha, god merges (the 0003 procedure).
- **Constitution check:** the repository has no instruction file; `docs/ARCHITECTURE.md` is binding, and this plan adds the module row it already reserves and one webview folder its Frontend Folder Structure anticipates. The owner's global rules apply: clean code, no secrets in any file or log, explicit approval for remote actions.

## Scope

**In:**
- FR-001 to FR-008, the modified `0001/FR-005` and `0001/FR-008`, and SC-001 to SC-011 of spec 0006.
- Server: the `machine` module, the TLS listener, join codes and credentials, the lockout, the machine link with terminal and status relay, unreachable and reconnect, per-machine project folders, the new action kinds.
- Desktop: the machines view, the join and remove flows, the listener toggle, the machine picker, the machine name and `unreachable` in the agent list and details.
- Docs: the text amendments to `docs/PRD.md` 4.1 and 0001 (FR-005, FR-008, A13), README, and `docs/ARCHITECTURE.md`.
- `e2e/support/mock-core.ts` extensions for the machines fixtures.

**Out (guards against unrequested additions):**
- A relay or tunnel service, discovery, any route through a router or firewall (A2, A3).
- Desktop apps on other machines, web or phone clients (PB-2).
- Moving agents between machines (A7); peer-to-peer offices (A1).
- Single sign-on, user accounts, several owners (PRD 4.1).
- Install from the panel and the Docker Compose install (D-10, A9).
- Project file sync between machines.
- Any new visual design beyond the spec's flow (the spec says none exists).

## Phases

| Phase | Outcome | Tasks | Branch (from) | PR (head → base) |
|---|---|---|---|---|
| 1 Join security | Identity, join codes, listener, lockout, credentials, redaction | T1 to T7 | `feat/0006-p1-join-security` (umbrella) | p1 → umbrella |
| 2 Machine link | Join mode, relay, unreachable and reconnect | T8, T9 | `feat/0006-p2-machine-link` (umbrella after p1) | p2 → umbrella |
| 3 Desktop and ship | Machines view, picker, docs and amendments, QA and threat tests, security gate | T10 to T13 | `feat/0006-p3-desktop` (umbrella after p2) | p3 → umbrella, then umbrella → `main` |

The umbrella `feat/0006-distributed-machines` is created from `main` after 0003 merges and the owner approves the spec; it holds the approved spec and this plan (committed by the docs PR). Phases are sequential; each phase PR runs CI on three OSes and gets Efe's review at its head sha.

## Tasks

Paths are relative to the repository root. `M/` stands for `src/modules/`, `D/` for `src/apps/desktop/src/`, `H/` for `src/apps/office-harnessd/src/`.

### T1: Research and design artifacts (no product code)

- **Files:** `specs/changes/0006-distributed-machines/research.md`, `data-model.md`, `contracts/machine-protocol.md`.
- **Work:**
  - R1: rustls and rcgen exact versions for the workspace, certificate and key generation and storage on Linux, macOS, and Windows, and the SHA-256 fingerprint over the certificate DER; cite each source.
  - R2: getrandom in a join mode process and in the server, as `access` already uses it; confirm 128-bit and 256-bit draws and base32 encoding without a new dependency.
  - R3: the machine credential check: the credential travels as the first link message over the pinned TLS link, the server hashes it and compares it in constant time (a constant-time equality such as `subtle` or an equivalent) against the stored hash, and the proof scheme of `src/shared/wire/src/proof.rs` is left untouched because its verifier would need the credential itself as the HMAC key. Record the constant-time primitive chosen.
  - R4: heartbeat and timing figures against SC-006 and SC-011 (5 second ping, 15 second offline, 10 second resync); the delay-inserting proxy design for the 50 ms round-trip measurement.
  - R5: WAI-ARIA patterns for the machines view, the join code dialog, and the confirmation dialog; cite them.
  - `data-model.md`: `machine_machines`, `machine_join_codes`, and the listener state DDL with the hashed columns; the project-folder-per-machine DDL for the agent module; the migration ledger names.
  - `contracts/machine-protocol.md`: every new desktop request and event of D11, every machine-link message, the join and reconnect handshakes, and the close and error codes; nothing else.
- **Depends on:** none.
- **Acceptance:** every research item has a "verified <date>, source <file or link>" line; the contract lists every type that T2 to T11 use, and nothing else; the DDL carries no plaintext secret column.

### T2: Server identity and fingerprint (FR-004, security c)

- **Files:** `M/machine/domain/`, `M/machine/infrastructure/src/services/` (identity store, fingerprint), `src/shared/wire` if the fingerprint rides a wire type, `Cargo.toml`, `Cargo.lock`.
- **Work:**
  - (a) Create the self-signed identity once at the first listener start; the private key goes through `private-fs` (owner-only); the certificate lives in the data folder.
  - (b) The fingerprint is SHA-256 over the certificate DER (256 bits), compared in full; the short human form is display text next to the full value and never a comparison input (D2).
  - (c) The join code value structure carries the full fingerprint (D3), and the machine-side pin check refuses any other identity before anything is sent.
- **Depends on:** T1.
- **Acceptance:**
  - (a) The identity file is owner-only on disk (mode test on Unix); a second start reuses it; a deleted identity is recreated only while no machine has joined.
  - (b) Fingerprint tests: the hash is 256 bits, stable for the same certificate, different for another; a forged certificate that matches a truncated shown form fails the full comparison (SC-009 shape); a client presenting the pinned certificate with a different private key fails the handshake-signature check and sends 0 credential bytes.
  - fmt, clippy, `cargo test` for the changed crates, `scripts/check-boundaries.sh`.

### T3: Join codes (FR-002, FR-003, security a)

- **Files:** `M/machine/domain/` (join code value object), `M/machine/application/src/commands/create_join_code/`, `M/machine/application/src/commands/redeem_join_code/`, `M/machine/infrastructure/` (hashed storage), `H/` wiring.
- **Work:**
  - (a) The generator draws 16 bytes from getrandom (128 bits, above the 64-bit floor), encodes base32, and the code works once and expires after 10 minutes on the server clock (D3).
  - (b) Storage keeps the SHA-256 hash, the expiry, and the use time; the plaintext never reaches disk, a log, or a second response (D3, T7 verifies).
  - (c) Redemption checks hash, expiry, and use; wrong, used, and expired are distinct refusal reasons; the refusal is recorded with no code material (the action record kind `join refused`); several live codes work at once (edge case); a same-machine rejoin is refused on both sides (edge case): the machine that already holds a live credential refuses to run `join` again, and the server refuses the name that is already registered.
- **Depends on:** T2.
- **Acceptance:**
  - (a) Generator tests: 10,000 codes hold no repeat and each carries at least 64 bits of entropy from the secure source (SC-010 shape); the encoding round-trips.
  - (b) Redemption tests: a right code joins once, a second use is refused as used, an expired code is refused as expired, a wrong code is refused as wrong and counts toward the lockout; the expiry counts on the server clock with a shifted client clock (edge case).
  - (c) A storage scan test finds no plaintext code in the database file (SC-008 shape).
  - fmt, clippy, `cargo test`, `scripts/check-boundaries.sh`.

### T4: Listener off by default (FR-001, PB-1, security b)

- **Files:** `H/server/` (listener beside the loopback server), `M/machine/` (listener state and stored option), `H/bootstrap/`, `H/config/`.
- **Work:**
  - (a) The listener starts only when the stored state is on; a fresh office has it off, and nothing binds until the user turns it on (PB-1).
  - (b) The stored option "start the listener after a restart" is off by default; after a server restart the listener is off unless it is on (edge case).
  - (c) On: TLS accept loop with the 0001 close and error contract; off: every machine link closes and no new connection is accepted.
  - (d) A port already in use leaves the listener off and names the reason (edge case); the chosen port is stored and reused.
- **Depends on:** T3.
- **Acceptance:**
  - (a) Integration: a fresh server accepts no machine connection; after on, it does; after off, an existing link closes and a new one is refused (US1 scenario 1, US3 scenario 5, PB-1).
  - (b) Restart tests: listener off after restart by default, on with the option; joined machines reconnect with their credentials once it is on (edge case).
  - (c) A busy-port test leaves the office usable on the home machine and shows the reason.
  - fmt, clippy, `cargo test`, `scripts/check-boundaries.sh`.

### T5: Per-source lockout (FR-005, security d)

- **Files:** `M/machine/application/src/` (lockout service), `M/machine/infrastructure/` if the counter needs a port, `H/` wiring.
- **Work:**
  - (a) Count refused join attempts per source address in a sliding 1 minute window; the fifth refusal locks that source out of join attempts for 5 minutes (D6).
  - (b) The lockout blocks join attempts only: connected machines keep working, credentials keep working, and a valid code is not consumed by a locked-out attempt.
  - (c) Turning the listener off and on clears the lockout; the panel learns that a source is locked out (edge case).
- **Depends on:** T4.
- **Acceptance:**
  - (a) Unit tests: 4 wrong codes keep the source open, the 5th within the window locks it, a wrong code after the window does not; the 6th attempt with a valid code is refused during the lockout and works after 5 minutes (SC-003 shape); more distinct addresses than the map cap evict the idle entries and the lockout still works.
  - (b) Tests that a connected machine and a stored valid code are untouched by a lockout on another source; that the clearing works.
  - fmt, clippy, `cargo test`.

### T6: Machine credentials, scoping, revocation (FR-003, FR-008, modified `0001/FR-008`, security e)

- **Files:** `M/machine/application/src/commands/register_machine/`, `M/machine/application/src/commands/remove_machine/`, `M/machine/infrastructure/` (credential hash store, link registry), `H/bridges/` (agent machine sync of D10), `M/agent/` (delete path for a machine's agents).
- **Work:**
  - (a) On the first successful join: register under the given name (empty or duplicate names refused as the edge cases say), issue the 256-bit credential, store only the hash, send it once over the fresh TLS link, and record `machine joined` (D4, D3).
  - (b) Every link authenticates to one machine id through the constant-time credential check over the pinned link; the dispatch refuses any request whose agent does not belong to that machine; every reported-up frame, status, or output for an agent outside that machine's own set is dropped, recorded, and refused; the link is never a hub subscriber, so no broadcast event reaches it; and a joined machine never receives the office token (D4, D7, modified `0001/FR-008`).
  - (c) Removal: state the affected agent count, invalidate the credential hash, close the live link, delete the machine's agents through the agent module, record `machine removed`; removal of an offline machine works the same and the panel says its agents may still run there (edge case); an old or removed credential is refused and recorded (edge case).
- **Depends on:** T5.
- **Acceptance:**
  - (a) Integration: join, credential stored hashed (no plaintext in the database file), name conflicts and empty names refused (US1 scenarios 3 and 4).
  - (b) Scoping tests: a valid credential reads 0 terminals and controls 0 agents of other machines in 20 attempts, and 50 connections with no, wrong, or removed credentials are all refused with none reading a terminal (SC-002, SC-005 shapes); a database file that holds only the credential hashes cannot authenticate a machine: a client that reads it and connects with a stored hash instead of the credential is refused (stolen database shape); a machine that reports an agent it does not own has that message dropped, refused, and recorded (compromised machine shape); the machine link receives no broadcast event aimed at desktop subscribers.
  - (c) Revocation: after removal, 20 connections with the old credential are refused within 5 seconds and the live link is closed (SC-005); the affected-agent count matches the agent list.
  - fmt, clippy, `cargo test`, `scripts/check-boundaries.sh`.

### T7: Secret redaction everywhere (FR-002, PB-6, SC-008, security f)

- **Files:** `M/machine/` (redaction tests beside the code), `H/bootstrap/server_log.rs` tests, `M/audit/` tests (new action kinds hold no secret), `M/terminal/infrastructure/src/services/output_store.rs` tests, a repository scan script under `scripts/` if the CI step needs one.
- **Work:**
  - (a) Redaction tests that join, refuse a join, and remove a machine, then assert 0 occurrences of the code, the credential, and the office token in the server log, the action record entries, the stored output files, and every error message body.
  - (b) The new action kinds (`machine joined`, `join refused`, `machine removed`) carry names, times, and reasons only; the refusal reason names the kind of failure, never the attempted value.
  - (c) A scan test for SC-008: after a join, a removal, and a refused join, search the logs, the action record, the stored output, and the repository files for the three secrets and find 0 occurrences.
- **Depends on:** T6.
- **Acceptance:** the redaction tests fail if any secret reaches any of the places; the scan test of SC-008 runs in CI; fmt, clippy, `cargo test` for the touched crates.

### T8: Machine link, remote start, and terminal relay (FR-006, FR-007, US2)

- **Files:** the join mode in `H/cli/` and `H/` bootstrap (D7), the machine-link message set from `contracts/machine-protocol.md`, `M/machine/presentation/` (link handlers), `M/machine/infrastructure/` (link sessions), `H/server/` routing, `M/agent/` (create with `machine_id`, spawn dispatch by machine), `M/terminal/` (relay of frames and input), `M/audit/` (new action kinds wired).
- **Work:**
  - (a) `office-harnessd join`: read the address, code, and machine name, reach the server, check the pinned fingerprint before sending anything, redeem, store the credential owner-only, and enter the link loop (US1 scenario 6 guarded here at the machine side).
  - (b) The link authenticates on every connection: the machine rechecks the fingerprint pin first (FR-004), then sends the credential as the first link message, and the server accepts the link only after the constant-time hash comparison (D7, T1 R3).
  - (c) Remote start: the office server records the agent and sends the spawn with command, folder, additions, and the machine's login environment; the machine runs the CLI in its own PTY (PB-3, 0001/FR-020).
  - (d) Relay: `terminal.attach`, `terminal.input`, and `terminal.resize` go down the link; frames and statuses come back unchanged and reach desktop clients as today (D7); the office server keeps the capped recent output copy (A6).
  - (e) Revocation closes the live link (completing T6 c); a machine that presents its credential to a server with another identity sends 0 credential bytes and refuses (US3 scenario 6).
- **Depends on:** T7.
- **Acceptance:**
  - (a) Integration on loopback with two processes: join, create an agent on the joined machine, attach, type, see output and colors as on the home machine (US2 scenarios 1 and 2); a status change reaches the office server and the client within 2 seconds (SC-007 shape).
  - (b) Fingerprint tests: a wrong-identity server refuses the join with 0 code bytes (US1 scenario 6) and 20 reconnects after an identity change refuse with 0 credential bytes until a new join (SC-009 shape).
  - (c) Relay tests: snapshot then live frames arrive exactly once; keys reach the remote CLI; the recent output copy matches the machine's capped output.
  - fmt, clippy, `cargo test`, `scripts/check-boundaries.sh`.

### T9: Unreachable, reconnect, and restart behavior (FR-007, US3)

- **Files:** `M/machine/` (heartbeat, state changes), `H/bridges/` (machine state to agent display state), `M/agent/` (snapshot carries machine state; D8), `M/machine/presentation/` (events).
- **Work:**
  - (a) Heartbeat every 5 seconds; 15 seconds without traffic marks the machine offline and its agents `unreachable` (D8).
  - (b) The office server shows the stored recent output for an `unreachable` agent and refuses input delivery (edge case); keys typed as it goes offline are not delivered (edge case).
  - (c) On return: the machine reports every agent's real status within 10 seconds and recent output resumes (US3 scenario 2); a create for an offline machine fails with that reason and no agent exists (edge case).
  - (d) Restarts: a machine restart shows its agents `interrupted` and none restarts (0001/FR-004); a server restart leaves the listener per the stored option and the machines reconnect (edge case); the listener turning off marks machines offline (US3 scenario 5).
- **Depends on:** T8.
- **Acceptance:**
  - (a) Integration: cutting the link shows offline and `unreachable` within 15 seconds; restoring shows real status within 10 seconds (SC-006); the stored output view matches the limit of 0001 A2.
  - (b) Restart matrix tests for the four restart rows of the edge-case table.
  - fmt, clippy, `cargo test`.

### T10: Machines view and join flow in the desktop (FR-001 to FR-005, FR-008, US1, US3)

- **Files:** `D/modules/machines/` (view, components, state, service), `D/app.svelte` (open the machines view from the panel), `D/modules/agent/` (machine state in the snapshot and status text), `D/e2e/support/mock-core.ts` (machines fixtures), `D/modules/panel/` exports if the merged panel needs one.
- **Work:**
  - (a) The machines view lists the home machine and the joined machines with state as text and last-seen time, and the listener state with the on and off actions (FR-001, D12).
  - (b) The join code dialog shows the code, the expiry time, and the server fingerprint once, offers the copy action, and cannot be shown again (FR-002, D12).
  - (c) Refusal reasons show as text: wrong, used, expired, locked out (FR-003, FR-005); a lockout says that a source is locked out (edge case).
  - (d) The remove flow states the affected agent count and confirms before it removes (FR-008); removal of an offline machine says its agents may still run there (edge case).
  - (e) Every control works from the keyboard, focus is visible, state changes are announced to screen readers, and no state is color alone (PB-5, the spec's accessibility list).
- **Depends on:** T9. Before it starts: re-read the merged 0003 panel code for the view opening and the dialog patterns, as 0005's T1 did.
- **Acceptance:**
  - (a) Playwright with axe at 1280 and 360 widths: the machines view with the home machine only, then with one joined machine online and one offline; state as text; last-seen times present.
  - (b) Playwright: the join code dialog shows code, expiry, and fingerprint, and a second open shows no code (FR-002).
  - (c) Playwright: the remove confirmation names the count and cancels cleanly (US3 scenario 4); the refusal reasons render from the mock core.
  - (d) Keyboard-only run over the machines view, the join dialog, and the confirmation (PB-5).
  - svelte-check, eslint, prettier --check.

### T11: Machine picker and per-machine folders (FR-006, FR-007, US2)

- **Files:** `D/modules/agent/components/agent-form-dialog.svelte`, `D/modules/agent/services/agent-service.ts`, `D/modules/agent/state/office-controller.ts`, `D/modules/agent/models/agent.ts`, `D/modules/agent/components/agent-list.svelte` and `agent-details.svelte` (machine name), `M/agent/` (create with `machine_id`, folder-per-machine migration and port, D9), `H/bridges/` wiring.
- **Work:**
  - (a) The create dialog offers the home machine and the `online` joined machines; an offline machine is not offered, and a create that races an offline transition fails with that reason and no agent exists (FR-006, edge case).
  - (b) The project folder is per machine: the dialog asks for the folder on the chosen machine when none is set (US2 scenario 5, A5); `machine.folder.set` stores it.
  - (c) The agent list and details show the machine's name; an `unreachable` agent shows as text with the machine and last-seen time and opens on the stored output (FR-007, D8, D12).
- **Depends on:** T10.
- **Acceptance:**
  - (a) `bun test` for the store: the picker lists home plus online machines only; the folder requirement per machine; the unreachable display state derivation.
  - (b) Playwright with axe: create on a joined machine shows the machine name in list and details (US2 scenarios 1, 3, 4); the folder ask appears when none is set; an unreachable row shows text and opens the stored output (US3 scenario 3).
  - (c) Integration on loopback: the migration keeps the home folder value; the create with `machine_id` starts the agent on the joined machine (D9).
  - bun test, svelte-check, eslint, prettier --check, fmt, clippy, `cargo test` for the agent module.

### T12: Documentation and amendments (FR-005 and FR-008 texts, security g)

- **Files:** `docs/PRD.md`, `specs/changes/0001-office-server-and-terminal/SPEC.md`, `README.md`, `docs/ARCHITECTURE.md`.
- **Work:**
  - (a) Amend the texts the spec's Clarifications pin: `docs/PRD.md` 4.1 (the office token rule gains the machine credential) and 0001 A13 (joined machines prove themselves with their own credential; the office token stays on the home machine and the desktop app) (security g).
  - (b) Record in 0001 that `0006/FR-005` and `0006/FR-008` supersede FR-005 and FR-008 in part: office data stays inside the user's machines, and client connections come from the same machine or from joined machines with a valid credential scoped to their own agents (the spec's MODIFIED entries).
  - (c) README: the machines view, joining a machine, what a credential may do, removing a machine, the listener default and restart option. `docs/ARCHITECTURE.md`: the `machine` module row leaves the "later" mark, the listener beside the loopback server, and the phases status line for 0006.
- **Depends on:** T11.
- **Acceptance:** the amended texts say exactly what the spec's MODIFIED entries and Clarifications say; every behavior in the README matches the code and the spec; `rg` finds no other project's name, outside path, or secret in the changed docs.

### T13: QA report, threat tests, and security gate (SC-001 to SC-011)

- **Files:** `specs/changes/0006-distributed-machines/qa-report.md`, the integration tests under `src/apps/office-harnessd/tests/` and `M/machine/`, the e2e specs under `src/apps/desktop/e2e/`.
- **Work:**
  - (a) Traffic capture for SC-004: run a join and a 100-line terminal session through a recording TCP proxy and assert 0 readable bytes of the code, the credential, or the terminal text; assert 0 code bytes toward a wrong-fingerprint server.
  - (b) Forged short fingerprint for SC-009: a server whose certificate matches only the truncated shown form; 20 reconnects refused with 0 credential bytes, and the machine joins again only with a new code; a server that presents the pinned certificate with a different private key fails the handshake-signature check, and the joining side sends 0 bytes of the code or the credential.
  - (c) Guessing for SC-010: the generator test of T3 plus 1000 wrong attempts from 1000 simulated source addresses, all refused, no valid code guessed; the lockout timing of SC-003 with 20 reuses and 20 expired uses.
  - (d) Revoked credential and cross-machine access for SC-005 and SC-002: the T6 suites re-run in the report, plus 50 unauthenticated attempts, plus a run in which a joined machine reports frames and statuses for another machine's agents and every one is dropped, refused, and recorded (SC-002 shape).
  - (e) Timing: SC-006 (15 and 10 seconds) and SC-007 (2 seconds and 100 ms at a 5 ms round trip) with the spec's scale of 3 joined machines and 30 agents in total, on loopback; SC-011 through the delay proxy at 50 ms round trip; SC-001 as a loopback join under 3 minutes with the code and the address typed by hand, not only with the copy action (the typed text is roughly 80 characters of code plus the address, and the run must fit the 3 minutes), plus a hands-on row on a real second machine.
  - (f) Security gate: the repository's security review runs on this plan before the work starts (a start gate; see Blockers), and again on the whole umbrella diff before the PR to `main`; both verdicts land in the QA report with their findings closed or recorded.
- **Depends on:** T12.
- **Acceptance:** one verdict per SC with the evidence; SC-002, SC-003, SC-004, SC-005, SC-008, SC-009, SC-010 are automated and green; SC-006, SC-007, SC-011 are automated against loopback and the delay proxy; SC-001's real two-machine run, the screen reader pass, and the 200 percent zoom check are hands-on rows recorded as BLOCKED until a person runs them, never as PASS; the security-review verdicts for the plan (start gate) and the umbrella diff are recorded before the merge.

## Dependencies

T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9 → T10 → T11 → T12 → T13.

- **Sequential:** T2 to T7 build one security core on the same new module and the host wiring; T8 and T9 change the same link code; T10 and T11 change the same desktop files and the agent module; T13 re-runs every suite.
- **Parallel:** none; the shared crates, manifests, `Cargo.lock`, and migrations make parallel tasks collide.
- **Exclusive:** `Cargo.lock`, the migration ledgers, `src/shared/wire`, and the e2e mock core are owned by one running task at a time.
- **Gate before everything:** the owner approves spec 0006, and 0003 (and therefore 0002) is merged to `main`; the umbrella branch is created from that `main`; T1 starts only then. A second gate holds before T2: a security review of this plan (advisor or `/security-review`) passes with its findings closed or recorded.

## Commit strategy

One local commit per verified task, staged by explicit path (never `git add -A`; the sandbox dotfiles are never staged). After each commit, `git status --porcelain` lists only the sandbox dotfiles and the other draft specs.

| Commit | Task | Stages | Proposed message |
|---|---|---|---|
| 0 | spec | the docs PR | `docs(spec-0006): approve distributed machines` |
| 1 | T1 | `specs/changes/0006-distributed-machines/{research.md,data-model.md,contracts/}` | `docs(spec-0006): verify machine protocol and identity research` |
| 2 | T2 | `M/machine/`, `Cargo.toml`, `Cargo.lock` | `feat(machine): create the server identity and fingerprint` |
| 3 | T3 | `M/machine/`, `H/` | `feat(machine): add one-time join codes` |
| 4 | T4 | `H/server/`, `H/bootstrap/`, `M/machine/`, `H/config/` | `feat(server): add the machine listener, off by default` |
| 5 | T5 | `M/machine/` | `feat(machine): lock out repeated join failures per source` |
| 6 | T6 | `M/machine/`, `M/agent/`, `H/bridges/` | `feat(machine): issue scoped credentials and revoke them` |
| 7 | T7 | `M/machine/`, `M/audit/`, `M/terminal/`, `H/` | `test(machine): keep every secret out of logs and records` |
| 8 | T8 | `H/`, `M/machine/`, `M/agent/`, `M/terminal/`, `M/audit/` | `feat(machine): link machines and relay remote terminals` |
| 9 | T9 | `M/machine/`, `M/agent/`, `H/bridges/` | `feat(machine): mark offline machines and their agents` |
| 10 | T10 | `src/apps/desktop/src/modules/machines/`, `app.svelte`, `mock-core.ts` | `feat(desktop): add the machines view and the join flow` |
| 11 | T11 | `D/modules/agent/`, `M/agent/`, `H/bridges/` | `feat(desktop): pick a machine and set its project folder` |
| 12 | T12 | `docs/PRD.md`, `specs/changes/0001-office-server-and-terminal/SPEC.md`, `README.md`, `docs/ARCHITECTURE.md` | `docs: amend the token rule and describe the machines` |
| 13 | T13 | `specs/changes/0006-distributed-machines/qa-report.md`, tests | `test(spec-0006): record QA and threat test results` |

## QA and evidence

- **Per task:** fmt, clippy, `cargo test --workspace` for the crates the task touches, `scripts/check-boundaries.sh`, `bun test`, `svelte-check`, `eslint`, `prettier --check`, Playwright with axe, each as far as the task touches it. Ask god for the build slot before each local build or test run.
- **Per phase PR:** CI green on all three OSes, Efe's review at the head sha, evidence in the PR body.
- **Security pass:** the dedicated security review of this plan before the work starts, and on the whole umbrella diff before the merge (T13 f), on top of `secure-engineering` checks per phase: the listener default, the full-hash fingerprint comparison on every connection, the hashed storage of codes and credentials, the source lockout, the machine scoping at dispatch, and the redaction suite.
- **Real surface:** the two-machine join of SC-001 on a real network, the screen reader pass over the machines view and dialogs, and the 200 percent zoom check are hands-on rows in the QA report; under the overnight directive they are recorded as BLOCKED (need a person), never PASS.
- **Rollback:** each phase is a PR into the umbrella; a failed phase is reverted there. On a user's machine the listener is off by default, so an office that never turns it on behaves exactly as 0001 built it (PB-1). The new tables and the folder-per-machine migration are additive; reverting the code leaves an old server reading the same database, and removing a machine before a rollback leaves no live credential behind. The docs amendments of T12 are reverted with the same PR if the change is abandoned.

## Final verification

1. **Spec compliance:** every Traceability row has evidence; `qa-report.md` has a verdict for SC-001 to SC-011 and the threat model rows.
2. **Architecture compliance:** `scripts/check-boundaries.sh` and `boundaries.spec.ts` pass; the `machine` crates and the `machines` folder match `docs/ARCHITECTURE.md`; no module crate depends on another.
3. **Code quality:** all lints clean; a `/ac:review` of the umbrella diff has no open blocking finding.
4. **Migration safety:** an office created before this change opens and works after the upgrade; the home machine's project folders survive the new migration (integration test, all three OSes in CI).
5. **Scope fidelity:** nothing from the Out list is in the diff; with the listener off, no port is open and no machine connects (PB-1); `rg` finds no other project's name, outside path, person, or secret in docs or code.
6. **Readiness:** `/ac:verify` passes on the umbrella head, and the security-review verdict is recorded, before the PR to `main`.

## Traceability

| ID | Tasks | Evidence |
|---|---|---|
| FR-001 | T4, T10 | T4(a) off until the user turns it on, T4(c) off closes every link; T10(a) the state and the actions as text |
| FR-002 | T3, T10 | T3(a) 128 bits, one use, 10 minutes; T3(b) hash-only storage; T10(b) shown once with expiry and fingerprint |
| FR-003 | T3, T6, T10 | T6(a) registration, credential, `machine joined`; T3(c) refusal reasons recorded; T10(c) reasons as text |
| FR-004 | T2, T8 | T2(b) full 256-bit comparison and the handshake-signature check of the pin verifier; T8(b) pin checked before anything is sent on every connection; T8(e) changed identity refused until a new join |
| FR-005 | T5 | T5(a) 5 in 1 minute locks the source for 5 minutes; T5(b) joins only; T5(c) cleared by off and on |
| FR-006 | T8, T11 | T8(c) remote start in the machine's own environment; T11(a) the picker; T11(b) the per-machine folder |
| FR-007 | T8, T9, T11 | T8(d) relayed terminal, status, output, action record; T9(a) `unreachable` within 15 seconds and the return within 10; T11(c) the machine name in list and details |
| FR-008 | T6, T10 | T6(c) count, confirm, revoke, close; T10(d) the confirmation flow |
| 0001/FR-005 (modified) | T12, T4 | T12(b) the amended text; T4 machine data crosses only the FR-004 connection |
| 0001/FR-008 (modified) | T6, T12 | T6(b) the credential gate and the own-agents scope; T12(b) the amended text |
| US1 | T2, T3, T4, T5, T6, T10 | the join path end to end in the T8(a) integration run and the T10 Playwright runs |
| US2 | T8, T11 | T8(a) remote terminal use; T11 the picker, folders, machine name |
| US3 | T6, T9, T10 | T9 unreachable and reconnect; T6(c) and T10(d) the safe removal |
| SC-001 | T13 | the loopback join timing plus the hands-on two-machine row |
| SC-002 | T6, T13 | the 50-attempt suite of T6(b) |
| SC-003 | T3, T5, T13 | the reuse, expiry, and lockout suites |
| SC-004 | T2, T8, T13 | the capture proxy run of T13(a) |
| SC-005 | T6, T13 | revocation and cross-machine suites of T6(b), T6(c) |
| SC-006 | T9, T13 | the 15 and 10 second measurements |
| SC-007 | T8, T13 | the 2 second status and 100 ms typing measurements on loopback at the spec's scale of 3 joined machines and 30 agents |
| SC-008 | T7, T13 | the redaction suite and the scan test |
| SC-009 | T2, T8, T13 | the forged short fingerprint run of T13(b) |
| SC-010 | T3, T13 | the generator test and the 1000-address run |
| SC-011 | T8, T13 | the delay-proxy measurement at 50 ms round trip |
| PB-1 to PB-6 | T4, T6, T7, T10, T13 | PB-1: no connection accepted while off in T4(a); PB-2: office data crosses only FR-004 links in T4; PB-3: the machine runs the CLI with its own configuration in T8(c); PB-4: agents keep running without a link in T9; PB-5: keyboard and text states in T10(e); PB-6: owner-only files in T2(a), T8(a) and the redaction suite in T7 |
| Threat model rows | T2 to T7, T13 | one row per threat: transport and fake server in T2 and T8, stranger and guessing in T3, T5, T6, stolen credential in T6, compromised machine in T6(b) (own-agents check and no hub subscription), secrets in T7, listener state in T4 and T10 |
