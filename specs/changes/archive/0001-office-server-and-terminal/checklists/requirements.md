# Requirements checklist: 0001-office-server-and-terminal

Spec: `specs/changes/archive/0001-office-server-and-terminal/SPEC.md` · Status: shipped (archived 2026-11-23; the owner decided the scope on 2026-11-15; the spec has a Deviations section) · Date: 2026-11-15 · Result: 16 of 16 items pass. No open `[NEEDS CLARIFICATION]` marker.

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
| 1 | Split from the former MVP spec | The former spec's markers for the supported CLIs and logout were answered by the owner. Without the panel change, the spec had no way to list agents, no empty state, and no place for projects that both 0002 and 0003 need. The action record named agents as actors before agents could act. | Recorded the answers in Clarifications, A10, and SC-002 (four CLIs). Added FR-013 (projects), FR-014 (agents with status), and US1 scenario 7 (empty state). Limited the actor to the user (A4). |
| 2 | Hostile reader | Deleting a project with agents was not defined. The focus shortcut could take a key that a CLI needs. "A new task" had no owner once parent agents exist. | Added the project-delete edge case and A12. Wrote FR-024 without naming who starts the task, so 0002 can add parent agents (0002 PB-2). |
| 3 | Hostile reader on the fixed text | No new gap. | None. |

## Notes

- **No implementation detail:** passes for Intent, stories, requirements, entities, and criteria. The Context section names the owner's stack decision ("a separate Rust background server, `office-harnessd` ... A thin Tauri v2 desktop app with an xterm.js terminal"), marked as a recorded decision and not a requirement. FR-015 says "pseudo-terminal" because R6 in `docs/PRD.md` sets the PTY level as user-visible behavior. SC-002 names the four CLIs because the owner chose them as the supported products.
- **Sections:** the spec adds Context and Terms beyond the contract body. Deviations was absent at the first pass; the executor appended it (see the spec).
- **PRESERVED statements:** not applicable. This is the first change to the new capability `office`, so no existing behavior exists.
- **Size rule:** 28 FRs, above the limit of ten. Out of scope records: "Split: done by owner decision on 2026-11-15 (see Clarifications). This change keeps 28 FRs, above the limit of ten, because the owner set its scope." A further split (server and terminal, then configuration and sessions) is possible if the owner wants smaller reviews.
