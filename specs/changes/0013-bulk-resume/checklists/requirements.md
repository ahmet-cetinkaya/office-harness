# Requirements checklist: 0013-bulk-resume

Spec: `specs/changes/0013-bulk-resume/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0007/`, and `0010/` reference was checked against the specs on `main` at 6e3d197.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (The research file names the test file; the spec does not.)
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
| 1 | Requirements checklist | "Quickly" had no number. A failing agent could be started on every confirm. | The check period is 20 seconds (A2, Terms). FR-005 holds the agent; FR-002 lists a held agent unchecked. |
| 2 | Hostile reader | A user stop inside the check period looked like a refusal and would start the agent again. Two clients could resume one agent twice. A plain command that ends with code 0 had no rule. | "Ends by itself" in Terms excludes user actions and a server stop; US2 scenario 4. FR-003 skips agents no longer `interrupted`; edge case for two clients. A3 counts any exit. |
| 3 | Hostile reader on the fixed text | The hold of FR-005 also covers descendants (0007 A2), which a reader may not expect. A second hold of an agent already on hold had no rule. | A9 states both. |
| 4 | Review guidance (2026-11-23) | A new session drops the CLI's earlier context; with the dialog closed, the user could not see which agent lost it, or why. | FR-004 and the new FR-007 keep the reason with the start's action record entry; US2 scenarios, UX step 6, a visible state, the entity, SC-002, and A11 follow. |
| 5 | Spec review r1/r2, M1 | A5 relied on a leftover CLI blocking the continue, but no real CLI was measured; a resume could run a second copy next to a leftover. | Research R3 checked the four real CLIs (the `opencode` service survives). FR-008 marks the agents whose last session was open at an unclean stop and warns before Confirm; A5 no longer relies on a block; SC-006, scenarios 7 and 8, an edge case, a visible state, the session entity, and accessibility follow. |

## Notes

- **Size rule:** 8 ADDED FRs, three more than the size S guide of 5. FR-006 (the result text), FR-007 (the reason in the record), and FR-008 (the warning after an unclean stop) are the feedback and the safety of the flow; a split would leave a change that drops context silently or resumes next to leftovers without a word. No split.
- **No MODIFIED requirement:** 0001/FR-004 says no agent starts "without an explicit action"; the confirmed action is one, so FR-004 stays as written (PB-1). 0007/FR-008 (automatic hold after rate-limit refusals) is unchanged; FR-005 adds a second reason for an office hold through the same hold.
- **Research:** R1 (stand-ins) and R3 (the four real CLIs, by hand) ran; the orphan sweep is out of scope with a stated reason, and FR-008 warns instead.
