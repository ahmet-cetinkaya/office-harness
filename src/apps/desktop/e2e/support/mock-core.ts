// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Page } from "@playwright/test";
import type { ConnectionState, ServerEvent } from "../../src/core/api";
import type {
  AdditionsView,
  Agent,
  Project,
  Session,
} from "../../src/modules/agent/models/agent";
import type { ActionEntry } from "../../src/modules/audit/models/action-entry";
import type { HierarchyTree } from "../../src/modules/hierarchy/models/hierarchy";
import type { Hold, HoldState } from "../../src/modules/hold/models/hold";
import type {
  NeedsYouItem,
  NeedsYouKind,
} from "../../src/modules/needs-you/models/needs-you";
import type { NotificationRow } from "../../src/modules/notification/models/notification";

/** One seeded agent with the fields the 0003 suites assert on. The mock
 * answers them on every agent it returns; left out, the office holds none. */
export type SeededAgent = Agent & {
  /** The agent's place in the company, as the tree names it. */
  role?: "ceo" | "lead" | "member" | null;
  /** The name of the team the agent sits in. */
  team?: string | null;
};

/** What the faked office holds when the page opens. */
export interface OfficeSeed {
  agents?: SeededAgent[];
  projects?: Project[];
  sessions?: Record<string, Session[]>;
  additions?: Record<string, AdditionsView>;
  /** The 0002 tree shape that `hierarchy.tree` answers with. Left out, the
   * office holds no tree answer at all and `hierarchy.tree` answers empty. */
  hierarchy?: HierarchyTree;
  /** The text of the snapshot frame that follows `terminal.attach`. */
  snapshots?: Record<string, string>;
  /** The `focus_shortcut` setting. The server's default when left out. */
  focusShortcut?: string;
  /** The notifications the faked server keeps. None when left out. */
  notifications?: NotificationRow[];
  /** The Needs you items the faked server keeps when the page opens. None
   * when left out. */
  needsYou?: NeedsYouItem[];
  /** Milliseconds that `notification.read_agent` waits before it answers and
   * clears the agent's item, as a slow server. Answers at once when left out. */
  readAgentDelayMs?: number;
  /** The holds the faked server keeps. None when left out. */
  holds?: Partial<HoldState>;
  /** Limit settings that differ from the server's defaults, by key. */
  limits?: Record<string, string>;
  /** The stored `last_view` JSON text, as `settings.get` answers with it. */
  lastView?: string;
  /** Milliseconds the `last_view` read waits before it answers, as a slow
   * server. Answers at once when left out. */
  lastViewDelayMs?: number;
  /** Milliseconds the `agent.list` read waits before it answers, as an
   * office that is slow to load. Answers at once when left out. */
  agentListDelayMs?: number;
  /** A refusal to answer with, by request type. The tree reads it too. */
  refusals?: Record<string, { code: string; message: string }>;
}

export interface MockCoreOptions {
  initialState: ConnectionState;
  /** The office that the faked server holds. It starts empty. */
  office?: OfficeSeed;
  /** What "start_server" answers with. When it succeeds the state becomes
   * `stateAfterStart`. */
  startServer?:
    | { outcome: "ok"; stateAfterStart: ConnectionState }
    | { outcome: "fail"; code: string; message: string };
  /** Milliseconds that "agent.stop" waits before it answers, as a CLI that
   * takes its time to end. Answers at once when left out. */
  stopDelayMs?: number;
  /** The first "hierarchy.tree" read fails with this, so the failed state
   * and its retry can be tested. Later reads answer the office. */
  hierarchyTreeFailure?: { code: string; message: string };
  /** The login item. Not registered, and registering works, when left out. */
  loginStart?: {
    registered?: boolean;
    failure?: { code: string; message: string };
    statusFailure?: { code: string; message: string };
  };
}

export interface RecordedCall {
  command: string;
  args: Record<string, unknown>;
}

/** One `settings.set` invoke, as the mock records it beside `calls`. */
export interface RecordedSetting {
  key: string;
  value: unknown;
}

/** One `notification.list` entry, as the 0003 contract carries it. */
export interface NotificationSeed {
  id: string;
  agent_id: string;
  agent_name: string;
  event: "exited" | "failed" | "waiting_for_input" | "bell" | "auto_hold";
  detail?: string | null;
  created_at: number;
  read_at: number | null;
}

