# PRD: office-harness, a self-hosted control room for AI agent CLIs

| Field | Value |
| --- | --- |
| Version | 0.3 |
| Date | 2026-11-15 |
| Status | Draft for the owner's approval. The MVP scope, stack, and architecture are chosen. Spec 0001 is approved. The 0001 plan is reviewed and waits for approval. No product code exists yet. |
| First user | The owner, a developer who runs many AI agent CLIs |
| First goal | Run, watch, and steer every agent of the owner from one window, on his own machines |
| Long-term possibility | An open-source tool for other developers who run many agents |
| License | AGPL-3.0-or-later (D-06) |
| Architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Behavior specs | [`specs/changes/`](../specs/changes/): 0001 office server and terminal (approved), 0002 agent hierarchy (draft), 0003 panel practicality (draft) |

## 1. Product summary

office-harness is a self-hosted control room for the AI coding agent CLIs that a developer already uses, such as `claude`, `codex`, `gemini`, and `opencode`. A background server keeps the agents running all the time. A desktop app shows every agent in one panel, and the user attaches to any agent's live terminal and types into it.

The agents are organized as a company:

```
CEO agent
└── Team (one per area or project)
    ├── Team lead agent (takes directives from the CEO, gives work to members)
    └── Member agent (an orchestrator that plans its task and uses its CLI's own sub-agents)
```

The product answers one question: **"What is each of my agents doing now, and how do I step in?"**

office-harness is not a new coding agent. It runs the CLIs that the user already has, with the user's own configuration.

## 2. Basis of the scope

### Facts that the owner confirmed

- He runs many agent CLIs. To follow and steer them, he switches between many terminals, apps, and editors. He wants one panel from which he can manage and step into all of his work.
- He wants a company structure: a CEO agent on top, teams under it, a team lead per team that takes directives from the CEO, and members that are orchestrators themselves.
- The office must live all the time (self-hosted), whether or not a window is open.
- Later, agents must run on several machines (a Linux server, a Mac, a Windows PC) and appear in one interface.
- The CLI harness that he opens in a terminal must stay exactly as it is inside the app. office-harness must not override his local configuration. Additions must stay visible and editable.
- When he opens the app, it must continue where he left off. He does not want one session to be reused forever.
- Sub-agents need no office structure, because the agent CLIs already have a sub-agent feature (2026-11-15).
- A dashboard view or an approve button is not enough. He must be able to type into the live terminal (2026-11-12).
- He tried an agent office with a 3D scene. He liked its meeting rooms with presets. He did not like walking to an agent to see results, or the slow moves between projects.

### Assumptions not yet confirmed

| ID | Assumption | What must be checked |
| --- | --- | --- |
| A-01 | The owner runs 12 to 30 agents at the same time on one machine. | The real number during the first weeks of use. Spec 0001 measures the panel with 30 agents. |
| A-02 | The four CLIs keep their session flags stable enough for the adapters. | T1 of the 0001 plan checks each CLI's `--help`. Later CLI updates can break an adapter. |
| A-03 | `gemini` stays usable for the owner. | Its documentation says that Antigravity CLI replaced it on 2026-06-18 for some account tiers (plan PB-1). |
| A-04 | A team maps to at most one project. | The owner's real team layout once 0002 is in use. |
| A-05 | Running only while the user is logged in is enough for daily use. | Whether the owner needs agents to keep working after he logs out (D-07). |

### Prior art

| Tool | What it does well | Why it does not cover the need |
| --- | --- | --- |
| A desktop office app for agents | Several agents on one floor, with an orchestrator | Limited to 12 agents. No hierarchy or teams. Each floor opens a new window. No self-hosting. No machines. Overrides some local CLI configuration. |
| A 3D office app for agents | Feature-rich 3D office, meeting rooms with presets | You must walk to an agent to see its result, and moving between projects is slow. office-harness may use it only as an example, not as a fork. |
| Terminal multiplexers (tmux and similar) | Persistent terminals | No company structure, no agent status, no single panel across projects. |

## 3. Users and daily needs

| User | Main need | Access in the first release |
| --- | --- | --- |
| The owner | Start, watch, and steer all agents, step into any terminal, and find results fast | Everything on his own machine |
| Other developers (later) | The same, on their own machines | Self-install from the open-source release. Not designed for yet. |

