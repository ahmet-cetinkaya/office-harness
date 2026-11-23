# 0019 research: sign-in status of the supported CLIs

Checked on 2026-11-23 on Linux with the installed CLIs. Each finding names the CLI version. Only `--help`, the read-only status command, and a text search of the installed program were used. No sign-in, sign-out, or other account action was done.

## R1: The read-only status command of each CLI

| CLI | Version | Status command | Signed in | Not signed in |
|---|---|---|---|---|
| `claude` | 2.1.322 | `claude auth status --text` (`--json` is the default output) | exit 0 | exit 1, text `Not logged in. Run claude auth login to authenticate.` |
| `codex` | from npm, Linux x64 | `codex login status` | exit 0, a line that starts with `Logged in using` (the program holds `Logged in using ChatGPT`, `Logged in using an API key - `, and five other forms) | exit 1, text `Not logged in` on the error stream |
| `gemini` | 0.70.0 | none: `--help` lists `mcp`, `extensions`, `skills`, `hooks`, `gemma`, and `models`, and no sign-in status | | |
| `opencode` | 2.2.5 | none used: `auth list` lists stored credentials, but the CLI can use models without a stored credential, so an empty list does not mean "cannot work", and every command of this CLI writes its own log file and uses its background service | | |

Notes:

- The `codex` signed-in line with an API key ends with a masked part of the key. A signed-in answer is therefore never shown or recorded.
- A CLI version without the status subcommand ends with a non-zero exit and a usage text. A non-zero exit alone does not mean "not signed in".

## R2: A status command can write the CLI's own files

- `claude auth status`, run with an empty home folder, created `.claude.json`, `.claude.json.lock`, and `.claude/` in that home folder.
- `codex` tries at each run, also for `--help` and `login status`, to create helper links in its own folder, and prints a warning when it cannot.
- `opencode` writes its own log file at each run, also for `--help`.

These are writes of the CLI's own program into its own folders, the same as at any start of the CLI. The office itself opens no file of a CLI for the check.

## R3: Time of a status command

| Command | Measured |
|---|---|
| `claude auth status` | 0.09 s to 2.3 s (the first run after a pause was the slowest) |
| `codex login status` | 0.03 s |

## R4: Sign-in screens

Words that the installed programs show on their sign-in screen (a text search of the installed program and of a first screen in an empty home folder):

| CLI | Words |
|---|---|
| `claude` | `Claude account with subscription`, `Anthropic Console account` (the choice of a sign-in method), `Not logged in · Run /login`, `Please run /login` |
| `codex` | `Sign in with ChatGPT`, `Provide your own API key` |
| `gemini` | `How would you like to authenticate for this project?`, `Waiting for authentication...` |
| `opencode` | none found |

The words can change with a CLI version. A word that is not found makes the watch miss the screen; the start then behaves as before this change.
