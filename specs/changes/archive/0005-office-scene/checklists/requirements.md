# Requirements checklist: 0005-office-scene

Spec: `specs/changes/archive/0005-office-scene/SPEC.md` · Status: draft · Date: 2026-11-17 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

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
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). This change modifies 0003/FR-008 (open view at start) in one entry and lists six preserved statements.
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
| 1 | Draft against R7 | R7 says "optional second view"; the owner disliked walking to agents. A 3D scene or a layout editor would add work and a second way to act. | Recorded a flat 2D scene that only shows data and opens terminals (A1, PB-1), with layout that follows the data (A3). |
| 2 | Review of PR 16 (3 LOW, 2 INFO) | The last-view scope and start view contradicted 0003, FR-007 had no MODIFIED entry, no SC covered the last output line, and the CEO desk, cross-floor lead, and stale data were open. | Added the MODIFIED entry for 0003/FR-008 and filled `supersedes`, aligned the scope with 0003 A4, widened SC-002, and recorded the three INFO items as decisions. |
| 3 | Plan review of PR 20 (2 MEDIUM, 5 LOW) | The empty scene offered the create action, which the Clarification and PB-1 forbid, and the focus rule after a deletion did not cover the last desk. | The empty scene now offers the panel action, where the create lives, and focus moves to the previous desk or the empty-state heading when no desk is left. |
