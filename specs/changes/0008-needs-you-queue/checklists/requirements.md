# Requirements checklist: 0008-needs-you-queue

Spec: `specs/changes/0008-needs-you-queue/SPEC.md` · Status: draft · Date: 2026-11-19 · Result: 15 of 16 items pass. One `[NEEDS CLARIFICATION]` marker remains for the owner (order by wait time only, or grouped by event kind). Every `0001/` and `0003/` reference was checked against the approved specs on `main` at 19f2148.

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

- [ ] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec. (One marker remains; the count passes, but the spec cannot be approved until the owner answers it.)
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | None open except the order marker. | None. |
| 2 | Hostile reader | The edge case for a hidden window title promised a count in the panel that no requirement stated. A dismissed item and a later event had no stated rule. | FR-009 now shows the count next to Needs you too. The dismissed-then-new-event case points to FR-001 (a new item). |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **Not a new detector:** the queue reuses the 0003/FR-006 events, so 0001 A1 (no "finished" vs "asks a question") holds. OSC 9/99/777 sequences as new events are out of scope; the bell stays the only in-band signal.
- **Jump shortcut:** it must work while a terminal has focus, so the spec modifies the 0003 version of 0001/FR-017 to add it to the documented app shortcuts.
- **Current-task line (MVP item e):** kept out of this change. It changes the agent list (0003/FR-002), not the queue, and folding it in would pass the Size rule's limit of ten FRs. It goes to its own small spec that amends 0003.
- **Size rule:** 9 ADDED FRs and 1 MODIFIED FR.
