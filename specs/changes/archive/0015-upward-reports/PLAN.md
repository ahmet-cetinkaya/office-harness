---
status: completed
spec: specs/changes/archive/0015-upward-reports/SPEC.md
commits: per-task
---

<!-- Drafted 2026-11-23 with the spec, for one review of both. Review r1 (CHANGES) applied: queue share D7, notification coalescing D6, resume hold D6. Review r2 PASS for coding at fd21cae; its notes N1 and N2 are in D7 and T5. -->

# Plan: Upward reports (0015)

## Summary

A running agent gets one new call, `report.send`, on the connection it already uses for directives. It takes a kind, a text, and an optional "for the person" flag, and nothing that names an agent. The server picks the receiver from the tree: the parent, or the person when the agent has no parent or asks for the person. A report to a parent goes into the parent's 0012 directive queue as one visible-form line with a fixed heading. A report for the person becomes a notification and a Needs you item that keeps the report until the owner opens the terminal or dismisses it.

**What you get:**
- `report.send` on the agent connection; the allow list grows from 7 to 8 types.
- Report rules in the hierarchy domain: four kinds, 1,000 characters, the receiver from the tree.
- A new rate limit "reports sent by one agent" (default 5 per 60 s) with its setting and its row in the desktop Limits dialog.
- A `report` notification event and Needs you attention kind; Needs you items keep the latest report.
- The desktop Needs you row shows the report's kind and text, with a keyboard control for a long text.
- Reports share the 0012 queue per target, so they are typed with the same checks as directives.
- Integration tests through the real helper, desktop unit tests, and Playwright e2e for the Needs you row.

**Why this approach:** everything the channel needs exists already: the agent connection and its refusal recording (0002, 0014), the hold and limit gate (0007), notifications that feed Needs you (`NotificationBridge::record`, as the automatic hold does), the visible form (`subtree_status::visible_text`, 0014), and the safe delivery queue (0012). The change adds one request, one limit, one event kind, and one item field, and passes reports through the existing paths.

**What it will not do:** the spec's Out of scope list: no reply request, no report ids or threads, no answer form in Needs you, no sibling reports.

**Shape of the work:** 7 tasks, one PR, one commit per task. **Effort:** M.

**Main risks:**
- **0012 not on `main`.** T5 (delivery to a parent) needs the 0012 queue. T2 to T4 and T6 (person path and desktop) do not, and go first. T5 starts after PR #115 merges; until then the branch answers no parent report (T5 is the commit that adds the parent path, so no half path ships).
- **A report item cleared by the next status change.** The agent is `working` when it sends; 0008 clears items on `working`. The clear reason is split (D6) and tested (SC-009).
- **Control characters reaching a terminal.** The text is put in visible form once, in the gateway, before any other code sees it (D3); the integration test checks the parent's terminal bytes (SC-004).

**Decisions made for you:** the request name and fields, the heading form, where the visible form is made, the limit's default, the Needs you storage, the clear-reason split, and the desktop row's long-text control.

**Blockers:** PR #115 (0012) must merge before T5. The spec review gate passes first. The build slot rule applies to every build or test run.

## Goal

Deliver spec 0015: a running agent reports upward to its parent or to the person, bounded, visible-form, held and limited like any agent-made action, recorded, and typed only under the 0012 rules.

## Context

Paths: `H` = `src/apps/office-harnessd`, `M` = `src/modules`, `D` = `src/apps/desktop`. Read on `main` at 468ad56 (0013 merged) and on `feat/0012-safe-directive-delivery` at 92d7a4c.

