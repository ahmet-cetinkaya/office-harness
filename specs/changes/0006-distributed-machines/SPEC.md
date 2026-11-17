---
status: draft
capability: office
plan:
supersedes: 0001/FR-005 (in part), 0001/FR-008 (in part)
---

# 0006: Distributed machines

## Intent

The owner wants agents on several machines (a Linux server, a Mac, a Windows PC) to appear in one office and one panel. This change lets a second machine join the office, so that the user creates an agent on a chosen machine and then watches it and types into its terminal as if it ran locally. It opens the office to the network for the first time, so the access rules for it are part of the behavior (see Threat model). This change covers R5 of `docs/PRD.md` and the network listener with encryption that decision D-10 needs.

## Context

- Source of truth: `docs/PRD.md`. The IDs `R1` to `R9` refer to its section 4. R5 has the priority "Later"; this spec exists so that the owner can approve it before the work starts.
- This change depends on 0001 (agents, projects, statuses, terminals, the office token, the action record, the background server) and on 0003 (the panel and its agent list). It does not depend on 0002, 0004, or 0005; where they are present, their views show agents of every machine in the same way. References such as `0001/FR-008` point to requirements in those specs.
- Stack decision, made by the owner on 2026-11-15. It is a recorded decision, not a requirement of this spec: a separate Rust background server, `office-harnessd`, runs as an always-on user service, owns the terminals through portable-pty, keeps its state in SQLite, and accepts clients over a loopback WebSocket with a token. A thin Tauri v2 desktop app with an xterm.js terminal connects to it.
- 0001 A13 prepared this change: each agent belongs to a machine, and every client uses the office token, so no second access model is needed.
- The requirements below describe behavior only. The plan decides how to build them.
- This spec is security-relevant. The Threat model section states what the change must keep out; the plan must pass the repository's security review before the work starts.
- This spec is a draft. The decisions in Clarifications are reversible defaults; the owner approves or amends them with the spec.

## Terms

| Term | Meaning |
|---|---|
| Agent, project, status, quiet period, terminal, office token, action record, desktop app | As defined in 0001. |
| Panel, agent list | As defined in 0003. |
| Office server | The one background server that holds the office: the agent list, the projects, the action record, and the office token. The desktop app connects to it. |
| Machine | A computer that runs agents for the office. The machine of the office server is the home machine. Every other machine joins the office. |
| Joined machine | A machine that is registered in the office and runs the same background server software as the home machine, which starts the agents there and reports to the office server. |
| Listener | The part of the office server that accepts connections from joined machines. It is off until the user turns it on. |
| Join code | A one-time text that the user creates on the office server and enters on the machine that should join. It holds what the machine needs to find and verify the office server. |
| Machine credential | The secret that a joined machine holds after it joined. It proves to the office server which machine it is, and it works for that machine only. |
| Server fingerprint | A short value, part of the join code, that identifies the office server's encryption identity, so that a joining machine knows it talks to the right server. |
| Unreachable | The state of an agent whose joined machine is offline: the office server cannot tell what the agent does. |

## User stories

### US1: Add a machine to the office (Priority: P1)

The user turns on the listener, creates a join code, enters it on a second machine, and sees that machine in the panel.

**Why this priority:** this is R5. Without a joined machine, nothing else in this change has value.

**Independent test:** with the home machine and one other machine on the same network, turn on the listener, create a join code, and enter it on the other machine. The panel lists the other machine as `online`. Enter the same code on a third machine: it is refused.

**Scenarios:**

1. Given the listener is off, When the user turns it on, Then the office server accepts connections from joined machines and shows that the listener is on.
2. Given the listener is on, When the user creates a join code, Then the panel shows the code once, with the time it expires and the server fingerprint.
3. Given a valid join code, When the user enters it on another machine, Then that machine appears in the panel as `online` under the name that the user gave it.
4. Given a join code was used once, When a machine tries it again, Then the office server refuses it and records the refusal.
5. Given a join code expired, When a machine tries it, Then the office server refuses it and says that the code expired.
6. Given a machine presents a join code, When the server it reaches does not have the fingerprint in the code, Then the machine refuses to send the code and says that the server is not the expected one.

### US2: Run and watch an agent on another machine (Priority: P2)

