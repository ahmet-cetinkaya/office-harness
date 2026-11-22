# File view protocol (0016)

Six new requests on the user connection (the desktop app's connection of 0001, after the office token). The envelope and the error codes are the 0001 ones. The agent connection (0002) does not offer them: an agent that sends one gets the not-allowed refusal of its connection, recorded as for every type outside its allow list (PB-1).

Every request carries `agent_id`. The server reads the root again on each request: the agent's working folder, or its project's folder when it has none, canonicalized (spec Terms, A1). Nothing is cached between requests, and no request writes anything or publishes an event (FR-007, PB-4).

## Paths

A `path` field is relative to the root and uses `/` between its parts. The server refuses it as `invalid` when it is absolute, starts with `/`, holds an empty part, a `.` part, a `..` part, or a zero byte, or, on Windows, holds `\` or `:`. The root itself is the empty string `""`, accepted only by `agent.files.list`.

Before it reads, the server resolves the path, links included, and refuses it as `refused` ("the path is outside the agent's working folder") unless the resolved path is the root or lies below it (FR-003). A path that does not exist on disk (a deleted file in a diff) is resolved through its deepest existing parent; the parts after that parent do not exist, so no link can be among them. A path whose resolved form holds a part named `.git` (compared without regard to ASCII case) is refused the same way: the git data is never listed or read.

Names and paths in answers are text. A name that is not valid UTF-8 arrives with each invalid sequence replaced by U+FFFD; such an entry cannot be opened, because its path does not round-trip (Edge cases).

## Content encoding

File content and diffs travel as `content_base64`: the raw bytes, standard base64 with padding. The size limit is 524,288 raw bytes (512 KiB), so the encoded answer stays under the 1 MiB frame limit of 0001. The desktop decodes the bytes as UTF-8 with replacement (A9).

## Answer budget

A list answer (`entries`, `files`, `commits`) stops adding items before its encoded size would pass 768 KiB, and counts every item it left out in `more`. With the caps below this cut is reached only by names full of escaped characters.

## Requests

### `agent.files.list`

Fields: `agent_id`, `path` (`""` for the root).

Result:

| Field | Type | Meaning |
|---|---|---|
| `root` | string | The root, as text, for the header. |
| `path` | string | The listed folder, as asked. |
| `entries` | array | At most 1,000 entries: folders first, then the rest; each group sorted by name, by Unicode code point. |
| `more` | number | Entries left out. |
| `read_at` | number | Unix ms of the read. |

Entry: `{name, kind, size, secret, target}`. `kind` is `folder`, `file`, `link`, or `other` (a pipe, a socket, a device). `size` is the byte size for a `file`, else `null`. `secret` is `true` for a secret-looking name (spec Terms). `target` is `null` unless `kind` is `link`; then it is `folder`, `file`, or `other` when the link resolves inside the root, `outside` when it resolves outside, and `broken` when it is missing or a loop. An entry named `.git` is never listed.

Errors: `not_found` (no such agent), `invalid` (path form; the path is a file), `refused` (outside the root), `internal` (the folder cannot be read; the message names the reason, for example that it does not exist or that access was denied).

### `agent.files.read`

Fields: `agent_id`, `path`, `reveal` (boolean, default `false`).

Result: `{path, state, size, content_base64, read_at}`.

| `state` | When | `content_base64` |
|---|---|---|
| `secret` | The requested name or the resolved name is secret-looking and `reveal` is not `true`. Checked first. | `null` |
| `too_large` | `size` is over 524,288. | `null` |
| `binary` | The first 8,000 bytes hold a zero byte. | `null` |
| `text` | Otherwise. An empty file is `text` with `size` 0. | the bytes |

Errors: as for `agent.files.list`; `invalid` also for a folder or an `other` entry ("not a regular file"), which is never opened.

### `agent.files.changes`

Fields: `agent_id`.

Result: `{repository, files, more, read_at}`. `repository` is `false` when the root is not inside a git work tree; then `files` is `[]`.

File: `{path, kind, old_path, secret}`. `path` is relative to the root. `kind` is `modified`, `added`, `deleted`, `renamed`, `type_changed`, `untracked`, `conflicted`, or `submodule`. `old_path` is set only for `renamed`, and only when the old path is inside the root. At most 1,000 files, sorted by path; `more` counts the rest. Ignored files are not listed.

Errors: `not_found`, `internal` (git cannot start, does not answer in 10 seconds, or lists more than 8 MiB; the message says which).

### `agent.files.diff`

Fields: `agent_id`, `path`, `old_path` (optional, for a rename), `commit` (optional), `reveal` (boolean, default `false`).

Without `commit`: the change of the path in the root against HEAD, staged and unstaged together. For an untracked file the server builds the diff itself: every line added. With `commit`: the change of the path in that commit against its first parent, or against nothing for a commit without a parent.

Result: `{path, state, size, content_base64}`.

| `state` | When | `content_base64` |
|---|---|---|
| `secret` | The path or `old_path` is secret-looking and `reveal` is not `true`. | `null` |
| `too_large` | The diff is over 524,288 bytes; `size` is its size when known, else `null`. | `null` |
| `binary` | git reports a binary change, or the untracked file is binary. | `null` |
| `none` | No change of that path. | `null` |
| `diff` | Otherwise: a unified diff with three lines of context, without git's header lines before the first `@@`. | the bytes |

Errors: `invalid` (path form; `commit` not 40 or 64 lowercase hex characters, or not a commit of the repository), `not_found`, `refused`, `internal`.

### `agent.files.history`

Fields: `agent_id`, `skip` (number, default 0, at most 1,000,000).

Result: `{repository, commits, more}`. `commits` holds at most 50 commits of HEAD that change a path inside the root, newest first, after skipping `skip` of them. `more` is `true` when at least one more exists. A repository without a commit gives `[]`.

Commit: `{id, short_id, subject, author, time}`. `subject` is the first line of the message, at most 200 characters. `time` is the author time in Unix ms.

Errors: `invalid` (`skip`), `not_found`, `internal`.

### `agent.files.commit`

Fields: `agent_id`, `commit`.

Result: `{id, parents, author, email, time, message, files, more}`. `message` is the full message, at most 65,536 bytes. `files` uses the shape of `agent.files.changes`, against the first parent, limited to paths inside the root.

Errors: `invalid`, `not_found`, `internal`.

## Git runs

Every git run of these requests is a read run of the office's git runner (0010): no shell, a cleared `GIT_*` environment, no hooks, `core.fsmonitor=false`, `GIT_OPTIONAL_LOCKS=0`, no transport, a 10-second timeout. In addition it passes `--literal-pathspecs`, `--no-ext-diff`, `--no-textconv`, `--no-show-signature` where the command accepts them, `-c log.showSignature=false`, and, for every filter driver that the configuration names, empty `clean`, `smudge`, and `process` commands with `required=false` (FR-007). A commit is passed to git only after it matches the hex form above, so it can never be read as an option.

## Amends

None. The agent connection's allow list (0002, 0014) does not change.