**User stories:**
- I want to start an agent and type into its live terminal, as I do in my own terminal.
- I want my agents to keep working after I close the window, and to see what they did when I come back.
- I want my agents to start with my own CLI configuration, with additions that I can see and edit.
- I want to see the status and last output of every agent in every project in one list.
- I want to open any agent's terminal with one click or one shortcut.
- I want a notification when an agent finishes or needs me.
- I want to organize agents as a CEO, team leads, and members, and let a parent agent give work to the agents below it.
- (Later) I want agents on my Linux server, my Mac, and my Windows PC in the same panel.
- (Later) I want to call chosen agents to a meeting on one topic and get a decision and a summary.

## 4. Scope by release

**MVP:** the first release, for the owner's daily use. **Later:** the design must not block it, but it is not built in the MVP.

| ID | Requirement | Release | Spec |
| --- | --- | --- | --- |
| R1 | **Single panel.** The user watches and steps into all agents from one window. | MVP | 0003 |
| R2 | **Deep hierarchy.** CEO, team lead, and member orchestrator. Members use their CLI's own sub-agents. There is no small cap on the number of agents (12 or more must work). | MVP | 0002 |
| R3 | **Always on.** A background server keeps the office alive while the desktop app is closed. | MVP | 0001 |
| R4 | **Self-host.** Everything runs on the user's own hardware and sends no data out. | MVP | 0001 |
| R6 | **Real terminal.** The user attaches to a live agent CLI and types into it, at pseudo-terminal (PTY) level. | MVP | 0001 |
| R9 | **Practicality.** Agents and results are reachable fast, without walking or searching (criteria in 4.2). | MVP | 0003 |
| R7 | **Office scene.** An optional view shows agents as desks, floors, and departments. It is a second view, not the main interface. | MVP, after 0003 | 0005 |
| R5 | **Distributed machines.** Agents run on Linux, macOS, and Windows machines that join one office. | Deferred (post-MVP) | |
| R8 | **Meeting room.** The user holds a meeting on a topic with chosen agents and a preset. The result is a decision and a summary. | Post-MVP (after the MVP) | 0004 |

There is no small agent cap (12 or more must work). Anything not in this table is out of scope until the owner adds it (section 12).

### 4.1 Rules for every release

- **The user's CLI configuration stays untouched.** office-harness starts each CLI as the user's shell would, with the same home folder, environment, and configuration files. It never writes to a CLI's configuration files. Office additions (flags, environment variables, a startup prompt) live in office-harness's own configuration, where the user sees and edits them per agent.
- **Resume where you left off.** The app opens in the state the user left it in.
- **A fresh session by default.** Each new task starts a new CLI session. Continuing an old session is an explicit choice.
- **The panel is the main interface.** A later view (R7, R8) opens from the panel and does not replace it.
- **Only the owner reaches the office.** Every client proves that it knows the office token, from the first release on, so remote clients (R5) need no second access model.

### 4.2 R9 acceptance criteria (approved 2026-11-15)

1. The user opens any agent's terminal in one click or one keyboard shortcut.
2. One list shows the status and last output of all agents in all projects.
3. The user switches projects in one step.
4. The user gets a notification when an agent finishes or needs the user.

## 5. Screens and main flow

- **Panel (main view):** one list of every agent with name, role, team, project, status, and last output line. It has a project switcher with "All projects" and the hierarchy tree (0002, 0003).
- **Terminal:** the agent's live terminal. Keys go to the CLI, except one documented shortcut that returns focus to the panel (0001).
- **Agent details:** office additions, session list with "continue", "new task", and the action record (0001).
- **Connection states:** loading, server not reachable (with "start server"), not authorized, and version mismatch (0001).

**Example flow:**
1. The owner opens the app. It shows the project and the terminal that he had open last.
2. He creates a team lead for project P and asks it to plan a feature. The lead creates two members and sends each a directive.
3. He closes the app. The agents keep working.
4. A notification says that a member needs input. He clicks it, the member's terminal opens, and he answers.
5. He switches to "All projects" and sees every agent's last output line.

## 6. Agent states and office rules