The user creates an agent on a chosen machine and then uses it as any other agent.

**Why this priority:** this is the reason to join a machine, but it needs US1.

**Independent test:** with one joined machine, create an agent on it and open its terminal from the panel. Type a command, see its output, and see the agent's status and last output line in the agent list with the machine's name.

**Scenarios:**

1. Given a machine is `online`, When the user creates an agent and picks that machine, Then the agent starts on that machine and shows its machine's name in the agent list and in its details.
2. Given an agent runs on a joined machine, When the user opens its terminal, Then the live terminal opens in the same window, and the user's keys reach the agent's CLI on that machine (0001/FR-017).
3. Given an agent runs on a joined machine, When its status changes, Then the agent list shows the new status as it does for a local agent (0001/FR-014).
4. Given agents run on the home machine and on joined machines, When the user opens the panel, Then one agent list shows all of them, and the project switcher works across machines (0003).
5. Given the user picks a machine for a new agent, When the agent's project has a folder, Then the agent starts in the folder that the user set for that project on that machine, or the system asks for one when none is set (0001 A6).

### US3: Lose and remove a machine safely (Priority: P3)

The user sees when a machine goes offline, gets its agents back when it returns, and removes a machine so that it can no longer reach the office.

**Why this priority:** machines go offline and get lost, and the user must be able to cut access. It follows US1 and US2.

**Independent test:** with one joined machine that runs 2 agents, cut its network. The panel shows the machine `offline` and both agents `unreachable`. Restore the network: both agents show their real status. Remove the machine: its credential no longer connects.

**Scenarios:**

1. Given a joined machine runs agents, When its connection to the office server ends, Then the machine shows `offline` and each of its agents shows `unreachable`.
2. Given a machine is `offline`, When its connection returns, Then the machine shows `online`, and each agent shows its real status and its recent output.
3. Given a machine is `offline`, When the user opens one of its agents, Then the panel shows the last output that the office server holds and says that the agent is unreachable.
4. Given a joined machine runs 2 agents, When the user removes the machine, Then the panel states how many agents it affects and asks for confirmation, and after the user confirms, the machine's credential stops working.
5. Given the user turns off the listener, When a joined machine is connected, Then the office server closes that connection and the machine shows `offline`, and the office remains usable on the home machine.

## Requirements

### ADDED

- **FR-001** (US1): THE SYSTEM SHALL keep the listener off until the user turns it on in the desktop app, and SHALL close every joined machine's connection when the user turns it off.
- **FR-002** (US1): WHEN the user asks for a join code, THE SYSTEM SHALL create a code that works once and expires after 10 minutes, SHALL show it once with the server fingerprint, and SHALL NOT show it again.
- **FR-003** (US1): WHEN a machine presents a valid join code, THE SYSTEM SHALL register the machine under the name that the user gave it, issue it a machine credential, and record the join with the machine's name and time; IF the code is wrong, used, or expired, THEN THE SYSTEM SHALL refuse it and record the refusal.
- **FR-004** (US1): THE SYSTEM SHALL encrypt and authenticate every connection between the office server and a joined machine, and a joining machine SHALL send the join code only to a server that has the fingerprint in the code.
- **FR-005** (US1): IF a source makes 5 failed join attempts within 1 minute, THEN THE SYSTEM SHALL refuse every join attempt from that source for the next 5 minutes.
- **FR-006** (US2): WHEN the user creates an agent, THE SYSTEM SHALL let the user pick the home machine or an `online` joined machine, start the agent there with that machine's own user environment (0001/FR-020), and show the machine's name in the agent list and the agent's details.
- **FR-007** (US2, US3): THE SYSTEM SHALL give every agent on a joined machine the same terminal, status, last output, and action record as a local agent, and WHEN a joined machine is offline THE SYSTEM SHALL show its agents as `unreachable`, keep them running on that machine, and show their real status when the connection returns.
- **FR-008** (US3): WHEN the user removes a joined machine, THE SYSTEM SHALL state how many agents it affects, ask for confirmation, then make the machine's credential stop working and close its connection.

### MODIFIED

