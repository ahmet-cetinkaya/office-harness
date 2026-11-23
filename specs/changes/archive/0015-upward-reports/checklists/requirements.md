# Requirements checklist: 0015-upward-reports

Spec: `specs/changes/archive/0015-upward-reports/SPEC.md` · Status: shipped · Date: 2026-11-23 · Result: 16 of 16 items pass after four rounds. No `[NEEDS CLARIFICATION]` marker; the judgment calls (any agent may mark a report for the person, A7; control characters are shown, not refused, A3) are flagged for the review gate in the Clarifications. Every `0002/`, `0003/`, `0007/`, `0008/`, and `0014/` reference was checked against the specs on `main` at 468ad56; every `0012/` reference against `feat/0012-safe-directive-delivery` at 92d7a4c.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two).
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | "Questions meant for the person" had no rule for who may send one, or how it differs from a report to the parent. | One "for the person" choice on any report; an agent without a parent always reaches the person (FR-002, A7). |
| 2 | Hostile reader | (a) A field could name another agent. (b) A line break in the text could submit part of it early in the parent's CLI. (c) A report item would leave Needs you as soon as the sender printed output, because 0008 clears an item on `working`. (d) A burst of member reports would hit the directive-per-target limit. (e) A submitted report could become the parent's task line. | (a) FR-002, closed field list in the contract, SC-002. (b) FR-003 and A3: line breaks are visible symbols too; one line. (c) FR-008 and 0008/FR-005 as modified. (d) A5: a limit of its own, not the directive limits; the 0012 queue absorbs the burst. (e) PB-3. |
| 3 | Secure-engineering contract | The data flow and the prompt-injection path upward were not named; a fake heading inside the text was not covered. | Data flow and trust boundary section with the residual risk; edge case for a fake heading. |
| 4 | Spec review r1 (CHANGES) | M1: children's reports could fill the parent's 20-entry queue and lock out directives from above. L1: one OS notification per report for the person could flood the owner. L2: the plan and contract predated 0013's resume hold. | FR-010 (one waiting report per sender, replaced in place; at most 10 reports per queue), edge rows, SC-011, PLAN D7 and T5 tests; bell rule for report notifications in 0003/FR-006 (a), SC-012; resume hold named in 0008/FR-003, FR-005, the contract, and PLAN D6. |

## Notes

- **Size rule:** 10 ADDED FRs and 5 MODIFIED requirements; the M size the gap report set for this row. A split into "person path" and "parent path" was considered and declined: the parent path is the main value, and both share the request, the limit, and the record.
- **Dependency:** the parent path depends on 0012's queue; the plan builds it after 0012 is on `main`.
