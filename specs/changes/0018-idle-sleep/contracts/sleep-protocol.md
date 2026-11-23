# Contract: sleep protocol (0018)

Date: 2026-11-23. Extends the 0001 client protocol (`specs/changes/archive/0001-office-server-and-terminal/contracts/client-protocol.md`), the 0007 hold contract (`specs/changes/archive/0007-hold-and-limits/contracts/hold-protocol.md`), and the 0013 resume contract (`specs/changes/archive/0013-bulk-resume/contracts/resume-protocol.md`). Error codes stay `invalid`, `conflict`, `not_found`, `refused`, `internal`.

## Changed shapes

- **Status.** `agent.list`, `agent.status`, `hierarchy.*` rows, and the `agent.subtree` entries (0014) may carry the status `"asleep"`. It is not a running status.
- **Actions.** Action entries (`agent.actions`, `office.actions`, `action.recorded`) may carry the actions `"sleep"` and `"wake"`.
  - `sleep`: actor `office`, `detail` "idle for N minutes". A failed sleep is a `sleep` entry with `failure` set.
  - `wake`: actor `user`. `detail` is "new session: <reason>" when the wake started a new session instead of a continue, otherwise null.
  - A fallback start after a refused continue stays `start` with actor `office` and its 0013 detail.
- **Setting.** `settings.get` and `settings.set` know the key `idle_sleep_minutes`. The value is the text of `0` (off, the default) or of a whole number from 5 to 1440. Any other value is refused with `invalid` and the message "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440"; the stored value stays. A change applies at once.
- **Refusals.** A `directive.send` or `report.send` to an agent that is asleep or going to sleep is refused with the message "the agent is not running, it is asleep: <name>" in place of the not-running message "the agent is not running: <name>". A waiting directive or report of a target that went to sleep is withdrawn with the same text after "withdrawn before typing: ".
- **Notification.** A failed wake holds the agent with the `resume_hold` notification of 0013; its `detail` starts with "the wake failed: ". The desktop shows `resume_hold` as "was put on hold: <detail>".

## Request made by the office only

### `agent.sleep`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to put to sleep |
| `idle_minutes` | integer | the idle time, for the record detail |

The server sends it to itself. A user client gets `invalid` "agent.sleep is made by the office only"; the agent-call allowlist does not name it.

## New requests (user client only)

The agent connection never answers these; they are outside its allowlist and are refused as not allowed (0002 contract). A field that a request does not name below is refused with `invalid` and the message `<field> is not a field of <request>`.

### `agent.wake`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to wake |

The one-agent step: the agent must be `asleep`. The server continues its last session when the CLI can continue it, otherwise it starts a new session. Result: `{session, continued: bool, reason?}` as `agent.resume`. It starts no fallback and no watch; clients use `wake.start`.

Errors: `conflict` "the agent is no longer asleep"; `not_found` for an unknown agent; `refused` "the server is stopping and starts no agent".

### `wake.start`

| Field | Type | Rule |
|---|---|---|
| `agent_id` | string | the agent to wake |

Runs `agent.wake` as the user and answers with its result or its error. After a success, the server watches the start for the 0013 check period on its own task and reports through `wake.result`: a continued session that ends by itself, or does not start, gets one new session (actor `office`); a new session that ends by itself, or does not start, puts the agent on hold (`set_by: "office"`) with one notification.

## New event

### `wake.result`

```
{ type: "wake.result", agent_id, result, continued?: bool, reason?, is_final: bool }
result := "checking" | "continued" | "new_session" | "held"
```

The fields mean what they mean in `resume.result` (0013). Every user client receives it.
