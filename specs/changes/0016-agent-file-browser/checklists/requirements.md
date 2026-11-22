# Requirements checklist: 0016-agent-file-browser

Spec: `specs/changes/0016-agent-file-browser/SPEC.md` · Status: draft · Date: 2026-11-23 · Result: 16 of 16 items pass after four rounds. No `[NEEDS CLARIFICATION]` marker; the size limit (512 KiB) is flagged for the review gate in the Clarifications. Every `0001/`, `0002/`, and `0010/` reference was checked against the specs on `main` at c07f21c.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. The secret-looking names and the `.git` folder are names the user sees, not implementation.
- [x] Focused on user value and business need, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A". Key entities says that nothing is stored.

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and at least one scenario.
- [x] Every `SC-###` meets the success-criteria rules.
- [x] User experience lists the flow, states, design, and accessibility, with every state covered by a scenario or `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed when the change involves data.
- [x] Changes to existing behavior include `PRESERVED` statements (at least two).
- [x] Assumptions and dependencies are listed; no silent guess remains in the requirement text.
- [x] Scope is bounded by explicit out-of-scope entries.

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none inside an approved spec.
- [x] No vague words ("works," "fast," "secure," "robust," "intuitive") and no placeholders without an observable.
- [x] Terminology is consistent across stories, requirements, entities, and criteria.
- [x] The spec meets the spec contract's Size rule (9 FRs, 3 stories), or Out of scope records the declined split.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | "Stay inside the folder" had no rule for links, for a root inside a larger repository, or for the git data. | Terms "Root" and "Inside the root"; FR-003 resolves links first; US2 scenario 5 limits changes to the root; the `.git` edge case. |
| 2 | Hostile reader (secure-engineering contract) | (a) A commit field could carry a git option that writes a file. (b) git can run programs that the repository names during a diff (external diff, text conversion, filters, signature check). (c) Opening a named pipe blocks the reader. (d) Files and commit texts can carry markup or bidirectional characters that mislead the reviewer. (e) A link whose name is harmless can point at a secret-looking file. | (a) Commits are named by full id only (FR-006 edge case). (b) FR-007 lists them; SC-004 tests each. (c) Only regular files are read (edge case). (d) FR-009, SC-008. (e) Contract: the requested and the resolved names are both checked. |
| 3 | Transport check against the code | A 1 MiB limit plus text escaping can pass the 1 MiB frame limit of 0001; a folder of 5,000 long names can too. | Size limit 512 KiB with base64 transport; 1,000 entries per folder; an answer budget in the contract. |
| 4 | Spec review r1 (PASS for coding, condition C1) | C1: a `.env` renamed to `notes.txt` showed `.env`'s lines through the rename diff. L1: opening a FIFO blocks before the kind check. L2: the monitor control was not named. L3: more secret names. | FR-004, SC-003, edge case, contract and PLAN D5 apply the rule to the old name; PLAN D3 `O_NONBLOCK`; PLAN D7 names `core.fsmonitor=false`; `kubeconfig`, `.ovpn`, `.asc` added. |

## Notes

- The reveal is not recorded (A5) and reading is not an action (A7); both are judgment calls for the review gate.
