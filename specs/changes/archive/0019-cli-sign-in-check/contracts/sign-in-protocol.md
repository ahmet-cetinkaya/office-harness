# 0019 contract: what the sign-in check changes on the wire

This change adds no request and no event. It changes the values that existing requests and events carry. The base shapes are in the 0001 client protocol (`specs/changes/archive/0001-office-server-and-terminal/contracts/client-protocol.md`).

## Session end results

`exit_result` in `session.list` and in the session of a start answer can hold two more forms:

| Value | Meaning |
|---|---|
| `spawn-error: <reason>` | Already in 0001. When the sign-in status command says the CLI is not signed in, the reason starts with `<cli> is not signed in:` (FR-002, FR-007). |
| `not-signed-in: <reason>` | New. The office ended the CLI at its sign-in screen in the first 20 seconds of the session (FR-005). |

`<reason>` quotes at most one line of what the CLI showed, at most 200 characters, in visible form (FR-007).

## Refusals

On a running agent, `agent.new_task` and `session.continue` answer `{"ok": false, "error": {"code": "refused", "message": "<reason>"}}` when the CLI is not signed in, and the running session goes on (FR-002).

## Action record

- A start that the check refused has the reason as its `result` (failure text), as any failed start.
- A start whose check gave no answer has `detail` `the sign-in check gave no answer: <why>`, where `<why>` is `it timed out after 5 s`, `it could not run: <error>`, `its answer was unclear`, or `the agent changed during the check` (FR-004). A 0013 start that already has a `detail` joins both with `; `.
- The office's end of a CLI at its sign-in screen is a `stop` entry with actor `office` and the reason as its `result` (FR-005).

## Status events

The office's end at a sign-in screen publishes `agent.status` with status `failed` and the reason as its detail, which makes one `failed` notification (FR-005, 0003/FR-006).
