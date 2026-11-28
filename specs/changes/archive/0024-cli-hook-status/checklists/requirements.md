# Requirements checklist: 0024-cli-hook-status

Spec: `specs/changes/archive/0024-cli-hook-status/SPEC.md` · Status: shipped · Date: 2026-11-23 · Result: 16 of 16 items pass after round 3. No `[NEEDS CLARIFICATION]` marker. References to 0001, 0003, 0008, 0012, 0018, 0019, 0020, and 0022 were checked against the specs on this branch.

## Content quality

- [x] No implementation detail: no language, framework, storage, API, or file names. (The CLI name `claude` and the four other CLIs are product names. How events map to the CLI's own events, where the file lives, and how the secret travels are left to the plan.)
- [x] Focused on user value, readable by a non-technical stakeholder.
- [x] Sections that do not apply are removed, not left as "N/A," except User experience. (REMOVED says "None"; Deviations is empty until execution.)

## Requirement completeness

- [x] Every `FR-###` uses an EARS pattern with `SHALL` and one observable behavior.
- [x] Every `FR-###` traces to a user story; every story has a priority reason, an independent test, and scenarios.
- [x] Every `SC-###` is measurable (counts, 1 s and 2 s limits, 64 KiB, 50 per second).
- [x] User experience lists the flow, states, and accessibility, and each state is covered by an `FR-###`.
- [x] Edge cases cover empty, invalid, boundary, concurrent, unauthorized, and failure inputs.
- [x] Key entities are listed.
- [x] The change to existing behavior has `PRESERVED` statements (eight).
- [x] Assumptions and dependencies are listed; the CLI facts that carry risk are named in A1 with a fallback.
- [x] Scope is bounded by explicit out-of-scope entries (rows 7, 8, 10, other CLIs).

## Readiness

- [x] At most three `[NEEDS CLARIFICATION]` markers; none present.
- [x] No vague words without an observable.
- [x] Terminology is consistent: exact, estimated, and late sessions; permission ask; report; session secret.
- [x] The spec meets the Size rule: 24 ADDED and 5 MODIFIED requirements, one theme (status from hooks). A split into a server part and a security part was considered and declined, because the security rules (FR-007 to FR-010) are the condition of owner decision B and cannot ship apart.

## Passes

| Round | Pass | Findings | Fix |
|---|---|---|---|
| 1 | Requirements checklist | FR-011 said an exact session is `working` for the whole open turn, which contradicted FR-013 (a permission ask is `waiting for input` inside an open turn). FR-009 dropped every content of a report, but FR-023 needs to tell a helper sub-agent's report from the main one. | FR-011 names the permission ask as the exception. FR-009 keeps one flag, whether the report comes from a helper sub-agent. |
| 2 | Hostile reader | Esc interrupts a turn with no stop report, so an open turn would stay `working` forever. A permission ask had no end event inside the four events. The settings file could make the CLI fail on an old version. A person's own hooks could be changed or answered by the office hooks. | FR-014 (late session by the quiet rule) and SC-011. FR-013 ends the ask at the first person input and A5 records the simplification, with the screen checks kept (PB-4). Edge case: the session starts without the file. FR-005: hooks only report, never answer. |
| 3 | Spec review r1 (83779cb) | The secret is in the CLI's environment, so its tools and an injected model can read it and a print can reach stored output (M1). The temp idle end ignored an open ask (M2). The plan said additions, which would store the secret in the session row (M3). Three MODIFIED entries were fragments (L1). FR-004 missed the agent's command (L2). Rate-limit state and refusal logs were unbounded (L3). | FR-007 narrowed to what the office writes, A3 and the Unauthorized row say what a forged report can do. FR-025 and SC-013 added. D3 passes the secret in the spawn environment only, SC-014 searches the database and log. Full final text for 0008/FR-002, 0012/FR-002, 0018/FR-002. FR-004 covers the command. D1 bounds state and counts refusals. |

## Notes

- **Defaults the owner or review can change:** the office-wide off switch, default on (FR-024); a permission ask as an event of `waiting for input`, not a ninth status (FR-013); the first session start is not a report (A7); the limits in A8.
- **Open for the plan:** which CLI events map to the six event kinds; how the CLI version is checked before the file is passed; how the secret reaches the hooks; where the endpoint listens; the hands-on run that checks A1 on the installed `claude`.
