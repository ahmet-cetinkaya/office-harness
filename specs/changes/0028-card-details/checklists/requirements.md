# Requirements checklist: 0028-card-details

Spec: `specs/changes/0028-card-details/SPEC.md` · Status: draft · Shape: full · Date: 2026-11-28 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every cross-reference was checked against the living specs under `specs/` on this branch; ids of 0027 and 0028 point to the changes in this series, which ship in order.

## Checklist

- [x] No implementation detail: no language, framework, storage, API, or file names.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.
- [x] Every `FR-###` uses an EARS pattern with `SHALL`; each requirement covers one part of the behavior; FR-001 and FR-002 list the field rules of one save
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] `PRESERVED` statements are present (at least two).
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.
- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule (6 FR; follows 0027).

## Notes

- **Size rule:** 6 FR; follows 0027. The three specs ship in the order 0027, 0028, 0029; each merges its requirements into the living spec `task-board` when it ships.
- **Owner decisions applied:** enrichment is after the first release; Undo, field-level merging, ordering inside a column, and the time of the last change are cut; delete uses a plain confirmation; CEO and team lead agents read and change the board, members cannot.
- **Open with the owner:** whether the project switcher of the panel filters the board (0027, A9). The spec says it does not.
