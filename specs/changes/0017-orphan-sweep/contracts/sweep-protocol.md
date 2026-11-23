# Contract: orphan sweep protocol (0017)

Date: 2026-11-23. Extends the 0001 wire protocol, the 0013 resume contract, and the 0003 notification list. Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

After an unclean stop, the server finds the stored processes of the `lost` sessions and ends those that it can prove are its own. This work is the sweep. The server binds its listener first, so a client can ask about the sweep. The sweep runs only when at least one `lost` session has a stored open process.

## New request (user client only)

The agent connection never answers this request. A field that the request does not name is refused with `invalid` and the message `<field> is not a field of sweep.state`.

### `sweep.state`

No fields. Result: `{state: "running" | "done"}`.

- `running`: the sweep has started and has not ended.
- `done`: the sweep has ended, or no sweep was needed.

The state never goes back from `done` to `running` while the server runs.

## New event

### `office.sweep`

```
{ type: "office.sweep", state: "running" | "done" }
```

The server sends `running` when the sweep starts and `done` when it ends. Every user client receives it. The server sends neither when no sweep is needed. A client that connects during the sweep does not get an event for the past: it reads `sweep.state`.

## Requests that wait

While the state is `running`, these requests wait. They run after the sweep ends.

- `agent.start`
- `agent.new_task`
- `session.continue`
- `agent.resume`
- `resume.start`

All other requests answer at once. If the server stops during the wait, the waiting request ends with `refused` and the message `the server is stopping and starts no agent`.

## Changed shape: `resume.plan`

While the sweep runs, `resume.plan` answers at once and lists no agent:

```
{ sweeping: true, agents: [] }
```

After the sweep, it answers `sweeping: false` and the list:

```
{ sweeping: false,
  agents: [ { agent_id, last_session: session?, can_continue: bool, reason?, held: bool,
              unclean_stop: null | {ended: n} | {may_run: reason},
              after_unclean_stop: bool } ] }
```

`unclean_stop` is `null` unless the last session of the agent is `lost`. For a `lost` session:

- `{may_run: reason}`: a CLI or helper process of the session may still run. `reason` is the first of this list that holds:
  1. `could not end`: the sweep tried to end a stored process and failed.
  2. `not readable`: the sweep could not read a stored process, or the OS refused to open it. Only "no such process" counts as `not running`; every other open failure is `not readable`.
  3. `boot time changed`: the process matches in all but the boot time (macOS and Windows).
  4. `no stored process`: the session has no stored CLI identity (helper rows alone do not count).
  5. `helper limit`: the session reached the limit of 200 stored helpers.
  6. `not swept`: a stored process has no sweep result.
- `{ended: n}`: no `may_run` reason holds, and the sweep ended `n` processes (`ended` and `ended by force` both count). `n` is at least 1.
- `null`: no `may_run` reason holds, and the sweep ended nothing. Every stored process was gone or was another process.

`may_run` wins over `ended`.

`after_unclean_stop` stays for an older desktop app. It is `true` exactly when `unclean_stop` is `{may_run: ...}`, and `false` otherwise. It was `true` for every `lost` session in 0013.

## Action record

Both kinds below are `ActionKind` values. Their actor is `office`. They use the `detail` field that 0013 added to action entries (`agent.actions`, `office.actions`, `action.recorded`).

### `end_leftover`

One entry for each stored process that the sweep handled. The target is the agent of the session.

Detail format:

```
identity <row id>: <result>: pid <pid>, session <session id>, <command in visible form>
```

`<result>` is one of:

| Result | Meaning | Action result |
|---|---|---|
| `ended` | The process matched and ended after the end request. | ok |
| `ended by force` | The process matched and ended by force (after the grace period, or at once on Windows, where a detached process has no end request). | ok |
| `could not end: <reason>` | The process still matched after the forced end, or the OS refused. `<reason>` is the OS error text. | failed, with the reason |
| `left: <reason>` | The process was not ended. The reasons are `not running`, `not readable`, `machine restarted`, `other start time`, `other command line`, and `boot time changed`. | ok |

The sweep ends a process only when the start time, the command line hash, and the machine run all match. The OS handle that the sweep opened ends the process. A process that exits after the end request counts as ended.

The result text is in visible form too.

### `helper_limit`

One entry when a running session reaches 200 stored helper processes and the office stops storing more. The target is the agent of the session. Detail:

```
the office stores at most 200 helper processes for a session and stopped storing more
```

The entry is written once for each session and server run. The session is then marked, and a sweep after an unclean stop gives `may_run` with the reason `helper limit` for it.

## Visible form of a command line

The server stores the first 1,000 characters of a command line, and the hash of the whole line. The detail shows the stored text in visible form:

1. Each Unicode control character (category Cc) and format character (category Cf) is written as `\u{<hex>}`, with the code point in lower-case hex, and so are U+2028 and U+2029. The format characters include the zero-width characters (U+200B to U+200D, U+FEFF), the soft hyphen U+00AD, and the bidirectional controls (U+061C, U+200E and U+200F, U+202A to U+202E, U+2066 to U+2069). The server and the desktop escape the same set.
2. If the stored text has 1,000 characters or more, the line was cut. The server then adds an ellipsis (`…`) and the first 8 hex characters of the SHA-256 hash of the whole line.

## Changed shape: notifications

`notification` entries, `notification.created`, and Needs-you items: `event` and `kind` may be `"leftover_kept"`.

- The sweep writes one row for each agent that has a `could not end` result, not one for each process.
- `detail` is the OS reason of the first such result, with control characters escaped as above.
- The Needs-you item uses the attention kind `leftover_kept`.
- The desktop app shows one OS notification for each server start. It names the agents of those rows.

This extends the event list of the archived panel protocol (0003), the hold protocol (0007), and the resume protocol (0013). Those archived files stay as they were.

## Stored data

Table `agent_session_processes` (agent migration `0008_session_processes.sql`), in the office database:

| Column | Rule |
|---|---|
| `id` | integer key |
| `session_id` | the session, `ON DELETE CASCADE` |
| `pid` | process id |
| `started_at` | process start time in the unit of the OS: clock ticks (Linux), seconds (macOS), or 100 ns FILETIME units (Windows) |
| `command_hash` | SHA-256 of the whole command line |
| `command_display` | the first 1,000 characters of the command line |
| `boot` | the machine run: the boot id (Linux), or the boot time in seconds (macOS, Windows) |
| `kind` | `cli` or `helper` |
| `state` | `open` or `ended` |
| `sweep_outcome` | `ended`, `could_not_end`, `not_readable`, `boot_time_changed`, `left`, or null |

The key `(session_id, pid, started_at)` is unique. `sweep_outcome` holds what the sweep found, and `resume.plan` reads it. It is null for a row that a session end or a helper end closed. `ended` and `ended by force` both give `ended`; every `left` reason except `not readable` and `boot time changed` gives `left`.

The session row also has `helpers_capped`, set when the helper limit stopped the storing of helpers.
