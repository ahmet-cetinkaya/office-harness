# Requirements checklist: 0002-agent-hierarchy

Spec: `specs/changes/0002-agent-hierarchy/SPEC.md` · Status: approved (overnight directive 2026-11-16; owner may amend) · Date: 2026-11-16 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker. All `0001/FR-###` references re-verified against the merged 0001 on `main` (commit 4425166).

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
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Split from the former MVP spec | The owner's three-level answer removed the sub-agent role and its marker. Agents without a role from 0001 had no defined place. The spec changes 0001's capability, so it needs delta blocks. | Wrote FR-001 for three levels, with A3 and A4 and the Clarifications entry. Wrote the requirements as `ADDED` and added PB-1 to PB-5, including PB-1 for agents without a role. |
| 2 | Hostile reader | An agent could create agents in a loop, and the user had no fast way to stop the subtree. Directives to stopped or deleted agents, a second CEO, a team without a lead, and the CEO's own deletion were not defined. FR-002 had two observables. | Added FR-007 (stop descendants in one action) and the matching edge cases. Split FR-002 into FR-002 (teams) and FR-003 (role placement), then renumbered. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **No implementation detail:** passes for Intent, stories, requirements, entities, and criteria. The Context section names the owner's stack decision, marked as a recorded decision and not a requirement.
- **Sections:** the spec adds Context and Terms beyond the contract body. Deviations is absent because the executor appends it.
- **Cross-spec IDs:** references such as `0001/FR-024` point to requirements in change 0001.
- **Size rule:** 11 FRs, one above the limit of ten. Out of scope records: "Split: done by owner decision on 2026-11-15 (see Clarifications)." The owner set this spec's scope.
