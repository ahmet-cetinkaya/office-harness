---
status: approved
capability: office
plan: specs/changes/0026-agent-secrets/PLAN.md
supersedes: 0002/FR-008 (as modified by 0014 and 0015), 0007/FR-007, 0007/FR-009
---

# 0026: Secrets that reach running agents

## Intent

Today the person gives an agent an API key as an environment variable in the agent's office additions (0001/FR-022). The key sits in the office's storage as plain text, it is applied only at the next session start (0001/FR-023), and a key that changes means a restart of every agent that uses it. This change adds a secret store. The person saves a secret once in the desktop app. The office keeps it sealed at rest, with a key that the OS keyring or an owner-only file protects. A running agent reads the secrets that the person granted to it, at the moment it needs them, through an office command it can call. Replacing a secret reaches every running agent at its next read, with no restart. This change also fixes what may never hold a secret value: logs, the action record, notifications, exports, crash files, and stored terminal output.

## Context

- Source of truth: `docs/PRD.md`. The living spec is `specs/office/SPEC.md`. Ids such as `0001/FR-023` point to requirements there.
- This is row 15 of the gap list: "secrets reach running agents", approved by the owner on 2026-11-23. The gap list asks for the `secure-engineering` contract: threat model, who may read which secret, rotation, deletion.
- The change depends on:
  - 0001: the office token, the owner-only storage (0001/FR-009), office additions (0001/FR-022, 0001/FR-023), the action record (0001/FR-011), recent output (0001/FR-003), and the keyboard rule (0001/FR-028).
  - 0002: the agent connection and its allowlist. An agent proves a per-session credential and calls the office through `office-harnessd agent-call`. The allowlist today is `agent.whoami`, `agent.create`, `agent.start`, `agent.new_task`, `agent.stop`, `directive.send`, `report.send`, and `agent.subtree` (`src/apps/office-harnessd/src/server/agent_call.rs:30-42`). Any other type is `refused`.
  - 0007: the holds and the rate limits for agent-made actions, and the settings keys that carry them (`src/modules/agent/domain/src/value_objects/settings.rs:130`).
  - 0009: export. It blanks every environment value because 0001 has no secret mark (0009 A6). This change does not add a secret mark to environment variables (Out of scope).
  - 0023: the office connection runs with `secure_delete` on (`src/apps/office-harnessd/src/bootstrap/database.rs:15`), so a removed row does not stay readable in freed space.
- Stored recent output is written by the output store, one owner-only file per session (`src/modules/terminal/infrastructure/src/services/output_store.rs:64`). The action record kinds are `src/modules/audit/domain/src/action_kind.rs:10`.
- The server log holds ids, status, and error kinds, and never the token, terminal output, prompts, or environment values (`src/apps/office-harnessd/src/bootstrap/server_log.rs:11`). This change keeps that rule and adds secret values to it.
- The desktop app has no settings screen. Settings sit in dialogs such as the limits dialog (`src/apps/desktop/src/modules/hold/components/limits-dialog.svelte`). The person's "Settings" is a new Secrets dialog next to it (A9).
- The requirements below describe behavior. The shapes of the requests are in `contracts/secrets-protocol.md`. The plan decides the sealing method and the crates.

## Terms