- **Agent connection:** `H/src/server/agent_call.rs`. `ALLOWED_REQUESTS` (7 types), `answer_request` dispatch, `NOT_ALLOWED`, `journal_refusal`, `smuggled_field`, `truncated`, `answer_subtree` (the no-target request pattern), `answer_directive` (blocking pool plus `ActorScope` plus `recorded_and_published`).
- **Visible form:** `H/src/server/subtree_status.rs` `visible_text` (0014 D5).
- **Tree:** `M/hierarchy/domain/src/services/tree_rules.rs` `TreeRow {agent_id, name, role, parent_agent_id, team_id}`; `TreeQuery::store_snapshot`.
- **Holds and limits:** `M/hierarchy/domain/src/services/rate_windows.rs` (`AgentAction`, `RateWindows::try_take`), `rate_limits.rs` (`LimitKind`, `RateLimits`, defaults), `HoldService::guard(caller, action, target, rows)`; settings in `M/agent/domain/src/value_objects/settings.rs` (`SettingKey`, `limit_*`, `parse_limit`); `H/src/bridges/rate_limit_source.rs`; `D/src/modules/hold/models/limits.ts`.
- **Notifications and Needs you:** `M/notification/domain` (`NotificationEvent` and `AttentionKind`, 6 kinds each since 0013 added `ResumeHold`; `AttentionEvent`, `NeedsYouItem`, `order`, `record_event`), `M/notification/application/src/commands/needs_you` (`NeedsYouService`, `NeedsYouClearReason`), `M/notification/infrastructure/src/persistence` (migrations `0001` to `0003`, `NeedsYouRepository`), `H/src/bridges/notification_bridge.rs` (`record_auto_hold`, `record_resume_hold`, `CLEARING_STATUSES`, `record`), the bell coalescing in `M/notification/application/src/commands/create_notification` (a coalesced row answers `Ok(None)`), `H/src/bridges/needs_you_bridge.rs`, `H/src/bridges/hold_bridges.rs` (`HoldReleased`, which clears `AutoHold` and `ResumeHold` items).
- **Action record:** `M/audit/domain/src/action_kind.rs` (`ActionKind`, 13 kinds); `DirectiveJournal` port (`M/hierarchy/application/src/common/ports/directive_ports.rs`) and its bridge in `H/src/bridges/directive_bridges.rs`; `D/src/modules/audit/models/action-entry.ts` (`ACTION_TEXT`).
- **0012 queue (branch):** `M/hierarchy/application/src/commands/send_directive/directive_queue.rs` (`DirectiveQueue`, `QUEUE_LIMIT`, `WaitingDirective`) and `send_directive_handler.rs` (`DirectiveOutcome`, recheck, withdrawals, `DirectiveReceipt::received` on submit).
- **Desktop Needs you:** `D/src/modules/needs-you/models/needs-you.ts` (`NeedsYouKind`, `readItems`, `eventText`, `rowOf`), `components/needs-you-section.svelte`; notifications `D/src/modules/notification/models/notification.ts`; e2e mock `D/e2e/support/mock-core.ts`.
- **Tests:** `H/tests/agent_directive.rs` and `agent_subtree_status.rs` (fake CLIs, `place()`, `call_helper()`, `actions_of()`), `H/tests/needs_you.rs`, `H/tests/agent_hold.rs`.

## Decisions

