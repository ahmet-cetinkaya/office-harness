# 0001 data model

The background server stores these entities in one SQLite database (`office.db`) in its data folder. Each module owns the tables that carry its prefix (`agent_`, `terminal_`, `audit_`, `access_`) and keeps its own migration ledger table (`<module>_schema_version`), as `docs/ARCHITECTURE.md` describes. Migrations only go forward, and each one runs in a transaction.

## Office

One per OS user per machine. It is not a table. It is the data folder itself.

| Item | Meaning | Rules |
|---|---|---|
| data folder | Database, token, and server file | Created with owner-only access (FR-009). |
| office token | 32 random bytes, hex-encoded, in the file `token` | Owner-only (FR-009). Regenerating it replaces the file and closes all connections (FR-010). |
| server file | `server.json` with the port, process id, protocol, server version, and start nonce | Owner-only. The desktop app reads it to find the server. |
| lock file | `server.lock`, held with an OS file lock while the server runs | A second server for the same user exits. A missing clean-stop marker at start means an unclean stop (FR-004). |

## Machine

| Field | Meaning | Rules |
|---|---|---|
| id | UUID | Primary key. |
| name | Host name | MVP: exactly one row, created at the first start (A13). |

## Project

| Field | Meaning | Rules |
|---|---|---|
| id | UUID | Primary key. |
| name | Display name | 1 to 64 characters, unique in the office (case-insensitive). |
| folder | Absolute path | Must exist when an agent starts (edge case). |

Delete is refused while agents reference the project. The refusal names those agents.

## Agent

| Field | Meaning | Rules |
|---|---|---|
| id | UUID | Primary key. |
| name | Display name | 1 to 64 characters, unique in the office (case-insensitive). |
| command | CLI program and its fixed arguments | Not empty. The adapter is chosen by the program name: `claude`, `codex`, `gemini`, `opencode`, or generic (research R3). |
| working_folder | Absolute path, or empty to use the project folder | One of the two is required (A6). |
| project_id | Project, optional | Foreign key. |
| machine_id | Machine | Foreign key (A13). |
| status | See the state machine below | Stored. Shown as text (FR-014). |
| created_at | Time | |

## Office additions

Each agent has exactly one row, and the row is deleted together with its agent.

| Field | Meaning | Rules |
|---|---|---|
| flags | Ordered list of extra arguments | Each item is a non-empty string. |
| env | Ordered list of name and value pairs | A name matches `[A-Za-z_][A-Za-z0-9_]*`. A malformed entry is refused and named. |
| startup_prompt | Text, optional | Pasted into the terminal after the first output of a new session, never submitted and never passed as an argument. |
| updated_at | Time | When an edit lands on a running agent, it applies to the next session (FR-023). |

## Session

| Field | Meaning | Rules |
|---|---|---|
| id | UUID | Primary key. |
| agent_id | Agent | Foreign key. At most one running session per agent. |
| cli_session_id | The CLI's own session id, optional | Filled by the adapter. Empty for generic commands. |
| continued_from | Session, optional | Set only when the user explicitly continues a session (FR-025). |
| additions_snapshot | The office additions used at start | Proves FR-023 and SC-009. |
| started_at, ended_at | Time | |
| exit_result | Exit code, or `signal`, `spawn-error: <text>`, `interrupted`, or `lost` (amended by 0013) | `lost`: the session was still open when the server stopped uncleanly; the next server start ends it. A clean stop gives `interrupted`. |

The recent output lives outside the database, in one file per session (`output/<session-id>.log`). It is capped at the recent-output limit (A2) and at 16 MiB, whichever comes first, because full-screen interfaces redraw without newlines. It has owner-only access.

## Action record entry

| Field | Meaning | Rules |
|---|---|---|
| id | Integer | Primary key. |
| actor | `user` in 0001 | A4. 0002 adds agent actors. |
| target_agent_id | Agent | Kept after the agent is deleted, so that the history stays readable. |
| action | `create`, `start`, `stop`, `delete`, `new_task`, or `continue` in 0001. Later changes added `team_create`, `team_delete`, `directive`, `refused` (0002), `hold`, `release` (0007), and `task_text` (0011). The full set of 13 is `ActionKind` in `src/modules/audit/domain/src/action_kind.rs`. | FR-011. |
| result | `ok`, or an error text | |
| at | Time | |

## Settings

Key and value rows: `quiet_period_ms` (default 5000, A1), `output_line_limit` (default 10000, A2), `focus_shortcut` (A12), and `stop_grace_ms` (default 10000, A3).

## Agent status state machine (FR-014, FR-004)

```
(created) ──start──▶ starting ──first output──▶ working
working ──no output for quiet period──▶ waiting for input ──output──▶ working
starting|working|waiting for input ──exit code 0──▶ exited
starting|working|waiting for input ──non-zero exit, signal, or spawn error──▶ failed
starting|working|waiting for input ──user stop──▶ stopped
starting|working|waiting for input ──server stops──▶ interrupted (set at the next server start)
exited|failed|stopped|interrupted ──start or new task──▶ starting
```

The text `waiting for input` is the display text. The stored and wire value is `waiting_for_input`; the other six values are the same word in both places (`src/modules/agent/domain/src/enums/agent_status.rs`).

A new agent that has never run shows `stopped`. Only an explicit start or a new task moves an agent out of `interrupted` (FR-004).

> Amended by 0013: the user's "Resume interrupted" action is such an explicit start; it continues the last session or starts a new one (`specs/changes/0013-bulk-resume/SPEC.md`). After an unclean stop, the session that was open ends as `lost`, not `interrupted`; the agent still shows `interrupted`.
