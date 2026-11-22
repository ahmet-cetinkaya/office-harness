# Requirements checklist: 0012-safe-directive-delivery

Spec: `specs/changes/0012-safe-directive-delivery/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass. No `[NEEDS CLARIFICATION]` marker. Every `0001/`, `0002/`, `0007/` (including FR-008), and `0011/` reference was checked against the specs on `main` at 6e3d197.

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
| 1 | Requirements checklist | "Screen" and "bottom lines" could read as implementation. | Kept: they are what the user sees in the terminal; the Terms table defines an open question in user words, and A2 lists the recognised shapes. |
| 2 | Hostile reader | The status "waiting for input" is also shown while a CLI asks a question (0001 A1), so a status gate alone would still type into a question. A text letter, not only Enter, can answer a choice list. A waiting directive could be typed after its sender lost the right to send it. Nothing bounded the queue. | FR-002 checks the screen and types neither text nor Enter. FR-005 withdraws on a tree change or a hold. FR-004 caps the queue at 20. |
| 3 | Hostile reader on the fixed text | A false match could hold a directive forever. A server stop loses waiting directives. | FR-005 withdraws after 10 minutes with the reason (A3). A4 states the loss and that the acceptance stays in the record. |
| 4 | Review guidance before the review | A quiet CLI showing a prompt the question check does not know still reads as waiting for input, so text and Enter would reach it. | FR-003: type the text, press Enter only when its end shows at the input line within 2 seconds, else leave it unsubmitted and record it. The Residual risk section states what stays open. |
| 5 | Review r1 | A pasted startup prompt is office input and the CLI goes quiet, so a directive's Enter could submit it. "Never" in US1 overstated the guard. A normal stop left no record. A queue-full refusal could be read as a rate-limit refusal. | FR-002 holds directives while a startup prompt waits (A11). US1 and the Intent now promise no Enter, with the residual risk named; a paste placeholder counts (A9). FR-005 records a normal stop. A6 says a queue-full refusal does not count toward 0007/FR-008. |

## Notes

- **Safety direction:** every uncertain case delays a directive or withholds its Enter: an unknown status, a recent person input, anything that looks like a question, or a text that does not show at the input line. What stays open is in the spec's Residual risk section.
- **Owner decision pending elsewhere:** a full check through CLI hooks needs the owner's decision on an office settings file for the CLI; this spec does not depend on it (Residual risk, Out of scope).
