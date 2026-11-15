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

/** What the faked office holds when the page opens. */
export interface OfficeSeed {
  agents?: Agent[];
  projects?: Project[];
  sessions?: Record<string, Session[]>;
  additions?: Record<string, AdditionsView>;
  /** The text of the snapshot frame that follows `terminal.attach`. */
  snapshots?: Record<string, string>;
  /** The `focus_shortcut` setting. The server's default when left out. */
  focusShortcut?: string;
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
}

export interface RecordedCall {
  command: string;
  args: Record<string, unknown>;
}

declare global {
  interface Window {
    __mockCore: {
      push(state: ConnectionState): void;
      /** Sends a server event to the webview, as the core forwards it. */
      pushEvent(event: ServerEvent): void;
      /** Sends a binary terminal frame: kind 1 snapshot, 2 output, 3 dropped. */
      pushTerminalFrame(kind: number, agentId: string, text: string): void;
      calls: RecordedCall[];
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
    const terminalChannels: number[] = [];
    const calls: RecordedCall[] = [];
    let nextCallbackId = 1;
    let current = mock.initialState;

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
      actions: [] as ActionEntry[],
      nextId: 1,
    };
    for (const agent of office.agents) {
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
    const isActive = (status: string) =>
      ["starting", "working", "waiting_for_input"].includes(status);
    const findAgent = (agentId: unknown) =>
      office.agents.find((agent) => agent.id === agentId) ??
      refuse("not_found", "no such agent");
    const record = (agentId: string, action: string) => {
      const entry: ActionEntry = {
        id: office.actions.length + 1,
        actor: "user",
        agent_id: agentId,
        action,
        result: "ok",
        at: Date.now(),
      };
      office.actions.push(entry);
      pushEvent({ type: "action.recorded", entry });
    };
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

    // The requests that the real server answers, in the shapes of the
    // contract. Each change also sends the events that the server sends.
    const answer = (kind: string, fields: Record<string, unknown>): unknown => {
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
          return office.agents;
        case "agent.create": {
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
          };
          office.agents.push(agent);
          office.additions[agent.id] = {
            flags: [],
            env: [],
            startup_prompt: null,
            updated_at: Date.now(),
            applies_next_session: false,
          };
          pushEvent({ type: "agent.changed", agent });
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
          const agent = findAgent(fields.agent_id);
          endSessions(agent.id);
          setStatus(agent, "stopped");
          record(agent.id, "stop");
          return null;
        }
        case "agent.delete": {
          const agent = findAgent(fields.agent_id);
          office.agents = office.agents.filter((known) => known !== agent);
          pushEvent({ type: "agent.deleted", agent_id: agent.id });
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
        case "settings.get":
          return {
            key: fields.key,
            value: mock.office?.focusShortcut ?? "Ctrl+Alt+Escape",
          };
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
          case "watch_terminal_frames": {
            const { id } = args.onFrame as { id: number };
            terminalChannels.push(id);
            return undefined;
          }
          case "send_request":
            return answer(
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
          default:
            return undefined;
        }
      },
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: internals,
      __mockCore: { push, pushEvent, pushTerminalFrame, calls },
    });
  }, options);
}
