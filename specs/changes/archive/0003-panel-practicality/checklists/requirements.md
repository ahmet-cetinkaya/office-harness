# Requirements checklist: 0003-panel-practicality

Spec: `specs/changes/archive/0003-panel-practicality/SPEC.md` · Status: approved (overnight directive) · Date: 2026-11-17 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

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
| 1 | Split from the former MVP spec | The owner approved the R9 criteria, so the marker and the "proposed" labels went away. This change depends only on 0001, but the list showed role and team, which come from 0002. Direct agent shortcuts conflicted with 0001/FR-017, which lets only one shortcut out of a focused terminal. | Recorded the approval in Clarifications. Made role and team an optional feature (FR-003). Added a `MODIFIED` entry for 0001/FR-017 and set `supersedes: 0001/FR-017`. |
| 2 | Hostile reader | "Last output line" was undefined for color codes and long lines. The last view could point at a deleted agent or project. Blocked OS notifications, notifications for deleted agents, shortcut conflicts, and many events at once were not defined. | Defined the term "Last output line" and added US3 scenario 2 plus the matching edge cases. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |
| 4 | Plannability (2026-11-17) | Nothing cleared an unread notification except opening it. A repeating bell could flood the list. Direct and switcher shortcuts had no keys, and A1 said the user can change them with no requirement for it. `stopped` and `interrupted` had no notify rule. SC-004 counted from the stop of output, not from the status change. | Added FR-009 and US2 scenarios 6 and 7. Recorded the shortcut defaults, fixed shortcuts, the bell rule, the focus rule (A7), and the notify rule as decisions in Clarifications. Made the shortcut conflict edge case about the changeable focus shortcut. Counted SC-004 from the status change. Set status to approved. |
| 5 | Review fixes (PR 13) | The Clarification about the last line named SC-001, but SC-001 counted status changes only. FR-006 had its exceptions outside the FR. The shortcut note did not mention the Fn key on macOS. | SC-001 now counts the last output line. FR-006 states the bell and focus exceptions. The shortcut decision names the macOS Fn behavior. |

## Notes

- **No implementation detail:** passes for Intent, stories, requirements, entities, and criteria. The Context section names the owner's stack decision, marked as a recorded decision and not a requirement.
- **Sections:** the spec adds Context and Terms beyond the contract body. Deviations is absent because the executor appends it.
- **Cross-spec IDs:** the `MODIFIED` block keeps the ID `0001/FR-017`, as the contract requires.
- **R9 mapping:** criterion 1 is FR-004, criterion 2 is FR-002, criterion 3 is FR-005, and criterion 4 is FR-006.
- **Size rule:** 9 FRs and 1 modified requirement, at the limit of ten.
- **Decisions:** every open question from rounds 1 to 3 is now a dated decision with a reversible default in Clarifications.
