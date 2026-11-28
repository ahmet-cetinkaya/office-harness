# Requirements checklist: 0011-task-line

Spec: `specs/changes/archive/0011-task-line/SPEC.md` · Status: approved by the owner on 2026-11-19 · Date: 2026-11-19 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, and `0003/` reference was checked against the approved specs on `main` at 2df9f59.

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

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | A new task in 0001 carries no text, so "the last new-task text" from the scope paragraph has no source. | The task text comes from directives (FR-002) and the user (FR-003); a new task clears it (FR-004). |
| 2 | Hostile reader | Stale text could read as live; a refused directive, an empty directive, and an edit racing a directive were not defined; escape codes in directive text could reach the list. | The line is labelled "Task given N min ago"; added the edge cases; FR-006 shows codes without applying them; FR-007 forbids output parsing. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **Item (e) of the MVP additions:** kept as its own change, not folded into 0008, because it changes the agent list (0003/FR-002), not the needs-you queue.
- **Size rule:** 7 ADDED FRs (FR-001 to FR-007) and 1 MODIFIED FR (0003/FR-002).

