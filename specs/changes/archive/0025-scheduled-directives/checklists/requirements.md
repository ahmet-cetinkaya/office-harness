# Requirements checklist: 0025-scheduled-directives

Spec: `specs/changes/archive/0025-scheduled-directives/SPEC.md` · Status: shipped · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0007/`, `0012/`, `0013/`, `0017/`, `0018/`, `0019/`, and `0020/` reference was checked against `specs/office/SPEC.md` (since split into `specs/<capability>/SPEC.md`) and the archived change specs on this branch. The MODIFIED texts of 0007/FR-010, 0012/FR-003, 0012/FR-005, and 0018/FR-004 start from the living text in `specs/office/SPEC.md` (since split into `specs/<capability>/SPEC.md`); the 0003 Notification entity starts from its row in the same file.

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
| 1 | Requirements checklist | 15 ADDED requirements were over the limit of 10. 0018/FR-004 listed every way out of `asleep`, and a schedule wake was not one of them. "Idle sleep works as before" used a vague word. | Split proposed and recorded. MODIFIED 0018/FR-004 adds the schedule wake. PB-6 says "stays as before" and names the two behaviors. |
| 2 | Hostile reader | A once schedule that ran was never switched off. A missed once schedule notified twice. A temp target that ended did not switch its schedule off. Run now during a wake could make two runs at once. The anchor moved on any edit. Names could differ only by case. A save did not clear an off reason. | FR-005 switches a once schedule off; FR-011 notifies only for a switch off under FR-010; FR-010 covers a removed temp target; FR-008 skips while the latest run has no result; the Anchor term moves only on a change of the repeat or a switch on; FR-001 compares names without case; FR-003 clears the off reason at a save. |
| 3 | Spec review r1 (CHANGES) | Split: the scheduled temp made the spec too large. M1: the sender of a scheduled directive was not fixed, and 0012/FR-003 and 0012/FR-005 record results "in the sender's and the target's action records", which a schedule does not have. L1: the 0003 Notification entity says a notification refers to one agent, and 0007/FR-010 names the actors of refusals; both needed a MODIFIED text. L2: scheduled directives in the agents' per-target count could make a parent's directive fail. L4: the missed notification rule had to match the owner's choice. | The scheduled temp left the spec (US5, the temp FR, its terms, entities, states, edge rows, criteria, PB, and assumption); ids are renumbered dense, and Out of scope sends scheduled temps to a follow-up change. FR-009, A11, and the Scheduled directive term fix the schedule as the sender; MODIFIED 0012/FR-003 and 0012/FR-005 record each result once, in the target's action record, with the schedule as actor; SC-019 measures it. MODIFIED 0003 Key entities Notification and 0007/FR-010 add the schedule. FR-007, the Schedule window term, PB-4, A4, and SC-008 give scheduled directives a count of their own with the same value. FR-006 keeps one notification per minute, as the owner decided, and SC-004 matches it. |
| 4 | Hostile reader on the revised text | A notification that refers to a schedule and to no agent had no rule for what opening it does (0003/FR-009 opens an agent's terminal). The person's schedule actions were shown "in the details of its target" even after the target was deleted. | FR-011 opens the schedule in the schedules view, or the view itself for missed runs, and opens no terminal; a visible state row covers it. FR-012 shows those entries in the target's details while the target exists. |

## Notes

- **Size rule:** 14 ADDED and 5 MODIFIED entries. The ADDED count is over the limit of 10. The review split the scheduled temp out to a follow-up change (Out of scope). The 14 ADDED requirements all describe one capability, the scheduled directive. Splitting it further would ship a schedule with no missed-run, hold, or view rule, which is not safe to ship alone. The MODIFIED entries only add the schedule to existing texts.
- **Delegation:** drafted while the owner is away. The owner accepted the earlier defaults (Clarifications, A1 to A15) and decided one missed-runs notification per minute.
- **Vague words:** "works" appears once, in Accessibility ("works from the keyboard with a visible focus"), where the observable follows in the same sentence.
