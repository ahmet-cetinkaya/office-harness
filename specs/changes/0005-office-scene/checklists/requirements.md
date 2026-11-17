# Requirements checklist: 0005-office-scene

Spec: `specs/changes/0005-office-scene/SPEC.md` · Status: draft · Date: 2026-11-17 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

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
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). This change modifies no requirement and lists six preserved statements; it extends the 0003 "Last view" term (FR-007).
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule: 10 FRs and 0 modified requirements, within the limit of ten.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Draft against R7 | R7 says "optional second view"; the owner disliked walking to agents. A 3D scene or a layout editor would add work and a second way to act. | Recorded a flat 2D scene that only shows data and opens terminals (A1, PB-1), with layout that follows the data (A3). |
