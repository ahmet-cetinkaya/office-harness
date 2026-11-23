# Requirements checklist: 0022-waiting-item-sleep

Spec: `specs/changes/0022-waiting-item-sleep/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0003/`, `0007/`, `0008/`, `0012/`, `0013/`, `0015/`, `0017/`, `0018/`, and `0020/` reference was checked against the specs on this branch. The item kinds were checked against the attention events that the office records today: failed, waiting for input, bell, automatic hold, resume hold, kept leftover, exited, and report. No "question" or "permission" item kind exists.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (Context names `docs/PRD.md` as the source of truth, as every change spec of this repository does.)
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience. (REMOVED says "None" to make the delta explicit. Deviations is kept empty, as the contract asks, until execution.)

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). (Eight.)
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
| 1 | Requirements checklist | The brief names "permission" and "question" items, but Needs you has no such kind: a question shows as a plain `waiting for input` item. 0003/SC-004 promises a notification for every change to `waiting for input`, which the dedupe breaks. An asleep agent's item had no rule (0018 assumed that no asleep agent has an item). | A waiting item counts as not plain while the 0012 question check finds an open question (Terms, 0018/FR-002 b). 0003/SC-004 is MODIFIED to "prompted change". FR-001 keeps the plain item through the sleep and removes it at the wake. |
| 2 | Hostile reader | A bell followed by a quiet period makes "waiting for input" the latest event, so a rule on the latest event alone would let a bell item count as plain. A redraw moves the idle-since moment of 0018, so a CLI that redraws more often than the idle time never sleeps, even with the item fixed. An unreadable screen had no rule. A resize had no rule. The failed-sleep retry of 0018/FR-009 could fire at each redraw once the idle-since moment stops moving. | "Waiting item" means an item that recorded only changes to `waiting for input`, not one whose latest event is one (A5). The idle-since moment counts from the last prompted change (0018/FR-002, A7). FR-002: an unreadable screen keeps the agent awake. A resize is not person input (edge case, US2 scenario 8). PB-7: the retry still needs a full idle time after the return. |
| 3 | Hostile reader on the fixed text | "A wait end while it shows `working`" in 0008/FR-005 also matched an exit, so an exit would both update and remove the item. "Waiting item" was used before it was defined. | 0008/FR-005 names only a person input or office text while the agent shows `working`; an exit keeps its 0008 rule (PB-4). Terms define "Waiting item" and "Plain waiting item" apart. |

## Notes

- **Size rule:** 2 ADDED and 4 MODIFIED requirements, plus 1 MODIFIED success criterion. No split.
- **MODIFIED:** 0003/FR-006 gets exception (c); 0008/FR-001 and 0008/FR-005 (the 0015 text) get the unprompted-change rule; 0018/FR-002 narrows condition (b), names each item kind, and states the idle-since moment inline. 0018 A1 changes in part (Context). 0020/FR-004 stays as it is (A9, Out of scope).
- **Decisions:** the two decisions of the brief came from the caller under the owner's spec delegation and are in the Clarifications. The no-notification choice for an unprompted change follows the brief's recommendation (A1).
- **Defaults that the owner can correct:** the idle-since moment ignores unprompted changes (A7); the plain item stays in Needs you while the agent sleeps (A6); the temp idle limit keeps its item rule (A9).
