# 0026 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0026-agent-secrets` with T1 to T6 and `origin/main` 2b80653 merged (flake files only). The gate numbers in the table come from local runs on that tree.
- **Overall status:** automated tests prove every requirement and criterion that can run on Linux. Four things are PARTIAL or NOT RUN and are named below: the real OS keyring on macOS and Windows, a memory dump on Windows (FR-013, spec A5), the 1-second bound of SC-007 with a real keyring, and a hands-on run with a real signed-in CLI.
- **Status of this file:** the CI result is in the PR, not here.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap. |
| NOT RUN | No check ran. The evidence column says why. |

### Scope of the automated evidence

- `tests/agent_secrets.rs`, `tests/agent_secrets_redaction.rs`, and `tests/agent_secrets_scale.rs` run the server inside the test process with fake CLIs (Unix shell scripts) and run the real `office-harnessd agent-call` helper as the agent. They use the key file custody: `disable_os_keyring()` makes the keyring unavailable so that no test touches the developer's own OS keyring.
- The real keyring is covered by one smoke test (`a_key_round_trips_through_the_os_keyring_where_one_exists`, `secret-infrastructure`). It stores, reads, and removes a key under a test account name and prints why it skips where the OS offers no keyring. On this machine (Linux, Secret Service reachable) it ran against the real keyring.
- Every byte scan reads each file under the office data folder (the database, its `-wal` and `-shm` files, `server.log`, `output/`, `secrets.key` where it exists), and, where a criterion names them, the action record, the notifications, the Needs you items, a recording client, and an export. The key file holds only the store key, so it is excluded from the scans for values.
- The marker of each secret is unique and appears in no other text of a test.
- `secret-infrastructure/tests/secret_service.rs` runs the service over a real SQLite file with a fake keyring that can refuse, lock, or lose its entry, the real sealer, and the real key file vault.
- The desktop is covered by 16 unit specs (`src/modules/secrets`), the Playwright specs `e2e/secrets.spec.ts` (9) and `e2e/hold.spec.ts` (24, the limits dialog gained a row), and the whole e2e suite (316).

### Local gates (2026-11-23)

| Gate | Scope | Result |
|---|---|---|
| `cargo fmt --all --check` | workspace | PASS |
| `bash scripts/check-boundaries.sh` | workspace | PASS (`boundaries ok`) |
| `cargo clippy --workspace --exclude office-harness-desktop --all-targets -- -D warnings` | workspace | PASS, no warnings. One pre-existing `must_use_candidate` finding of the newer local clippy in `cli/run.rs`, a file this change does not touch, is allowed locally (`-A clippy::must_use_candidate`) |
| `cargo test --workspace --exclude office-harness-desktop --no-fail-fast -- --test-threads=3` | workspace | PASS: 1889 passed, 0 failed, 123 result lines, on the tree with `main` 2b80653 merged |
| `bun run check`, `bun run lint`, `bun run format:check` | `src/apps/desktop` | PASS: 0 errors, 0 warnings, all files formatted |
| `bun test src` | `src/apps/desktop` | PASS: 489 passed, 0 failed |
| `bunx playwright test` | `src/apps/desktop` | PASS: 316 passed (secrets 9, hold 24) |

### Existing tests that this change changed

| Test | Change |
|---|---|
| `a_changed_setting_applies_to_the_next_call` (`bridges/rate_limit_source.tests.rs`), `rate_windows.tests.rs`, `settings.tests.rs`, `action_record.tests.rs` | Include the new setting `limit_secret_reads_per_agent`, the new window, and the two new action kinds (count 20 to 22). |
| `hold.spec.ts` (e2e), `hold.spec.ts` and `hold-controller.spec.ts` (unit) | The limits dialog lists six limits; the sixth is the secret reads limit. |
| `tests/support/running_office.rs` `serve_again` | Boxes the future of `serve_again_with`: the services of the office grew past the large-future threshold of clippy. |

## Success criteria

