# Requirements checklist: 0018-idle-sleep

Spec: `specs/changes/archive/0018-idle-sleep/SPEC.md` · Status: shipped · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0007/`, `0008/`, `0010/`, `0012/`, `0013/`, `0014/`, and `0015/` reference was checked against the specs on this branch.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience. (Deviations is kept empty, as the contract asks, until execution.)

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
| 1 | Requirements checklist | "Works" in a scenario, two preserved behaviors, an accessibility line, an edge case, and a clarification. "Cleanly" in the intent had no observable. The "Setting not saved" state had no edge case. | Each "works" now names the observable (the office carries out the action, continues the session). "Cleanly" reads "with the normal stop". An edge case covers a setting that cannot be saved. |
| 2 | Hostile reader | A wake pressed during a sleep stop that then fails had no rule. A sleep could add a stop entry beside the sleep entry. A bell from the ending CLI would add a Needs you item to an asleep agent. An agent-made stop of an asleep agent had no rule. A paste with a line break could count as the wake key. The person's stop or new task during the stop had no rule. A reader could take the sleep's end as an exit under 0003/FR-006. | FR-007 starts nothing and says "no longer asleep" when the stop fails. FR-003 records one sleep entry and no stop entry, and no event of the ending CLI notifies or adds an item. A10 and an edge case: an agent-made stop is refused as for an agent that is not running. Edge cases: only the Enter key wakes; the person's action runs after the CLI ended. Context states that a sleep is not an exit. |
| 3 | Hostile reader on the fixed text | FR-002 began on a "WHEN idle time passes" event, so an agent whose blocking condition cleared later never had a trigger. SC-003 could not test condition (a) "not idle" while the agent stays idle. PB-7 named Needs you as a place that shows `asleep`, but Needs you shows events, not statuses, and holds no item of an asleep agent. | FR-002 starts within 60 seconds of the first moment at which the agent is idle long enough and no blocking condition holds. SC-003 tests (b) to (f) by idle time and (a) by 10 tries of a change to `working` between the checks. PB-7 drops Needs you. |

## Notes

- **Size rule:** 9 ADDED and 1 MODIFIED requirement, 10 in all, which is at the limit and not over it. No split.
- **MODIFIED:** only 0001/FR-014 gets a new status. 0003/FR-006 stays as it is: a sleep is not an exit (the agent shows `asleep`), and FR-003 says that a sleep notifies nobody, as the person's stop does. 0015/FR-005 stays as it is: FR-005 of this change gives the more specific reason "not running because it is asleep", which is still the "not running" answer.
- **Decisions:** every direction came from the caller under the owner's spec delegation. The one choice left open, a Wake pressed during the sleep stop, is decided as "the wake runs right after the CLI has ended" (FR-007, A5, Clarifications).
- **Narrowed from the brief:** sleep and wake announcements to assistive technology cover only the agent whose terminal view or details are open (A12), so that 30 agents do not flood a screen reader. The reviewer can widen it.
