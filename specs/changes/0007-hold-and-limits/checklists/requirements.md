# Requirements checklist: 0007-hold-and-limits

Spec: `specs/changes/0007-hold-and-limits/SPEC.md` · Status: draft · Date: 2026-11-19 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker; the owner answered both on 2026-11-19 (see the spec's Clarifications). Every `0001/` and `0002/` reference was checked against the approved specs on `main` at 19f2148.

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

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec. (Both markers answered by the owner on 2026-11-19.)
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Hostile-reader pass (1 round)

- FR-007 refers to the limit table in A3, so the values and the requirement cannot drift apart.
- SC-005 says "from any panel view": while a terminal has focus, keys go to the CLI except the documented app shortcuts (0003, MODIFIED 0001/FR-017). Adding the hold shortcut to that list is a plan decision that would need a 0003 amendment; the spec does not promise it.
- The 0002 out-of-scope line "Limits on how many agents an agent may create" stays true: this change limits the creation rate, not the count. The owner declined a size cap on 2026-11-19, so that line stays true.
