# Requirements checklist: 0009-agent-templates

Spec: `specs/changes/archive/0009-agent-templates/SPEC.md` · Status: approved by the owner on 2026-11-19 · Date: 2026-11-19 · Result: 16 of 16 items pass (the Size rule item is resolved by the owner decision in Notes). No open `[NEEDS CLARIFICATION]` marker. Every `0001/` and `0002/` reference was checked against the approved specs on `main` at 2df9f59.

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
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split. (More than ten FR entries; split not done, accepted 2026-11-23; see Notes.)

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | The scope paragraph says environment values "flagged as secret" are left out, but 0001 has no secret mark. | A6: export leaves every environment value empty; a secret mark is out of scope because it would change 0001. |
| 2 | Hostile reader | An imported command is outside text that the office would run. A name collision on import, a lead entry with role CEO, and a deleted agent template used by a team template were not defined. | Import saves templates only and creates no agent; FR-008 shows every command as plain text before confirm. Added A3, A4, A5 and the matching edge cases. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **Unauthorized inputs:** the office token cases of 0001 apply unchanged; the untrusted input of this change is the imported file, covered by FR-008, FR-009, and the control-character edge case.
- **Limits:** the 50-agent limit per team template is a guard on one confirmation, not an office size cap, so 0002/FR-005 and the owner's 2026-11-19 "no size cap" answer on 0007 hold.
- **Size rule:** 11 ADDED FRs (FR-001 to FR-011); FR-011 came with review edits (commit 6ea5646) after this file said "10 ADDED FRs, at the limit". That is one over the ten of the Size rule, and Out of scope records no declined split. The owner approved the spec as written on 2026-11-19; the missing `Split:` line was an open owner decision. Decision: more than ten FR entries; shipped as one feature, split not done, accepted 2026-11-23. The item is resolved; the rule text is unchanged.
