# Requirements checklist: 0022-waiting-item-sleep

Spec: `specs/changes/0022-waiting-item-sleep/SPEC.md` · Status: draft · Date: 2026-11-23 · Revised for spec review r1 · Result: 16 of 16 items pass (16 before r1, 16 after). No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0003/`, `0007/`, `0008/`, `0012/`, `0013/`, `0015/`, `0017/`, `0018/`, and `0020/` reference was checked against the specs on this branch. The item kinds were checked against the attention events that the office records today: failed, waiting for input, bell, automatic hold, resume hold, kept leftover, exited, and report. No "question" or "permission" item kind exists.

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
| 4 | Spec review r1, M1 | A prompted pause gives an item; the person opens it; the agent works 20 minutes and returns unprompted, with no item, no notification, and a sleep within 60 s, because the idle-since moment is still the pause. The person loses the completion signal. | Terms: Work bound (the quiet period plus 10 seconds) and wait end (4), a stretch of `working` longer than it. 0008/FR-005 removes the item at the bound. US2 scenario 9, two new edge rows replace the A2 row, A2 and A3 rewritten, A10 records the bound and its reason, SC-007 added. |
| 5 | Spec review r1, L1 and L2 | The 0018 Term Idle since changed without a delta. PB-7 gave a new reading of 0018/FR-009 under PRESERVED, and it conflicted with the FR-002 idle-since rule. | Context says that Idle and Idle since change. MODIFIED gets the full new text of both 0018 Terms and of 0018/FR-009: a new try only after a prompted change and a full idle time from it. PB-7 now states only what stays: the status and the single record entry of a failed sleep. |
| 6 | Hostile reader on the r1 text | A fixed 10-second bound breaks once the person sets a quiet period of 10 seconds or more: the status stays `working` for one quiet period after any output, so every redraw would pass it. SC-003 (a line every 30 s) and SC-004 (a hint-line redraw) must stay under the bound. | The bound is the quiet period plus 10 seconds (A10, edge row on a longer quiet period). A redraw shows `working` for about one quiet period, 5 to 6 seconds by default, which is under the 15-second default bound, so SC-003 and SC-004 hold. |

## Notes

- **Size rule:** 2 ADDED and 5 MODIFIED requirements, plus 1 MODIFIED success criterion and 2 MODIFIED 0018 Terms. No split.
- **MODIFIED:** 0003/FR-006 gets exception (c); 0008/FR-001 and 0008/FR-005 (the 0015 text) get the unprompted-change rule; 0018/FR-002 narrows condition (b), names each item kind, and states the idle-since moment inline; 0018/FR-009 allows a new try only after a prompted change; the 0018 Terms Idle and Idle since are restated. 0018 A1 changes in part (Context). 0020/FR-004 stays as it is (A9, Out of scope).
- **Decisions:** the two decisions of the brief came from the caller under the owner's spec delegation and are in the Clarifications. The no-notification choice for an unprompted change follows the brief's recommendation (A1).
- **Defaults that the owner can correct:** the work bound is the quiet period plus 10 seconds (A10); the idle-since moment ignores unprompted changes (A7); the plain item stays in Needs you while the agent sleeps (A6); the temp idle limit keeps its item rule (A9).