| Term | Meaning |
|---|---|
| Secret | One named value that the person saved in the office: a name, a value, a version, and a list of grants. |
| Value | The text of a secret. Only the person's save request and a granted agent's read carry it. |
| Sealed | Stored encrypted and authenticated, so that the stored bytes show nothing about the value, and a stored record cannot be moved to another secret without detection. |
| Store key | The key that seals every secret. It is never kept in the office database file. |
| Key custody | Where the store key lives: `os_keyring` (macOS Keychain, Windows Credential Manager, Linux Secret Service) or `key_file` (an owner-only file in the office's private data folder). Shown to the person as text. |
| Locked | The state in which the office has sealed secrets and cannot get the store key at this moment, for example a keyring that is locked or not running. |
| Grant | The person's permission for one agent to read one secret. It belongs to the pair of that secret and that agent. |
| Version | A whole number that starts at 1 for a new secret and grows by 1 each time the person replaces the value. |
| Read | One accepted `secret.get` of an agent: it returns the value to that agent. |
| Placeholder | The text `[secret NAME]` that stands for a value in anything the office stores or sends (FR-012). |
| Agent-supplied text | Text that an agent or its processes produced or sent, and that the office stores or sends on: terminal output, a command line of a process (0017), a directive (0012), a report (0015), a task text (0011), a name or objective that an agent gives to a new agent or a temp (0002, 0020), and the texts the office builds from them. Text that the person typed in the desktop app is not agent-supplied text. |
| Agent, action record, hold, limit, office token | As defined in 0001, 0002, and 0007. |

## User stories

### US1: The person saves a secret and grants it to agents (Priority: P1)

The person opens the Secrets dialog, saves the secret `OPENAI_API_KEY`, and grants it to two agents. The dialog shows the name, the version, the time of the last change, the key custody, and the agents with a grant. It never shows the value again.

**Why this priority:** without a saved and granted secret nothing else in this change has a use.

**Independent test:** save a secret with a unique marker as its value and grant it to one agent. The dialog lists the name, version 1, the custody text, and that agent. A byte search of the office's files finds 0 copies of the marker (US5).

**Scenarios:**

1. Given no secret exists, When the person saves the name `API_KEY` and a value, Then the secret exists with version 1 and no grant, and the dialog shows the key custody (`OS keyring` or `key file`).
2. Given a secret, When the person grants it to an agent, Then the dialog lists that agent under the secret.
3. Given a secret with a grant, When the person revokes it, Then the agent is no longer listed, and its next read is refused (FR-007).
4. Given a saved secret, When the person looks for its value anywhere in the app, Then the app shows no way to read it back (FR-004).
5. Given the person types a value, When the person saves or closes the dialog, Then the field is empty (FR-018).

### US2: A running agent reads a secret without a restart (Priority: P1)

An agent that runs now calls the office command, names `OPENAI_API_KEY`, and gets the current value. The person did not restart the agent, and the agent's environment did not change.

**Why this priority:** this is the gap. A key that applies only at the next session cannot be rotated across running agents.

**Independent test:** start an agent on a stand-in CLI that has no grant. Grant a secret while it runs. The agent lists its secrets and reads the value in the same session, and the action record shows one `secret_read` entry that holds the name and the version and not the value.

**Scenarios:**

1. Given a running agent with a grant, When it asks for its secrets, Then it gets the names and versions that it may read, and no value.
2. Given a running agent with a grant, When it reads the secret by name, Then it gets the current value and the version.
3. Given a running agent without a grant, When it reads that secret, Then the office answers `not_found` with the same text as for a name that does not exist (FR-007).
4. Given an agent that holds a grant, When the agent's parent asks for the same secret, Then the parent is refused unless it has its own grant (FR-005).
5. Given the office is on hold, or the agent is on hold (0007), When the agent reads a secret, Then the read is refused with the hold reason (FR-011).
6. Given the person never granted anything to a new agent that a granted agent created, When the new agent reads the secret, Then it is refused (FR-005).

### US3: The person rotates a secret and every running agent gets the new value (Priority: P2)

The key `OPENAI_API_KEY` leaked. The person saves a new value under the same name. All 30 running agents that use it read the new value at their next read. No agent restarts.

**Why this priority:** it is the named use of the gap ("rotating a key without restarting 30 agents"). It builds on US1 and US2.

**Independent test:** grant one secret to 30 running stand-in agents, each of which reads it once. Replace the value. Each agent reads again. All 30 get the new value and version 2, 0 sessions restart, and the dialog shows for each agent the version it read last.

**Scenarios:**

1. Given a secret at version 1 that 3 agents read, When the person saves a new value under the same name, Then the version is 2.
2. Given version 2, When an agent reads, Then it gets the new value; the old value is gone from the store.
3. Given the dialog, When the person looks at one secret, Then each granted agent shows the version it read last and when, or `not read yet`.
4. Given an agent that last read version 1 of a secret that is at version 2, When the person looks at the dialog, Then that agent shows as behind (text, not only color).

### US4: The person deletes, revokes, and resets (Priority: P2)

The person removes a secret, takes a grant away from one agent, or empties the whole store when the store key is lost.

**Why this priority:** the contract asks for deletion and a way out of a lost key. It comes after the value of reads and rotation.

**Independent test:** delete a secret that 2 agents read. Their next read is `not_found`, the dialog lists no such secret, and the office holds 0 sealed records and 0 grants of it. Then reset the store: no secrets remain and the next save makes a new store key.

**Scenarios:**

1. Given a secret with grants, When the person deletes it after the confirmation, Then the secret and its grants are gone and the next read of an agent is `not_found`.
2. Given an agent with grants, When the person deletes the agent, Then its grants are gone and the secrets stay.
3. Given the store is locked and the person chose to reset, When the person confirms with the confirmation that names the number of secrets that will be lost, Then every secret is removed and the store has no store key until the next save.
4. Given a deleted secret, When the person saves a secret with the same name, Then it is a new secret at version 1 with no grants.

### US5: A secret value never leaves the places that are meant to hold it (Priority: P1)

The person saves a secret, an agent reads it and prints it, and the office writes logs, action records, notifications, and exports. None of them holds the value.

**Why this priority:** a secret store that leaks is worse than the plain environment variable it replaces. This is the security contract of the change.

**Independent test:** save a secret with a unique marker. Let a stand-in agent read it, print it to its terminal, put it in a directive and a report, and cause an error. Stop the server and run a byte search over every file under the office's data folder, over the exported templates, and over every message that a recording client received. 0 matches of the marker, outside the one sealed record, which holds no readable text of it.

**Scenarios:**

1. Given a secret, When the office stores it, Then the database file, its `-wal` and `-shm` files, and every other file the office wrote hold no readable copy of the value (FR-002).
2. Given an agent prints the value in its terminal, When the office stores that output, Then the stored output holds `[secret NAME]` in its place (FR-012).
3. Given an agent puts the value in a report or a directive, When the office records and delivers it, Then the action record and every notification and Needs you item hold the placeholder (FR-012).
4. Given a failed save or a failed read, When the office writes the error to a log or an answer, Then it holds no part of a value (FR-013).
5. Given a template export, When the person exports, Then it holds no secret name, value, or grant (FR-014).
6. Given a helper process of an agent that carries the value in its command line for more than 10 seconds, When the server is killed and started again, Then the stored process identity and every sweep entry of the action record show the placeholder in place of the value, and the sweep still ends the process it matched (FR-012).
7. Given a parent that sends a directive or creates an agent or a temp with the value in its text, When the office queues, types, and records it, Then the queue, the child's terminal input, the stored task text, the temp's stored objective, and the records hold the placeholder (FR-012).

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL let the person save a secret with a name and a value in the desktop app, and SHALL show it in a list with its name, its version, the time of its last change, and the agents that hold a grant.
- **FR-002** (US1, US5): THE SYSTEM SHALL keep every secret value sealed, and SHALL NOT write a value as readable text to the office database file, its write log, its shared-memory file, or any other file the office creates. THE SYSTEM SHALL keep the store key outside the office database file.
- **FR-003** (US1, US4): THE SYSTEM SHALL keep the store key in the OS keyring where the OS offers one for the owner (macOS Keychain, Windows Credential Manager, Linux Secret Service) and otherwise in an owner-only key file, SHALL decide this once, when the store key is created, SHALL show the key custody as text in the Secrets dialog, and SHALL NOT move or recreate the store key without the person's reset (FR-015).
- **FR-004** (US1, US5): THE SYSTEM SHALL NOT return a stored value to a user client in any answer, event, export, or screen. The person can replace a value and delete a secret and cannot read a value back.
- **FR-005** (US1, US2): THE SYSTEM SHALL let the person grant a secret to an agent and revoke the grant, SHALL give a grant to no agent until the person does, SHALL NOT pass a grant to an agent's parent, children, or later agents, and SHALL let an agent read only the secrets it holds a grant for.
- **FR-006** (US2): WHEN a running agent asks the office for its secrets, THE SYSTEM SHALL answer the names and versions of the secrets that agent may read. WHEN it reads a granted secret by name, THE SYSTEM SHALL answer the current value and version, in the same session, without a restart of the agent.
- **FR-007** (US2, US4): IF an agent reads a secret that does not exist, was deleted, or was never granted to it, THEN THE SYSTEM SHALL answer `not_found` with one text that is the same in all three cases, so that an agent learns nothing about a secret it may not read. A user client cannot call the read request, and an agent cannot save, replace, delete, grant, or revoke; both are refused as any request outside the caller's allowlist (0002).
- **FR-008** (US3): WHEN the person saves a value under the name of an existing secret, THE SYSTEM SHALL replace the value, raise the version by 1, keep the grants, and answer every later read with the new value. THE SYSTEM SHALL show for each grant the version that the agent read last and the time, or `not read yet`.
- **FR-009** (US4): WHEN the person deletes a secret, THE SYSTEM SHALL remove its sealed value and all its grants. WHEN an agent is deleted, THE SYSTEM SHALL remove its grants.
- **FR-010** (US2, US5): THE SYSTEM SHALL record every read of an agent, accepted or refused, in the action record with the agent as actor, the secret name, and the version when there was one, and SHALL record every save, replace, delete, grant, revoke, and reset of the person with the person as actor and the name, and SHALL NOT put a value in any of these entries.
- **FR-011** (US2): THE SYSTEM SHALL refuse a read while the office or the agent is on hold (0007/FR-004), and SHALL limit the reads of one agent to the number in the setting `limit_secret_reads_per_agent` per rolling 60 seconds (1 to 1000, default 20), as 0007 limits the other actions of an agent. A refusal names the hold or the limit.
- **FR-012** (US5): BEFORE the office stores or sends agent-supplied text, THE SYSTEM SHALL replace each exact occurrence of any stored secret value, also an occurrence that is split across two chunks of terminal output, by the placeholder `[secret NAME]`. This covers every place below, and no place is left to a caller's judgment:
  - (a) stored recent output of a session and the replay shown when the person attaches;
  - (b) action record texts, including the entries of the 0017 sweep and the entries of directives, reports, refusals, and reads;
  - (c) the stored display text of every process identity and every helper identity (0017), where the hash of the whole command line is computed from the original command line before the replacement, so that the sweep still matches the process;
  - (d) notification texts, Needs you items, and reports (0015), including the report line typed into a parent's terminal;
  - (e) the stored task text of an agent (0011);
  - (f) a directive (0012): its entry in a target's queue, the text typed into the target's terminal, and its record entries;
  - (g) the name, objective, and startup prompt that an agent gives to a new agent or a temp (0002, 0020), as stored and as typed, and the finished temp records (0020);
  - (h) events and answers that the office sends to a client and that carry text from (a) to (g).

  It does not cover the live stream to an attached terminal (A3), text that the person typed in the desktop app, or text that the office stored before the secret was saved (A13).
- **FR-013** (US5): THE SYSTEM SHALL NOT put a value, or any part of it, in the server log, in an error text, in a panic message, in a refusal reason, or in a debug print, and SHALL ask the OS not to write a memory dump of the server process where the OS lets a process do so (A5).
- **FR-014** (US5): THE SYSTEM SHALL NOT include a secret name, value, or grant in a template, in an export of templates (0009), or in any other file the person can export.
- **FR-015** (US4): WHEN the store is locked or its store key is lost, THE SYSTEM SHALL let the person reset the store after a confirmation that names how many secrets it will remove. A reset removes every secret, grant, and the store key, and the next save creates a new store key under FR-003.
- **FR-016** (US1): THE SYSTEM SHALL accept a secret name of 1 to 64 characters that starts with a letter A to Z and holds only the letters A to Z, digits, and underscore; a value of 8 to 8192 bytes of UTF-8 text without a NUL character; and at most 100 secrets. It SHALL refuse anything else with the reason named.
- **FR-017** (US1, US4): THE SYSTEM SHALL make every action of the Secrets dialog available from the keyboard (0001/FR-028), SHALL show the state of every grant and every version as text and not only as color, and SHALL confirm a delete and a reset before it acts.
- **FR-018** (US1, US5): THE SYSTEM SHALL show the value field of the dialog as masked input, SHALL empty it when the person saves, cancels, or closes the dialog, and SHALL NOT keep a typed value in the app's saved state, its URL, its logs, or an error text.
- **FR-019** (US2, US4): WHILE the store is locked, THE SYSTEM SHALL refuse every read and every save with the reason `the secret store is locked: <custody>`, SHALL try the store key again at the next request without a server restart, and SHALL NOT overwrite or drop a sealed record.

### MODIFIED

- **0002/FR-008** (as modified by 0014 and 0015) (US2): THE SYSTEM SHALL let a running agent, from inside its own session, create, start, and stop agents, start new tasks, send directives, and read the state of agents only within its own descendants and only for the roles that 0002/FR-001 allows, send reports only to its own parent or to the person, and read only the secrets that the person granted to that agent.
- **0007/FR-007** (US2): IF an agent's agent-made action would exceed one of the rate limits in the table under Assumption A3 of 0007, with the row added by 0015 and the row added by 0026, THEN THE SYSTEM SHALL refuse the action with a reason that names the limit and its value.
- **0007/FR-009** (US2): THE SYSTEM SHALL let the user change each rate limit in the office settings, the limit of reads of secrets by one agent (`limit_secret_reads_per_agent`) included, and SHALL apply a changed value to the next agent-made action.

The table of rate limits under Assumption A3 of 0007 gains one row: reads of secrets by one agent (added by 0026), 20 per rolling 60 seconds by default, because an agent reads each secret once per task and a loop stops.

### PRESERVED

- **PB-1:** The environment variables in office additions keep their behavior: stored as before, applied at the next session start (0001/FR-022, 0001/FR-023), blanked in an export (0009 A6).
- **PB-2:** The agent connection keeps its proof and its allowlist. This change adds `secret.available` and `secret.get` to it and nothing else (0002).
- **PB-3:** The office token never appears in an agent's environment, and a user client still proves the office token (0001/FR-007, 0002).
- **PB-4:** A session's recent output is still stored and replayed in order, capped by lines and by bytes (0001/FR-003, 0001/FR-016); only a secret value in it changes to the placeholder (FR-012).
- **PB-5:** The server log keeps its rule: ids, status, and error kinds, and now never a secret value (FR-013).
- **PB-6:** The action record keeps its entry fields and its display in the agent's details (0001/FR-011); this change adds two kinds (the contract lists them).
- **PB-8:** The 0017 sweep still matches a process by the hash of its whole original command line, and ends it as before, also when the stored display text holds a placeholder (0017/FR-003 to 0017/FR-007).
- **PB-7:** A hold and every rate limit of 0007 keep their text and order for the actions they cover; reads join them as one more agent-made action (FR-011).

## Threat model

Assets: the secret values, and the store key. Trust boundary: the office process and its owner-only folder, with the OS keyring, on the owner's machine (0001/FR-005, 0001/FR-008, 0001/FR-009).

| # | Actor and attack | Answer in this change | Left over, stated plainly |
|---|---|---|---|
| T1 | Another OS user reads the office's files or the keyring. | Owner-only files (0001/FR-009); the keyring item belongs to the owner's login (FR-002, FR-003). | None above 0001/FR-009. |
| T2 | A copy of the office database leaves the machine alone: a backup, a sync folder, a support attachment. | Sealed values; the store key is not in the database (FR-002). With `os_keyring` the copy is useless without the keyring. With `key_file` the copy is useless without the key file, which sits in another file of the same folder. | With `key_file` a copy of the whole data folder opens the secrets. The dialog shows the custody, so the person knows. |
| T3 | A program that runs as the owner reads the keyring item or the key file, or the office token file. | Not stopped. Same boundary as 0001: the office token file is also owner-readable. | A hostile program with the owner's rights can read every secret. This change does not claim otherwise (A2). |
| T4 | An agent that is prompt-injected or hostile reads a secret it should not, or all of them. | Per-agent grants, no inheritance (FR-005); the same `not_found` for denied and missing names (FR-007); holds and a rate limit (FR-011); every read recorded (FR-010). | An agent that holds a grant can use or leak that one value (A1). Revoke the grant and rotate the value. |
| T5 | An agent changes or grants secrets, or reads another agent's secrets. | The agent allowlist has no write request; a user request is not offered (FR-007). | None. |
| T6 | A value reaches a log, an action record entry, a notification, a Needs you item, an event, or an export. | Placeholders at every storing point (FR-012); no value in logs or errors (FR-013); exports hold no secrets (FR-014); events and entries carry names only (FR-010). | None for an exact value. A changed form of a value (base64, split by an agent over several lines) is not recognized (A3). The replacement is a weak guessing oracle (A14). |
| T7 | A value reaches the stored terminal output, because the agent printed it. | Placeholder in stored and replayed output (FR-012). | The live stream to the attached terminal shows what the agent printed (A3). The CLI's own transcript, which the CLI keeps in its own folders, can hold it (Out of scope). |
| T8 | A crash or a panic writes memory or a message to disk. | No value in panic text or errors; the server asks the OS for no memory dump (FR-013). | An OS that writes a dump anyway (A5). |
| T9 | A sealed record is copied over another, or edited. | A sealed record is bound to its secret and its version, so a moved, old, or edited record fails to open and the read is refused with an internal error that holds no value (contract). | None. |
| T10 | The keyring is missing, locked, or replaced. | A locked state, no silent new key (FR-003, FR-019); a reset by the person only (FR-015). | The secrets are unreadable until the keyring returns or the person resets. |
| T11 | A leaked or stale client keeps using a grant. | A grant is checked at every request, so a revoke applies at the next read (FR-005). | A value that the agent already read stays with it (A1). |
| T12 | The desktop app keeps the typed value. | Masked input, cleared at save, cancel, and close (FR-018); the app's saved state and logs never hold it. | A screenshot tool or an input logger on the machine (T3). |

## User experience

- **Flow:** the person opens the Secrets dialog from the same place as the limits dialog. The dialog has a list of secrets and, for the chosen secret, its grants. The person adds a secret, replaces its value, deletes it, grants it to an agent from the list of agents, and revokes a grant. A line at the top shows the key custody, such as `Stored with: OS keyring`, or `Stored with: key file`, and when the store is locked, `Locked: <custody>` with the reset action.
- **Reading agents:** the dialog shows a short note that an agent must hand a secret to a tool through standard input or a file descriptor, as `office-harnessd agent-call secret.get ... --value-only | tool --password-stdin` or `tool --key-file <(office-harnessd agent-call secret.get ... --value-only)`, and never as an argument or inside `$(...)` in an argument, because a command line is stored (0017) and shown in the CLI's own transcript. The office replaces a stored value anyway (FR-012), and the note says that this is a second line of defense. The contract gives the exact helper form.
- **States:** empty (no secret yet, with the one action that adds one), loading, saved, locked (FR-019), error with the reason, delete and reset confirmation (FR-017).
- **Accessibility:** every action works from the keyboard; the dialog closes with Escape; focus returns to the opener; the value field is a labeled password input; status words are text (FR-017).
- No design link exists yet. The plan sets the layout inside the existing dialog patterns.

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Secret | One named value the person saved. | name, version, sealed value, created, last changed | Name is unique in the office. Has zero or more grants. The value never leaves the office except in a read answer. |
| Grant | One agent's permission to read one secret. | last read version, last read time | One per pair of secret and agent. Deleted with its secret or its agent. |
| Store key | The key that seals every secret. | custody (`os_keyring` or `key_file`) | One per office. Created at the first save. Replaced only by a reset. |
| Action record entry (extends 0001) | A read or a change. | two new kinds: `secret_read`, `secret_change` | Names a secret and a version, never a value. |

## Success criteria

- **SC-001:** After 20 secrets are saved, each with a unique marker, and the server stops, a byte search of the database file, its `-wal` and `-shm` files, and every file under the office's data folder (the key file excluded, which holds only the store key) finds 0 of the 20 markers.
- **SC-002:** With 5 agents that read 3 secrets each, print every value to their terminals, send one in a report and one in a directive to a child, give one to a new agent as its task text and one to a temp as its objective, run one helper process whose command line carries a value for more than 10 seconds and then kill the server and start it again, and cause one failed read, a byte search of the same files, of the action record, of every notification and Needs you item, and of every message that a recording client received finds 0 of the markers in any of them, including the stored process identities, the sweep entries, the directive queue, the stored task texts, and the stored temp objectives. The stored output and the records show `[secret NAME]` at 100% of the occurrences that an agent printed whole, and the sweep ends the helper process that carried a value in its command line (0017/SC-005 test).
- **SC-003:** With 30 running agents that each hold a grant and read the secret once, replacing the value makes 100% of the 30 next reads return the new value and version 2, with 0 sessions restarted and 0 environment changes.
- **SC-004:** In 100 reads by agents that hold no grant, that hold a grant for another name, or that name a deleted secret, 100% are answered `not_found` with the same text and none is answered with a value; a read by a user client is not offered.
- **SC-005:** After a delete, a read of the deleted name is `not_found` for 100% of the agents that held a grant, the office holds 0 sealed records and 0 grants of it, and the database file and its `-wal` and `-shm` files hold 0 copies of its sealed bytes after the next server start.
- **SC-006:** With the keyring unavailable at the first save on Linux, the store key is in the key file, the dialog says `key file`, the key file is readable only by the owner (mode 0600 on Linux and macOS, an owner-only access list on Windows), and a later start with the keyring available does not change the custody.
- **SC-007:** With the keyring locked after secrets were saved, every read and every save is refused with the locked reason within 1 second, the sealed records stay unchanged byte for byte, and the first request after the keyring unlocks succeeds without a server restart.
- **SC-008:** In a template export of an office that has 5 secrets with grants, a byte search finds 0 secret names, 0 values, and 0 grants.
- **SC-009:** A person who has the dialog open saves a secret, grants it to an agent, replaces the value, revokes the grant, and deletes the secret, using the keyboard only, in under 2 minutes, and the value field is empty after each save.
- **SC-010:** An agent that reads more than the limit within 60 seconds gets a refusal that names `limit_secret_reads_per_agent` on read 21 (default), and a read while the agent is on hold is refused with the hold reason; both are recorded with the agent as actor.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The person saves a name that exists. | Boundary | It replaces the value and raises the version (FR-008). The dialog says `replace`, not `add`. |
| The name is empty, too long, lower case, or holds a space. | Invalid | Refused with the reason (FR-016). |
| The value is shorter than 8 bytes, over 8192 bytes, or holds NUL. | Invalid | Refused with the reason, and the value is not echoed (FR-016, FR-018). The minimum protects the person's stored output from a replacement of a very common text (A4). |
| The person saves a 101st secret. | Boundary | Refused with `at most 100 secrets` (FR-016). |
| An agent reads a name that is not in the secret name form. | Invalid | `invalid` with the reason that names the field, and no value (contract). |
| An agent sends a field that names another agent, such as `agent_id`. | Unauthorized | `invalid`, recorded; a read never takes a target (contract). |
| An agent reads while it has a grant and the secret is replaced at that moment. | Concurrent | The agent gets either the old or the new value with its version, never a mix (contract). |
| Two saves of one name at the same time. | Concurrent | One wins and the version rises by 2; neither value is lost silently, and the later save is the current one. |
| A revoke arrives while a read runs. | Concurrent | The read that was already answered stays; the next read is `not_found` (FR-005). |
| The server stops during a save. | Failure | The old value stays, or the new one is complete; never a half record (contract). |
| A sealed record cannot be opened (damaged, moved). | Failure | The read is refused with `internal` and no value, the record stays, and the dialog marks that secret as unreadable so the person can replace it (contract). |
| The OS keyring fails at the first save. | Failure | The office uses the key file and says so in the dialog (FR-003). |
| The key file is deleted while the store key was in it. | Failure | The store is locked with the reason `key file missing`; the person can reset (FR-015, FR-019). |
| The agent is deleted while it holds grants. | Boundary | Its grants are removed (FR-009). |
| An agent prints a value in two chunks. | Boundary | The stored output holds the placeholder (FR-012). |
| An agent prints a value in another form, such as base64. | Boundary | Not recognized, stored as printed (A3). |
| A helper process, a directive, a report, a task text, or a temp objective carries a value. | Boundary | Stored and sent with the placeholder (FR-012). The process is still matched by the hash of its original command line (PB-8). |
| Text that the office stored before the secret was saved already holds the same text. | Boundary | Not rewritten (A13). |
| An export includes an agent with a secret environment variable. | Boundary | Unchanged: environment values are blanked (PB-1, 0009). |
| Another OS user, or a program without the office token, asks for a secret. | Unauthorized | Refused as any request without a valid proof (0001/FR-007). |
| The person saves while the office is on hold. | Boundary | Allowed: a hold limits agent actions, not the person (0007). |
| The office restarts with sealed secrets and the keyring entry is gone. | Failure | Locked with the reason; no new key is made (FR-003, FR-019). |

## Assumptions

- **A1:** A value that an agent reads is in that agent's hands. The office cannot take it back, cannot see where the agent puts it, and cannot stop the agent from writing it to a file or sending it to a tool. Revoke and rotate is the answer (FR-005, FR-008). Granting is the person's decision, per agent.
- **A2:** The office defends against copies of its files, other OS users, and agents. It does not defend against a hostile program that runs as the owner, which can already read the office token and the keyring (T3).
- **A3:** Replacement of values (FR-012) is exact-match and best effort: it finds each occurrence of the value as it is, also across chunk borders, and not an encoded or re-split form. It runs at the points where the office stores or sends text. The live stream to an attached terminal is the person's own screen, and changing it would break escape sequences, so it is not changed. The end of a chunk that could be the start of a value is held back for at most 100 ms of silence and then goes to the screen and the stored output as it is, so a screen and an input-line check never wait for more output; a value split across a longer pause is not replaced.
- **A4:** FR-012 replaces text by value. A value of 8 bytes or more is the minimum (FR-016), because a one- or two-character value would replace unrelated text across all stored output, which the person cannot undo. A value that is a common word of 8 or more characters can still match unrelated text; the person chooses the value.
- **A5:** Asking the OS for no memory dump means: core dump size 0 on Linux and macOS for the server process, and on Linux also marking the process not dumpable, which stops a same-user debugger or a read of its memory through `/proc`. Windows has no per-process switch that the office can rely on; the plan records what it can do there, and a dump written by the OS anyway is outside the office's reach.
- **A6:** Key custody per OS: macOS uses the Keychain; Windows uses the Credential Manager, whose items are protected for the logged-in user; Linux uses the Secret Service on the session D-Bus when it answers (GNOME Keyring, KWallet), and falls back to the key file when it does not, for example on a headless machine. The key file sits in the office's private data folder with the same owner-only rule as the office token (0001/FR-009).
- **A7:** The value reaches the server over the office connection, which accepts only the same machine and a valid token (0001/FR-007, 0001/FR-008). It is not encrypted again on that hop.
- **A8:** The value reaches an agent only in the answer of `secret.get`. Nothing is added to the agent's environment or to its startup prompt.
- **A9:** The Secrets dialog is the person's "Settings" for this change. A full settings screen is a separate change.
- **A10:** The person who reads the dialog is the owner. The office has one owner, so no per-person rights exist (0001).
- **A12:** The office knows the values only while it holds them open. After a server start with a locked store it knows none, so it cannot replace a value in text that an agent produced during that time (FR-012). No agent can read a secret while the store is locked (FR-019), so such an agent holds a value only from an earlier read. Revoke and rotate answers it (A1).
- **A13:** The office does not rewrite text that it stored before a secret was saved, or text of a value that was replaced by a newer one: the replacement works on what the office stores from then on. A person who saves a value that an agent already printed should rotate it (A1).
- **A14:** The replacement is a weak oracle: an agent that holds no grant can send a guess in a directive to its own child and see whether it arrives as `[secret NAME]`. For a high-entropy key this tells it nothing in practice, and the minimum length (FR-016) keeps short values out.
- **A11:** The sealing method, the crates, the table layout, and how the placeholder search runs on a stream are for the plan.

## Out of scope

- A secret mark on environment variables, and moving existing environment values into the store.
- A secret that an agent saves, grants, or shares with another agent.
- Grants through templates, teams, or inheritance (0009, 0002).
- Placing a secret in an agent's environment or startup prompt at session start.
- Changing the transcripts that the CLIs keep in their own folders (0001/FR-021).
- Rotating the store key itself while secrets exist (a reset is the way, FR-015).
- Moving the store key between the keyring and the key file.
- Syncing secrets to another machine (0006), and import or export of secrets.
- A settings screen for anything else, and a master password.
- Replacing values in the live terminal stream, or recognizing encoded forms (A3).
- Defending against a hostile program that runs as the owner (A2).

## Clarifications

- Q: Where does the store key live? → A: The OS keyring where there is one, otherwise an owner-only key file, shown as text (FR-003, A6). Owner's brief, 2026-11-23.
- Q: Can the person read a value back? → A: No. The dialog is write-only; a lost value is replaced (FR-004). Accepted under the owner's delegation, 2026-11-23, with the review support of r1.
- Q: Do grants pass to parents, children, or later agents? → A: No. One grant, one agent, set by the person (FR-005). Accepted, 2026-11-23.
- Q: Does the office replace a value in terminal output? → A: Yes, in what it stores and replays, not in the live stream (FR-012, A3). Accepted, 2026-11-23.
- Q: Is a directive that is delivered to a child replaced? → A: Yes (FR-012 f). Decided 2026-11-23 after spec review r1 (M1).
- Q: Minimum length of a value? → A: 8 bytes, refused below (FR-016). Decided 2026-11-23 after spec review r1 (L2).

## Deviations

None yet.
