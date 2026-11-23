# Requirements checklist: 0023-drop-ended-commands

Spec: `specs/changes/0023-drop-ended-commands/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. One open question (the time bound of FR-004) is carried as assumption A4 with a default in the requirement text. Every `0001/`, `0013/`, and `0017/` reference was checked against `specs/office/SPEC.md` and `specs/changes/archive/0017-orphan-sweep/SPEC.md` at 5272d6f.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (Context cites file:line evidence of every reader and writer, as the caller asked, so that the reviewer can check the claim that nothing else reads the identities. Requirements, stories, and criteria name no file, table, column, or request.)
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience. (Deviations and Clarifications are kept as "None yet", as the contract asks.)

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`; or it states `none: no user interface`. (The contract line uses a long dash; the caller's style rules forbid it, so a colon replaces it.)
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). (Six.)
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec. (None.)
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | The UX line of the contract holds a long dash, which the style rules forbid. | "none: no user interface". |
| 2 | Hostile reader | The MODIFIED 0017/FR-008 listed only three session ends, but a start failure and an end at the sign-in screen also end a session while the server runs, so a reader could keep their identities. FR-002 did not say that the server-start removal runs after open sessions end as `lost`, so a reader could remove the open identities of a crashed session before the sweep. FR-004 asked for "no readable copy", but the action record keeps the swept command line on purpose and a running session can hold the same line, so FR-004 could not pass. FR-003 said "until a rule of this change removes it" and ignored the delete of an agent. | 0017/FR-008 lists all five normal ends, matching the Terms. FR-002 runs after the `lost` end. FR-004 excludes copies inside an action record entry or a kept identity, and names freed space and the write log. FR-003 names FR-001, FR-002, and the agent delete. |
| 3 | Hostile reader on the fixed text | The byte search of SC-001 and SC-002 could find a marker in stored terminal output if the stand-in prints it. Identity numbers named by sweep entries can be reused after a remove. | SC-001 says the marker shows in no terminal output. A7 records the reuse and why nothing reads the number back. |

## Notes

- **Size rule:** 4 ADDED, 1 MODIFIED (0017/FR-008), 6 PRESERVED, 6 criteria. One capability. No split.
- **supersedes:** `0017/FR-008`, because its full text changes (mark as ended becomes remove).
- **Option chosen:** (a) remove the whole identity, not (b) blank the display text and keep the hash. Reasons are in A2.
- **Earlier build rows:** removed at each server start (FR-002, A5), not left.
- **Interrupted sessions:** ended identities go when the agent's next session is stored (FR-001) or at the next server start once the `lost` session is no longer the last session (FR-002). Open identities stay for the sweep (FR-003).
- **Names:** `curl` appears once in the Intent inside a code span, as the reviewer's example of a command line with a secret. No other product name outside office-harness and the CLIs claude, codex, and gemini.
- **Open for the owner:** whether FR-004 stays bound to the next server start (A4), or also clears freed space at each removal.