- **0001/FR-005** (US1): THE SYSTEM SHALL NOT send office data (agent definitions, terminal output, configuration, or usage data) to any destination outside the user's machines, where the user's machines are the home machine and the joined machines of the same office, reached only through the connection of FR-004.
- **0001/FR-008** (US1): THE SYSTEM SHALL accept client connections only from the same machine, and, only while the listener is on, connections from joined machines that present a valid machine credential. A joined machine's credential gives access to that machine's own agents and to nothing else in the office.

### PRESERVED

- **PB-1:** With the listener off, the office behaves as in 0001: no connection from another machine is accepted (0001/FR-008, 0001 SC-006).
- **PB-2:** The desktop app connects to the office server from the same machine, with the office token, as in 0001. A desktop app on another machine is not in this change.
- **PB-3:** office-harness creates, changes, or deletes no file in an agent CLI's configuration files or folders on any machine (0001/FR-021), and starts each agent with the user's own CLI configuration on that machine.
- **PB-4:** Agents keep running while no desktop app is open and while their machine has no connection to the office server (0001/FR-019).
- **PB-5:** Every action in this change is available from the keyboard, and every state shows as text, never color alone (0001/FR-028, 0001/FR-014).
- **PB-6:** The office token, the join codes, and the machine credentials are readable only by the OS user who owns the office on each machine (0001/FR-009).

## Threat model

This section states what the change protects, from whom, and what is not promised. The plan builds against it and the security review checks it.

**Protected:** the terminals and output of every agent (they can hold source code and secrets), the control of every agent, and the secrets of this change (office token, join codes, machine credentials).

**Trust boundary:** the network between machines is not trusted. The home machine and a joined machine are trusted as far as their OS user is (0001 A7).

| Threat | Required behavior | Covered by |
|---|---|---|
| Someone on the network reads or changes traffic between machines. | Every connection is encrypted and authenticated; plain connections are refused. | FR-004, SC-004 |
| A fake server answers a joining machine and collects the join code. | The machine sends the code only to a server with the expected fingerprint. | FR-004, US1 scenario 6, SC-004 |
| A stranger on the network connects to the listener. | The listener accepts only a valid join code or a valid machine credential, and refuses all else. | FR-003, 0001/FR-008, SC-002 |
| A join code is stolen or guessed. | A code works once, expires in 10 minutes, and repeated wrong attempts lock the source out. | FR-002, FR-005, SC-003 |
| A machine credential is stolen. | The user removes the machine, and the credential stops working at once. The credential works for that machine's agents only. | FR-008, 0001/FR-008, SC-005 |
| A joined machine is compromised. | It can read and control only its own agents. It cannot read the terminals of other machines or the office token. | 0001/FR-008, SC-005 |
| A secret appears in a log, the action record, stored terminal output, an error message, or a repository file. | No such place holds a join code, a machine credential, or the office token. | FR-002, PB-6, SC-008 |
| The listener is on without the user knowing. | The listener is off until the user turns it on, and the panel shows its state as text. | FR-001, PB-1 |

**Not promised:** protection against an agent CLI that runs as the same OS user as the office on its own machine and reads that machine's credential (0001 A7); protection against a compromised home machine; hiding the existence of the listener port from the network; protection against a machine that the user lets join on purpose.

## User experience

**Flow:**

1. The user opens the machines view from the panel. It shows the home machine, the joined machines with their state, and whether the listener is on.
2. The user turns the listener on, creates a join code, and enters it with a machine name on the machine that should join.
3. The new machine shows `online`. When the user creates an agent, the create action offers the machines.
4. The agent list and details show each agent's machine. An `unreachable` agent shows as text, with the time it was last seen.
5. The user removes a machine from the machines view, reads the number of affected agents, and confirms.

**Visible states:**

| State | What the user sees | Covered by |
|---|---|---|
| Loading, server not reachable, unauthorized | As in 0001 and 0003. | PB-5, 0003/PB-5 |
| Listener off | The machines view says that the listener is off and offers the action that turns it on. | FR-001, PB-1 |
| No machine joined | The machines view lists only the home machine and offers a join code. | US1 |
| Join code shown | The code, its expiry time, and the server fingerprint, once. | FR-002 |
| Join refused | The reason as text: wrong, used, expired, or locked out. | FR-003, FR-005 |
| Machine `online` or `offline` | The machine's state as text, with the time it was last seen. | FR-007 |
| Agent `unreachable` | The status as text, with the machine and the last-seen time. | FR-007, US3 scenario 3 |
| Confirmation | The number of affected agents, with confirm and cancel actions. | FR-008 |

