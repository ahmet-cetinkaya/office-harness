# Requirements checklist: 0006-distributed-machines

Spec: `specs/changes/0006-distributed-machines/SPEC.md` · Status: draft · Date: 2026-11-17 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names beyond the recorded stack decision.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). This change modifies 0001/FR-005 and 0001/FR-008 and lists six preserved statements.
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule: 8 FRs and 2 modified requirements, within the limit of ten.

## Security

- [x] The Threat model section covers authentication of joining machines (FR-002, FR-003, FR-005), transport (FR-004), secrets in logs and files (PB-6, SC-008), the default of the listener (FR-001), and the blast radius of a stolen credential or compromised machine (SC-005).

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Draft against R5 and 0001 A13 | 0001 FR-005 and FR-008 forbid any outside connection, so a joined machine needs them modified. The new listener is the first network exposure. | Recorded two MODIFIED entries, a listener that is off by default, a threat model with measurable checks, and the status `unreachable`. |
