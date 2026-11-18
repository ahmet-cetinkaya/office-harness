# 0002 QA report

## Summary

- **Date:** 2026-11-18.
- **Build under test:** `main` at 6a15b77 plus the two open review heads: PR #40 (slice 2 of the agent gateway) at a8a0228 and PR #36 (create flows, T7) at 3a63400. The merged slice PRs are #11, #25, #29, #30, #31, #33, and #35.
- **Overall status:** the automated evidence is recorded at the heads below. The owner-level UI walkthrough did not happen (see Manual QA). PRs #40 and #36 were still open when this file was written.
- **Status of this file:** every sha, PR number, file path, and test name below was read from the repository and from the GitHub PR records on 2026-11-18. No value in this file is an estimate.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap row states what is not. |
| NOT RUN | Nobody ran the check. It is not a PASS and not a WAIVER: no person performed it. |

### Scope of the automated evidence

- The server tests use fake CLIs, not the real agent CLIs.
- The integration tests in `src/apps/office-harnessd/tests/agent_call.rs`, `agent_directive.rs`, and `agent_subtree.rs` carry `#![cfg(unix)]`. They run on Linux and macOS only. On Windows, CI still builds them and runs the rest of the suite.
- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server.
- Every commit at every head cited below is GPG-signed: `git log --format=%G?` reports `G` for 71c1fdd, 2c16405, d6cfe13, f5d7042, 2e9ddde, d9adbd0, cbe6fbf, a8a0228, and 3a63400.
- The local Rust gates are `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` with the pedantic lints, run through `scripts/lint-rust.sh` on the toolchain that `rust-toolchain.toml` pins (1.100.0). The workspace tests ran with `src/apps/desktop/src-tauri` excluded: the `office-harness-desktop` crate needs gdk-sys and does not build on the Linux machine used for this work. CI builds that crate on all three OSes.

### Local gates per slice

The gate result lines come from the build record kept during the run; the signature status was checked for this report.

| Slice | PR | Head | Gates | Recorded result |
|---|---|---|---|---|
| Phase 1: roles, teams, tree storage (+ T5 S1, S2) | #11 | 71c1fdd | workspace tests (desktop excluded), boundaries, lint, fmt | Full gate at each review round; 68 test binaries, 0 failed at the round head 295c04e; re-gated to 71c1fdd |
| T6: desktop tree view | #25 | 2c16405 | webview gates (`bun test`, `check`, `lint`, `format:check`) | Webview-only slice; Rust gates untouched |
| T8: README and agent-call docs | #29 | d6cfe13 | docs only | No code gates |
| T5 S4: directive | #30 | f5d7042 | workspace tests (desktop excluded), boundaries, lint, fmt | 523 tests, boundaries exit 0, lint clean at f86f66e; review fixes re-gated at f5d7042 |
| T5 review lows | #31 | 2e9ddde | workspace tests (desktop excluded), boundaries, lint | 527 tests, boundaries exit 0 |
| T5 S3: create with role | #33 | d9adbd0 | workspace tests (desktop excluded), boundaries, lint | 16 create-role tests, boundaries exit 0 at the draft head 565425f; fixes re-gated at d9adbd0 |
| T5 S5 slice 1: gateway bridges | #35 | cbe6fbf | full workspace build and tests with the desktop exception | 554 tests |
| T5 S5 slice 2: gateway commands and races | #40 | a8a0228 | full workspace build and tests with the desktop exception, race suite | 70 suites, 0 failed; all race tests green |
| T7: create flows and confirmations | #36 | 3a63400 | webview gates and Playwright; review PASS | Awaiting #40 per the merge order |

`scripts/check-boundaries.sh` exited 0 on every slice that changed Rust code. One boundaries failure happened during #11 (the hierarchy crates depended on the agent domain for `ProjectId` only); it was fixed by moving `ProjectId` to the kernel (d2dba27), not by weakening the check.

### CI

CI runs on ubuntu-latest, macos-latest, and windows-latest: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (including `office-harness-desktop`), `scripts/check-boundaries.sh --self-test` and the check itself, the webview lint and unit tests, the Playwright e2e run, and `tauri build --no-bundle`.

- Merged green on all three OSes at merge, per the merge records and the check rollups: #11 (merge 4f009da), #25 (4292e93), #29 (e4d9103), #30 (e873371), #31 (bc068d3), #33 (d03dbc3), #35 (99f14d1).
- Open with green CI at head, three SUCCESS checks each: #40 at a8a0228 and #36 at 3a63400.

## FR-001: The tree with CEO, team lead, and member