**Design:** no fixed design exists. The plan must not add visuals beyond this flow without a design that the owner approves.

**Accessibility:**

- The machines view, the join code, and the confirmation dialog work from the keyboard. Keyboard focus is always visible (PB-5).
- Machine state, listener state, and `unreachable` are text, optionally with an icon, never color alone.
- A change of a machine's state is announced to screen readers.
- The confirmation dialog takes focus, states the number of affected agents as text, and closes with Escape as a cancel.
- Text meets WCAG 2.2 AA contrast (4.5:1 for normal text, 3:1 for large text).

## Key entities

| Entity | Represents | Key attributes | Identity and relationships |
|---|---|---|---|
| Machine | A computer that runs agents. | name, state (`online`, `offline`), last-seen time, home or joined | Name is unique in the office. Has zero or more agents. The home machine always exists. |
| Join code | The one-time right to join. | expiry time, used or not, server fingerprint | Not stored in a readable form after it is shown. Belongs to the office. |
| Machine credential | A joined machine's proof of identity. | machine, issue time | One per joined machine. Replaced when the machine is removed and joins again. |
| Agent (extends 0001) | One agent CLI that office-harness runs. | machine; status now also `unreachable` | Belongs to exactly one machine. |
| Action record entry (extends 0001) | One recorded action. | action now also `machine joined`, `join refused`, and `machine removed` | Holds no secret. |

## Success criteria

- **SC-001:** With a second machine on the same network, a user turns on the listener, creates a join code, and has the machine `online` in the panel in under 3 minutes.
- **SC-002:** In a test of 50 connection attempts with no credential, a wrong credential, or a removed credential, 100% are refused, and none reads a terminal or controls an agent.
- **SC-003:** In a test of 20 reuses of a used code and 20 uses of an expired code, 100% are refused. After 5 wrong codes within 1 minute from one source, the 6th attempt with a valid code is refused, and a valid code works again after 5 minutes.
- **SC-004:** A capture of all network traffic during a join and a 100-line terminal session holds 0 readable bytes of the join code, the machine credential, or the terminal text, and a machine that reaches a server with another fingerprint sends 0 bytes of the join code.
- **SC-005:** After the user removes a machine, 100% of 20 connections that use its old credential are refused within 5 seconds, and a test machine with a valid credential reads 0 terminals of other machines in 20 attempts.
- **SC-006:** After a joined machine loses its network, its agents show `unreachable` within 15 seconds, and after the network returns, they show their real status within 10 seconds.
- **SC-007:** With 3 joined machines and 30 agents in total, the agent list shows a status change of any agent within 2 seconds (`docs/PRD.md` NFR-03), and 95% of typed characters in a remote agent's terminal appear within 250 milliseconds on a link with a 50 millisecond round trip.
- **SC-008:** After a join, a removal, and a refused join, a search of the logs, the action record, the stored terminal output, and the repository files finds 0 occurrences of the join code, the machine credential, or the office token.

## Edge cases

