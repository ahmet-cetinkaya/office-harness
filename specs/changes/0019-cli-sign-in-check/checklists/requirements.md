# Requirements checklist: 0019-cli-sign-in-check

Spec: `specs/changes/0019-cli-sign-in-check/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0003/`, `0011/`, `0012`, and `0013/` reference was checked against the specs on `main` at 6e2a8bd.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (The research file names the CLI commands and words; the spec names the commands only in A1, as product facts the user sees.)
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
| 1 | Requirements checklist | "Not signed in" had no test that a signed-in user could not fail. "Long" had no number. | A2 needs a non-zero exit AND the words; A3 sets 5 seconds; SC-002 measures it. |
| 2 | Hostile reader | A status command could see the agent-call credential of the session. The output of a signed-in `codex` holds a masked key. A false match on a working agent had no limit. | FR-006 runs it without the credential. FR-008 never shows a signed-in or no-answer output. A5 limits the watch to 20 seconds, the bottom rows, and ends it at the person's first key; US3 scenarios 2 to 4 and SC-004 test it. |
| 3 | Hostile reader on the fixed text | A hanging status command holds the one-after-another order of starts and stops. A bulk resume of a not-signed-in CLI had no rule. | A10 states the wait and its bound. PB-4 uses the 0013 path for a start that does not start. |
| 4 | Spec review r1 (M1 to M4) | A hanging status command held the one-after-another order of starts and stops (A10 rejected). Arguments, a settings file, a profile, or a key variable can sign a CLI in where its status command cannot see it. Office-typed text (startup prompt, directive) could match the watch. A new task on a running agent ended the running session before it failed. | FR-009 and A10: the check runs before the start takes its turn, and a changed agent gives no answer; SC-008. FR-010 and A11: no check with own arguments, flags, or a key variable; SC-009. FR-011 and A5: a prompt with a sign-in word turns the watch off, a directive ends it; SC-010. FR-002: a running agent's new task or continue is refused and its session goes on; US1 scenario 3. |

## Notes

- **Size rule:** 11 ADDED FRs, six more than the size S guide of 5. FR-006 to FR-011 are the safety rules of the check (no credential, no file, visible and capped text, nothing else shown, no wait for others, no refusal it cannot judge, no match on the office's own text); a split would ship a check that can leak output, block stops, or refuse working starts. No split.
- **No MODIFIED requirement:** a not-signed-in start uses the existing "never started" path of 0001 and the "does not start" rule of 0013/FR-004 and 0013/FR-005 as written (PB-4).
- **Research:** R1 to R4 ran on the installed CLIs with `--help`, the read-only status commands, and a text search of the installed programs. No sign-in action was done.
