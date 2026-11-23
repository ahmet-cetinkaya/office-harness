# Requirements checklist: 0020-temporary-agents

Spec: `specs/changes/0020-temporary-agents/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0007/`, `0008/`, `0010/`, `0012/`, `0013/`, `0015/`, and `0018/` reference was checked against the specs on this branch.

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
| 1 | Requirements checklist | The temps view showed a "quiet time" that no term defined. The objective's place among the directive rate limits had no rule. | A "Quiet time" term. A3 says the objective counts toward no directive rate limit and the create counts toward "agents created by one agent". |
| 2 | Hostile reader | An exit of a temp with no running parent would notify the person twice (0003/FR-006 and FR-007). An end notice that finds no room in the parent's queue had no rule. A person's stop of a temp from its details ended it without the confirmation that End asks. An edge case sent an end notice to the person for a refused `done` report, which FR-007 does not cover. The subtree stop confirmation lived in an assumption only. | FR-007 notifies only for an idle-limit end whose notice reaches no queue; an exit or failure keeps its 0003 notification. FR-009 asks the same confirmation for a single stop by the person. The edge case now says the temp may mark the report for the person. The subtree stop keeps the 0002 offer, and an edge case says that temps among the descendants end. |
| 3 | Hostile reader on the fixed text | 0018 sleep could keep a temp alive without end, since the idle limit counted only running temps. | FR-004 counts a temp that is asleep as quiet, so 0018 needs no change. |
| 4 | Spec review r1 | M1: a new task or a continued session on a temp passes through a stop and a start, which a poll could read as an end. M2: the idle limit ended a temp that waits on a person's question. L1: results in files that git ignores were not named. | FR-005 decides each end on the status under the order of status changes, a scenario, an edge row, and SC-012. FR-004, Terms, an edge row, SC-005, and A12 pause the idle limit while the temp has a Needs you item. An edge row, FR-009, and the finished entry term name ignored files. |

## Notes

- **Size rule:** 9 ADDED and 1 MODIFIED requirement, 10 in all, which is at the limit and not over it. No split.
- **Delegation:** drafted while the owner is away; the open product choices are answered under Clarifications and A1 to A11 for the review gate to confirm.
