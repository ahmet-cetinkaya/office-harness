# Client protocol v1

This is the interface between `office-harnessd` and its clients. In 0001 the only client is the desktop app. Later releases add more clients (R5) and the same protocol must serve them, so every client authenticates with the office token.

## Discovery

- The server listens on `127.0.0.1` on a free port and writes `server.json` (`{"port": <u16>, "pid": <u32>, "protocol": 1, "version": "<server version>", "start_nonce": "<hex>"}`) to its data folder with owner-only access.
- In 0001 the only client is the desktop app's Rust side. The token never enters the webview. The webview talks to the Rust side through Tauri commands and channels.
- The server refuses any connection whose peer address is not a loopback address (FR-008), and it never binds to another interface in 0001.

## Endpoints

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /v1/health` | none | Returns `{"ok":true,"protocol":1}`. It reveals no office data. |
| `GET /v1/ws` (WebSocket) | office token, challenge and response | All other operations. |

## Authentication (FR-007, FR-010)

The token itself never travels over the socket. A stale `server.json` can point to a port that another local user has taken, so both sides prove that they know the token.

1. **Upgrade checks.** The server refuses the WebSocket upgrade when:
   - the `Host` header is not `127.0.0.1:<port>` (this blocks DNS rebinding);
   - the request carries an `Origin` header (browsers always send one, and the 0001 client is not a browser).
2. **Challenge.** After the upgrade, the server sends `{"type":"auth.challenge","nonce":"<32 random bytes, hex>"}`. The nonce is exactly 64 lowercase hex characters, and the client refuses any other, because the proof joins its parts without separators.
3. **Client proof.** The client answers `{"type":"auth","proof":"<hex HMAC-SHA256(token, nonce || port || "client")>"}`. `port` is the decimal port from `server.json`. Binding the proof to the port stops a process on a stale port from relaying the proof to the real server.
4. **Server proof.** The server checks the proof in constant time. If it is right, the server answers `{"type":"auth.ok","proof":"<hex HMAC-SHA256(token, nonce || port || "server")>"}`. The client checks this proof and closes the connection if it is wrong. A client never sends requests to a server that has not proven itself.
5. **Failure.** The server closes with code `4401` when the proof is wrong, when any other frame comes first, when a frame before `auth.ok` is larger than 1 KiB, or when no proof arrives within 5 seconds.
   When all handshake slots are busy (at most 8 handshakes run at once), the server closes with code `1013` (Try Again Later) before it sends a challenge. This is not a token refusal: the client shows that the server is busy, retries with backoff, and never shows the not-authorized state for `1013`.
6. **No data before auth.** Before `auth.ok`, the server sends nothing except the challenge.
7. **Regeneration.** When the token is regenerated, the server closes every connection that authenticated with the old token, with code `4401`.

## Messages

Every request is a JSON text frame `{"id":<u32>,"type":"<name>",...}`. The response is `{"id":<same>,"ok":true,"result":...}` or `{"id":<same>,"ok":false,"error":{"code":"<code>","message":"<text>"}}`. Error codes are `invalid`, `conflict`, `not_found`, `refused`, and `internal`. A message names the field or entry at fault (edge cases).

| Request | Fields | Result | FR |
|---|---|---|---|
| `project.list` | none | projects | FR-013 |
| `project.create` | name, folder | project | FR-013 |
| `project.delete` | project_id | none (`conflict` lists the agents that still use it) | FR-013 |
| `agent.list` | none | agents with status | FR-014 |
| `agent.create` | name, command, working_folder?, project_id?, additions? | agent | FR-012 |
| `agent.delete` | agent_id | none | FR-012 |
| `agent.start` | agent_id | session | FR-012, FR-024 |
| `agent.stop` | agent_id | none | FR-012, A3 |
| `agent.new_task` | agent_id | session (ends a running session first; the client confirms with the user before it sends this) | FR-024 |
| `agent.additions.get` / `.set` | agent_id, additions | additions plus `applies_next_session: bool` | FR-022, FR-023 |
| `server.restart` | none. Interrupts running agents, so the client confirms with the user first | none | Server upgrade (plan D13) |
| `agent.actions` | agent_id | action record entries | FR-011 |
| `session.list` | agent_id | sessions, each with `can_continue: bool` and, when false, `reason` | FR-025 |
| `session.continue` | session_id | new session | FR-025 |
| `terminal.attach` | agent_id, cols, rows | none. Then the server sends a binary snapshot frame and live frames. The snapshot and the live subscription are taken atomically, so each byte arrives exactly once. For an agent that is not running, the snapshot holds the saved output of its last session and no live frames follow | FR-016, FR-003 |
| `terminal.detach` | agent_id | none | FR-019 |
| `terminal.input` | agent_id, data (base64) | none | FR-017 |
| `terminal.resize` | agent_id, cols, rows | none. A resize to the current size is ignored (research R2) | FR-018 |
| `token.regenerate` | none | none. The caller's connection also closes | FR-010 |
| `settings.get` / `.set` | key, value | setting | A1, A2, A12 |

## Events (server to client, no `id`)

| Event | Fields | FR |
|---|---|---|
| `agent.status` | agent_id, status, detail? | FR-014 |
| `agent.changed` | agent | FR-012 |
| `agent.deleted` | agent_id (an attached client closes its terminal) | Edge cases |
| `project.changed` | cause (`created` or `deleted`), project_id. Sent after a successful `project.create` or `project.delete`, to every client including the sender; a client reads `project.list` again (a project that another client made shows without a restart) | ACF-166 |
| `action.recorded` | entry | FR-011 |
| `agent.additions.changed` | agent_id, additions (another client's open editor refreshes) | Edge cases |
| `storage.error` | message (for example, disk full) | Edge cases |

## Terminal data frames

These are binary frames: 1 byte kind, 16 bytes agent UUID, then the payload.

- Kind `0x01` is a snapshot, sent once after `terminal.attach`. It has three parts: the scrollback rows rendered by the server-side emulator, the screen (`state_formatted()`), and the input modes (`input_mode_formatted()`: cursor keys, bracketed paste, and mouse).
- Kind `0x02` is live output.
- Kind `0x03` is a notice that older output was dropped.

A client that falls behind on terminal frames or on events is disconnected with close code `1011` and must reconnect, reload the agent list, and attach again.

## Terminal queries

The server answers the terminal's cursor-position and device-attribute queries itself, whether or not a client is attached. The client drops xterm.js's own automatic replies to those queries, so a CLI never gets two answers.

## Compatibility

The `protocol` number changes only on a breaking change. Clients refuse to connect to a different number and say why.