declare global {
  interface Window {
    __mockCore: {
      push(state: ConnectionState): void;
      /** Sends a server event to the webview, as the core forwards it. */
      pushEvent(event: ServerEvent): void;
      /** Sends a binary terminal frame: kind 1 snapshot, 2 output, 3 dropped. */
      pushTerminalFrame(kind: number, agentId: string, text: string): void;
      /** Replaces the tree that `hierarchy.tree` answers with. */
      setHierarchy(tree: HierarchyTree): void;
      /** Sends one `agent.line` frame, as the line bridge does (0003). */
      pushAgentLine(agentId: string, line: string): void;
      /** Taps an OS notification of the agent, as the notification plugin reports it. */
      pushNotificationTap(agentId: string): void;
      /** Creates one notification, as the bridge does, with its event. */
      notify(
        agentName: string,
        agentId: string,
        event: string,
        detail?: string,
      ): void;
      /**
       * An attention event, as the server sees it: the notification is
       * created and announced, then the agent's Needs you item is recorded or
       * updated and `needs_you.changed` follows (0008 D3).
       */
      attention(
        agentId: string,
        kind: NeedsYouKind,
        detail?: string,
        firstEventAt?: number,
      ): void;
      /** The office puts an agent on hold by itself, as a limit does (0007). */
      autoHold(agentId: string, limitName: string): void;
      calls: RecordedCall[];
      /** Every `settings.set` invoke, oldest first, beside `calls`. */
      settings: RecordedSetting[];
    };
  }
}

/**
 * Stands in for the Tauri IPC the webview talks to, so the real client code
 * runs against a scripted core. The mock never holds a token, as the real
 * webview does not either.
 */
