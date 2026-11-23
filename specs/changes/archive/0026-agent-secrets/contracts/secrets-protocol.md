# Contract: secrets protocol (0026)

Extends the 0001 wire protocol, the 0002 agent connection, and the 0007 holds and limits. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`. Nothing here returns a stored value to a user client.

## Who may call

| Request | User client (office token) | Agent connection (`agent-call`) |
|---|---|---|
| `secret.list`, `secret.set`, `secret.delete`, `secret.grant`, `secret.revoke`, `secret.reset` | yes | not allowed: `refused` with the 0002 not-allowed text, recorded |
| `secret.available`, `secret.get` | unknown type: `not_found`, as for `report.send` (0015) | yes, for the caller only |

The actor of an agent request is fixed to the calling agent, as in 0002. A frame that names `actor` or `caller` is `refused` first. A request that names a field it does not take is `invalid` with `<field> is not a field of <request>`, so a misspelt name never acts on the office.

## Shapes

```
name        := string, 1 to 64 chars, /^[A-Z][A-Z0-9_]*$/
custody     := "os_keyring" | "key_file"
store       := { state: "empty" | "ready" | "locked", custody?: custody, locked_reason?: text }
secret      := { name, version, created_at, updated_at, unreadable: bool, grants: [grant] }   // Unix ms
grant       := { agent_id, last_read_version: number|null, last_read_at: number|null }
```

`state` is `empty` before the first save, `ready` when the store key is available, and `locked` when sealed records exist and the store key is not available. `custody` is absent while `empty`. `locked_reason` is one of `the keyring is locked`, `the keyring is not running`, `the keyring entry is missing`, `key file missing`. No answer carries a value.

## User requests

### `secret.list`

No fields. Result: `{store, secrets: [secret]}`, ordered by name. Works while locked (names and grants are not sealed).

### `secret.set` (FR-001, FR-008, FR-016)

| Field | Type | Rule |
|---|---|---|
| `name` | name | creates when it does not exist, replaces when it does |
| `value` | string | 8 to 8192 bytes of UTF-8, no NUL |

Result: `{secret}` (no value). The first save creates the store key and chooses the custody (FR-003). Errors: `invalid` for the name, the value, or the 101st secret, with the reason and never the value; `refused` `the secret store is locked: <locked_reason>` (FR-019); `internal` when the record cannot be stored (the previous state stays).

### `secret.delete` (FR-009)

`{name}`. Result: `{}`. Errors: `not_found` `no secret named <name>`.

### `secret.grant`, `secret.revoke` (FR-005)

`{name, agent_id}`. Result: `{secret}`. Errors: `not_found` for an unknown name or agent; `conflict` `the agent already holds this grant` or `the agent holds no grant`. Both work while locked.

### `secret.reset` (FR-015)

`{confirm_count: number}`: the number of secrets the person saw in the confirmation. The server compares it with the number it holds; when they differ it answers `conflict` with the current count and removes nothing. Result: `{removed: number}`. Allowed in any state.

## Agent requests

### `secret.available` (FR-006)

No fields. Result: `{secrets: [{name, version}]}` for the secrets the caller holds a grant for, ordered by name. An empty list is a normal answer. It reads no value, writes no action entry, and does not count toward a limit; a hold on the office or the agent still refuses it (FR-011).

### `secret.get` (FR-006, FR-007, FR-010, FR-011)

| Field | Type | Rule |
|---|---|---|
| `name` | name | the secret to read |

Result: `{name, value, version}`. The server opens the record, checks the grant, and answers in one step, so an answer is always one version, whole.

Errors:

| Code | When | Text |
|---|---|---|
| `invalid` | `name` missing or not in the name form; a field that is not taken. | names the field, never a value |
| `not_found` | The secret does not exist, was deleted, or the caller holds no grant. One text for all three. | `no secret named <name> is available to this agent` |
| `refused` | The office or the agent is on hold; the reads limit is reached; the store is locked. | the 0007 hold text; `reads of secrets by one agent` limit text; `the secret store is locked: <locked_reason>` |
| `internal` | The sealed record cannot be opened (damaged or moved). | `the secret <name> cannot be read`; no value, no detail of the failure |

Every `secret.get` is recorded (FR-010), accepted or refused. Refusals for a hold or the limit follow the 0007 order; a `not_found` and an `invalid` count toward the limit, so that guessing names is limited too.

### Helper

`office-harnessd agent-call secret.get '{"name":"NAME"}' --value-only` prints the value alone, with no line break and no JSON, to standard output, and nothing else. On any error it prints the error text to standard error and exits with a non-zero status, and prints nothing to standard output. Without `--value-only` the helper prints the result JSON as it does for other requests. `--value-only` is accepted only for `secret.get`. The intended use is a pipe into a tool's standard input (`... --value-only | tool --password-stdin`) or a file descriptor (`tool --key-file <(...)`), never an argument and never `$(...)` inside an argument: a command line is stored (0017) and shown in the CLI's transcript.

## Settings (amends 0007 `hold-protocol.md`)

A new key `limit_secret_reads_per_agent`, a whole number from 1 to 1000, default `20`: the `secret.get` requests one agent may send per rolling 60 seconds. A refusal names it "reads of secrets by one agent". `secret.get` is an agent-made action for the holds and the limits.

## Action record

Two new kinds. An entry never holds a value.

| Kind | Entry's agent | Actor | Text |
|---|---|---|---|
| `secret_read` | the caller | the caller | `<name> v<version>` when accepted; `<name>: refused, <reason>` when refused; `<name>: not available` for `not_found`; the reason text for `invalid` and `internal` |
| `secret_change` | the agent for a grant or revoke; none for the others | the person | `saved <name> v1`, `replaced <name> v<n>`, `deleted <name>`, `granted <name>`, `revoked <name>`, `reset <count> secrets` |

## Event

`secret.changed`: `{cause: "saved" | "replaced" | "deleted" | "granted" | "revoked" | "reset" | "read" | "store", name?: name, agent_id?: string}`. Sent after the change succeeded, to every user client. `read` is sent after an accepted read, so the dialog shows the last read version. `store` is sent when the state of the store changes between `ready` and `locked`. Clients re-read `secret.list`. No event holds a value.

## Sealing and storage rules (binding on the plan)

- The value is sealed with an authenticated cipher under the store key and a fresh random nonce for every seal, including every replace. The secret's per-secret random id, its name, and its version are bound to the record as authenticated data, so a record moved to another secret, or an old version put back after a rotation, fails to open. The record holds the nonce and the ciphertext only.
- The store key is a random 256-bit key made by the OS random source. It is never written to the office database, to the server log, or to an event. In `key_file` custody it is written with the private-file rule of 0001/FR-009 and replaced atomically.
- A save writes the new sealed record and the version in one transaction. A replace seals the new value with the new version in the associated data. The server stops during it with the old record or the new one in the database, never half of one.
- A delete and a reset remove the rows in one transaction. The office connection runs with `secure_delete` on (0023), and a reset ends with a write-log checkpoint that truncates the log.
- A value held in memory for a seal, an open, or a replacement search is held in a buffer that is cleared when it is dropped. A value type has no `Debug`, `Display`, `Serialize`, or `Clone` that shows it; its printed form is `[secret]`.
- The replacement of values (FR-012) is one function that every storing point listed in FR-012 calls. For process identities it runs on the display text only; the command hash is computed from the original command line before it. A storing point that is not on the list is a defect of this change, and a test enumerates the stored agent-supplied text columns and the send paths against the list. It holds the values of all secrets, opened when the store is ready at a server start and kept until the secret changes, is deleted, or the process ends, and it refreshes at each save, replace, delete, and reset. While the store is locked from the start, it holds no value and cannot replace one (A12).

## Amends

- 0002 `hierarchy-protocol.md`, "Agent connection": `secret.available` and `secret.get` join the allowed types; the not-allowed refusal text names secrets among what an agent may not change.
- 0001 data model: two action kinds, `secret_read` and `secret_change`.
