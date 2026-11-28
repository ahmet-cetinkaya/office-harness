# Requirements checklist: 0027-task-board

Spec: `specs/changes/0027-task-board/SPEC.md` · Status: draft · Shape: full · Date: 2026-11-28 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every cross-reference was checked against the living specs under `specs/` on this branch; ids of 0027 and 0028 point to the changes in this series, which ship in order.

## Checklist

- [x] No implementation detail: no language, framework, storage, API, or file names.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.
- [x] Every `FR-###` uses an EARS pattern with `SHALL`; each requirement covers one part of the behavior; FR-004 (rules of a new card, including the card cap) and FR-009 (not-kept handling) still list several rules of one behavior each, and the plan traces them as one unit each
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
- [x] The spec meets the spec contract's Size rule (10 FR; the second draft held about 35 single behaviors, so editing, project, assignee, and delete moved to 0028 and agent access to 0029).

## Notes

- **Size rule:** 10 FR; the second draft held about 35 single behaviors, so editing, project, assignee, and delete moved to 0028 and agent access to 0029. The three specs ship in the order 0027, 0028, 0029; each merges its requirements into the living spec `task-board` when it ships.
- **Owner decisions applied:** enrichment is after the first release; Undo, field-level merging, ordering inside a column, and the time of the last change are cut; delete uses a plain confirmation; CEO and team lead agents read and change the board, members cannot.
- **Owner decision applied:** the project switcher of the panel filters the board (0028, FR-007); a card with no project shows under All projects only.
