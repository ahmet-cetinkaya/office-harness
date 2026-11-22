# Requirements checklist: 0017-orphan-sweep

Spec: `specs/changes/0017-orphan-sweep/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass (round 4 after the review of Efe, r1 at fa44fb0). No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0007/`, and `0013/` reference was checked against the specs on `main` at b768b63.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (The Context names the 0013 research file and the living spec path; the requirements do not.)
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience.

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, a fixed design link when a design exists, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two). (Six: PB-1 to PB-6.)
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec. (None.)
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria. (Session process, process identity, match, leftover, sweep, sweep result, grace period, forced kill of the server.)
- [x] The spec meets the spec contract's Size rule, or Out of scope records the declined split. (11 ADDED and 1 MODIFIED requirement: 12; 9 success criteria; 24 edge cases.)

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | "Descendant" had no time bound, so a helper could be stored too late to be swept. FR-008 had two triggers in one requirement. "Hold back" reused the 0007 term "hold". SIGKILL does not exist on Windows. | FR-002 stores a descendant within 10 seconds (A2). FR-008 split into FR-008 (normal end) and FR-009 (sweep result). FR-007 says "keep waiting". Terms define "forced kill of the server"; SC-001, SC-005, and SC-007 use it. |
| 2 | Hostile reader | A process id freed during the grace period and reused by another process would get the forced end. After a machine restart, a start time counted from boot could match again. A server killed during the sweep could record a result twice. A command line can hold a secret. | FR-003 checks the match again before each end request; US2 scenario 4. "Match" excludes an earlier boot; FR-005 reason "machine restarted"; edge case. FR-009 marks the identity in the same step as its result; SC-007. FR-001 stores identities readable only by the owner; edge case. |
| 3 | Hostile reader on the fixed text | Treating an unreadable process as "not ours" would drop the second-copy warning for a process that may still run. Hiding "Resume interrupted" during the sweep would conflict with 0013/FR-001. A process list that cannot be read at all had no rule. | 0013/FR-008 (modified) keeps the warning for "not readable" and for incomplete stored identities. FR-007 keeps the action and shows the sweep text in the dialog in place of the list (the 0013 loading state). The edge case records each identity as "not readable", which keeps the warning. |
| 4 | Review r1 (Efe) | The command line is chosen by the agent and had no bound or visible form. A process that replaces itself with another program could not be stored again. A boot time difference could hide the warning without proof of a restart. The end was by process id after a separate read. | FR-001, FR-004 to FR-006 and SC-004 (visible form, hash, cap); FR-010 and SC-008 (helper limit); FR-011 and SC-009 (replace in place); Match, FR-005, and an edge case (boot time changed keeps the warning); FR-003 and A10 (end through a held process, residual risk on macOS). |

## Notes

All items pass. Defaults that the owner can correct are A1 to A9 in the spec, in particular A2 (10 seconds to see a new helper; no end for a process without a stored identity), A3 (the sweep uses the stop grace period), and A6 (a shared helper from a `lost` session is ended).
