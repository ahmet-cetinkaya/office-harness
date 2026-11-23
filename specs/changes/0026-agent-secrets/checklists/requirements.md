# Requirements checklist: 0026-agent-secrets

Spec: `specs/changes/0026-agent-secrets/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Revised for spec review r1 (M1, L1 to L3); the three owner proposals are accepted.

## Content quality

- [x] No implementation detail in the requirements: no crate, cipher, or table. (Context and the contract name files and the request shapes; the contract's sealing rules bind the plan, not the requirements.)
- [x] Focused on user value, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed.

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a story; every story has a priority reason, an independent test, and scenarios.
- [x] Every `SC-###` has a number and a method.
- [x] User experience lists flow, states, and accessibility.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed.
- [x] Changes to existing behavior include `PRESERVED` statements (seven).
- [x] Assumptions are listed; threat model T1 to T12 states what is left over.
- [x] Scope is bounded by Out of scope.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers. (None.)
- [x] No vague words without an observable.
- [x] Terminology is consistent (secret, value, grant, version, read, placeholder).
- [x] The spec fits one PR with 6 tasks.

## Security contract (secure-engineering)

- [x] Threat model with actor, attack, answer, and residual risk.
- [x] Values never in logs, action records, notifications, exports, crash files, or terminal output (FR-002, FR-010, FR-012 to FR-014; limits stated in A3, A5).
- [x] Rotation (FR-008), deletion (FR-009), reset (FR-015), who may read which secret (FR-005, FR-007).
- [x] Key custody per OS stated (A6, FR-003).