| SC | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `twenty_secrets_leave_no_value_in_any_file_of_the_office` (`tests/agent_secrets_scale.rs`): 20 secrets with 20 unique markers; 0 of 20 in any file of the data folder after the stop. |
| SC-002 | PASS (automated) | `agent_secrets_redaction.rs`: a CLI prints the value whole and in two parts (split by a 1 s pause) and the stored output holds `[secret API_KEY]` at both; a helper process carries the value in its command line for more than 10 s and the stored identity holds the placeholder (the hash is of the original, PB-8); a report, a directive that is refused, and a field named by the value reach the action record, Needs you, and the notifications with the placeholder; every file scan finds 0 markers. Gap: the unclean stop (a kill of the server) is not run in this change: the sweep entries copy the stored display text, which is stored redacted, and `drop_ended_commands.rs` and `server_sweep.rs` keep proving the sweep path. |
| SC-003 | PASS (automated) | `a_new_value_reaches_thirty_running_agents_at_their_next_read_without_a_restart`: 30 running stand-in agents read version 1, the person saves a new value, all 30 read version 2 and the new value; each agent still has 1 session; the list shows `last_read_version` 2 for all 30. The reads run in groups of 6, because the server lets 8 handshakes run at once (0001). |
| SC-004 | PASS (automated) | `a_hundred_reads_of_what_an_agent_may_not_read_all_get_one_text_and_no_value`: 100 reads (ungranted, missing, deleted names): 100 are `not_found` with one text (after the name is replaced), none carries a value; a read by a user client is `not_found`. |
| SC-005 | PASS (automated) | `a_delete_removes_the_secret_its_grants_and_the_sealed_bytes` (`secret_service.rs`): after the delete, the grants are gone, the next read is `not_found`, a new secret of the same name is version 1 with no grants, and the sealed bytes are not in the database file or its log after the checkpoint that every start runs (0023). |
| SC-006 | PASS (automated) on Linux; NOT RUN on macOS and Windows | `without_a_keyring_the_key_goes_to_an_owner_only_file_and_the_list_says_so`: the list says `key_file`, the file has 32 bytes and mode 0600; `the_custody_is_never_chosen_again_when_the_keyring_returns_later`: the custody does not change. The owner-only access list on Windows is the one of `private-fs` (0001/FR-009) and is covered by CI there. |
| SC-007 | PARTIAL (automated) | `a_locked_store_refuses_reads_and_saves_and_changes_no_record`: with a keyring that locks after it took the key, every read and save is refused with the locked reason, the sealed record is unchanged, and the next read after the unlock succeeds without a restart. Gap: the 1-second bound was not timed, and the lock is a fake keyring, not a locked real one. |
| SC-008 | PASS (automated) | `an_export_of_an_office_with_secrets_holds_no_name_value_or_grant`: 5 secrets with 5 grants on one agent, a template made from that agent, `template.export`: 0 names, 0 values, 0 agent ids. |
| SC-009 | PASS (automated) | `e2e/secrets.spec.ts`, the keyboard test: save, grant, replace, revoke, and delete with Tab, Enter, and typed text only, in under 2 minutes (the test takes under 1 s); the value field is empty after each save; the page never holds a value. |
| SC-010 | PASS (automated) | `the_reads_limit_and_a_hold_refuse_a_read_and_the_refusal_is_recorded`: with the limit 2, read 3 is refused and names `reads of secrets by one agent`; a hold refuses the next read with `on hold`; both are recorded as failed `secret_read` entries with the agent as actor. The unit tests of `rate_windows` prove the window. |

## Requirements

