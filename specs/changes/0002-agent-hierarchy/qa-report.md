# 0002 QA report

## Summary

- **Date:** 2026-11-18. Full refresh against the delivered state.
- **Build under test:** `main` at f767811 (the merge of PR #36). Every 0002 PR is merged: #11, #25, #29, #30, #31, #33, #35, #40 (the agent-call gateway, merged as 0ce4e9e), #49, #50, and #36 (T7 create flows, merged as f767811). PR #52 (the agent-path `team_id` and role-key LOWs) is open at 3abca65 and stays out of scope here except as a stated follow-up.
- **Overall status:** the automated evidence stands on `main`. The owner-level UI walkthrough did not happen (see Manual QA), so SC-001 stays NOT RUN.
- **Status of this file:** every sha, PR number, run id, file path, and test name below was read from the repository and from the GitHub PR and CI records on 2026-11-18, and the local gates in the table below were run for this refresh at f767811. No value in this file is an estimate.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap row states what is not. |
| NOT RUN | Nobody ran the check. It is not a PASS and not a WAIVER: no person performed it. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The integration tests in `src/apps/office-harnessd/tests/agent_call.rs`, `agent_directive.rs`, and `agent_subtree.rs` carry `#![cfg(unix)]`. They run on Linux and macOS only. On Windows, CI still builds them and runs the rest of the suite.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server. The e2e run for this report is the CI run, not a local run.
- Signatures: `git log --format=%G?` reports `G` for every cited PR head: 71c1fdd, 2c16405, d6cfe13, f5d7042, 2e9ddde, d9adbd0, cbe6fbf, 973f2cd, 677800b, 87a596a, 76f71b2, and the open #52 head 3abca65. The merge commits on `main` (4f009da through f767811) report `E` locally: they carry GitHub's web-flow signature, whose key (B5690EEEBB952194) is not in the local keyring, so this machine cannot check them. Every code commit of every cited PR is `G`.
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

`scripts/check-boundaries.sh` exited 0 on every slice that changed Rust code and again for this refresh. One boundaries failure happened during #11 (the hierarchy crates depended on the agent domain for `ProjectId` only); it was fixed by moving `ProjectId` to the kernel (d2dba27), not by weakening the check.

### CI

CI runs on ubuntu-latest, macos-latest, and windows-latest: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (including `office-harness-desktop`), `scripts/check-boundaries.sh --self-test` and the check itself, the webview lint and unit tests, the Playwright e2e run, and `tauri build --no-bundle`.

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

## FR-001: The tree with CEO, team lead, and member

| FR | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `src/modules/hierarchy/domain/src/services/tree_rules.tests.rs`: `a_lead_sits_under_the_ceo_when_one_exists`, `a_member_needs_a_lead_parent`, `descendants_go_deepest_first_and_exclude_the_agent`, `descendant_checks_follow_the_parent_chain`, `a_generated_tree_never_loops_or_overflows`, `a_looped_tree_terminates_instead_of_hanging`, `a_second_ceo_refusal_names_the_stored_ceo_not_its_id`. Role value object: `src/modules/hierarchy/domain/src/value_objects/role.tests.rs` (`the_wire_names_round_trip`, `each_role_allows_only_its_children`). Storage: `src/apps/office-harnessd/tests/migrations.rs`. |

## FR-002: Create a team with name, optional project, and lead

| FR | Verdict | Evidence |
|---|---|---|
| FR-002 | PASS (automated) | Name rules: `src/modules/hierarchy/domain/src/value_objects/team_name.tests.rs`, `a_name_trims_and_lowers_for_its_key` and `an_empty_or_oversize_name_is_refused`. Duplicate and case-only duplicate names are refused by the unique name key, proven at the database level in `src/apps/office-harnessd/tests/migrations.rs`, `a_name_key_that_differs_only_by_case_is_refused`. The `team.create` dispatch sits in `src/modules/hierarchy/presentation/src/handlers/hierarchy_handlers.rs`, and the US1 flow that creates a team runs in the e2e test `US1 builds a company from an empty office with the keyboard alone` (mock core, on `main`). The `create_team` handler itself has no dedicated unit test; its rules are covered by the pieces above. Its check-then-write window is Known gap 6. |

## FR-003: Create an agent with a role only where allowed, naming the allowed roles

| FR | Verdict | Evidence |
|---|---|---|
| FR-003 | PASS (automated) | `tree_rules.tests.rs`: `a_second_ceo_is_refused_and_names_the_first`, `a_member_needs_a_lead_parent`. `src/modules/hierarchy/application/src/commands/create_role_agent/create_role_agent_handler.tests.rs`: `a_second_ceo_is_refused_and_creates_nothing`, `a_refused_create_maps_to_the_matching_error_kind`, `a_failed_placement_removes_the_created_agent`, `a_failed_removal_after_a_failed_placement_names_both_failures`, `a_concurrent_delete_between_check_and_create_leaves_no_orphan`. Webview: e2e `the create actions list only the roles the place allows (FR-003)` and `a refused create shows the server's refusal text (FR-003)` (on `main`, mock core). |

## FR-004: Each agent shows under its parent in the hierarchy view

| FR | Verdict | Evidence |
|---|---|---|
| FR-004 | PASS (automated) | Server: `src/apps/office-harnessd/tests/agent_call.rs`, `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it` and `a_ceo_creates_a_lead_through_agent_call_and_the_tree_shows_it` read the tree over the wire after an agent creates a member or a lead. Webview: `src/apps/desktop/src/modules/hierarchy/state/hierarchy-controller.spec.ts` (`it reads the tree on load`, `a hierarchy.changed event reads the tree again`, `an older tree reply never overwrites a newer one`) and `src/apps/desktop/e2e/hierarchy.spec.ts`: `the tree shows each agent under its parent with role and status as text`, `a hierarchy.changed event re-renders the tree`. |

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
| FR-008 | PASS (automated) | Rules: `src/modules/hierarchy/domain/src/services/call_rules.tests.rs` (`a_direct_descendant_passes`, `two_levels_deep_passes`, `the_caller_itself_is_refused`, `an_upward_chain_is_refused`, `a_requested_descendant_lead_is_kept`, `an_outside_requested_parent_is_refused`). Gateway on `main`: a non-create request naming the caller is refused and journaled before the tree check, the descendant check runs on a fresh snapshot for every kind, and a create's client role is kept and validated against the caller's place (`src/apps/office-harnessd/src/server/agent_call.rs`, unit tests in `agent_call.tests.rs` such as `a_ceo_callers_create_keeps_the_requested_lead_role` and `a_member_caller_asking_for_any_child_role_is_refused`). Over the wire: `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, `a_ceo_creates_a_lead_through_agent_call_and_the_tree_shows_it`, `a_ceo_creates_a_member_under_its_descendant_lead`, `a_lead_stops_its_member_through_agent_call`, the self refusals `an_agent_starting_itself_over_agent_call_is_refused_and_recorded`, `an_agent_tasking_itself_over_agent_call_is_refused_and_recorded`, `an_agent_stopping_itself_over_agent_call_is_refused_and_recorded`, and the place refusals `a_member_caller_asking_for_a_child_role_is_refused_and_recorded`, `a_lead_caller_asking_for_a_lead_child_is_refused`, `an_agent_asking_for_a_ceo_child_is_refused`. Directives: `a_directive_to_a_running_non_descendant_is_refused_and_recorded`. Two notes: `agent.start` and `agent.new_task` have no positive agent-call wire test (Known gap 8), and the create role's `team_id` forwarding waits on PR #52 (Known gap 7). |

## FR-009: Refuse, tell the agent the reason, record the refusal

| FR | Verdict | Evidence |
|---|---|---|
| FR-009 | PASS (automated) | Every refusal class names its test and journals: `an_agent_may_not_ask_for_a_type_outside_the_allowlist` (the allowlist stays closed), `an_unsupported_command_or_field_is_refused_and_recorded` (also refuses an `additions` field), `a_directive_to_a_running_non_descendant_is_refused_and_recorded`, `an_escape_in_the_text_is_invalid_and_recorded`, `a_request_carrying_an_actor_field_is_refused_and_recorded`, the three self-action refusals of FR-008, and the place refusals of FR-008. Journal: `src/apps/office-harnessd/src/bridges/directive_bridges.tests.rs`, `a_gateway_refusal_journals_the_caller_as_actor_and_agent` and `a_control_character_in_a_gateway_refusal_is_escaped_in_the_journal`. The former gap is closed: a client-supplied `role` is honoured when the caller's place allows it and refused with its reason when it does not, in the wire tests above. What remains open is the silent trim of `role.team_id` and of unknown keys inside `role` (the #40 r2 LOWs, Known gap 7). |

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
| SC-001 | any | A person builds 1 CEO, 2 leads, 12 members from the app alone, timed | Not measured | NOT RUN | Manual QA did not happen (see Manual QA). The e2e test `US1 builds a company from an empty office with the keyboard alone` (on `main`) proves the flow and the keyboard path against a mock core; it proves neither the timing nor the real server. |

## SC-002: 30 running agents, 100% shown under their correct parents

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-002 | Linux, macOS | Domain test: generate a company only through `check_placement` and walk every parent chain | Parent chains hold for the generated company (per the test assertion) | PARTIAL (automated) | `tree_rules.tests.rs`, `a_generated_tree_never_loops_or_overflows`. This proves placement at the domain level with fake rows, not 30 live agents in the view. The view half is the mock-core e2e `the tree shows each agent under its parent with role and status as text`. No test runs 30 live agents. |

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
| SC-006 | Linux, macOS | The lead agent calls the office through the helper; the member's fake CLI records the input | The member is created and takes the directive as one input with Enter (per the test assertions) | PASS (automated) | `agent_call.rs`, `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, then `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter`. Both are driven by the agent alone, and both sit on `main` at f767811. |

## Known gaps (open only)

1. **The tree exclusion is proven with test fakes only.** No test drives a real writer through `tree_exclusion`. The review of #31 observed that the exclusion held across blocking stops and that a stop holding the exclusion delays `team.create`. The delay was observed, not measured, and no regression test pins it.
2. **A directive to a deleted target reads as "not running".** The spec's concurrent edge case (the user deletes an agent while its parent sends it a directive) requires the refusal to say the target was deleted. On `main` the `DirectiveTarget` port returns `Result`, and a store failure behind the lookup surfaces as `Storage` with its own message (`send_directive_handler.rs`, tested by `a_store_failure_in_is_running_is_the_error_not_a_no`); a deleted target and a stopped target both get the `NotRunning` wording, so the reason text still does not distinguish them.
3. **The Windows attach of the subtree, call, and directive bridges has no local run.** The three integration files carry `#![cfg(unix)]`, so no local Windows execution happened. The CI Windows job builds the workspace and runs the portable suite on every PR, which covers compilation and the non-unix paths, not the unix-gated flows.
4. **`office-harness-desktop` was never built on the Linux machine used for this work.** gdk-sys does not build there, so every local workspace gate excluded the crate. CI builds it on all three OSes, including `tauri build --no-bundle`.
5. **The team ledger is not updated by the agent path.** An agent-created lead is placed in the agents table and joined into `hierarchy_team_members`, but nothing on the agent path writes `hierarchy_teams.lead_agent_id`: `save_team_and_place_lead` (the `team.create` path) is the only writer, unchanged since #11. The wire tests show the shape this leaves: `a_ceo_creates_a_lead_through_agent_call_and_the_tree_shows_it` has the user run `team.create` afterwards so the tree shows the lead under its team. Pre-existing, flagged for a follow-up.
6. **`team.create` has a check-then-write window.** `CreateTeamHandler` checks `team_name_taken` and then saves with no `TreeExclusion` held across the two steps (verified on `main`). The unique `name_key` index refuses the losing create at the storage layer, so the race cannot produce a duplicate team; the loser gets a storage error instead of the `Conflict` text. Follow-up noted in the #40 review.
7. **PR #52 is open at 3abca65.** It forwards `role.team_id` for an agent-created lead, requires it there, and gates the role keys — closing the two #40 r2 LOWs (a supplied `team_id` is dropped without a word, and unknown keys inside `role` are not gated). Until it merges, `main` keeps that silent trim.
8. **No positive agent-call wire test drives `agent.start` or `agent.new_task`.** Their self refusals are wire-proven and they share the office-request runner that the `agent.stop` leg proves over the wire, but no test walks an agent starting a descendant or starting a new task on one through agent-call end to end. Found during this refresh.
9. **The mock core's second-CEO text pins the id form.** The e2e mock refuses a second CEO with the CEO's id, while `main`'s `SecondCeo` error carries the stored name and falls back to the id only when the name is empty. No product bug (the UI shows the server text verbatim); the mock text diverges from the server text (PR #36 r4 LOW).

## Manual QA

No owner-level UI walkthrough happened. Nobody built a hierarchy in the running app, attached to a real agent terminal, confirmed a subtree delete by hand, or watched a directive arrive in a live session. What stands in its place is the Playwright e2e suite: 26 tests on `main` (`src/apps/desktop/e2e/hierarchy.spec.ts`), covering the keyboard-only US1 build, the counted delete and stop confirmations, the FR-003 create roles and refusal texts, the axe checks (`the create flows run with the keyboard alone and axe stays clean`, `the stop and delete confirmations run axe clean while open`), tree navigation and focus, and the empty, failure, and re-render states. Those tests run against a mock core, and the run for this report is CI's (run 44317624213 on ubuntu), not a local run. They show the screens work; they do not show a person can use them, and SC-001 stays NOT RUN.
