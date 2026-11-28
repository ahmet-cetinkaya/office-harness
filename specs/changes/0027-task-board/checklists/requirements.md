# Requirements checklist: 0027-task-board

Spec: `specs/changes/0027-task-board/SPEC.md` · Status: draft · Shape: full · Date: 2026-11-28 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0005/`, `0008/`, `0011/`, `0020/`, and `0025/` reference was checked against the living specs under `specs/` on this branch.

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
| 1 | Requirements checklist | The Loading state had no requirement. "works" appeared in the Intent and in Accessibility. PB-2 named "Needs-you item", while 0025 uses "Needs you item". The Card entity listed a made time that no requirement uses. | FR-001 adds the loading state. The Intent and Accessibility say "can be used" and "can do". The term is "Needs you item". The made time is removed. |
| 2 | Hostile reader | Two quick keyboard moves in one window could refuse each other as stale. A move of one card shifted the places of others, which then counted as changed, so a move of a neighbor in another window was refused. A window without an answer from a server that stopped during a change could say "not kept" for a change that was stored. "Show every card as it was" fought the live updates of other cards. A focused card moved in another window jumped columns with no announcement. Undo set the last change in Terms but kept it in FR-009. The note rule said "Empty, or at most" before the line end rule. Character counting said "as the person sees them" and "code point" in one sentence. | FR-006 counts only changes from another window and says that a shifted place is not a change of that card. FR-005 covers "no answer within 2 seconds" as "not confirmed" and shows the board as kept when the window reaches the server again; the edge row says the change can be there. FR-005 restores only the card of that change. FR-004 announces a focused card moved in another window. Terms keep the old last change at Undo. The note rule and A11 are restated. |
| 3 | Hostile reader on the revised text | FR-004 announced only kept or refused changes, not a "not confirmed" one. An Undo on a full board had only an edge row, no requirement. | FR-004 announces the "not confirmed" result. FR-009 counts an Undo as a new card for the card cap. No further finding. |

## Notes

- **Size rule:** 10 ADDED requirements, at the limit of 10, and 0 MODIFIED or REMOVED. One capability, the task board. Agent access (owner decision of 2026-11-28) is folded into FR-001, FR-006, and FR-007 and adds user story US5 and SC-013 and SC-014. No split is proposed; Out of scope records the count and the fallback.
- **Shape:** `full`. The small shape does not apply: the spec has 5 user stories, 10 requirements, Key entities, and new user-interface states.
- **New capability:** `task-board` has no living spec yet, so all requirements are ADDED. The 7 PRESERVED statements guard the agent panel and the task line (PB-1), the Needs-you queue (PB-2), scheduled directives (PB-3), the agent's connection, now open only to board agents (PB-4), local data (PB-5), keyboard and text status (PB-6), and the last view of 0003/FR-008 (PB-7).
- **Placeholder check:** `todo` in Terms is the name of a status value, not a placeholder.
- **Text rules of the public repository:** 0 em dashes and 0 zero-width characters (`grep -P '[\x{2014}\x{200B}-\x{200D}\x{2060}\x{FEFF}]'`); no vendor, agent name, or office-internal word.
- **Design:** no design exists; the owner picks a look from mockups later (User experience, A13). A design picked after approval is a spec change.