export async function installMockCore(
  page: Page,
  options: MockCoreOptions,
): Promise<void> {
  await page.addInitScript((mock: MockCoreOptions) => {
    const callbacks = new Map<number, (message: unknown) => void>();
    const nextIndex = new Map<number, number>();
    const stateChannels: number[] = [];
    const eventChannels: number[] = [];
    const notificationActionChannels: number[] = [];
    const terminalChannels: number[] = [];
    const calls: RecordedCall[] = [];
    const settings: RecordedSetting[] = [];
    // The four limits of the contract, with the server's defaults (0007).
    const LIMIT_DEFAULTS: Record<string, string> = {
      limit_directives_per_agent: "10",
      limit_directives_per_target: "3",
      limit_creates_per_agent: "5",
      limit_lifecycle_per_agent: "10",
    };
    const LIMIT_INVALID =
      "invalid: a limit is a whole number of at least 1 and at most 1000";
    let nextCallbackId = 1;
    let current = mock.initialState;
    let loginStartRegistered = mock.loginStart?.registered ?? false;
    let hierarchyTreeFailedOnce = false;

    const send = (channelId: number, message: unknown) => {
      const index = nextIndex.get(channelId) ?? 0;
      nextIndex.set(channelId, index + 1);
      callbacks.get(channelId)?.({ index, message });
    };
    const push = (state: ConnectionState) => {
      current = state;
      stateChannels.forEach((id) => send(id, state));
    };

    const office = {
      agents: structuredClone(mock.office?.agents ?? []),
      projects: structuredClone(mock.office?.projects ?? []),
      sessions: structuredClone(mock.office?.sessions ?? {}),
      additions: structuredClone(mock.office?.additions ?? {}),
      /** The 0002 hierarchy shape, kept in step with the agents. */
      hierarchy: structuredClone(mock.office?.hierarchy ?? null),
      actions: [] as ActionEntry[],
      notifications: structuredClone(mock.office?.notifications ?? []),
      needsYou: structuredClone(mock.office?.needsYou ?? []),
      holds: {
        office: mock.office?.holds?.office ?? null,
        agents: structuredClone(mock.office?.holds?.agents ?? []),
      } as HoldState,
      settings: {
        ...LIMIT_DEFAULTS,
        ...mock.office?.limits,
        focus_shortcut: mock.office?.focusShortcut ?? "Ctrl+Shift+F12",
        last_view: mock.office?.lastView ?? null,
      } as Record<string, string | null>,
      nextId: 1,
    };
    for (const agent of office.agents) {
      // Seeded roles and teams answer as given, or as no role at all, so
      // every existing seed keeps its old shape.
      agent.role ??= null;
      agent.team ??= null;
      office.additions[agent.id] ??= {
        flags: [],
        env: [],
        startup_prompt: null,
        updated_at: 1,
        applies_next_session: false,
      };
    }

    const pushEvent = (event: ServerEvent) => {
      eventChannels.forEach((id) => send(id, event));
    };
    // The 0003 line event, as the line bridge sends it.
    const pushAgentLine = (agentId: string, line: string) => {
      pushEvent({ type: "agent.line", agent_id: agentId, line });
    };
    // The 0003 notification event, as the notification bridge sends it.
    const pushNotificationCreated = (notification: NotificationSeed) => {
      pushEvent({ type: "notification.created", notification });
    };
    // The contract's frame: 1 byte kind, 16 bytes agent UUID, then the payload.
    const pushTerminalFrame = (kind: number, agentId: string, text: string) => {
      const uuid = agentId.replaceAll("-", "").match(/../g) ?? [];
      const payload = new TextEncoder().encode(text);
      const frame = new Uint8Array(17 + payload.length);
      frame[0] = kind;
      frame.set(
        uuid.map((byte) => Number.parseInt(byte, 16)),
        1,
      );
      frame.set(payload, 17);
      terminalChannels.forEach((id) => send(id, frame.buffer));
    };
    const refuse = (code: string, message: string): never => {
      throw { code, message };
    };
    /** One new notification row, as the bridge makes it, with its event. */
    const createNotification = (
      agent: Agent,
      event: NotificationRow["event"],
      detail: string | null = null,
    ): NotificationRow => {
      const row: NotificationRow = {
        id: `notification-${office.nextId++}`,
        agent_id: agent.id,
        agent_name: agent.name,
        event,
        detail,
        created_at: Date.now(),
        read_at: null,
      };
      office.notifications.unshift(row);
      pushEvent({ type: "notification.created", notification: row });
      return row;
    };
    /** Marks one agent's unread rows read, as `notification.read_agent` does. */
    const readAgentRows = (agentId: string) => {
      for (const row of office.notifications) {
        if (row.agent_id === agentId && row.read_at === null) {
          row.read_at = Date.now();
        }
      }
    };
    // The group of the contract's order: failed, then the agents that wait,
    // then exited.
    const needsYouRank = (kind: string) =>
      kind === "failed" ? 0 : kind === "exited" ? 2 : 1;
    const nameOf = (agentId: string) =>
      office.agents.find((agent) => agent.id === agentId)?.name ?? agentId;
    /** The items in the FR-003 order: group, longest wait, then name. */
    const orderedNeedsYou = (): NeedsYouItem[] =>
      structuredClone(
        [...office.needsYou].sort(
          (first, second) =>
            needsYouRank(first.kind) - needsYouRank(second.kind) ||
            first.first_event_at - second.first_event_at ||
            nameOf(first.agent_id).localeCompare(nameOf(second.agent_id)),
        ),
      );
    const pushNeedsYouChanged = () => {
      pushEvent({ type: "needs_you.changed", items: orderedNeedsYou() });
    };
    /** Removes the items that match and publishes only when one left. */
    const clearNeedsYou = (matches: (item: NeedsYouItem) => boolean) => {
      const before = office.needsYou.length;
      office.needsYou = office.needsYou.filter((item) => !matches(item));
      if (office.needsYou.length !== before) pushNeedsYouChanged();
    };
    const recordAttention = (
      agent: Agent,
      kind: NeedsYouKind,
      detail: string | null,
      firstEventAt: number,
    ) => {
      const row = createNotification(agent, kind, detail);
      const known = office.needsYou.find((item) => item.agent_id === agent.id);
      if (known === undefined) {
        office.needsYou.push({
          agent_id: agent.id,
          kind,
          detail,
          first_event_at: firstEventAt,
          latest_at: Date.now(),
          origin_notification_id: row.id,
        });
      } else {
        Object.assign(known, { kind, detail, latest_at: Date.now() });
      }
      pushNeedsYouChanged();
    };
    const isActive = (status: string) =>
      ["starting", "working", "waiting_for_input"].includes(status);
    const findAgent = (agentId: unknown) =>
      office.agents.find((agent) => agent.id === agentId) ??
      refuse("not_found", "no such agent");
    const record = (agentId: string | null, action: string, actor = "user") => {
      const entry: ActionEntry = {
        id: office.actions.length + 1,
        actor,
        agent_id: agentId,
        action,
        result: "ok",
        at: Date.now(),
      };
      office.actions.push(entry);
      pushEvent({ type: "action.recorded", entry });
    };
    /** The full state after a hold change, as `hold.changed` carries it. */
    const pushHoldChanged = () => {
      pushEvent({ type: "hold.changed", ...structuredClone(office.holds) });
    };
    const heldAgent = (agentId: string) =>
      office.holds.agents.find((hold) => hold.agent_id === agentId);
    const setStatus = (agent: Agent, status: string) => {
      agent.status = status;
      pushEvent({ type: "agent.status", agent_id: agent.id, status });
    };
    const endSessions = (agentId: string) => {
      for (const session of office.sessions[agentId] ?? []) {
        session.ended_at ??= Date.now();
        session.exit_result ??= "signal";
      }
    };
    const startSession = (agent: Agent, continuedFrom: string | null) => {
      endSessions(agent.id);
      const session: Session = {
        id: `session-${office.nextId++}`,
        agent_id: agent.id,
        cli_session_id: null,
        continued_from: continuedFrom,
        additions_snapshot: { flags: [], env: [], startup_prompt: null },
        started_at: Date.now(),
        ended_at: null,
        exit_result: null,
        can_continue: true,
      };
      office.sessions[agent.id] = [
        session,
        ...(office.sessions[agent.id] ?? []),
      ];
      setStatus(agent, "starting");
      return session;
    };

    // The tree the faked server keeps in step with the agents: the CEO, the
    // teams with their leads and members, and the agents without a role.
    interface TreeAgentRow {
      id: string;
      role: string | null;
    }
    interface TreeTeamRow {
      id: string;
      name: string;
      project_id: string | null;
      lead: TreeAgentRow;
      members: TreeAgentRow[];
    }
    interface TreeRow {
      ceo: TreeAgentRow | null;
      teams: TreeTeamRow[];
      unassigned: TreeAgentRow[];
    }

    const EMPTY_TREE: TreeRow = { ceo: null, teams: [], unassigned: [] };
    const treeOf = (): TreeRow => {
      office.hierarchy ??= structuredClone(EMPTY_TREE);
      return office.hierarchy;
    };
    const treeChanged = (cause: string) => {
      if (office.hierarchy !== null) {
        pushEvent({ type: "hierarchy.changed", cause });
      }
    };

    /** Every agent's parent as the tree holds it: the CEO has none, a lead's
     * is the CEO, a member's is its lead. */
    const parentOf = (): Map<string, string | null> => {
      const tree = treeOf();
      const up = new Map<string, string | null>();
      if (tree.ceo !== null) up.set(tree.ceo.id, null);
      for (const team of tree.teams) {
        up.set(team.lead.id, tree.ceo?.id ?? null);
        for (const member of team.members) up.set(member.id, team.lead.id);
      }
      for (const agent of tree.unassigned) up.set(agent.id, null);
      return up;
    };

    /** The agents that sit below one agent, in tree order (FR-006, FR-007). */
    const descendantsOf = (agentId: string): string[] => {
      const up = parentOf();
      const below: string[] = [];
      const walk = (parent: string): void => {
        for (const [child, of] of up) {
          if (of === parent) {
            below.push(child);
            walk(child);
          }
        }
      };
      walk(agentId);
      return below;
    };

    /** Drops deleted agents from the tree rows, as the server's cascade
     * does: a deleted lead or member leaves its team list. */
    const removeFromTree = (agentIds: readonly string[]): void => {
      const gone = new Set(agentIds);
      const tree = treeOf();
      if (tree.ceo !== null && gone.has(tree.ceo.id)) tree.ceo = null;
      for (const team of tree.teams) {
        team.members = team.members.filter((member) => !gone.has(member.id));
      }
      tree.teams = tree.teams.filter((team) => !gone.has(team.lead.id));
      tree.unassigned = tree.unassigned.filter((agent) => !gone.has(agent.id));
    };

    /** One new agent row, with its additions, as `agent.create` makes. */
    const createAgentRow = (fields: Record<string, unknown>): Agent => {
      if (!fields.name) refuse("invalid", "name is required");
      const agent: Agent = {
        id: `agent-${office.nextId++}`,
        name: String(fields.name),
        command: String(fields.command),
        working_folder: (fields.working_folder as string) ?? null,
        project_id: (fields.project_id as string) ?? null,
        machine_id: "machine",
        status: "stopped",
        created_at: Date.now(),
        last_line: null,
      };
      office.agents.push(agent);
      office.additions[agent.id] = {
        flags: [],
        env: [],
        startup_prompt: null,
        updated_at: Date.now(),
        applies_next_session: false,
      };
      return agent;
    };

    /** Places one new agent by its `role` object, or refuses as the tree
     * rules refuse (FR-003). The refusal names the allowed roles. */
    const placeByRole = (
      agentId: string,
      role: Record<string, unknown> | undefined,
    ): void => {
      if (role === undefined) return;
      const kind = String(role.kind ?? "");
      const tree = treeOf();
      if (kind === "ceo") {
        if (tree.ceo !== null) {
          // Exact server shape: the application error's Display prefixes the
          // wire message with "invalid: ", and SecondCeo names the CEO by
          // its name, falling back to the id when the name is empty
          // (hierarchy_handlers.handler_error, tree_rules.check_ceo,
          // hierarchy_error.SecondCeo).
          const ceo = office.agents.find((agent) => agent.id === tree.ceo?.id);
          const named = ceo?.name || tree.ceo.id;
          refuse("invalid", `invalid: the office already has a CEO: ${named}`);
        }
        tree.ceo = { id: agentId, role: "ceo" };
        return;
      }
      if (kind === "lead") {
        const team = tree.teams.find((known) => known.id === role.team_id);
        if (team === undefined) {
          return refuse(
            "invalid",
            "invalid: the parent is not an agent of the office",
          );
        }
        tree.teams = tree.teams.filter((known) => known !== team);
        tree.teams.push({ ...team, lead: { id: agentId, role: "lead" } });
        return;
      }
      if (kind === "member") {
        const up = parentOf();
        const leadId = String(role.parent_agent_id ?? "");
        if (leadId === "") {
          return refuse("invalid", "invalid: a team needs its lead");
        }
        if (!up.has(leadId)) {
          return refuse(
            "invalid",
            "invalid: the parent is not an agent of the office",
          );
        }
        const team = tree.teams.find((known) => known.lead.id === leadId);
        if (team === undefined) {
          return refuse(
            "invalid",
            "invalid: a member cannot sit under a lead; allowed under a lead: member",
          );
        }
        team.members.push({ id: agentId, role: "member" });
        return;
      }
      refuse("invalid", `invalid: role is not a role of the company: ${kind}`);
    };

    // The requests that the real server answers, in the shapes of the
    // contract. Each change also sends the events that the server sends.
    const answer = async (
      kind: string,
      fields: Record<string, unknown>,
    ): Promise<unknown> => {
      const scripted = mock.office?.refusals?.[kind];
      if (scripted !== undefined) refuse(scripted.code, scripted.message);
      switch (kind) {
        case "project.list":
          return office.projects;
        case "project.create": {
          const project = {
            id: `project-${office.nextId++}`,
            name: String(fields.name),
            folder: String(fields.folder),
          };
          office.projects.push(project);
          return project;
        }
        case "project.delete": {
          const inUse = office.agents.filter(
            (agent) => agent.project_id === fields.project_id,
          );
          if (inUse.length > 0) {
            refuse("conflict", `agents still use it: ${inUse[0].name}`);
          }
          office.projects = office.projects.filter(
            (project) => project.id !== fields.project_id,
          );
          return null;
        }
        case "agent.list":
          if (mock.office?.agentListDelayMs !== undefined) {
            await new Promise((resolve) =>
              setTimeout(resolve, mock.office?.agentListDelayMs),
            );
          }
          return office.agents;
        case "hierarchy.tree": {
          if (mock.hierarchyTreeFailure && !hierarchyTreeFailedOnce) {
            hierarchyTreeFailedOnce = true;
            refuse(
              mock.hierarchyTreeFailure.code,
              mock.hierarchyTreeFailure.message,
            );
          }
          return office.hierarchy ?? { ceo: null, teams: [], unassigned: [] };
        }
        case "hierarchy.descendants": {
          findAgent(fields.agent_id);
          return { count: descendantsOf(String(fields.agent_id)).length };
        }
        case "team.create": {
          const name = String(fields.name ?? "");
          if (name.trim() === "")
            refuse("invalid", "invalid: team name must not be empty");
          const tree = treeOf();
          const clash = tree.teams.some(
            (team) => team.name.toLowerCase() === name.toLowerCase(),
          );
          if (clash)
            refuse("conflict", `conflict: a team named ${name} already exists`);
          const lead = (fields.lead ?? {}) as Record<string, unknown>;
          if (!lead.name) refuse("invalid", "name is required");
          const leadAgent = createAgentRow(lead);
          const team: TreeTeamRow = {
            id: `team-${office.nextId++}`,
            name,
            project_id: (fields.project_id as string) ?? null,
            lead: { id: leadAgent.id, role: "lead" },
            members: [],
          };
          tree.teams.push(team);
          pushEvent({ type: "agent.changed", agent: leadAgent });
          pushEvent({ type: "hierarchy.changed", cause: "team.created" });
          record(leadAgent.id, "create");
          return { id: team.id, name: team.name, lead_agent_id: leadAgent.id };
        }
        case "team.delete": {
          if (fields.confirm !== true) {
            return refuse("invalid", "invalid: confirm must not be empty");
          }
          const tree = treeOf();
          const team = tree.teams.find((known) => known.id === fields.team_id);
          if (team === undefined) {
            return refuse("not_found", "not found: the team does not exist");
          }
          tree.teams = tree.teams.filter((known) => known !== team);
          tree.unassigned.push(team.lead, ...team.members);
          pushEvent({ type: "hierarchy.changed", cause: "team.deleted" });
          record(team.lead.id, "team_delete");
          return null;
        }
        case "agent.create": {
          // The agent-module refusal kinds (invalid, not_found, conflict) are
          // unreachable on this branch: the real core drops `role` until S5
          // slice 2 wires create_role_agent onto agent.create.
          const agent = createAgentRow(fields);
          placeByRole(
            agent.id,
            fields.role as Record<string, unknown> | undefined,
          );
          pushEvent({ type: "agent.changed", agent });
          treeChanged("agent.placed");
          record(agent.id, "create");
          return agent;
        }
        case "agent.start":
        case "agent.new_task": {
          const agent = findAgent(fields.agent_id);
          const session = startSession(agent, null);
          record(agent.id, kind === "agent.start" ? "start" : "new_task");
          return session;
        }
        case "session.continue": {
          const agentId = Object.keys(office.sessions).find((id) =>
            office.sessions[id].some(
              (session) => session.id === fields.session_id,
            ),
          );
          const agent = findAgent(agentId);
          const session = startSession(agent, String(fields.session_id));
          record(agent.id, "continue");
          return session;
        }
        case "agent.stop": {
          // include_descendants false stops only the agent itself; the real
          // server (subtree_handler.affected) never refuses a plain stop for
          // having descendants.
          const agentId = String(fields.agent_id);
          const agent = findAgent(agentId);
          const descendants = descendantsOf(agentId);
          if (fields.include_descendants === true) {
            for (const below of descendants) {
              const stopped = findAgent(below);
              endSessions(stopped.id);
              setStatus(stopped, "stopped");
            }
          }
          if (mock.stopDelayMs) {
            await new Promise((resolve) =>
              setTimeout(resolve, mock.stopDelayMs),
            );
          }
          endSessions(agent.id);
          setStatus(agent, "stopped");
          record(agentId, "stop");
          return null;
        }
        case "agent.delete": {
          const agentId = String(fields.agent_id);
          findAgent(agentId);
          const descendants = descendantsOf(agentId);
          if (descendants.length > 0) {
            const subject =
              descendants.length === 1 ? "descendant" : "descendants";
            if (fields.include_descendants !== true) {
              refuse(
                "conflict",
                `conflict: the agent still has ${descendants.length} ${subject}; delete them too by sending include_descendants=true and descendant_count ${descendants.length}`,
              );
            }
            const count = Number(fields.descendant_count ?? -1);
            if (count !== descendants.length) {
              refuse(
                "conflict",
                `conflict: the agent has ${descendants.length} ${subject} now; send descendant_count ${descendants.length} to confirm the delete`,
              );
            }
            for (const below of descendantsOf(agentId)) {
              office.agents = office.agents.filter(
                (known) => known.id !== below,
              );
              pushEvent({ type: "agent.deleted", agent_id: below });
            }
          }
          office.agents = office.agents.filter((known) => known.id !== agentId);
          clearNeedsYou((item) => item.agent_id === agentId);
          removeFromTree([...descendantsOf(agentId), agentId]);
          pushEvent({ type: "agent.deleted", agent_id: agentId });
          treeChanged("agent.deleted");
          record(agentId, "delete");
          return null;
        }
        case "agent.additions.get": {
          const agent = findAgent(fields.agent_id);
          return {
            ...office.additions[agent.id],
            applies_next_session: isActive(agent.status),
          };
        }
        case "agent.additions.set": {
          const agent = findAgent(fields.agent_id);
          const saved = {
            ...(fields.additions as AdditionsView),
            updated_at: Date.now(),
            applies_next_session: isActive(agent.status),
          };
          office.additions[agent.id] = saved;
          pushEvent({
            type: "agent.additions.changed",
            agent_id: agent.id,
            additions: saved,
          });
          return saved;
        }
        case "session.list":
          return office.sessions[String(fields.agent_id)] ?? [];
        case "notification.list": {
          const unread = office.notifications.filter(
            (row) => row.read_at === null,
          );
          const read = office.notifications.filter(
            (row) => row.read_at !== null,
          );
          const byAge = (first: NotificationRow, second: NotificationRow) =>
            second.created_at - first.created_at;
          return [...unread.sort(byAge), ...read.sort(byAge)].slice(0, 500);
        }
        case "notification.read": {
          const row = office.notifications.find(
            (known) => known.id === fields.notification_id,
          );
          if (row === undefined) {
            return refuse("not_found", "not found: no such notification");
          }
          row.read_at ??= Date.now();
          clearNeedsYou((item) => item.origin_notification_id === row.id);
          return null;
        }
        case "notification.read_agent": {
          if (mock.office?.readAgentDelayMs) {
            await new Promise((resolve) =>
              setTimeout(resolve, mock.office?.readAgentDelayMs),
            );
          }
          readAgentRows(String(fields.agent_id));
          clearNeedsYou((item) => item.agent_id === fields.agent_id);
          return null;
        }
        case "needs_you.list":
          return { items: orderedNeedsYou() };
        case "needs_you.dismiss": {
          const agentId = String(fields.agent_id);
          if (!office.needsYou.some((item) => item.agent_id === agentId)) {
            return refuse("not_found", "no needs-you item for the agent");
          }
          clearNeedsYou((item) => item.agent_id === agentId);
          return { items: orderedNeedsYou() };
        }
        case "terminal.attach": {
          const agent = findAgent(fields.agent_id);
          const text = mock.office?.snapshots?.[agent.id] ?? "";
          setTimeout(() => pushTerminalFrame(1, agent.id, text), 0);
          return null;
        }
        case "terminal.input":
        case "terminal.resize":
        case "terminal.detach":
          return null;
        case "settings.get": {
          const answer = {
            key: fields.key,
            value: office.settings[String(fields.key)] ?? "",
          };
          if (
            fields.key === "last_view" &&
            mock.office?.lastViewDelayMs !== undefined
          ) {
            return new Promise((resolve) =>
              setTimeout(() => resolve(answer), mock.office?.lastViewDelayMs),
            );
          }
          return answer;
        }
        case "settings.set": {
          const key = String(fields.key);
          const value = String(fields.value ?? "");
          if (key === "last_view" && value.length > 1024) {
            refuse("invalid", "invalid: last_view is longer than 1 KiB");
          }
          const isLimit = key in LIMIT_DEFAULTS;
          if (isLimit && !/^[0-9]+$/.test(value))
            refuse("invalid", LIMIT_INVALID);
          if (isLimit && (Number(value) < 1 || Number(value) > 1000)) {
            refuse("invalid", LIMIT_INVALID);
          }
          office.settings[key] = value;
          settings.push({ key, value: fields.value });
          return { key, value: fields.value };
        }
        case "hold.list":
          return structuredClone(office.holds);
        case "hold.set": {
          if (fields.agent_id === undefined) {
            if (office.holds.office !== null) {
              refuse("conflict", "the office is already on hold");
            }
            const hold: Hold = { set_by: "user", since: Date.now() };
            office.holds.office = hold;
            record(null, "hold");
            pushHoldChanged();
            return { hold };
          }
          const agent = findAgent(fields.agent_id);
          if (heldAgent(agent.id) !== undefined) {
            refuse("conflict", "the agent is already on hold");
          }
          const hold: Hold = {
            agent_id: agent.id,
            set_by: "user",
            since: Date.now(),
          };
          office.holds.agents.push(hold);
          record(agent.id, "hold");
          pushHoldChanged();
          return { hold };
        }
        case "hold.release": {
          if (fields.agent_id === undefined) {
            if (office.holds.office === null) {
              refuse("conflict", "the office is not on hold");
            }
            office.holds.office = null;
            record(null, "release");
            pushHoldChanged();
            return {};
          }
          const agent = findAgent(fields.agent_id);
          if (heldAgent(agent.id) === undefined) {
            refuse("conflict", "the agent is not on hold");
          }
          office.holds.agents = office.holds.agents.filter(
            (hold) => hold.agent_id !== agent.id,
          );
          record(agent.id, "release");
          pushHoldChanged();
          return {};
        }
        case "office.actions":
          return office.actions
            .filter((entry) => entry.agent_id === null)
            .reverse();
        case "agent.actions":
          return office.actions.filter(
            (entry) => entry.agent_id === fields.agent_id,
          );
        default:
          return undefined;
      }
    };

    const internals = {
      transformCallback(callback: (message: unknown) => void) {
        const id = nextCallbackId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id: number) {
        callbacks.delete(id);
      },
      async invoke(command: string, args: Record<string, unknown> = {}) {
        calls.push({ command, args });
        switch (command) {
          case "watch_connection": {
            const { id } = args.onState as { id: number };
            stateChannels.push(id);
            send(id, current);
            return undefined;
          }
          case "watch_events": {
            const { id } = args.onEvent as { id: number };
            eventChannels.push(id);
            return undefined;
          }
          case "plugin:notification|register_listener": {
            if (args.event === "actionPerformed") {
              const { id } = args.handler as { id: number };
              notificationActionChannels.push(id);
            }
            return undefined;
          }
          case "watch_terminal_frames": {
            const { id } = args.onFrame as { id: number };
            terminalChannels.push(id);
            return undefined;
          }
          case "send_request":
            return await answer(
              args.kind as string,
              (args.fields as Record<string, unknown>) ?? {},
            );
          case "start_server":
            if (mock.startServer?.outcome === "fail") {
              throw {
                code: mock.startServer.code,
                message: mock.startServer.message,
              };
            }
            if (mock.startServer?.outcome === "ok") {
              push(mock.startServer.stateAfterStart);
            }
            return undefined;
          case "login_start_status":
            if (mock.loginStart?.statusFailure) {
              throw mock.loginStart.statusFailure;
            }
            return loginStartRegistered;
          case "register_login_start":
            if (mock.loginStart?.failure) throw mock.loginStart.failure;
            loginStartRegistered = true;
            return undefined;
          default:
            return undefined;
        }
      },
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: internals,
      __mockCore: {
        push,
        pushEvent,
        pushTerminalFrame,
        pushAgentLine,
        pushNotificationCreated,
        pushNotificationTap: (agentId: string) => {
          notificationActionChannels.forEach((id) =>
            send(id, {
              actionId: "tap",
              notification: { extra: { agent_id: agentId } },
            }),
          );
        },
        setHierarchy: (tree: HierarchyTree) => {
          office.hierarchy = structuredClone(tree);
        },
        notify: (
          agentName: string,
          agentId: string,
          event: string,
          detail?: string,
        ) => {
          createNotification(
            { id: agentId, name: agentName } as Agent,
            event as NotificationRow["event"],
            detail ?? null,
          );
        },
        attention: (
          agentId: string,
          kind: NeedsYouKind,
          detail?: string,
          firstEventAt?: number,
        ) => {
          recordAttention(
            findAgent(agentId),
            kind,
            detail ?? null,
            firstEventAt ?? Date.now(),
          );
        },
        autoHold: (agentId: string, limitName: string) => {
          const agent = findAgent(agentId);
          office.holds.agents.push({
            agent_id: agent.id,
            set_by: "office",
            reason: `the limit "${limitName}"`,
            since: Date.now(),
          });
          record(agent.id, "hold", "office");
          pushHoldChanged();
          createNotification(agent, "auto_hold", limitName);
        },
        calls,
        settings,
      },
    });
  }, options);
}
