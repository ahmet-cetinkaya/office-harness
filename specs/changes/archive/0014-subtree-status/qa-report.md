# 0014 QA report

## Summary

- **Date:** 2026-11-23.
- **Build under test:** branch `feat/0014-subtree-status` at bec7f8e (T2 eb6dc76, T3 25feaf3, T4 bec7f8e), toolchain 1.100.0, on Linux.
- **Overall status:** every requirement and success criterion has automated evidence. There is no user interface, so there is no hands-on or Playwright row. macOS and Windows rely on CI.
- **Status of this file:** the gate numbers come from local runs at bec7f8e on 2026-11-23. The CI result of the PR is not in this file.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PASS (review) | The criterion is about what the diff does not touch; a read of the diff proves it. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The integration tests run the real server and the real `office-harnessd agent-call` helper with the credential of a live session. The agents are fake CLIs (shell scripts), not real agent CLIs.
- The trees are placed parent first, so every lead stores its CEO as parent (spec A10).
- The Rust gates ran per crate: the desktop crate does not build on this Linux machine (libdbus-sys). CI builds it on three OSes.

### Local gates (2026-11-23, at bec7f8e)

| Gate | Scope | Result |
|---|---|---|
| `scripts/lint-rust.sh` (fmt check, clippy `-D warnings`, all targets) | `office-harnessd`, `hierarchy-domain`, `terminal-infrastructure`, `terminal-application`, `terminal-presentation` | PASS (clean) |
| `cargo test` | `office-harnessd` lib 183 (10 new in `server::subtree_status`), `hierarchy-domain` 51, `terminal-application` 6, `terminal-presentation` 7, `terminal-infrastructure` lib 49 and `terminal_runtime` 18 | PASS |
| `tests/agent_subtree_status.rs` | 9 tests, 3 runs in a row | PASS (9 passed each time) |
| Neighbour suites | `agent_call` 22, `agent_directive` 8, `agent_hold` 10, `needs_you` 15, `task_text` 14 | PASS |
| `scripts/check-boundaries.sh` | workspace | PASS (boundaries ok) |

## Requirements

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `an_entry_carries_exactly_the_contract_fields` (unit: every field and its value); `a_lead_sees_each_status_change_of_its_member`, `a_read_shows_the_needs_you_and_hold_flags_and_changes_neither`, `a_task_line_reaches_the_reader_as_one_line_of_visible_symbols` (integration). |
| FR-002 | PASS (automated) | `each_of_nine_readers_gets_exactly_its_descendants_and_no_terminal_output` (each of 9 readers gets exactly its descendant set, never itself, each entry after its parent); `a_lead_reads_its_own_branch_only`, `a_member_reads_an_empty_list`, `descendants_in_tree_order_put_each_one_after_its_parent_and_exclude_the_agent`; `a_read_that_names_another_agent_is_refused_and_recorded`. |
| FR-003 | PASS (automated) | Every entry of the 9 readers has exactly the 9 contract keys, and no answer holds the members' printed marker; the unit test checks that the CLI command never appears; the task carries `{line, set_at}` only. |
| FR-004 | PASS (automated) | `reads_pass_holds_count_toward_no_limit_and_leave_no_record`: 10 reads under an office hold answer; after 30 reads a directive is accepted and `hold.list` is empty. |
| FR-005 | PASS (automated) | Same test: the reader's action record does not grow over 30 answered reads; `a_read_that_names_another_agent_is_refused_and_recorded`: 10 refusals, 10 new entries; `a_failed_read_answers_internal_raises_the_storage_alert_and_is_recorded`: one entry. |
| 0002/FR-008 (modified) | PASS (automated) | The allow list holds `agent.subtree`; the integration tests reach it through the helper. |

## Success criteria