**Statuses:** `starting`, `working`, `waiting for input`, `exited`, `failed`, `stopped`, and `interrupted`. "Waiting for input" means that the CLI produced no output for a quiet period (5 seconds by default). The MVP does not tell "finished" apart from "asks a question" (spec 0001 A1).

**Rules:**
- A server restart never restarts agents by itself. Running agents show `interrupted` until the user acts.
- The CEO creates team leads, a team lead creates members of its own team, and a member creates no office agents. Agents never delete agents; only the user does (spec 0002).
- An agent acts only on its own descendants. Other actions are refused and recorded (spec 0002).
- A directive is text that a parent types into a child's terminal as one unbroken input (spec 0002).
- Every create, start, stop, delete, new task, continue, directive, and refusal is recorded with its actor and time.

## 7. Conceptual data model

| Concept | Meaning |
| --- | --- |
| Office | Everything one background server manages for one OS user |
| Machine | A computer that agents run on. The MVP has exactly one. |
| Project | A named folder that groups agents |
| Team | A group of agents for one area or project, with one team lead (0002) |
| Agent | One CLI that office-harness runs, with a name, a role, a command, a folder, and a status |
| Office additions | Flags, environment variables, and a startup prompt for one agent |
| Session | One run of an agent CLI, optionally continued from an earlier one |
| Directive | Text that a parent agent sends to a descendant (0002) |
| Action record entry | One recorded action with its actor, target, and time |
| Notification | A status event that the user must see, read or unread (0003) |

The detailed model for 0001 is in [data-model.md](../specs/changes/0001-office-server-and-terminal/data-model.md).

## 8. Quality, security, and privacy

These targets come from the specs. They are proposals for the owner's use, not a service level.

| ID | Target |
| --- | --- |
| NFR-01 | **Access:** a client without the office token reads no terminal and controls no agent. The server accepts connections from the same machine only (0001). |
| NFR-02 | **No outside data:** office-harness itself sends no office data, telemetry, or update check outside the machine. Connections that agent CLIs open are theirs (0001). |
| NFR-03 | **Responsiveness:** 95% of typed characters appear within 100 ms. Status changes appear in the list within 2 seconds with 30 agents (0001, 0003). |
| NFR-04 | **Durability:** agent definitions, projects, office additions, sessions, and the action record survive restarts. A full disk stops saving but not the agents (0001). |
| NFR-05 | **Platforms:** the server and the desktop app run on Linux, macOS, and Windows (0001). |
| NFR-06 | **Accessibility:** every action works from the keyboard, focus is visible, status is text and never color alone, and text meets WCAG 2.2 AA contrast (0001, 0003). |
| NFR-07 | **Configuration safety:** office-harness changes 0 files in a CLI's configuration folders (0001). |

**Threat boundary:** the token keeps out other OS users and programs that do not run as the owner. It is not a sandbox against an agent CLI that runs as the same OS user, because such an agent can read the user's files (spec 0001 A7).

## 9. MVP acceptance

The MVP is accepted when the success criteria of specs 0001, 0002, and 0003 pass on Linux, macOS, and Windows, with the evidence recorded beside each spec; the office scene of 0005, built after 0003, is accepted by its own success criteria. These criteria are requirements, not test results yet. The main ones are:

| ID | What must be shown | Source |
| --- | --- | --- |
| AC-01 | A full task in each of `claude`, `codex`, `gemini`, and `opencode`, through the attached terminal, without another terminal | 0001 SC-002 |
| AC-02 | Agents keep running for 60 minutes with the app closed, and their output is there on reopen | 0001 SC-003 |
| AC-03 | After a restart and login, the server runs within 60 seconds without the app, with all data present | 0001 SC-004 |
| AC-04 | 0 changed files in CLI configuration folders | 0001 SC-005 |
| AC-05 | 100% of connections without a valid token are refused | 0001 SC-006 |
| AC-06 | A 15-agent, 3-level hierarchy built from the desktop app alone in under 10 minutes | 0002 SC-001 |
| AC-07 | With 30 agents listed, any agent's terminal opens with 1 click or 1 shortcut and shows live output within 1 second | 0003 SC-001, SC-002 |

## 10. Success in use

These are proposed targets. There is no baseline yet, so no improvement percentage is claimed.