| Case | Kind | Required behavior |
|---|---|---|
| The machine name is empty or only spaces. | Invalid | The join is refused and the name is required. |
| Two machines get the same name. | Invalid | The second is refused and the conflict is named. |
| A join code has a typing error. | Invalid | The join is refused as wrong, and counts toward FR-005. |
| A join code expires while the user enters it. | Boundary | The join is refused as expired. |
| The user creates a second join code. | Concurrent | Both work once, until they expire; each shows once. |
| Two machines use two codes at the same time. | Concurrent | Both join, each with its own credential. |
| The same machine joins twice. | Invalid | The second join is refused: a machine joins once and is removed first to join again. |
| An agent on a joined machine is created while the machine goes offline. | Failure | The create fails with the reason that the machine is offline, and no agent exists. |
| The user types into an agent while its machine goes offline. | Failure | The terminal shows that the agent is unreachable, and the keys are not delivered. |
| The office server restarts while joined machines run agents. | Failure | The machines reconnect with their credentials, and the agents show their real status (0001/FR-004 applies to the home machine only). |
| A joined machine restarts. | Failure | Its agents show `interrupted` when it is back, and none restarts without an explicit action (0001/FR-004). |
| The listener is turned off while a machine is connected. | Concurrent | The connection closes (FR-001), and the machine shows `offline`. |
| The user removes a machine that is offline. | Failure | The credential is made invalid at the office server, the panel says that the machine's agents may still run there, and the machine's agents are deleted from the office. |
| The user deletes a project whose agents run on several machines. | Boundary | Agents on every machine are counted in the confirmation (0001). |
| The listener port is already used by another program. | Failure | The listener stays off and the panel says why. |
| The clocks of two machines differ. | Boundary | A join code's expiry counts on the office server's clock only. |
| A client without the office token connects to the home machine. | Unauthorized | Refused as in 0001. |
| A machine connects with an old or removed credential. | Unauthorized | Refused and recorded (FR-008). |

## Assumptions

- **A1:** One office, one office server. A joined machine runs the same background server software, in a mode that reports to the office server. The home machine is the single place where the office's records live. If the home machine is off, the joined machines' agents keep running but the user cannot reach them from the panel (PB-4).
- **A2:** Every machine is on a network where the office server's listener can be reached, for example one LAN or a VPN that the user sets up. This change opens no route through a router or firewall and makes no relay (see Out of scope).
- **A3:** The user knows the office server's reachable address and gives it to the joining machine together with the join code, or the join code holds it. The change does not discover machines on its own.
- **A4:** Encryption identity is created by the office server itself and checked by fingerprint, so a joining machine needs no certificate authority and no domain name. The user can compare the fingerprint that the panel shows with the one on the joining machine.
- **A5:** A project's folder is set per machine. A project with no folder on a machine cannot start an agent there until the user sets one (US2 scenario 5).
- **A6:** The recent-output limit and the quiet period of 0001 (A1, A2) hold per agent on every machine. The office server keeps a joined agent's recent output, so that US3 scenario 3 works.
- **A7:** Moving an agent from one machine to another is not in this change. The user deletes the agent and creates it again.
- **A8:** The terminal input of a remote agent is the same unbroken input as in 0002/FR-010 and 0004/FR-001 where those exist.
- **A9:** Later releases stay open: D-10 (Docker Compose install) needs the listener and encryption of this change; remote desktop clients can reuse the machine credential model later.

## Clarifications

### Session 2026-11-17 (overnight directive)

Decisions with reversible defaults, so that the owner can approve the spec.

- Q: Which topology? → A: One office server and joined machines that report to it (A1). Reversible: a later change can add peer machines.
- Q: How does a machine join? → A: With a one-time join code that holds the server fingerprint (FR-002, FR-004). Reversible: the code can later hold more, or be replaced by a scanned code.
- Q: Which encryption identity? → A: One created by the office server and checked by fingerprint (A4), not a public certificate. Reversible: a later change can accept a certificate from a certificate authority when the user has a domain.
- Q: Is the listener on by default? → A: No. It is off until the user turns it on (FR-001, PB-1), so that an office with one machine has no new exposure.
- Q: Can a desktop app on another machine connect? → A: No (PB-2). Only machines that run agents join. Reversible: later releases can add remote clients with the same credential model (A9).
- Q: What can a joined machine do? → A: Run and report its own agents only (0001/FR-008, modified).
- Q: Does the office find machines on its own? → A: No (A3). The user gives the address.
- Q: Does the change cross routers or firewalls? → A: No (A2).

## Out of scope

- A relay or tunnel service, discovery of machines, and any route through a router or firewall (A2, A3).
- Desktop apps on other machines, and web or phone clients (PB-2).
- Moving agents between machines (A7).
- Peer-to-peer machines and an office without a home machine (A1).
- Single sign-on, user accounts, and several owners in one office. Only the owner reaches the office (`docs/PRD.md` section 4.1).
- Installing office-harness on a machine from the panel, and the Docker Compose install (D-10, A9).
- Syncing project folders or files between machines.
- Protection against a compromised home machine or against an agent CLI that reads the credential on its own machine (Threat model, 0001 A7).