| Requirement | Verdict | Evidence |
|---|---|---|
| FR-001, FR-016 | PASS (automated) | `secret-domain` name and value rules (boundaries 8 and 8192 bytes, NUL, names); the 101st secret is refused (`the_101st_secret_is_refused`); `e2e` short value and bad name are refused on the page. |
| FR-002 | PASS (automated) | `the_first_save_chooses_the_keyring_and_the_value_is_not_in_the_files`, SC-001, `a_sealed_value_opens_to_the_same_value`, `the_record_shows_nothing_readable_of_the_value`. |
| FR-003, FR-019 | PASS (automated) | SC-006, SC-007, `a_lost_keyring_entry_is_locked_and_no_new_key_is_made`, `a_missing_file_is_locked_and_never_a_new_key`. |
| FR-004 | PASS (automated) | `list` answers carry no value (asserted in `agent_secrets.rs`); a user client cannot call `secret.get`. |
| FR-005, FR-007 | PASS (automated) | `an_agent_lists_only_what_it_may_read_and_revoke_takes_effect_at_once`, `a_missing_a_deleted_and_an_ungranted_secret_read_the_same`, `an_agent_cannot_change_secrets_and_a_user_client_cannot_read_one`, SC-004. |
| FR-006, FR-008 | PASS (automated) | `a_granted_secret_reaches_a_running_agent_and_a_new_value_needs_no_restart`, SC-003, `a_replace_raises_the_version_keeps_the_grants_and_answers_the_new_value`, `the_dialog_data_shows_the_version_each_agent_read_last`. |
| FR-009 | PASS (automated) | `the_grants_of_a_deleted_agent_go`, `the_clients_hear_of_a_change_by_name_and_the_grants_go_with_a_deleted_agent`, SC-005. |
| FR-010 | PASS (automated) | `a_read_a_refusal_and_a_change_are_recorded_without_the_value`. |
| FR-011 | PASS (automated) | SC-010. |
| FR-012 | PASS (automated) | SC-002; `secret-application` `SecretRedactor` tests (whole, split across 2 and 3 chunks, longest value first, held tail released at the end, deleted value no longer replaced, cut text); `every_text_column_is_decided_to_hold_agent_text_or_not` (every TEXT column of the office database is listed as agent text with its hook, or as not agent text; a new column fails the test). Gap: a value in the task text and in a temp objective is covered by the frame redaction (`agent_call.rs`) and proven by the report and directive cases; no test starts a real temp. |
| FR-013 | PARTIAL (automated) | No value in the server log (`no_file_of_the_office_holds_a_value` scans `server.log`), in refusals, or in answers; `a_core_dump_of_the_server_has_size_0_and_on_linux_the_process_is_not_dumpable` reads the limit and the dumpable flag back. Windows has no per-process switch (spec A5): NOT RUN there. |
| FR-014 | PASS (automated) | SC-008. |
| FR-015 | PASS (automated) | `a_reset_needs_the_confirmed_count_and_removes_everything_and_the_key`, `e2e` locked state and reset. |
| FR-017, FR-018 | PASS (automated) | `e2e/secrets.spec.ts` (keyboard, Escape and focus, text states, masked field, value field empty), `secrets-controller.spec.ts` (the snapshot never holds the value). |

## Deviations from the plan

- **The account name of the keyring entry is fixed** (`office-harness` / `secret-store-key`), so a second office of the same OS user would share the entry (spec: one office per OS user). Tests never touch it: `disable_os_keyring()`.
- **One seam for tests:** `disable_os_keyring()` (bootstrap) and `UnavailableKeyring` (infrastructure). A production start does not call it.
- **The redaction decorators sit at three points** (`RedactingActionLog`, `RedactingProcessStore`, the agent frame) and in the terminal session, not in each module's own writers; the enumeration test names every TEXT column and which hook covers it.
- **Frame strings:** the user's `secret.set` value is moved out of the request before the service runs, but the raw frame text and the JSON tree of a request are plain heap strings, not zeroized. The OS is asked for no memory dump (FR-013), and the frame of an agent never holds a value for a read.

## Hands-on

NOT RUN with a real signed-in CLI. The stand-in CLIs (shell scripts) ran the real helper, read the value, printed it, and ran a helper process with it in its command line; that is the behavior that a real CLI would add to. A real CLI also keeps its own transcript, which this change does not touch (spec Out of scope).
