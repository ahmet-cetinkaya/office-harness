# 0005 QA report

## Summary

- **Date:** 2026-11-19.
- **Build under test:** `main` at 50e5805 (merge of PR #77). The change landed in these merged PRs: #43 (T1 research), #45 (T2 floor groups, search model), #53 (T3 scene view), #55 (T4 return and restore), #69 (T5 e2e: role order, SC-004 scale, restore switch), and the follow-ups #70 (restore on the first office ready) and #72 (the restore does not read in the scene). The scene-aware reads of #74 and #77 touch the same restore path.
- **Overall status:** complete: every hands-on row is PASS or WAIVED (owner); open test gaps are listed in Known gaps. PASS (agent-run): the SC-004 machine part, the 200 percent zoom of the scene and the search, the measured part of the look and feel, and the scene restore on Linux. WAIVED (owner): SC-001 and the human part of SC-004 (not run by a person, accepted by owner 2026-11-22), the screen reader pass (see its row), the judgment of the look and feel, and the macOS and Windows rows (CI evidence, decided by owner 2026-11-20). The desk border is fixed (see the update below).
- **Owner decision, 2026-11-20:** separate hands-on runs on macOS and Windows are not needed; the CI evidence on Linux, macOS, and Windows is enough, as for 0001. The macOS and Windows hands-on rows are WAIVED (owner), not PASS: nobody ran them.
- **Update, 2026-11-23:** the 2026-11-19 run found a desk border at 2.58:1; the border is now `border-neutral-500` (`D/src/modules/office-scene/components/desk.svelte:36`) and `S` holds it at 3:1 or more (`the desk border meets 3:1 against the floor (WCAG 1.4.11)`). Known gap 4 is closed and its edge case row is PASS (automated). No other row verdict changed.
- **Status of this file:** the test names were read from the repository at 50e5805 on 2026-11-19. This task ran no test, build, or browser: it is a docs task. The run results below come from the PR reviews and from the local runs that the ticket reports.

### Verdict words

| Verdict | Meaning |
|---|---|
| PASS (automated) | An automated test proves the criterion. The evidence column names the test. |
| PARTIAL (automated) | Automated tests prove part of the criterion. The evidence column names what is proven and the gap, and Known gaps states what is not. |
| WAIVED (owner) | The owner accepted the row without a run. The note says when and why. It is not a PASS. |
| GAP | No test proves the criterion. The evidence column says why. |

### Scope of the automated evidence

- The Playwright e2e tests run against a mock core, so they prove the webview screens, not the real server. The scene reads the same agent state as the panel and adds no server request, so no server test belongs to this change.
- Playwright runs on Ubuntu only in CI (since #57). The unit and webview tests run on Linux, macOS, and Windows.
- The SC-004 timing test runs in a browser against the mock core on a CI runner under a dev build. It proves that the limits of 2 and 15 seconds hold there. It is not a measure on the owner's hardware.

### Run evidence

| Run | Scope | Result |
|---|---|---|
| Local, 2026-11-19 | Desktop `bun test src` on the #77/#78 trees | 224 pass, 0 fail |
| Local, 2026-11-19 | Desktop Playwright, full suite, on the #77/#78 trees | 157 to 159 passed, 0 failed |
| Local, 2026-11-19 | Desktop `svelte-check`, on the #77/#78 trees | 0 errors |
| CI, merges to `main` | Linux, macOS, Windows | green (reported on the ticket; this file did not read a run for 50e5805 itself) |
| CI 44537846067 | PR #77 at 0c83c65 (reviewed PASS) | green on 3 OSes; Ubuntu Playwright 157 of 157 |
| CI 44523179894 | PR #72 at 2ceb1ea (reviewed PASS) | green on 3 OSes; Ubuntu Playwright 137 of 137; `bun test src` 197 pass |
| CI 44519837164 | PR #70 at ee90f23 (reviewed PASS) | green on 3 OSes; Ubuntu Playwright 135 of 135 |
| CI 44518211169 | PR #69 at 1918cac (reviewed PASS, round 2) | green on 3 OSes; Ubuntu Playwright 134 of 134, so the 2 and 15 second limits held |
| CI 44512955715 | PR #55 at e52cc10 (reviewed PASS, round 4) | green on 3 OSes; Ubuntu Playwright 131 of 131 |
| CI 44358552409 | PR #55 at a6c9969 (reviewed PASS, round 3) | green on 3 OSes; Ubuntu Playwright 120 passed |
| CI 44344488502 | PR #53 at 5a44e3e (reviewed PASS) | green on 3 OSes |
| CI 44281687161 | PR #45 at 5c896da (reviewed PASS on code) | green on 3 OSes was the merge gate; the review ran no local suite |

Not run for this file: any Rust gate. The review of PR #53 records that Playwright was not run on each intermediate commit; CI ran it at the head only. PR #43 failed its first review round (one MEDIUM, a docs-only fix) and passed the next round at 7b2d125, its merged head.

## Requirements

Paths below are relative to the repository root. `D` is `src/apps/desktop`, `S` is `D/e2e/office-scene.spec.ts`, `F` is `D/src/modules/office-scene/state/floor-groups.spec.ts`, and `L` is `D/src/modules/office-scene/state/last-view.spec.ts`.

| ID | Verdict | Evidence |
|---|---|---|
| FR-001 | PASS (automated) | `S`: `the empty office says so and the shell returns to the panel`, `switching views sends no agent request (PB-4, recorded calls)`, `a click embeds the terminal with focus, no second window`. The shell has one view at a time. |
| FR-002 | PASS (automated) | `S`: `the floors show their desks with name, status, and last line`. `F`: `12 agents in 3 projects give 3 floors with 12 desks`, `the status text mirrors shownStatusText`, `the last line is empty before any output`. |
| FR-003 | PASS (automated) | `S`: `the floors name their departments and the roles order the desks (FR-003)`, `departments group the floors from the seeded tree (D2, FR-003)`. `F`: `a department lists its lead first, then members by name`, `a cross-floor department leads only on its lead's floor`, `departments sit after the unnamed group, sorted by team name`, `the CEO sits first in the unnamed group of its floor`, `an agent with a project but no team sits in the unnamed group`, `a department appears only on the floors where one of its agents sits`. |
| FR-004 | PASS (automated) | `S`: `a pushed status and line change update the desk under an active search (SC-002 shape)`. The scene reads the one agent state that the panel reads (see SC-003). |
| FR-005 | PASS (automated) | `S`: `a click embeds the terminal with focus, no second window`, `Enter on a focused desk opens the same terminal`, `a stopped agent's desk opens with its recent output`. |
| FR-006 | PASS (automated) | `S`: `a 20-trial round trip keeps the project choice and the selected agent (SC-006)`. `D/e2e/panel.spec.ts`: `returning from the scene does not re-run the restore (FR-008, D6)`. |
| FR-007 | PASS (automated) | `S`: `the switcher shows one project's floor and All projects brings the rest back`, `the search filters desks, focuses the match, and says when nothing matches`. `F`: `one project choice shows only that project's floor`, `one agent matches a name`, `several matches keep the reading order`, `no match is an empty list`, `empty text and spaces-only text match every agent`. Same match function as the panel: `D/src/modules/panel/models/name-matcher.spec.ts`. |
| FR-008 | PARTIAL (automated) | Keyboard: `S`: `the keyboard alone moves over every desk in reading order` (Tab, arrows, a focus outline of at least 2 px, axe), `Enter on a focused desk opens the same terminal`, `the focus-leave shortcut returns focus to the desk`. Announcements: `S`: `a pushed status change is announced with the agent's name`, `deleting the focused desk's agent moves focus forward, back, or to the empty state`. Gap: no test activates the panel and scene actions by keyboard alone; the tests click them (see SC-005). The screen reader pass is Manual QA. |
| FR-009 | PASS (automated) | `S`: `12 pushed status changes move or flash no element` (runs under `reducedMotion: "reduce"`; the test scans every scene element for an animation or a transition). |
| 0003/FR-008 (modified) | PASS (automated) | `S`: `a start with a stored view:scene reopens the scene`, `a stored last_view without the view field (a 0003-era write) opens the panel`, `an unknown view word opens the panel`, `a switch after a scene restore writes the new view`, `last_view's terminal_open follows the view that owns the terminal`, `a delayed restore never disables the writes after an early switch`, `a scene round trip during the restore read starts no second restore`, `the restore runs once when the scene opens before the office loads`. `D/src/modules/notification/models/last-view.spec.ts`: `the scene view parses, and an unknown view word means the panel`. The restore time against 3 seconds is not measured (see Known gaps). |
| PB-1 | PASS (automated) | `S`: `the scene exposes no create, start, stop, or delete control (PB-1)`. The only terminal input goes through the terminal that a desk opens: `a click embeds the terminal with focus, no second window`. |
| PB-2 | PASS (automated) | `S`: `the empty office says so and the shell returns to the panel`; `switching views sends no agent request (PB-4, recorded calls)`. The panel keeps its own suite in `D/e2e/panel.spec.ts`, which passes unchanged in the Playwright runs above. |
| PB-3 | PARTIAL (automated) | Text status: `S`, `the floors show their desks with name, status, and last line`. Keyboard: see FR-008. Axe at both widths: `S`, `the empty office says so and the shell returns to the panel`; `the keyboard alone moves over every desk in reading order`. Not proven: the keyboard half rests on FR-008, which is PARTIAL (no keyboard-only run of the shell actions). |
| PB-4 | PASS (automated) | `S`: `switching views sends no agent request (PB-4, recorded calls)` (the recorded calls are equal before and after the switches). |
| PB-5 | PARTIAL (automated) | `S`: `switching views sends no agent request (PB-4, recorded calls)`; `view switches write last_view with the view field, throttled`. The scene adds no request type, only the existing `settings.set` of `last_view`. No test captures network traffic; the network check is a 0001 row. Not proven: no test captures network traffic; the recorded calls prove only the request types. |
| PB-6 | PARTIAL (automated) | `S`: `switching views sends no agent request (PB-4, recorded calls)` asserts that every `settings.set` call carries the key `last_view` and no other. The mock core has no file system, so a write to a CLI file or a project file is not observable; the scene code has no such call. Not proven: a write to a CLI or project file is not observable in the mock core (Known gap 7). |
| SC-001 | PARTIAL (automated) | `S`: `100 agents in 10 projects: the scene opens and answers a find-and-open (SC-004)` runs the open, search, and open-the-terminal path in a browser against a 15-second limit. `S`: `the search filters desks, focuses the match, and says when nothing matches`. No test runs 12 agents against 10 seconds. The timed human run is WAIVED (owner): not run by a person, accepted by owner 2026-11-22 (Manual QA). |
| SC-002 | PARTIAL (automated) | `S`: `a pushed status and line change update the desk under an active search (SC-002 shape)` proves one change of each kind. `D/e2e/panel.spec.ts`: `a status and a line change update the row in place and keep the selection and focus`. No test makes 50 changes of each kind or times them against 2 seconds. |
| SC-003 | PARTIAL (automated) | `S`: `the floors show their desks with name, status, and last line` compares the desk text with the panel row text for the same agents. One snapshot feeds both views. No test samples 12 agents 50 times. |
| SC-004 | PASS (automated) | `S`: `100 agents in 10 projects: the scene opens and answers a find-and-open (SC-004)`: the open takes under 2 seconds (`SC004_OPEN_LIMIT_MS`), the find-and-open under 15 seconds (`SC004_FIND_OPEN_LIMIT_MS`), every desk is present, and the page scrolls. Scope: the mock core, a CI runner, a dev build. `F`: `100 agents in 10 projects keep a stable sorted order`. The human 15-second task is WAIVED (owner): not run by a person, accepted by owner 2026-11-22. The run on real hardware: PASS (agent-run, see Manual QA). |
| SC-005 | PARTIAL (automated) | Desk keyboard: `S`: `the keyboard alone moves over every desk in reading order`, `Enter on a focused desk opens the same terminal`, `the focus-leave shortcut returns focus to the desk`. The tests that open and leave the scene click the actions `To the scene` and `To the panel`. No test runs open scene, open desk, and return with 0 pointer actions. |
| SC-006 | PASS (automated) | `S`: `a 20-trial round trip keeps the project choice and the selected agent (SC-006)` (alternates the project and the agent in each trial). |
| SC-007 | PASS (automated) | `S`: `12 pushed status changes move or flash no element` (12 status changes in one second under `reducedMotion: "reduce"`; the scan finds no animation or transition). |
| US1, US2, US3 | PARTIAL (automated) | US1: FR-001 to FR-005. US2: FR-006, FR-007, and the restore tests of `0003/FR-008`. US3: FR-008 and FR-009. PARTIAL because FR-008 and SC-005 lack a keyboard-only run of the shell actions and a person has not used a screen reader. |

## Edge cases

| Case | Verdict | Evidence |
|---|---|---|
| The office has no agent (Empty) | PASS (automated) | `S`: `the empty office says so and the shell returns to the panel` (the scene offers only the way back; the create action is in the panel). |
| A project has no agent (Empty) | PASS (automated) | `F`: `a project without agents keeps its floor with count 0`; `S`: `the floors show their desks with name, status, and last line` expects the text `This floor has no agents.` on the `Empty (0)` floor (`S:166`; the text is in `D/src/modules/office-scene/components/floor.svelte:38`). |
| The search matches no desk (Empty) | PASS (automated) | `S`: `the search filters desks, focuses the match, and says when nothing matches`. `F`: `no match is an empty list`. |
| The search text is only spaces (Invalid) | PASS (automated) | `F`: `empty text and spaces-only text match every agent`. `D/src/modules/panel/models/name-matcher.spec.ts`: `the query's surrounding spaces are ignored`. |
| An agent has no project (Boundary) | PASS (automated) | `F`: `the no-project floor comes last and holds the agents without a project`, `an agent whose project is missing from the choice sits on the no-project floor`. `S`: `the floors show their desks with name, status, and last line` (the exited agent Dee has no project). |
| An agent has a project but no team, with 0002 (Boundary) | PASS (automated) | `F`: `an agent with a project but no team sits in the unnamed group`. |
| A last output line is very long, or the agent has no output yet (Boundary) | PARTIAL (automated) | No output: `F`, `the last line is empty before any output`. No test checks the cut of a long line at the desk's width or the sign of the cut. |
| An agent name is very long (Boundary) | GAP | No test uses a very long name. The desk's accessible name is the full `aria-label` (read in `S`, `the floors name their departments...`), but nothing asserts the visible cut or the full accessible name for a long one. |
| 100 or more agents (Boundary) | PASS (automated) | See SC-004. |
| Agents are created, stopped, or deleted while the scene is open (Concurrent) | PARTIAL (automated) | `S`: `a pushed status and line change update the desk under an active search (SC-002 shape)`, `deleting the focused desk's agent moves focus forward, back, or to the empty state`. Not proven: no scene test pushes an agent creation; the tests push `agent.status`, `agent.line`, and `agent.deleted` only. |
| The agent under the focused desk is deleted (Concurrent) | PASS (automated) | `S`: `deleting the focused desk's agent moves focus forward, back, or to the empty state` (next desk, previous desk, empty-state heading). |
| Two views would show different data (Concurrent) | PARTIAL (automated) | See SC-003. |
| 0002 is not present (Boundary) | PASS (automated) | `F`: `an undefined tree groups by floors only`, `an empty tree groups by floors only`. |
| The app cannot reach the server, or its token is rejected (Unauthorized) | PASS (automated) | The shell shows the 0001 states before either view: `D/e2e/app.spec.ts`, `lists the steps to give the app a valid token`, `offers to start the server and reaches the office when it starts`. |
| A desk of an agent with status `exited`, `failed`, or `stopped` opens (Failure) | PASS (automated) | `S`: `a stopped agent's desk opens with its recent output`, `Enter on a focused desk opens the same terminal` (Bo is stopped). |
| The server stops while the scene is open (Failure) | PARTIAL (automated) | `D/e2e/app.spec.ts`, `a state change from the core replaces the view without a reload` proves the shell state replaces the view. No test starts in the scene and stops the server. |
| The system asks to reduce motion (Boundary) | PASS (automated) | See FR-009. |

## Known gaps (open only)

1. **No test runs 12 agents against the 10-second find-and-open (SC-001), 50 changes of each kind (SC-002), or 50 samples (SC-003).** The e2e suite proves one change and one sample of each, and the 100-agent run of SC-004.
2. **No test runs the shell by keyboard alone (SC-005, FR-008).** The e2e tests click `To the scene` and `To the panel`.
3. **No test uses a very long agent name or last line.** The cut with a visible sign is untested.
4. **Closed 2026-11-23.** The text of a project floor that has no agents is tested: `S:166` expects `This floor has no agents.` (see the edge case row). This number stays so the other numbers do not move.
5. **No test stops the server while the scene is open.**
6. **The restore of 0003/FR-008 and 0003/SC-006 is not timed against 3 seconds for either view.**
7. **No test reads the CLI configuration folder or a project file** (PB-6). The mock core has no file system, so only the `settings.set` calls are observable.
8. **The SC-004 timing runs on a CI runner with the mock core,** not on real hardware or against the real server.
9. **The review of PR #53 records that Playwright ran only at the head** and not at each intermediate commit; the head and the later PRs ran green.
10. **Playwright runs on Ubuntu only in CI.** The e2e suite is not run on macOS or Windows.
11. **No test captures network traffic** (PB-5). The recorded calls prove only that the scene adds no request type.
12. **No scene test pushes an agent creation while the scene is open.** The tests push `agent.status`, `agent.line`, and `agent.deleted`.
13. **Screen reader announcements are not voiced on WebKitGTK (GAP).** Names PASS by agent run. Live announcements are not voiced: the app live region is present and tested, but Orca received no text-changed event for the live region change and said nothing for it. The platform (WebKitGTK, no AT-SPI text-changed event) is the likely cause, not a proven one: it is not proven whether a different markup would make WebKitGTK send the event. The audio pass by a person is waived (owner, 2026-11-22). Evidence: `qa/evidence/b-orca/summary.txt` in the QA agent folder (outside the repository). The status announcement and the focus move after a delete were not exercised with Orca.

## Manual QA

Rows marked agent-run were run by an agent on Linux/KDE Wayland with fake CLIs on 2026-11-19, against the built app and server of `main` at f6dc663. Rows marked agent-run with the date 2026-11-22 or 2026-11-23 ran on the same system against `main` at b56d2ad. Every row has a final verdict: PASS or WAIVED (owner).

Raw evidence (screenshots, traces, logs, timing files) is kept outside the repository; repository readers cannot open it. Each row therefore states its own method, setup, and measured values. Run setup: debug builds of the desktop app and `office-harnessd`, WebDriver through `tauri-driver`, an isolated data and config folder per run with login start off, fake CLIs (shell scripts) in place of agent CLIs, and no change to system settings.

| Check | Verdict | Note |
|---|---|---|
| SC-001: with 12 agents in 3 projects, see the panel, open the scene, find a named desk, and open its terminal in under 10 seconds | WAIVED (owner) | Not run by a person, accepted by owner 2026-11-22. A timed human task. |
| SC-004, human part: with 100 agents in 10 projects, find and open a named agent in under 15 seconds, and see the scene open in under 2 seconds on real hardware | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) for the machine part; WAIVED (owner) for the human part | Machine part: 100 fake agents in 10 projects, real server and app, all desks present. A click on To the scene to 100 desks in the DOM: 13 to 20 ms over 5 trials (limit 2000 ms). Window 1100x720; the page scrolls (5967 px high) and has no horizontal overflow. The human find-and-open task in under 15 seconds: WAIVED (owner), not run by a person, accepted by owner 2026-11-22. |
| Screen reader pass: desk names, the status announcement, the focus move after a delete | WAIVED (owner) | AT-SPI tree: the scene is a landmark "Office scene" with a landmark per floor ("No project (2)"), a heading per team, and each desk is a button named "<name>, <status>, <last line>" (with "project <name>" when the agent has a project). A search entry and a project combo box have names. A polite status region exists. No live-region or announcement event reached the AT-SPI event stream, and the focus move after a delete was not run. The finding in the 0003 row applies here: no app fault found, and the probe's event stream is inconclusive. Unproven: spoken output, the status announcement, and the focus move. Update 2026-11-23 (agent-run, Linux/KDE Wayland, main b56d2ad): Orca ran with a debug file and no sound; Orca logs each utterance as "SPEECH OUTPUT". Names: PASS. A desk is spoken as "ceo-real, exited, <last line>, button". The scene status announcement and the focus move after a delete were not exercised in this run. Audio by a person: WAIVED (owner), not run by a person, accepted by owner 2026-11-22. Evidence: `qa/evidence/b-orca/` and `qa/evidence/b-orca/summary.txt` in the QA agent folder (outside the repository). Cause: live announcements are not voiced on WebKitGTK (no AT-SPI text-changed event). The platform is the likely cause, not a proven one: it is not proven whether a different markup would make WebKitGTK send the event. |
| 200 percent zoom: the scene, the search, and the floors keep all text | PASS (agent-run, Linux/KDE Wayland, 2026-11-22) | Emulated by halving the window with WebDriver (550x360 CSS px for a 1100x720 window), not by the webview zoom. With 100 agents: no horizontal overflow on the scene with floors or on the search, 0 elements past the right edge, screenshots show the floors, desks and the search field with no cut text (long names and lines use an ellipsis by design). Unproven: the real zoom control and a person's reading check. Update 2026-11-22 (agent-run, Linux/KDE Wayland, main b56d2ad, 18 agents, 20 notifications): the app ran with GDK_SCALE=2, which renders the webview at devicePixelRatio 2, as a 200 percent display scale does. The window was 960x525 CSS px. Scene floors and scene search: no horizontal overflow, 0 elements past the edge, desks readable in 3 columns: PASS. Evidence: `qa/evidence/b-zoom200-real/` in the QA agent folder (outside the repository). |
| Look and feel of the flat scene: readable desks, a visible cut mark, a visible focus mark, and contrast on each floor | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) for the measured part; WAIVED (owner) for the readability judgment | Measured part, in the app with 100 agents on 10 floors plus a No project floor: computed-style contrast of desk text 10.37 to 17.93 (limit 4.5) and floor headings 17.93, the same on every floor; the focus mark (reached by Tab) is a 2 px solid outline, 6.83 against the desk and floor background (limit 3); a 63 character name and a 1539 px last line in a 308 px desk get an ellipsis (the cut mark) and the full name stays in the accessible name. One finding: the desk border is 2.58 against the floor, under 3 for a border that alone would mark the desk; the text of the desk carries it. Fixed after this run: the desk border is now neutral-500 (4.74 against white), and an e2e test holds it at 3 or more. Only one theme exists. Whether the desks read well: WAIVED (owner), not run by a person, accepted by owner 2026-11-22. Desk border: fixed in commit 4c2bf14 (`border-neutral-500`; `S` enforces 3:1). The owner had also accepted the old value of 2.58 (the desk label identifies the desk, owner informed). |
| Open the scene, restart the app, and see the scene open again, on Linux, macOS, and Windows | PASS (agent-run, Linux/KDE Wayland, 2026-11-19) for Linux; WAIVED (owner) for macOS and Windows | Linux only: the app was closed while it showed the scene (100 agents, 10 projects) and started again; the Office scene heading and the 10 floors showed 0.95 s after the launch call, with no click. macOS and Windows are WAIVED (owner): CI evidence, decided by owner 2026-11-20. |
