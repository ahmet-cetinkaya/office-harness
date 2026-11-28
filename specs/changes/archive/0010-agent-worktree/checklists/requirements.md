# Requirements checklist: 0010-agent-worktree

Spec: `specs/changes/archive/0010-agent-worktree/SPEC.md` · Status: approved by the owner on 2026-11-19 · Date: 2026-11-19 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker; the question whether templates carry the own-worktree choice is answered in the spec (A2 and FR-009: they do). Every `0001/` and `0002/` reference was checked against the approved specs on `main` at 2df9f59.

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

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec. (The one marker was answered; none remains.)
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | "Git" in the requirements is the user-facing subject of the change, not an implementation choice. | Kept; the Terms table defines worktree and base in user words. |
| 2 | Hostile reader | Removing a worktree could delete a branch that holds the agent's only commits. Deleting a lead with many member worktrees could ask once per member. A hand-deleted folder had no rule. | FR-007 deletes the branch only without own commits. FR-005 asks once and lists each worktree. FR-008 refuses a start without the folder. |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **Data safety:** the delete question defaults to keep; uncommitted changes need a second confirmation; branches with own commits are never deleted by the office.
- **Shared history:** a worktree separates files, not branches and commits; Context and Out of scope say so.
- **Size rule:** 9 ADDED FRs (FR-001 to FR-009) and two MODIFIED requirements (0009/FR-001, 0009/FR-003); FR-009 came with the template answer. The spec has more than ten FR entries; shipped as one feature, split not done, accepted 2026-11-23.
