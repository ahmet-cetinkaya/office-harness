# 0002 QA report

## Summary

- **Date:** 2026-11-18 full refresh; delta amend 2026-11-19 after the follow-up PRs merged.
- **Build under test:** the full refresh measured `main` at f767811 (the merge of PR #36). The 2026-11-19 amend moves the evidence to `main` at 19f2148, after the follow-up PRs merged: #52 (agent-path `team_id` and role keys), #54 (positive `agent.start` and `agent.new_task` wire tests), #56 (agent-call refusal race), #57 (CI matrix: ubuntu runs every step; macOS and Windows run the Rust tests), #58 (one lead per team), #60 (lead seating in one transaction), #61 (review LOWs), and #62 (agent rows carry role and team; a lead cannot move teams). Every 0002 PR is merged: #11, #25, #29, #30, #31, #33, #35, #40 (merged as 0ce4e9e), #49, #50, #36 (merged as f767811), and the follow-ups above.
- **Overall status:** complete: every hands-on row is PASS or WAIVED (owner); open test gaps are listed in Known gaps. The automated evidence stands on `main`. The person-timed SC-001 is WAIVED (owner): not run by a person, accepted by owner 2026-11-22. The agent-run SC-001 row is PASS (agent).
- **Update, 2026-11-23:** this update changes no row verdict. It adds gaps 10 and 11 to Known gaps. The build under test stays the one named above.
- **Status of this file:** every sha, PR number, run id, file path, and test name below was read from the repository and from the GitHub PR and CI records on 2026-11-18, and again on 2026-11-19 for the amend. The local gates in the table below ran at f767811; the follow-up PRs were gated by their own CI at their heads (CI table). No value in this file is an estimate.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap row states what is not. |
| WAIVED (owner) | The owner accepted the row without a run. The note says when and why. It is not a PASS. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The integration tests in `src/apps/office-harnessd/tests/agent_call.rs`, `agent_directive.rs`, and `agent_subtree.rs` carry `#![cfg(unix)]`. They run on Linux and macOS only. On Windows, CI still builds them and runs the rest of the suite.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server. The e2e run for this report is the CI run, not a local run.
- Signatures: `git log --format=%G?` reports `G` for every cited PR head: 71c1fdd, 2c16405, d6cfe13, f5d7042, 2e9ddde, d9adbd0, cbe6fbf, 973f2cd, 677800b, 87a596a, 76f71b2, b3e302c, c98eba0, 24dba9f, eded414, be6a277, 5215b44, 0f56e3f, and 2f22772. The merge commits on `main` (4f009da through 19f2148) report `E` locally: they carry GitHub's web-flow signature, whose key (B5690EEEBB952194) is not in the local keyring, so this machine cannot check them. Every code commit of every cited PR is `G`.
- The local Rust gates ran on the toolchain that `rust-toolchain.toml` pins (1.100.0) through `nix-shell -p rustup --run "rustup run 1.100.0 ..."`, under a `flock` build lock with `CARGO_BUILD_JOBS=4`. The workspace gates excluded `office-harness-desktop`: the crate needs gdk-sys and does not build on the Linux machine used for this work. CI builds it on all three OSes.

### Local gates run for this refresh (at f767811, 2026-11-18)

The worktree carried only this file's two docs commits on top of f767811, so the gates measured f767811's code.

| Gate | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --exclude office-harness-desktop --all-targets` | exit 0 |
| Tests | `cargo test --workspace --exclude office-harness-desktop -- --test-threads=4` | 80 test binaries, 681 passed, 0 failed |
| Boundaries | `scripts/check-boundaries.sh` | exit 0 (`boundaries ok`) |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --workspace --all-targets --exclude office-harness-desktop -- -D warnings` | exit 0 |
| Webview units | `bun test src` (in `src/apps/desktop`) | 154 pass, 0 fail, 18 files |
| Webview types | `bun run check` (svelte-check) | 410 files, 0 errors, 0 warnings |

Not run locally for this refresh: the `office-harness-desktop` crate build (gdk-sys), the Playwright e2e run (CI runs it), and the Windows paths of the unix-gated flows. The first `bun test src` run in this worktree failed on missing modules because the fresh worktree had no `node_modules`; after `bun install --frozen-lockfile` the counts above hold.

### Local gates per slice (recorded during each slice's run)

| Slice | PR | Head | Gates | Recorded result |
|---|---|---|---|---|
| Phase 1: roles, teams, tree storage (+ T5 S1, S2) | #11 | 71c1fdd | workspace tests (desktop excluded), boundaries, lint, fmt | Full gate at each review round; 68 test binaries, 0 failed at the round head 295c04e; re-gated to 71c1fdd |
| T6: desktop tree view | #25 | 2c16405 | webview gates (`bun test`, `check`, `lint`, `format:check`) | Webview-only slice; Rust gates untouched |
| T8: README and agent-call docs | #29 | d6cfe13 | docs only | No code gates |
| T5 S4: directive | #30 | f5d7042 | workspace tests (desktop excluded), boundaries, lint, fmt | 523 tests, boundaries exit 0, lint clean at f86f66e; review fixes re-gated at f5d7042 |
| T5 review lows | #31 | 2e9ddde | workspace tests (desktop excluded), boundaries, lint | 527 tests, boundaries exit 0 |
| T5 S3: create with role | #33 | d9adbd0 | workspace tests (desktop excluded), boundaries, lint | 16 create-role tests, boundaries exit 0 at the draft head 565425f; fixes re-gated at d9adbd0 |
| T5 S5 slice 1: gateway bridges | #35 | cbe6fbf | full workspace build and tests with the desktop exception | 554 tests |
| T5 S5 slice 2: gateway commands and races | #40 | a8a0228, re-reviewed PASS at 973f2cd | full workspace build and tests with the desktop exception, race suite | 70 suites, 0 failed; all race tests green; r2 verdict PASS (LOWs only) |
| T7: create flows and confirmations | #36 | 677800b | webview gates and Playwright; review PASS at r4 | svelte-check 0 errors at each commit; Playwright green in CI at head; merged as f767811 |
| 0003 T6: agent shortcuts and switcher | #49 | 87a596a | webview and Rust gates in CI | 3-OS CI green at head |
| Agent-call env flake fix | #50 | 76f71b2 | Rust gates in CI | 3-OS CI green at head; closes the macOS `env NotFound` flake the #36 and #49 reviews saw |
| Agent-path `team_id` and role keys | #52 | b3e302c | workspace tests (desktop excluded), webview, Playwright | 39 suites, bun 154, Playwright 26; review PASS at r2 |
| Positive start and new-task wire tests | #54 | c98eba0 | agent_call suite, gates | agent_call 19/0 |
| Agent-call refusal race | #56 | 24dba9f | agent_call x4, agent_directive x3 | 0 failed each run |
| One lead per team | #58 | be6a277 | office-harnessd and hierarchy crates, lint | 269 passed, 0 failed |
| Lead seating in one transaction | #60 | 5215b44 | same crates, lint | 271 passed, 0 failed |
| Review LOWs (#58, #60) | #61 | 0f56e3f | same crates, lint, boundaries | 271 passed, 0 failed |
| Rows carry role and team; no lead move | #62 | 2f22772 | same crates, lint, boundaries | agent_call 22/22; two clean full runs |

`scripts/check-boundaries.sh` exited 0 on every slice that changed Rust code and again for this refresh. One boundaries failure happened during #11 (the hierarchy crates depended on the agent domain for `ProjectId` only); it was fixed by moving `ProjectId` to the kernel (d2dba27), not by weakening the check.

### CI

CI runs on ubuntu-latest, macos-latest, and windows-latest. Up to #57, every OS ran `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (including `office-harness-desktop`), `scripts/check-boundaries.sh --self-test` and the check itself, the webview lint and unit tests, the Playwright e2e run, and `tauri build --no-bundle`. From #57 on (owner decision), ubuntu runs all of these steps, and macOS and Windows run the Rust tests only; the three jobs keep their names.

Merge shas and head run ids below were read from `gh pr view` and `gh run view` on 2026-11-18; each head run lists all three OS jobs `success`.

| PR | Head | Merge | CI run at head | 3 OS |
|---|---|---|---|---|
| #11 | 71c1fdd | 4f009da | 44119551929 | success |
| #25 | 2c16405 | 4292e93 | 44150493062 | success |
| #29 | d6cfe13 | e4d9103 | 44150618072 | success |
| #30 | f5d7042 | e873371 | 44159848212 | success |
| #31 | 2e9ddde | bc068d3 | 44170642172 | success |
| #33 | d9adbd0 | d03dbc3 | 44184227889 | success |
| #35 | cbe6fbf | 99f14d1 | 44194065977 | success |
| #40 | 973f2cd | 0ce4e9e | 44306542699 | success |
| #36 | 677800b | f767811 | 44317624213 | success |
| #49 | 87a596a | d2247ae | 44310189285 | success |
| #50 | 76f71b2 | e2d00c0 | 44315869846 | success |
| #52 | b3e302c | 3beb24c | 44337909702 | success |
| #54 | c98eba0 | 9e8710b | 44349691777 | success |
| #56 | 24dba9f | 3fa23ca | 44359559850 | success |
| #57 | eded414 | 82fd798 | 44359195735 | success |
| #58 | be6a277 | 40862d6 | 44470868858 | success |
| #60 | 5215b44 | cec9e3a | 44474536498 | success |
| #61 | 0f56e3f | 649e237 | 44475981571 | success |
| #62 | 2f22772 | 19f2148 | 44491821141 | success |

## FR-001: The tree with CEO, team lead, and member

| FR | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `src/modules/hierarchy/domain/src/services/tree_rules.tests.rs`: `a_lead_sits_under_the_ceo_when_one_exists`, `a_member_needs_a_lead_parent`, `descendants_go_deepest_first_and_exclude_the_agent`, `descendant_checks_follow_the_parent_chain`, `a_generated_tree_never_loops_or_overflows`, `a_looped_tree_terminates_instead_of_hanging`, `a_second_ceo_refusal_names_the_stored_ceo_not_its_id`. Role value object: `src/modules/hierarchy/domain/src/value_objects/role.tests.rs` (`the_wire_names_round_trip`, `each_role_allows_only_its_children`). Storage: `src/apps/office-harnessd/tests/migrations.rs`. |

## FR-002: Create a team with name, optional project, and lead

| FR | Verdict | Evidence |
|---|---|---|
| FR-002 | PASS (automated) | Name rules: `src/modules/hierarchy/domain/src/value_objects/team_name.tests.rs`, `a_name_trims_and_lowers_for_its_key` and `an_empty_or_oversize_name_is_refused`. Duplicate and case-only duplicate names are refused by the unique name key, proven at the database level in `src/apps/office-harnessd/tests/migrations.rs`, `a_name_key_that_differs_only_by_case_is_refused`. The `team.create` dispatch sits in `src/modules/hierarchy/presentation/src/handlers/hierarchy_handlers.rs`, and the US1 flow that creates a team runs in the e2e test `US1 builds a company from an empty office with the keyboard alone` (mock core, on `main`). The `create_team` handler itself has no dedicated unit test; its rules are covered by the pieces above. A lead that already leads a team is refused as `conflict`, never moved (`tree_rules.tests.rs`, `a_lead_of_one_team_is_refused_for_another_team`; wire: `agent_call.rs`, `a_lead_of_one_team_cannot_create_a_second_team`). Its check-then-write window is Known gap 6. |

## FR-003: Create an agent with a role only where allowed, naming the allowed roles

| FR | Verdict | Evidence |
|---|---|---|
| FR-003 | PASS (automated) | `tree_rules.tests.rs`: `a_second_ceo_is_refused_and_names_the_first`, `a_member_needs_a_lead_parent`. `src/modules/hierarchy/application/src/commands/create_role_agent/create_role_agent_handler.tests.rs`: `a_second_ceo_is_refused_and_creates_nothing`, `a_refused_create_maps_to_the_matching_error_kind`, `a_failed_placement_removes_the_created_agent`, `a_failed_removal_after_a_failed_placement_names_both_failures`, `a_concurrent_delete_between_check_and_create_leaves_no_orphan`, `a_second_lead_for_a_held_team_is_refused_and_creates_nothing`. One lead per team: `tree_rules.tests.rs`, `a_second_lead_for_a_held_team_is_refused_and_names_the_seated_lead`; over the wire, `agent_call.rs`, `a_second_lead_for_a_held_team_is_refused_and_recorded` (refused as `conflict`, recorded, the tree keeps the seated lead). Webview: e2e `the create actions list only the roles the place allows (FR-003)` and `a refused create shows the server's refusal text (FR-003)` (on `main`, mock core). |

## FR-004: Each agent shows under its parent in the hierarchy view

| FR | Verdict | Evidence |
|---|---|---|
| FR-004 | PASS (automated) | Server: `src/apps/office-harnessd/tests/agent_call.rs`, `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it` and `a_ceo_creates_a_lead_through_agent_call_and_the_row_carries_the_team` read the tree over the wire after an agent creates a member or a lead (the lead test asserts the team's `lead` in `hierarchy.tree`, which #58 restored). The tree finds a lead's team through the lead's own row (`tree_query.tests.rs`, `a_lead_shows_on_its_rows_team_even_when_the_seat_column_is_stale`). Agent rows carry `role` and the team name at answer time: `agent_rows_carry_role_and_the_current_team_name`. Webview: `src/apps/desktop/src/modules/hierarchy/state/hierarchy-controller.spec.ts` (`it reads the tree on load`, `a hierarchy.changed event reads the tree again`, `an older tree reply never overwrites a newer one`) and `src/apps/desktop/e2e/hierarchy.spec.ts`: `the tree shows each agent under its parent with role and status as text`, `a hierarchy.changed event re-renders the tree`. |

## FR-005: No fixed maximum on the number of agents

| FR | Verdict | Evidence |
|---|---|---|
| FR-005 | PASS (automated) | No cap exists in the placement rules or the schema. `tree_rules.tests.rs` builds a generated multi-level company only through `check_placement` and walks every parent chain without a cap (`a_generated_tree_never_loops_or_overflows`). The same test proves a looped tree terminates instead of hanging (`a_looped_tree_terminates_instead_of_hanging`). |

## FR-006: Counted delete confirmation

| FR | Verdict | Evidence |
|---|---|---|
| FR-006 | PASS (automated) | `src/apps/office-harnessd/tests/agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` and `a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing` (a stale count deletes nothing). Handler unit tests: `src/modules/hierarchy/application/src/commands/subtree/subtree_handler.tests.rs`, including `a_delete_confirmed_on_one_tree_deletes_no_child_placed_after_its_count`. Webview: `the delete confirmation states the counted descendants and cancel deletes 0`, `a confirmed lead delete removes the lead and its 2 members`, `a stale counted delete shows the server's recount refusal`. |

## FR-007: Offer to stop all descendants in the same action

| FR | Verdict | Evidence |
|---|---|---|
| FR-007 | PASS (automated) | `agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` stops the subtree over the wire, one event per affected agent. Webview: `the stop confirmation offers only-this-agent and with-descendants choices`, `a confirmed only-this-agent stop sends include_descendants false`, `a confirmed stop with descendants sends include_descendants`. |

## FR-008: A running agent acts only within its own descendants

| FR | Verdict | Evidence |
|---|---|---|
| FR-008 | PASS (automated) | Rules: `src/modules/hierarchy/domain/src/services/call_rules.tests.rs` (`a_direct_descendant_passes`, `two_levels_deep_passes`, `the_caller_itself_is_refused`, `an_upward_chain_is_refused`, `a_requested_descendant_lead_is_kept`, `an_outside_requested_parent_is_refused`). Gateway on `main`: a non-create request naming the caller is refused and journaled before the tree check, the descendant check runs on a fresh snapshot for every kind, and a create's client role is kept and validated against the caller's place (`src/apps/office-harnessd/src/server/agent_call.rs`, unit tests in `agent_call.tests.rs` such as `a_ceo_callers_create_keeps_the_requested_lead_role_and_team` and `a_member_caller_asking_for_any_child_role_is_refused`). Over the wire: `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, `a_ceo_creates_a_lead_through_agent_call_and_the_row_carries_the_team`, `a_ceo_creating_a_lead_without_a_team_is_refused_and_recorded`, `a_lead_starts_its_member_over_agent_call`, `a_lead_gives_its_member_a_new_task_over_agent_call`, `a_ceo_creates_a_member_under_its_descendant_lead`, `a_lead_stops_its_member_through_agent_call`, the self refusals `an_agent_starting_itself_over_agent_call_is_refused_and_recorded`, `an_agent_tasking_itself_over_agent_call_is_refused_and_recorded`, `an_agent_stopping_itself_over_agent_call_is_refused_and_recorded`, and the place refusals `a_member_caller_asking_for_a_child_role_is_refused_and_recorded`, `a_lead_caller_asking_for_a_lead_child_is_refused`, `an_agent_asking_for_a_ceo_child_is_refused`. Directives: `a_directive_to_a_running_non_descendant_is_refused_and_recorded`. The create role's `team_id` is forwarded and required for a lead (#52). |

## FR-009: Refuse, tell the agent the reason, record the refusal

| FR | Verdict | Evidence |
|---|---|---|
| FR-009 | PASS (automated) | Every refusal class names its test and journals: `an_agent_may_not_ask_for_a_type_outside_the_allowlist` (the allowlist stays closed), `an_unsupported_command_or_field_is_refused_and_recorded` (also refuses an `additions` field), `a_directive_to_a_running_non_descendant_is_refused_and_recorded`, `an_escape_in_the_text_is_invalid_and_recorded`, `a_request_carrying_an_actor_field_is_refused_and_recorded`, the three self-action refusals of FR-008, and the place refusals of FR-008. Journal: `src/apps/office-harnessd/src/bridges/directive_bridges.tests.rs`, `a_gateway_refusal_journals_the_caller_as_actor_and_agent` and `a_control_character_in_a_gateway_refusal_is_escaped_in_the_journal`. A client-supplied `role` is honoured when the caller's place allows it and refused with its reason when it does not, in the wire tests above. Nothing is trimmed silently any more: #52 refuses an unknown key inside `role` and a `team_id` the role does not take (`agent_call.tests.rs`: `a_role_with_an_unknown_field_is_invalid`, `a_lead_role_without_a_team_is_invalid`, `a_member_role_with_a_team_is_invalid`; over the wire, `a_ceo_creating_a_lead_without_a_team_is_refused_and_recorded`), and #58 records a refused `agent.create` with the caller as actor (`a_second_lead_for_a_held_team_is_refused_and_recorded`). |

## FR-010: A directive enters as one unbroken input followed by Enter

| FR | Verdict | Evidence |
|---|---|---|
| FR-010 | PASS (automated) | Writer: `directive_bridges.tests.rs`, `the_writer_delivers_text_and_enter_in_one_write` (text plus Enter in one write call). End to end: `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter` reads the bytes the fake CLI recorded. |

## FR-011: Directives and refusals in the action record, with a named actor

| FR | Verdict | Evidence |
|---|---|---|
| FR-011 | PASS (automated) | Actor scope: `src/modules/agent/application/src/common/actor_scope.tests.rs` (`the_default_actor_is_the_user`, `an_entered_agent_actor_holds_until_the_guard_drops`, `a_scope_on_another_thread_does_not_leak`, `a_dropped_scope_leaves_no_actor_on_a_reused_thread`). Identity: `agent_call.rs`, `an_agent_proves_itself_and_the_office_names_it`, and `the_child_gets_its_own_credential_and_never_the_office_token`. Refusals recorded with the caller as actor: `a_gateway_refusal_journals_the_caller_as_actor_and_agent` and the recorded-refusal tests of `agent_directive.rs`. Journal text rules: `a_control_character_in_a_refused_text_is_escaped_in_the_journal`, `a_nine_thousand_character_text_is_capped_at_the_bound_with_a_hash`. |

## SC-001: A 15-agent hierarchy built from the desktop app in under 10 minutes

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-001 | any | A person builds 1 CEO, 2 leads, 12 members from the app alone, timed | Not measured | WAIVED (owner) | Not run by a person, accepted by owner 2026-11-22. Manual QA did not happen (see Manual QA). The e2e test `US1 builds a company from an empty office with the keyboard alone` (on `main`) proves the flow and the keyboard path against a mock core; it proves neither the timing nor the real server. Note for the human run: a fresh office has no project, so create a project first (a team needs one for its lead and members) and give the CEO an absolute working folder. |
| SC-001 (agent run) | Linux, KDE | The same build through the real app and server, keyboard events only (WebDriver key actions), on branch `fix/missing-change-events` at b2dd010 | 15 agents (1 CEO, 2 leads, 12 members, all in project Office) in 6.19 s after the server start (0.62 s); load average 1.28 at the start, 1.08 at the end | PASS (agent) | The first runs on `main` 3c91cc7 stopped at the first team lead: the tree never showed the new CEO (no `hierarchy.changed` for `agent.placed`), and a member with no folder was refused (no team project, A6). Both are fixed on this branch. The agent run proves the flow and the server; the human timing above is WAIVED (owner). Raw evidence: `run.json` (steps and times, the 15 rows) and `hierarchy-15.png`, kept outside the repository in the QA agent's folder `qa/evidence/0002-SC-001/`. |

## SC-002: 30 running agents, 100% shown under their correct parents

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-002 | Linux, macOS, Windows | Domain test: generate a company only through `check_placement` and walk every parent chain | Parent chains hold for the generated company (per the test assertion) | PARTIAL (automated) | `tree_rules.tests.rs`, `a_generated_tree_never_loops_or_overflows` (a domain unit test with no `cfg(unix)` gate, so it runs on all three CI OSes). This proves placement at the domain level with fake rows, not 30 live agents in the view. The view half is the mock-core e2e `the tree shows each agent under its parent with role and status as text`. No test runs 30 live agents. |

## SC-003: 20 disallowed agent requests, 100% refused and recorded

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-003 | Linux, macOS | Each refusal class proven by its own test; every refusal asserted in the action record | Every tested class is refused and recorded (per the test assertions) | PARTIAL (automated) | The classes proven: a type outside the allowlist (`an_agent_may_not_ask_for_a_type_outside_the_allowlist`), an unsupported command or an `additions` field (`an_unsupported_command_or_field_is_refused_and_recorded`), a directive to a non-descendant (`a_directive_to_a_running_non_descendant_is_refused_and_recorded`), an actor field smuggled in a request (`a_request_carrying_an_actor_field_is_refused_and_recorded`), invalid directive text (`an_escape_in_the_text_is_invalid_and_recorded`), the caller itself as the target of start, new task, and stop over agent-call (the three `an_agent_*_itself_over_agent_call_is_refused_and_recorded` tests, plus `the_caller_itself_is_refused` at the domain for directives), a role the caller's place does not allow (`a_member_caller_asking_for_a_child_role_is_refused_and_recorded`, `a_lead_caller_asking_for_a_lead_child_is_refused`, `an_agent_asking_for_a_ceo_child_is_refused`), and a stale delete count (`a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing`). The criterion's single 20-request batch test was not built. |

## SC-004: 100 directives land unbroken while the user types

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-004 | Linux, macOS | One write per directive, read back from the target's terminal record | One directive lands as one input plus Enter (per the test assertion) | PARTIAL (automated) | The mechanism is proven: `directive_bridges.tests.rs`, `the_writer_delivers_text_and_enter_in_one_write`, and end to end in `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter`. The 100-directive loop with concurrent user typing was not built. |

## SC-005: A confirmed lead delete removes exactly N + 1; a cancel removes 0

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-005 | Linux, macOS | Delete a parent over the wire; delete again with a stale count | The subtree is removed after confirm; a stale count deletes nothing (per the test assertions) | PASS (automated) | `agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` and `a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing`, with fake CLIs against the real server. Webview half (mock core): `a confirmed lead delete removes the lead and its 2 members` and `the delete confirmation states the counted descendants and cancel deletes 0`. |

## SC-006: A lead creates a member and sends it a directive with 0 user actions

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-006 | Linux, macOS | The lead agent calls the office through the helper; the member's fake CLI records the input | The member is created and takes the directive as one input with Enter (per the test assertions) | PASS (automated) | `agent_call.rs`, `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, then `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter`. Both are driven by the agent alone, and both sit on `main`. |

## Known gaps (open only)

1. **The tree exclusion is proven with test fakes only.** No test drives a real writer through `tree_exclusion`. The review of #31 observed that the exclusion held across blocking stops and that a stop holding the exclusion delays `team.create`. The delay was observed, not measured, and no regression test pins it.
2. **A directive to a deleted target reads as "not running".** The spec's concurrent edge case (the user deletes an agent while its parent sends it a directive) requires the refusal to say the target was deleted. On `main` the `DirectiveTarget` port returns `Result`, and a store failure behind the lookup surfaces as `Storage` with its own message (`send_directive_handler.rs`, tested by `a_store_failure_in_is_running_is_the_error_not_a_no`); a deleted target and a stopped target both get the `NotRunning` wording, so the reason text still does not distinguish them.
3. **The Windows attach of the subtree, call, and directive bridges has no local run.** The three integration files carry `#![cfg(unix)]`, so no local Windows execution happened. The CI Windows job builds the workspace and runs the portable suite on every PR, which covers compilation and the non-unix paths, not the unix-gated flows.
4. **`office-harness-desktop` was never built on the Linux machine used for this work.** gdk-sys does not build there, so every local workspace gate excluded the crate. CI builds it on all three OSes, including `tauri build --no-bundle`.
5. **`team.create` has a check-then-write window.** `CreateTeamHandler` checks `team_name_taken` and then saves with no `TreeExclusion` held across the two steps. The unique `name_key` index refuses the losing create at the storage layer, so the race cannot produce a duplicate team; the loser gets a storage error instead of the `Conflict` text. Follow-up noted in the #40 review.
6. **The repository test fixture lists the agent migrations by hand.** `hierarchy_repository_tests.rs` runs the real migration SQL, but it names each agent migration file. A new agent migration must be added there too, or the fixture lags the real schema (#61 review, LOW).
7. **No `team.rename` request exists.** Agent rows read the team name at answer time, so a rename shows in the next answer; the test writes the rename to storage because no request does it yet.
8. **`session.*` answers carry `role` and `team` as null.** The gateway fills the place fields into every object of an `agent.*` answer; a session object inside such an answer gets `role: null` and `team: null`, the same way it already gets `last_line` (#62 review, LOW).
9. **`PlaceAgentHandler` could demote a lead and leave a stale seat.** The handler is not routed to any wire request today, so nothing reaches it; a future caller that re-places a lead as a member would leave the team's seat naming it (#62 review, LOW, latent).
11. **A directive to a deleted target.** Refined 2026-11-23: a target that is already deleted is not in the tree, so the refusal says "not a descendant". Only a delete between the tree read and the lookup gives the "not running" text with the agent id (`a_stopped_unknown_target_is_refused_by_id_before_any_write`). The two texts are not the "deleted" text that the spec edge case asks for. See gap 2.

Closed by the 2026-11-19 amend: the agent path not seating a lead on its team, and the hidden second lead it allowed (#58, #60, #61); a lead move leaving a stale seat (#62: refused); the silent trim of `role.team_id` and unknown role keys (#52); no positive `agent.start` or `agent.new_task` wire test (#54); the mock core's second-CEO text in id form (#52).

## Manual QA

No owner-level UI walkthrough happened. Nobody built a hierarchy in the running app, attached to a real agent terminal, confirmed a subtree delete by hand, or watched a directive arrive in a live session. What stands in its place is the Playwright e2e suite: 26 tests on `main` (`src/apps/desktop/e2e/hierarchy.spec.ts`), covering the keyboard-only US1 build, the counted delete and stop confirmations, the FR-003 create roles and refusal texts, the axe checks (`the create flows run with the keyboard alone and axe stays clean`, `the stop and delete confirmations run axe clean while open`), tree navigation and focus, and the empty, failure, and re-render states. Those tests run against a mock core, and the run for this report is CI's (run 44317624213 on ubuntu), not a local run. They show the screens work; they do not show a person can use them, and the person-timed SC-001 is WAIVED (owner): not run by a person, accepted by owner 2026-11-22.

## Updates

- 2026-11-23: closed Known gap 10 (A5 not shipped). Placing a CEO now moves the leads without a parent under it, in one transaction. Tests: `hierarchy_repository_tests.rs` `placing_a_ceo_moves_the_parentless_leads_under_it`, `a_failed_ceo_placement_moves_no_lead`; `H/tests/agent_subtree.rs` `a_ceo_created_after_a_team_takes_the_team_lead_under_it` (fails without the fix: the CEO has 0 descendants instead of 1). An office saved before the fix is repaired at start when exactly one CEO exists: `H/tests/startup.rs`, `a_ceo_and_parentless_leads_from_before_the_fix_are_repaired_at_start` and `the_start_repair_moves_no_lead_when_two_ceos_exist`.