| FR | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `src/modules/hierarchy/domain/src/services/tree_rules.tests.rs`: `a_lead_sits_under_the_ceo_when_one_exists`, `a_member_needs_a_lead_parent`, `descendants_go_deepest_first_and_exclude_the_agent`, `descendant_checks_follow_the_parent_chain`, `a_generated_tree_never_loops_or_overflows`, `a_looped_tree_terminates_instead_of_hanging`. Role value object: `src/modules/hierarchy/domain/src/value_objects/role.tests.rs`. Storage: `src/apps/office-harnessd/tests/migrations.rs`. |

## FR-002: Create a team with name, optional project, and lead

| FR | Verdict | Evidence |
|---|---|---|
| FR-002 | PASS (automated) | Name rules: `src/modules/hierarchy/domain/src/value_objects/team_name.tests.rs`, `a_name_trims_and_lowers_for_its_key` and `an_empty_or_oversize_name_is_refused`. Duplicate and case-only duplicate names are refused by the unique name key, proven at the database level in `src/apps/office-harnessd/tests/migrations.rs`, `a_name_key_that_differs_only_by_case_is_refused`. The `team.create` dispatch sits in `src/modules/hierarchy/presentation/src/handlers/hierarchy_handlers.rs`, and the US1 flow that creates a team runs in the e2e test `US1 builds a company from an empty office with the keyboard alone` (mock core, #36 head). The `create_team` handler itself has no dedicated unit test; its rules are covered by the pieces above. |

## FR-003: Create an agent with a role only where allowed, naming the allowed roles

| FR | Verdict | Evidence |
|---|---|---|
| FR-003 | PASS (automated) | `tree_rules.tests.rs`: `a_second_ceo_is_refused_and_names_the_first`, `a_member_needs_a_lead_parent`. `src/modules/hierarchy/application/src/commands/create_role_agent/create_role_agent_handler.tests.rs`: `a_second_ceo_is_refused_and_creates_nothing`, `a_refused_create_maps_to_the_matching_error_kind`, `a_failed_placement_removes_the_created_agent`. Webview: e2e `the create actions list only the roles the place allows (FR-003)` and `a refused create shows the server's refusal text (FR-003)` (#36 head, mock core). |

## FR-004: Each agent shows under its parent in the hierarchy view

| FR | Verdict | Evidence |
|---|---|---|
| FR-004 | PASS (automated) | Server: `src/apps/office-harnessd/tests/agent_call.rs` (a8a0228), `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it` reads the tree over the wire after an agent creates a member. Webview: `src/apps/desktop/src/modules/hierarchy/state/hierarchy-controller.spec.ts` (`it reads the tree on load`, `a hierarchy.changed event reads the tree again`, `an older tree reply never overwrites a newer one`) and `src/apps/desktop/e2e/hierarchy.spec.ts`: `the tree shows each agent under its parent with role and status as text`, `a hierarchy.changed event re-renders the tree`. |

## FR-005: No fixed maximum on the number of agents

| FR | Verdict | Evidence |
|---|---|---|
| FR-005 | PASS (automated) | No cap exists in the placement rules or the schema. `tree_rules.tests.rs` builds a generated multi-level company only through `check_placement` and walks every parent chain without a cap (`a_generated_tree_never_loops_or_overflows`). The same test proves a looped tree terminates instead of hanging (`a_looped_tree_terminates_instead_of_hanging`). |

## FR-006: Counted delete confirmation

| FR | Verdict | Evidence |
|---|---|---|
| FR-006 | PASS (automated) | `src/apps/office-harnessd/tests/agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` and `a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing` (a stale count deletes nothing). Handler unit tests: `src/modules/hierarchy/application/src/commands/subtree/subtree_handler.tests.rs`. Webview (#36 head): `the delete confirmation states the counted descendants and cancel deletes 0`, `a confirmed lead delete removes the lead and its 2 members`, `a stale counted delete shows the server's recount refusal`. |

## FR-007: Offer to stop all descendants in the same action

| FR | Verdict | Evidence |
|---|---|---|
| FR-007 | PASS (automated) | `agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` stops the subtree over the wire, one event per affected agent. Webview (#36 head): `the stop confirmation offers only-this-agent and with-descendants choices`, `a confirmed only-this-agent stop sends include_descendants false`, `a confirmed stop with descendants sends include_descendants`. |

## FR-008: A running agent acts only within its own descendants

| FR | Verdict | Evidence |
|---|---|---|
| FR-008 | PARTIAL (automated) | Rules: `src/modules/hierarchy/domain/src/services/call_rules.tests.rs` (`a_direct_descendant_passes`, `the_caller_itself_is_refused`, `a_requested_descendant_lead_is_kept`). Over the wire: `agent_call.rs` (a8a0228) `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, `a_lead_stops_its_member_through_agent_call`; `agent_directive.rs` `a_directive_to_a_running_non_descendant_is_refused_and_recorded`. Handler: `send_directive_handler.tests.rs`, `a_directive_to_a_running_descendant_is_written_and_journaled_clean`. The domain rules and the lead-on-its-member flows hold; the gateway itself does not enforce descendants-only (it accepts the caller as the target of start, stop, and `new_task`) and it forces every agent create to `member`, so a CEO agent cannot create a lead. Known gap 6 names the open #40 FAIL behind this. |

## FR-009: Refuse, tell the agent the reason, record the refusal

| FR | Verdict | Evidence |
|---|---|---|
| FR-009 | PARTIAL (automated) | `agent_call.rs`: `an_agent_may_not_ask_for_a_type_outside_the_allowlist` (the allowlist stays closed) and, at a8a0228, `an_unsupported_command_or_field_is_refused_and_recorded` (also refuses an `additions` field). `agent_directive.rs`: `a_directive_to_a_running_non_descendant_is_refused_and_recorded`, `an_escape_in_the_text_is_invalid_and_recorded`, `a_request_carrying_an_actor_field_is_refused_and_recorded`. Journal: `src/apps/office-harnessd/src/bridges/directive_bridges.tests.rs`, `a_gateway_refusal_journals_the_caller_as_actor_and_agent`. The one gap: a client-supplied `role` on `agent.create` is replaced with `member` under the caller instead of refused. Known gap 6 names the open #40 FAIL behind this. |

## FR-010: A directive enters as one unbroken input followed by Enter

| FR | Verdict | Evidence |
|---|---|---|
| FR-010 | PASS (automated) | Writer: `directive_bridges.tests.rs`, `the_writer_delivers_text_and_enter_in_one_write` (text plus Enter in one write call). End to end: `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter` reads the bytes the fake CLI recorded. |

## FR-011: Directives and refusals in the action record, with a named actor

| FR | Verdict | Evidence |
|---|---|---|
| FR-011 | PASS (automated) | Actor scope: `src/modules/agent/application/src/common/actor_scope.tests.rs` (`the_default_actor_is_the_user`, `an_entered_agent_actor_holds_until_the_guard_drops`, `a_scope_on_another_thread_does_not_leak`, `a_dropped_scope_leaves_no_actor_on_a_reused_thread`). Identity: `agent_call.rs`, `an_agent_proves_itself_and_the_office_names_it`. Refusals recorded with the caller as actor: `a_gateway_refusal_journals_the_caller_as_actor_and_agent` and the recorded-refusal tests of `agent_directive.rs`. Journal text rules: `a_control_character_in_a_refused_text_is_escaped_in_the_journal`, `a_nine_thousand_character_text_is_capped_at_the_bound_with_a_hash`. |

## SC-001: A 15-agent hierarchy built from the desktop app in under 10 minutes

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-001 | any | A person builds 1 CEO, 2 leads, 12 members from the app alone, timed | Not measured | NOT RUN | Manual QA did not happen (see Manual QA). The e2e test `US1 builds a company from an empty office with the keyboard alone` (#36 head) proves the flow and the keyboard path against a mock core; it proves neither the timing nor the real server. |

## SC-002: 30 running agents, 100% shown under their correct parents

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-002 | Linux, macOS | Domain test: generate a company only through `check_placement` and walk every parent chain | Parent chains hold for the generated company (per the test assertion) | PARTIAL (automated) | `tree_rules.tests.rs`, `a_generated_tree_never_loops_or_overflows`. This proves placement at the domain level with fake rows, not 30 live agents in the view. The view half is the mock-core e2e `the tree shows each agent under its parent with role and status as text`. No test runs 30 live agents. |

## SC-003: 20 disallowed agent requests, 100% refused and recorded

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-003 | Linux, macOS | Each refusal class proven by its own test; every refusal asserted in the action record | Every tested class is refused and recorded (per the test assertions) | PARTIAL (automated) | The classes proven: a type outside the allowlist (`an_agent_may_not_ask_for_a_type_outside_the_allowlist`), an unsupported command or an `additions` field (`an_unsupported_command_or_field_is_refused_and_recorded`), a directive to a non-descendant (`a_directive_to_a_running_non_descendant_is_refused_and_recorded`), an actor field smuggled in a request (`a_request_carrying_an_actor_field_is_refused_and_recorded`), invalid directive text (`an_escape_in_the_text_is_invalid_and_recorded`), the caller itself as target (`the_caller_itself_is_refused`), and a stale delete count (`a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing`). The criterion's single 20-request batch test was not built. |

## SC-004: 100 directives land unbroken while the user types

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-004 | Linux, macOS | One write per directive, read back from the target's terminal record | One directive lands as one input plus Enter (per the test assertion) | PARTIAL (automated) | The mechanism is proven: `directive_bridges.tests.rs`, `the_writer_delivers_text_and_enter_in_one_write`, and end to end in `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter`. The 100-directive loop with concurrent user typing was not built. |

## SC-005: A confirmed lead delete removes exactly N + 1; a cancel removes 0

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-005 | Linux, macOS | Delete a parent over the wire; delete again with a stale count | The subtree is removed after confirm; a stale count deletes nothing (per the test assertions) | PASS (automated) | `agent_subtree.rs`: `a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree` and `a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing`, with fake CLIs against the real server. Webview half (#36 head, mock core): `a confirmed lead delete removes the lead and its 2 members` and `the delete confirmation states the counted descendants and cancel deletes 0`. |

## SC-006: A lead creates a member and sends it a directive with 0 user actions

| SC | OS | Method | Measured value | Verdict | Evidence |
|---|---|---|---|---|---|
| SC-006 | Linux, macOS | The lead agent calls the office through the helper; the member's fake CLI records the input | The member is created and takes the directive as one input with Enter (per the test assertions) | PASS (automated) | `agent_call.rs` (a8a0228), `a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it`, then `agent_directive.rs`, `a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter`. Both are driven by the agent alone. These tests exist only at the a8a0228 head; they are not on `main` yet. |

## Known gaps and parked review findings

1. **The tree exclusion is proven with test fakes only.** No test drives a real writer through `tree_exclusion`. The review of #31 observed that the exclusion held across blocking stops and that a stop holding the exclusion delays `team.create`. The delay was observed, not measured, and no regression test pins it.
2. **A directive to a deleted target reads as "not running".** The spec's concurrent edge case (the user deletes an agent while its parent sends it a directive) requires the refusal to say the target was deleted. `SendDirectiveHandler` asks the `DirectiveTarget` port and reports `Invalid` with the "not running" wording; the reason text does not distinguish deleted from stopped. A store failure behind the lookup no longer reads as not running in the build under test: at a8a0228 the port returns `Result` and the handler surfaces `Storage` (PR #40). On `main` the bridge still maps a store failure to not running until #40 merges.
3. **The Windows bootstrap attach of the subtree bridge has no local run.** The three integration files carry `#![cfg(unix)]`, so no local Windows execution happened. The CI Windows job builds the workspace and runs the portable suite on every PR, which covers compilation and the non-unix paths, not the unix-gated flows.
4. **`office-harness-desktop` was never built on the Linux machine used for this work.** gdk-sys does not build there, so every local workspace gate excluded the crate. CI builds it on all three OSes, including `tauri build --no-bundle`.
5. **PRs #40 and #36 were open when this report was written.** The SC-006 and FR-008 evidence from the gateway slice sits at a8a0228, and the confirmation-flow evidence sits at 3a63400. Neither is on `main` yet; the recorded merge order is #40, then #36.
6. **The #40 review FAIL on role honouring and descendants-only is the blocker for FR-008 and FR-009.** The review of #40 at a8a0228 found that the gateway accepts the caller itself as the target of start, stop, and `new_task` (`agent_call.rs` skips the descendant check when `target == caller`, so `the_caller_itself_is_refused` holds only for directives), that `allowed_fields_of` forces every agent create to a `member` role under the caller (a CEO agent cannot create a lead, and no test covers one), and that a client-supplied `role` is replaced instead of refused. These findings keep FR-008 and FR-009 at PARTIAL above; re-rate both after #40 r2.

## Manual QA

No owner-level UI walkthrough happened. Nobody built a hierarchy in the running app, attached to a real agent terminal, confirmed a subtree delete by hand, or watched a directive arrive in a live session. What stands in its place is the Playwright e2e suite: 14 tests on `main` (`src/apps/desktop/e2e/hierarchy.spec.ts`), 26 at the #36 head, including the keyboard-only US1 build, the counted confirmations, the axe check (`the create flows run with the keyboard alone and axe stays clean`), and the collapse, empty-state, and failure states. Those tests run against a mock core. They show the screens work; they do not show a person can use them, and SC-001 stays NOT RUN.