- **D1 Request:** `report.send`, agent connection only, fields `kind`, `text`, `for_person` (optional boolean), and no others; `invalid` and recorded for any other field (closed allowlist, as `parse_directive`). `actor` and `caller` stay `refused` by `smuggled_field`.
- **D2 Domain rules:** new `M/hierarchy/domain/src/services/report_rules.rs`: `ReportKind {Done, Question, Blocked, Info}` with `from_wire`/`as_str`; `REPORT_MAX_CHARS = 1000`; `validate_report_text` (blank is `Empty`, past the limit is `TooLong`; control characters are allowed, unlike directives); `report_receiver(rows, sender, for_person) -> ReportReceiver {Parent(AgentId), Person}` (the sender's `parent_agent_id`, or `Person` when none or `for_person`).
- **D3 Visible form, once:** `answer_report` in `agent_call.rs` validates the raw text (D2), then calls `visible_text` on the text and on the sender's name, and passes only visible strings on. The heading is built by a pure `report_line(name, sender, kind, text)` in the hierarchy domain: `[report from {name} ({sender}): {kind}] {text}`. The record text is `{kind}: {text}`.
- **D4 Limit:** `AgentAction::Report`, `LimitKind::Report` ("reports sent by one agent"), `RateLimits.report` with `DEFAULT_REPORT_LIMIT = 5`, a `reports` window per caller in `RateWindows`. `SettingKey::LimitReportsPerAgent` (`limit_reports_per_agent`, 1 to 1000, default 5), mapped in `SettingsRateLimits`. The hold target is the parent, or `None` for the person (an office or agent hold on the sender or an ancestor covers it, 0007/FR-004).
- **D5 Handler:** new `M/hierarchy/application/src/commands/send_report/` `SendReportHandler` with ports: the store (snapshot), the hold guard, a `ReportJournal` (accepted, refused, per D9), a `PersonReports` port (`deliver(sender, kind, text) -> Result<(), String>`), and, from T5, the directive handler's queue. Order: snapshot, receiver, running check of the parent, hold and limit, then delivery. Every refusal journals before it returns (FR-009).
- **D6 Needs you report field and clear split:** `NeedsYouItem.report: Option<ItemReport {kind: String, text: String, at: Timestamp}>`; migration `0004_needs_you_report.sql` adds `report_kind`, `report_text`, `report_at` (NULL for old rows). `record_event` keeps the old report unless the event carries one. `AttentionKind::Report` and `NotificationEvent::Report` (`"report"`, detail = report kind), in the waiting group with `AutoHold` and `ResumeHold`; `ALL` counts 7 in both. `NeedsYouClearReason::StatusChanged` splits into `Resumed` (`starting`, `working`) and `Stopped`; `Resumed` and `HoldReleased` (for `AutoHold` and `ResumeHold` alike) do not clear an item that holds a report. The create handler coalesces a `Report` like a `Bell`: while an unread `report` row of the agent exists, it adds no row and answers `Ok(None)`, and the bridge still feeds Needs you (0003/FR-006 (a) as modified). `NotificationBridge::record_report(agent_id, kind, text)` mirrors `record_auto_hold`; the `PersonReports` bridge calls it. A report notification is never read on arrival.
- **D7 Parent delivery (T5, after 0012):** the 0012 `WaitingDirective` gains an origin `Directive | Report {sender_kind}`; `SendDirectiveHandler` gets `enqueue_report(sender, parent, line, record_text)` that runs the same readiness, typing, input-line check, and Enter as a directive. For a report: the recheck asks "is the sender still a child of the parent" instead of "is the target still a descendant"; on submit `DirectiveReceipt::received` is not called (PB-3); journal entries use `ActionKind::Report`. The queue limit of 20 counts reports and directives together; inside it, `REPORT_SHARE = 10` waiting reports at most (FR-010). `enqueue_report` first looks for a waiting report of the same sender in the parent's queue: if one exists, the new one takes its place, and the old one is journaled as withdrawn with `replaced by a newer report`; otherwise, with 10 reports waiting, it refuses with `the parent's queue has no room for reports`. Both checks run before the hold guard, as 0012 runs queue-full first, so neither refusal counts toward a limit or the automatic hold. A replacement never touches the head that the delivery has claimed: when the sender's waiting report is that head, the newer report goes to the back of the queue (inside the 10-report share) instead, and the head is typed only when taking it returns the claimed entry (the #115 r5 fix). A replacement keeps the replaced entry's place but gets its own acceptance time, so the 10-minute limit counts from the newer report; an entry that expires is withdrawn when it reaches the front, never typed late (review r2 N1, N2). The answer maps `DirectiveOutcome` to `delivery`.
- **D8 Answer:** `{receiver, delivery, position?}` per the contract.
- **D9 Record:** `ActionKind::Report` (`"report"`). Accepted to a parent: one entry, agent = parent, actor = sender. For the person: agent = sender. Refusals: `refused` in the sender's record and, with a parent, in the parent's (as 0007/FR-010 does for directives). Withdrawn and unsubmitted: the 0012 journal path, kind `report`.
- **D10 Desktop:** `NeedsYouKind` adds `report`; `readItems` reads `report` (`{kind, text, at}` or null); `eventText` writes "sent a report: <kind>"; the row shows the report text under the event line, clamped to 3 lines, with a "Show the whole report" / "Show less" button (`aria-expanded`, visible focus) when the text is longer than 160 characters; the text element is the row's `aria-describedby`. `notification.ts` adds `report` ("sent a report: <kind>"); `limits.ts` adds the reports row; `action-entry.ts` adds `report: "Sent a report"`; `mock-core.ts` gets the new setting, a way to push a report item, and the notification.
- **D11 Refusal text:** `NOT_ALLOWED` becomes "an agent may only create, start, stop, give new tasks to, send directives to, and read the state of its own descendants, and send reports to its parent or the person"; the `agent_hold.rs` prefix check stays.
- **D12 Constitution check:** AGPL header on new files; no new dependency; module boundaries kept (hierarchy does not depend on notification: the person path is a port that the server app's bridge implements); no code from any other project.

## Scope

- **In:** D1 to D12, tests, contract amendments (0002, 0003, 0007, 0008), README agent-call section, `qa-report.md`, the spec report page.
- **Out:** the spec's Out of scope list.

## Tasks

### T1: Spec, contract, plan (no product code)

SPEC.md, `checklists/requirements.md`, `contracts/report-protocol.md`, this plan. **Accept:** the spec review gate passes; no product file changes.

### T2: Domain and limits (D2, D4, D9)

`report_rules.rs` with unit tests (each kind, an unknown kind, 1,000 and 1,001 characters, blank, controls allowed, receiver: parent, no parent, `for_person`, the CEO). `report_line` test with a visible name. `AgentAction::Report`, `LimitKind::Report`, the window, the default, unit tests (5 pass, 6th refused, the directive windows untouched). `SettingKey::LimitReportsPerAgent` with parse tests. `ActionKind::Report`. **Accept:** `cargo test -p hierarchy-domain -p agent-domain -p audit-domain` and clippy pass.

### T3: Needs you report and notification (D6)

`NotificationEvent::Report`, `AttentionKind::Report`, `ItemReport`, `record_event` keeps the report, the group rank, the migration, the repository read and write, the clear split, `record_report` on the bridge, the hold bridge's release, the status bridge's `Resumed`/`Stopped`, the wire mapping in `needs_you_changed_event` and `needs_you.list`. Unit tests: a report survives a later wait event, a `Resumed` clear, and a `HoldReleased` clear of an `AutoHold` and of a `ResumeHold` item; `Stopped`, `TerminalOpened`, dismiss, and delete clear it; a second report while the first report notification is unread adds no notification row and still updates the item; a second report replaces the first and keeps the wait time; order puts a report in the waiting group; an old row without report columns reads as `report: null`. **Accept:** notification crates and office-harnessd unit tests pass; `migrations` test counts updated.

### T4: Request and person path (D1, D3, D5 without the queue, D8, D11)

`SendReportHandler` with the person path and every refusal; `answer_report` in `agent_call.rs`; the allow list (8); `NOT_ALLOWED`; the `PersonReports` bridge; bootstrap wiring. A report to a parent answers `refused` "reports to a parent are not delivered yet" only until T5 lands in the same PR; the commit message says so. Unit tests for field refusals and the visible form. **Accept:** unit tests pass.

### T5: Parent delivery through the 0012 queue (D7)

After PR #115 is on `main`: merge `main`, then the queue origin, `enqueue_report`, the recheck, the journal kind, no receipt, and the answer mapping; T4's interim refusal is removed. Unit tests in `send_directive_handler.tests.rs`: a report waits behind a directive and is typed in order; a report to a parent with a question on screen is not typed; a moved sender's report is withdrawn; a submitted report leaves the parent's task text unchanged; a second report of one sender replaces its waiting one in place and journals the withdrawal; a second report while the first is the claimed head goes to the back and both are typed in order; a replacement's 10-minute limit counts from its own acceptance; with 10 reports waiting an 11th is refused and a directive is still accepted (SC-011); neither refusal takes a rate slot. **Accept:** hierarchy application and office-harnessd unit tests pass.

### T6: Integration and desktop tests (SC-001 to SC-012)

New `H/tests/agent_report.rs` with the real server and helper:
- SC-001: CEO, 2 leads, 6 members, all running and waiting for input; each member and lead reports; each report reaches only its parent's terminal (marker per sender).
- SC-002: 10 frames with `target_agent_id`, `agent_id`, `parent_agent_id`, `team_id`, `to`: `invalid`, recorded, nothing typed, no Needs you item.
- SC-003: the CEO reports; `needs_you.changed` with the report within 2 s; a `report` notification.
- SC-004: text with U+001B, U+009B, U+202E, and LF: the parent's typed bytes, the Needs you item, and the record text hold none of them as themselves; the typed line has no LF before the Enter.
- SC-005: 1,000 accepted; 1,001 and blank refused and recorded.
- SC-006: a parent with a permission question on screen; 10 reports; no bytes typed while it shows.
- SC-007: office hold refuses 5 of 5; 6th report in 60 s refused with the limit; the third limit refusal puts the sender on hold.
- SC-008: the lead sends a directive to the id in the heading; the member receives it.
- SC-009: the CEO's report item survives `working` and `waiting_for_input`; `notification.read_agent` clears it; a second item clears on dismiss.
- SC-010: the record entries of D9 for accepted, refused, withdrawn (stop the parent while a report waits), replaced, and unsubmitted reports.
- SC-011: a working lead with 11 members (placed by the user); 10 members report; the CEO's directive to the lead answers `waiting`; the 11th member's report is refused with the no-room reason; a member that reports twice has one waiting entry.
- SC-012: the CEO sends 5 reports for the person; exactly 1 `report` notification row; the item shows the 5th report.
Desktop: `needs-you.spec.ts` (report kind, report text, long-text control, old items without `report`), `notification.spec.ts`, `limits` and `action-entry` specs; Playwright `D/e2e/needs-you-report.spec.ts` on the mock core: a report row shows kind and text, the toggle works from the keyboard with `aria-expanded`, the jump shortcut opens the terminal and the row leaves, axe finds no violation in the section.
**Accept:** `agent_report` passes 3 times in a row locally; bun unit tests, svelte-check, eslint, prettier, and the Playwright file pass.

### T7: Docs and QA

Contract amendment notes in 0002 `hierarchy-protocol.md`, 0003 `panel-protocol.md`, 0007 `hold-protocol.md`, 0008 `needs-you-protocol.md`. README: the agent call section lists `report.send` with an example; the Limits paragraph names the new limit. `qa-report.md` with a row per FR, SC, and PB and its evidence. The spec report page under `.agents/work/spec-reports/` (outside the repo). **Accept:** every row has evidence or a stated gap; the diff names no other project.

## Dependencies

T2 before T3 and T4; T3 before T4; T4 before T5; T5 needs PR #115 on `main`; T6 after T5 (its parent tests need T5; its person and desktop tests can run after T4); T7 last. T1 is the PR's first commit.

## Commit strategy

`per-task`: one signed commit for each of T1 to T7 on `feat/0015-upward-reports`, one PR to `main`. A merge commit of `main` comes before T5.

## QA and evidence

Local gates per run: `cargo fmt --check`, clippy on the touched crates, the unit tests of T2 to T5, the integration files `agent_report`, `agent_call`, `agent_directive`, `agent_hold`, `needs_you`, `notifications`, `migrations`; desktop `bun test`, svelte-check, eslint, prettier, and Playwright for the Needs you specs. CI covers three OSes. Each local run follows the floor's build-slot rule.

## Final verification

All tasks accepted; CI green on three OSes; `qa-report.md` complete; the diff names no other project; the spec review gate's verdict at the final head.

## Traceability

| Criterion | Task | Evidence |
|---|---|---|
| FR-001 | T2, T4, T6 | rule tests; SC-005 |
| FR-002 | T2, T4, T6 | receiver tests; SC-001, SC-002 |
| FR-003 | T4, T6 | visible-form tests; SC-004 |
| FR-004 | T5, T6 | queue tests; SC-001, SC-006 |
| FR-005 | T4, T5, T6 | not-running refusal; moved-sender withdrawal |
| FR-006 | T2, T4, T6 | window tests; SC-007 |
| FR-007 | T3, T4, T6 | SC-003 |
| FR-008 | T3, T6 | item tests; SC-009 |
| FR-009 | T2, T4, T5, T6 | SC-010 |
| FR-010 | T5, T6 | queue-share unit tests; SC-011 |
| 0002/FR-008 (modified) | T4, T7 | allow list; contract note |
| 0003/FR-006 (modified) | T3, T6 | `report` notification never read on arrival; coalescing test; SC-012 |
| 0008/FR-002, FR-003, FR-005 (modified) | T3, T6 | item, order, and clear tests; desktop specs; e2e |
| SC-001 to SC-012 | T6 | `agent_report.rs`, desktop specs, e2e |
| PB-1 | T6 | `agent_call.rs`, `agent_directive.rs` stay green |
| PB-2 | T5, T6 | 0012 tests stay green |
| PB-3 | T5, T6 | task text unchanged after a submitted report |
| PB-4 | T3, T6 | existing Needs you tests and e2e stay green |
| PB-5 | T4 | no configuration file touched (diff review) |
| PB-6 | T2 | existing limit tests stay green |
| PB-7 | T4 | no event on the agent connection; existing protocol tests green |
