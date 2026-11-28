# Requirements checklist: 0007-hold-and-limits

Spec: `specs/changes/archive/0007-hold-and-limits/SPEC.md` · Status: approved by the owner on 2026-11-19 · Date: 2026-11-19 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker; the owner answered both on 2026-11-19 (see the spec's Clarifications). Every `0001/`, `0002/`, and `0003/` reference was checked against the approved specs on `main` at 19f2148.

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
- SC-005 says "from any panel view", a focused terminal included: while a terminal has focus, keys go to the CLI except the documented app shortcuts (0003, MODIFIED 0001/FR-017). The hold shortcut `Ctrl+Shift+H` is one of them (shipped D9 = b, owner-decided 2026-11-19).
- The 0002 out-of-scope line "Limits on how many agents an agent may create" stays true: this change limits the creation rate, not the count. The owner declined a size cap on 2026-11-19, so that line stays true.

## Spec gate (round 1 at 51d1b5e)

| Finding | Fix |
|---|---|
| MEDIUM-1: `supersedes:` empty despite MODIFIED 0002/FR-009. | `supersedes: 0002/FR-009, 0003/FR-006 (in part)`. |
| MEDIUM-2: SC-001 and the US3 test said every refusal names the limit, but after the automatic hold the reason is the hold. | Both now say: limit reasons before the automatic hold, the hold reason after it. |
| MEDIUM-3: the automatic-hold notification is a fifth 0003/FR-006 event without a MODIFIED entry. | Added MODIFIED 0003/FR-006 (in part); exception (b) does not apply to the automatic hold. |
| LOW-1: the SC-005 limit with a focused terminal was only in the checklist. | A9 (as changed by D9 = b) and SC-005 state it: the hold key works while a terminal has focus. |
| LOW-2: no precedence between hold and limit reasons. | A10. |
| LOW-3: where office-level record entries show. | A11 and the Office record UX state. |
| LOW-4: holds are not a security boundary; limit changes are not recorded. | A12. |
| LOW-5: stale PR body. | PR body updated. |
| LOW-6: PB-1 cited the superseded 0001/FR-017. | PB-1 cites 0001/FR-017 as modified by 0003. |

These are consistency fixes; scope is unchanged, so the owner's approval of 2026-11-19 stands (2026-11-19).

