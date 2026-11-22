# Requirements checklist: 0014-subtree-status

Spec: `specs/changes/0014-subtree-status/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass after four rounds. No `[NEEDS CLARIFICATION]` marker; the one judgment call (holds and rate limits do not apply to the read, A2) is flagged for the review gate in the Clarifications. Every `0001/`, `0002/`, `0007/`, `0008/`, and `0011/` reference was checked against the approved specs on `main` at 6e3d197.

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
| 1 | Requirements checklist | "Last activity" from the request had no observable meaning. | Defined as the last output time (Terms), a time and never content (FR-003). |
| 2 | Hostile reader | (a) A read that names an agent could point outside the subtree. (b) A polling lead could put itself on hold through the rate limits. (c) Recording every read would bury the action record. (d) An agent field added later could leak terminal content into the answer. (e) Escape codes in a user task text could reach the reader's terminal. | (a) The read takes no input; any input is refused and recorded (Edge cases). (b) FR-004 and A2. (c) FR-005 and A3. (d) FR-003 lists what never leaves; the contract builds each entry field by field. (e) Edge case: the text travels as data, encoded. |
| 3 | Hostile reader on the fixed text | "Covered by a hold" was unclear for a hold on the reader itself. | A4: the reader's own hold covers its descendants, the same coverage as 0007/FR-004. |
| 4 | Review guidance (secure-engineering contract) | Terminal output shown to another agent is a prompt-injection path and a possible secret leak; the data flow was not named; the full task text could carry a descendant lead's long text upward; no criterion tested a refused read of another branch. | Data flow and trust boundary section; task line only (FR-001, FR-003, A9); SC-006 and SC-007. |

## Notes

- **Size rule:** 5 ADDED FRs and 1 MODIFIED requirement (0002/FR-008); within the S size the request set.
- **No user interface:** the change adds no screen, so the accessibility items refer to the unchanged desktop app.
- **Next change:** the upward report channel (child to parent, CEO to owner) is listed under Out of scope.