| Measure | How it is observed | Proposed target |
| --- | --- | --- |
| One window | The owner's own count of other terminals that he opened for agent work during a week | 0 by the second week |
| Fast reach | Time from a notification to typing in that agent's terminal, with 30 agents running | Under 10 seconds (0003 SC-007) |
| Trust in "always on" | Agents found `interrupted` without a restart or logout | 0 |
| Configuration safety | Any change found in a CLI's configuration that office-harness caused | 0 |

## 11. Phases and open decisions

| Phase | Output | Exit condition |
| --- | --- | --- |
| 1 Foundation (0001) | Server, terminal, sessions, desktop app, installers | 0001 success criteria pass on three OSes |
| 2 Company (0002) | Hierarchy, teams, directives, agent actors | 0002 success criteria pass |
| 3 Practicality (0003) | One list, shortcuts, project switcher, notifications, resume | 0003 success criteria pass, so the MVP is complete |
| 4 Office scene (0005) | The desks, floors, and departments view | 0005 success criteria pass, so the MVP is complete |
| 5 Post-MVP | R8 meeting room (0004), then R5 machines (0006) when the owner asks for it | Each starts with its own spec when the owner asks for it |

Approving this PRD does not start development. Each change starts when its spec and plan are approved.

| ID | Decision | Status |
| --- | --- | --- |
| D-01 | Name | **Decided:** office-harness (2026-11-15) |
| D-02 | First platform | **Decided:** desktop app on Linux, macOS, and Windows (2026-11-15) |
| D-03 | Stack | **Decided:** a separate Rust background server `office-harnessd` (PTYs, SQLite, loopback WebSocket with a token) and a Tauri v2 desktop app with xterm.js (2026-11-15) |
| D-04 | Architecture | **Decided:** a modular monolith with Clean Architecture in each module, as in [ARCHITECTURE.md](ARCHITECTURE.md) (2026-11-15) |
| D-05 | Frontend | **Decided:** Svelte + TypeScript (2026-11-15). Tailwind CSS with shadcn-svelte is proposed. |
| D-06 | License | **Decided:** AGPL-3.0-or-later (2026-11-15) |
| D-07 | Running after logout | **Decided:** not in the MVP. Agents run while the user is logged in (2026-11-15) |
| D-08 | CLIs with full support in the MVP | **Decided:** `claude`, `codex`, `gemini`, and `opencode`, including continuing a session (2026-11-15). `gemini` is under review (A-03). |
| D-09 | Hierarchy levels | **Decided:** CEO, team lead, member. No office sub-agents (2026-11-15) |
| D-10 | Server install with Docker Compose | **Later.** The container runs `office-harnessd` and the CLIs, with configuration and project folders mounted. It needs the R5 network listener with TLS first. |
| D-11 | R8 meeting presets and output format | **Decided:** four fixed presets (Decision, Brainstorm, Review, Status round) and one meeting record per meeting, as 0004's approved Clarifications state (2026-11-17). |
| D-12 | macOS and Windows test machines | **Open:** which machines and who runs the tests (plan PB-3). Needed before the 0001 terminal gate. |

## 12. Non-goals

- A fork of another agent office app.
- A new coding agent, or changes to how a CLI works inside.
- Features that the owner did not ask for: voice input, screen sharing, and project boards on walls.
- A hosted service run by anyone other than the user.

## 13. Version history

| Version | Date | Change |
| --- | --- | --- |
| 0.1 | 2026-11-15 | First PRD: problem, product, users, R1 to R9, rules, stack decision, non-goals, open questions. |
| 0.2 | 2026-11-15 | No office sub-agents (CEO, team lead, member). R9 criteria approved. MVP CLIs, running only while logged in, and no outside paths in the docs. |
| 0.3 | 2026-11-15 | Restructured: confirmed facts and assumptions, prior art, user stories, scope by release with spec links, screens and flow, agent states and rules, data concepts, quality targets, MVP acceptance, success measures, phases, and a decision log. Architecture and license recorded. |
| 0.4 | 2026-11-17 | Specs 0004 and 0005 approved; 0005 moved into the MVP (built after 0003), 0004 post-MVP, R5 machines (0006) deferred. D-11 settled by 0004's approved Clarifications. |