| ID | Verdict | Evidence |
|---|---|---|
| SC-001 | PASS (automated) | `each_of_nine_readers_gets_exactly_its_descendants_and_no_terminal_output`: 1 CEO, 2 leads, 6 members; 9 of 9 answers equal the reader's descendant set. |
| SC-002 | PASS (automated) | Same test: each member prints `marker-7Q`; 0 of 9 answers contain it. |
| SC-003 | PASS (automated) | `a_lead_sees_each_status_change_of_its_member`: `working`, `waiting_for_input`, `stopped`, 3 of 3. |
| SC-004 | PASS (automated) | `reads_pass_holds_count_toward_no_limit_and_leave_no_record`. |
| SC-005 | PASS (automated) | `a_read_answers_within_two_seconds_with_thirty_agents`: 30 agents, 29 entries, under 2 s (the whole file runs in about 2.3 s). |
| SC-006 | PASS (automated) | `a_read_that_names_another_agent_is_refused_and_recorded`: 10 attempts naming a sibling, the CEO, or a member of another branch; 10 `invalid`, 10 recorded, no id or name of those agents in the answers. |
| SC-007 | PASS (automated) | `a_task_line_reaches_the_reader_as_one_line_of_visible_symbols`: U+001B, U+009B, U+202E never reach the helper's output as themselves; the line is `␛[31m\u009B‮red…`. Unit: `control_and_bidirectional_characters_become_visible_symbols`. |

## Preserved behavior

| ID | Verdict | Evidence |
|---|---|---|
| PB-1 | PASS (automated) | `agent_call` 22 and `agent_directive` 8 pass unchanged. |
| PB-2 | PASS (automated) | `agent_hold` 10 passes; its not-allowed check keeps the start of the refusal text. |
| PB-3 | PASS (automated) | `a_read_shows_the_needs_you_and_hold_flags_and_changes_neither`: the Needs you list and the status are the same after the read; answered reads add no record entry (FR-005 row). |
| PB-4 | PASS (review) | The diff touches no CLI configuration path; the read only reads office state. |
| PB-5 | PASS (automated) | No desktop file changed; `the_desktop_connection_does_not_offer_the_read` gets `not_found`; the read publishes no event (only the failure path raises the existing `storage.error`). |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| No descendants | PASS (automated) | `a_member_reads_an_empty_list`. |
| Descendant not running | PASS (automated) | `a_descendant_without_a_task_or_a_running_child_shows_null`; after the stop in SC-003, `last_output_at` is null. |
| Descendant with no output yet | PASS (automated) | Runtime test `the_quiet_time_grows_in_silence_restarts_with_output_and_ends_with_the_child` (the quiet time counts from the start). |
| Long or multi-line task text | PASS (automated) | `the_task_line_keeps_eighty_characters_of_the_first_line`. |
| Control or bidirectional characters | PASS (automated) | SC-007 row. |
| Input fields, actor or caller | PASS (automated) | SC-006 row; the actor check is the shared gate covered by `agent_call`. |
| Tree changes between reads | PASS (automated) | `a_descendant_the_agents_no_longer_hold_is_left_out`; each read takes one snapshot. |
| Storage read failure | PASS (automated) | `a_failed_read_answers_internal_raises_the_storage_alert_and_is_recorded`. |
| Desktop sends the read | PASS (automated) | `the_desktop_connection_does_not_offer_the_read`. |
| Lead placed before the CEO (spec A10) | PASS (automated, by ACF-197) | The 0002 A5 gap is closed: `H/tests/agent_subtree.rs`, `a_ceo_created_after_a_team_takes_the_team_lead_under_it` (the CEO's descendants include the earlier lead), and `H/tests/startup.rs`, `a_ceo_and_parentless_leads_from_before_the_fix_are_repaired_at_start`. This change reads the stored parent, so it lists that lead. |

## Deviations from the plan

- The contract named the user-connection answer `invalid`; the server answers an unknown `agent.*` type with `not_found`. The contract now says `not_found`.
- The contract listed the status as `waiting for input`; the wire form is `waiting_for_input`, as in `agent.list`. The contract now says so.
- A failed read also raises the existing `storage.error` alert, as the gateway's other reads do (plan D6 named the `internal` answer and the record only).
