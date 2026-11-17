# Requirements checklist: 0004-meeting-room

Spec: `specs/changes/0004-meeting-room/SPEC.md` · Status: draft · Date: 2026-11-17 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

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
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). This change modifies 0003/FR-006 in one respect and lists five preserved statements.
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule: 9 FRs and 1 modified requirement, within the limit of ten.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Draft against R8 and D-11 | The PRD leaves the presets and the output format open. An agent CLI has no answer protocol. | Recorded four fixed presets and one record format as decisions. Defined "Answer" by the status change to `waiting for input` (A1), so that every CLI works. |
| 2 | Hostile reader | A `working` agent would mix the brief into its work. A silent participant, a failed facilitator, a restart, a deleted agent, and overlapping meetings had no rule. The outcome values were not defined. | Added FR-003, FR-008, the time limit, the outcome definition, and the matching edge cases. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |
| 4 | Review fixes (PR 13) | The answer could be redraw noise for full-screen CLIs, and no criterion showed it. A meeting would send about 9 notifications. The second-meeting refusal gave a false reason. The outcome rule missed a failed facilitator and a restart. A CLI that stays idle after the brief and the start of the time limit were undefined. | Narrowed A1, added SC-007 per 0001 CLI, and listed full-screen cleanup as out of scope. Added the MODIFIED entry for 0003/FR-006. Folded the second-meeting refusal into FR-003. Completed the outcome rule, the idle edge case, and the time limit start. Used the 0001 term "recent-output limit". |

## Notes

- **No implementation detail:** passes for Intent, stories, requirements, entities, and criteria. The Context section names the owner's stack decision, marked as a recorded decision and not a requirement.
- **Status:** draft on purpose. R8 has the priority "Later" and the PRD says each later change starts when the owner asks. The owner approves the spec and the decisions in Clarifications together.
- **Sections:** the spec adds Context and Terms beyond the contract body, as 0002 and 0003 do.
