# Requirements checklist: 0028-safe-board-changes

Spec: `specs/changes/0028-safe-board-changes/SPEC.md` · Status: approved · Shape: full · Date: 2026-11-28 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every cross-reference to a living spec under `specs/` was checked on this branch; ids of 0027 to 0030 point to the changes of this series, which ship in the order 0027, 0028, 0029, 0030.

## Checklist

- [x] No implementation detail: no language, framework, storage, API, or file names.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.
- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two).
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.
- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule.

## Notes

- **Size rule:** 6 ADDED requirements; 0 MODIFIED.
- **Owner decisions applied:** enrichment is after the first release; Undo, field-level merging, ordering inside a column, and the time of the last change are cut; delete uses a plain confirmation; the project switcher filters the board; CEO and team lead agents read and change the board and members cannot; a lead may change any card of the office; an agent's delete is recorded.
