# Requirements checklist: 0025-scheduled-directives

Spec: `specs/changes/0025-scheduled-directives/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0007/`, `0012/`, `0013/`, `0015/`, `0017/`, `0018/`, `0019/`, and `0020/` reference was checked against `specs/office/SPEC.md` and the archived change specs on this branch.

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
| 1 | Requirements checklist | The FR count is 15 ADDED and 1 MODIFIED, over the limit of 10. 0018/FR-004 lists every way out of `asleep`, and a schedule wake was not one of them. "Idle sleep works as before" used a vague word. | Out of scope records the proposed split and why the draft keeps one spec. MODIFIED 0018/FR-004 adds the schedule wake; `supersedes: 0018/FR-004`. PB-6 now says "stays as before" and names the two behaviors. |
| 2 | Hostile reader | A once schedule that ran had no rule that switched it off. A once schedule switched off for a missed time would notify twice (FR-006 and FR-012). A target that is a temp leaves by the office's end, not by the person's delete, so FR-010 missed it. Run now during a wake could make two runs of one schedule at once, because FR-008 looked only at a waiting directive. The anchor moved on any edit, which a text edit should not do. An edit of a once schedule that already ran was refused for its past time, even when the person only renamed it while off. Action record entries of a scheduled temp with no parent had no agent to show in. | FR-005 switches a once schedule off when its due time is handled. FR-012 notifies only for a switch off under FR-010; FR-006 keeps its one notification for missed runs. FR-010 covers a target or temp parent that is deleted or removed as an ended temp, and acts at once. FR-008 skips while the latest run has no result yet. The Anchor term moves only on a change of the repeat or a switch on. FR-001 asks a future time only of a once schedule saved on. FR-013 shows entries in an agent's details "when there is one". |
| 3 | Hostile reader on the fixed text | Names "Nightly" and "nightly" could both exist. The every-N-hours wording "each whole multiple" could include 0, a run at the anchor. A save did not clear an off reason, so a fixed schedule could stay refused. Key entities named an internal "notification state". | FR-001 compares names without case. FR-004 names N, 2N, 3N as the multiples. FR-003 clears the off reason at a save. Key entities says "whether a schedule notification waits for a successful run". |

## Notes

- **Size rule:** 15 ADDED and 1 MODIFIED requirement, 16 in all, over the limit of 10. Out of scope records: "Split: proposed `0025-scheduled-directives` (scheduled directives, FR-001 to FR-010 and FR-012 to FR-015) and a later `scheduled-temps` change (FR-011 and the temp parts of FR-001, FR-007, FR-008, and FR-010); kept as one in this draft, because the owner approved one gap that names both actions, and the two share every schedule rule (save, time, missed runs, holds, overlap, history, view); the temp action adds one requirement. The owner confirms or splits at approval."
- **Delegation:** drafted while the owner is away, with no questions asked. The product choices of the brief are under Clarifications and A1 to A16 for the owner or the review gate to confirm.
- **Vague words:** "works" appears once, in Accessibility ("works from the keyboard with a visible focus"), where the observable follows in the same sentence.
