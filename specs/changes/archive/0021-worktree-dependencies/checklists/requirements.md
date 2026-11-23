# Requirements checklist: 0021-worktree-dependencies

Spec: `specs/changes/archive/0021-worktree-dependencies/SPEC.md` · Status: shipped · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0009/`, `0010/`, and `0018` reference was checked against the specs on this branch.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (`node_modules`, `package.json`, and `git status` are named because they are what the person and the agent see, not how the office builds the change.)
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience. (MODIFIED is removed because no requirement of 0010 changes. Deviations is kept empty, as the contract asks, until execution.)

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
| 1 | Requirements checklist | PB-1 and PB-2 said "user" while the rest of the spec says "person". An Out of scope line said that virtual environments "do not work". The intent said that the setting "turns this off" with no actor. | PB-1 and PB-2 say "person"; Terms define person as the user of 0001 and 0010. The line says that a link to a virtual environment does not run from another folder. The intent says that the setting lets the person turn the links off. |
| 2 | Hostile reader | The office writes no ignore entry, so on a base branch without an ignore rule the links would show as changes, against FR-005. A link in the base checkout that points at nothing had no rule. The removal rule protected only the base checkout, so a link that the agent pointed elsewhere could be followed at removal. A failed place could keep an empty folder that the office made. "Tracked `package.json`" did not say where it is tracked. Two creates at the same time had no rule. | FR-003 (e): a place gets links only when git ignores a dependency folder there inside the new worktree too (A9, Clarifications). Outside target includes a place that does not exist (Terms, A11, edge case). FR-006: the removal follows no link and changes nothing outside the worktree folder. FR-007: the office leaves nothing that it made at a failed place. Dependency place: tracked in the worktree. Edge case for two creates at the same time. |
| 3 | Hostile reader on the fixed text | A link that the office cannot remove after a failure had no rule. A setting changed while the create form is open could make the note and the create disagree. | A12: such links stay as ignored content and FR-006 removes them before the worktree. A10 and an edge case: the create uses the value that the server holds when it runs. |

## Notes

- **Size rule:** 8 ADDED requirements, none MODIFIED. One capability. No split.
- **No MODIFIED:** 0010 A8 and US1 scenario 3 describe the create form note. FR-008 adds the package sentence beside the 0010 note without restating 0010 text, so `supersedes` is omitted. 0010 A9 already keeps ignored files out of the count and removes them with the worktree; FR-005 and FR-006 add only the rules for the links.
- **Decisions:** every direction came from the caller under the owner's spec delegation. The one rule added beyond the brief is FR-003 (e), the ignore check inside the new worktree, because it is the only way that settled decision 5 (the links never show as a change) holds while the office writes no ignore or exclude entry.
- **Names:** the Out of scope lines name languages and platforms (Python, Rust, PHP, Ruby, Go, Java, .NET, Dart, Windows) as the caller's brief asks, and avoid the names of their separate build tools.
- **Logged, not shown:** a link that the office did not make (FR-004, FR-007) is written to the background server's log, not shown in the desktop app (A7). The reviewer can widen it.
