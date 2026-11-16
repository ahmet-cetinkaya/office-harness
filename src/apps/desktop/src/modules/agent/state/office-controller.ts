// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import type { ActionEntry, ActionRecordSource } from "../../audit";
import {
  isActive,
  type Additions,
  type AdditionsView,
  type Agent,
  type NewAgent,
  type NewProject,
  type Project,
  type Session,
} from "../models/agent";
import { createAgentService } from "../services/agent-service";
import {
  EVENT_ACTION_RECORDED,
  EVENT_ADDITIONS_CHANGED,
  EVENT_AGENT_CHANGED,
  EVENT_AGENT_DELETED,
  EVENT_AGENT_STATUS,
  EVENT_CLIENT_RESYNC,
  EVENT_STORAGE_ERROR,
  readActionRecorded,
  readAdditionsChanged,
  readAgentChanged,
  readAgentDeleted,
  readStatusEvent,
  readStorageError,
  upsertAgent,
  withoutAgent,
  withStatus,
} from "./office-events";

export interface Notice {
  lead: string;
  message: string;
}

export interface AgentDetails {
  agentId: string;
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  additions: AdditionsView | null;
  sessions: Session[];
  actions: ActionEntry[];
}

export interface OfficeSnapshot {
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  agents: Agent[];
  projects: Project[];
  /** The text that came with a status, for example why an agent failed. */
  statusDetails: Record<string, string>;
  /** Agents with a stop request that the server has not answered yet. */
  stoppingAgentIds: ReadonlySet<string>;
  selectedAgentId: string | null;
  details: AgentDetails | null;
  notice: Notice | null;
  isBusy: boolean;
}

/** The text of a refused or failed request, or null when it succeeded. */
export type FailureText = string | null;

export const INITIAL_OFFICE: OfficeSnapshot = {
  phase: "loading",
  failure: null,
  agents: [],
  projects: [],
  statusDetails: {},
  stoppingAgentIds: new Set(),
  selectedAgentId: null,
  details: null,
  notice: null,
  isBusy: false,
};

const REQUEST_FAILED = "The request failed.";

export function createOfficeController(
  client: CoreClient,
  onChange: (snapshot: OfficeSnapshot) => void,
  actionRecord: ActionRecordSource,
) {
  const service = createAgentService(client);
  let snapshot: OfficeSnapshot = INITIAL_OFFICE;
  /** Ids of agents that this client deletes, so their event is no news. */
  const ownDeletes = new Set<string>();

  function update(changes: Partial<OfficeSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    onChange(snapshot);
  }

  function updateDetails(
    agentId: string,
    changes: Partial<AgentDetails>,
  ): void {
    if (snapshot.details?.agentId !== agentId) return;
    update({ details: { ...snapshot.details, ...changes } });
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function loadDetails(agentId: string): Promise<void> {
    try {
      const [additions, sessions, actions] = await Promise.all([
        service.getAdditions(agentId),
        service.listSessions(agentId),
        actionRecord.list(agentId),
      ]);
      updateDetails(agentId, {
        phase: "ready",
        failure: null,
        additions,
        sessions,
        actions,
      });
    } catch (rejection) {
      updateDetails(agentId, { phase: "failed", failure: describe(rejection) });
    }
  }

  function showDetails(agentId: string): void {
    update({
      selectedAgentId: agentId,
      details: {
        agentId,
        phase: "loading",
        failure: null,
        additions: null,
        sessions: [],
        actions: [],
      },
    });
    void loadDetails(agentId);
  }

  async function refreshSessions(agentId: string): Promise<void> {
    try {
      updateDetails(agentId, { sessions: await service.listSessions(agentId) });
    } catch (rejection) {
      update({
        notice: {
          lead: "The session list did not load.",
          message: describe(rejection),
        },
      });
    }
  }

  /** Runs one request for the user. A failure shows as the notice. */
  async function perform(
    lead: string,
    work: () => Promise<void>,
  ): Promise<void> {
    update({ isBusy: true, notice: null });
    try {
      await work();
    } catch (rejection) {
      update({ notice: { lead, message: describe(rejection) } });
    } finally {
      update({ isBusy: false });
    }
  }

  /** Runs one request for a form, which shows the failure itself. */
  async function submit(work: () => Promise<void>): Promise<FailureText> {
    update({ isBusy: true, notice: null });
    try {
      await work();
      return null;
    } catch (rejection) {
      return describe(rejection);
    } finally {
      update({ isBusy: false });
    }
  }

  function handleStatus(event: ServerEvent): void {
    const { agentId, status, detail } = readStatusEvent(event);
    const before = snapshot.agents.find((agent) => agent.id === agentId);
    const statusDetails = { ...snapshot.statusDetails };
    if (detail === null) delete statusDetails[agentId];
    else statusDetails[agentId] = detail;
    update({
      agents: withStatus(snapshot.agents, agentId, status),
      statusDetails,
    });
    const sessionChanged =
      before !== undefined && isActive(before.status) !== isActive(status);
    if (sessionChanged && snapshot.selectedAgentId === agentId) {
      void refreshSessions(agentId);
    }
  }

  function handleDeleted(event: ServerEvent): void {
    const agentId = readAgentDeleted(event);
    const wasSelected = snapshot.selectedAgentId === agentId;
    const wasOwn = ownDeletes.delete(agentId);
    update({
      agents: withoutAgent(snapshot.agents, agentId),
      ...(wasSelected ? { selectedAgentId: null, details: null } : {}),
      ...(wasSelected && !wasOwn
        ? {
            notice: {
              lead: "The agent was deleted.",
              message: "Another client removed the agent you had open.",
            },
          }
        : {}),
    });
  }

  function handleEvent(event: ServerEvent): void {
    switch (event.type) {
      case EVENT_AGENT_STATUS:
        return handleStatus(event);
      case EVENT_AGENT_CHANGED:
        return update({
          agents: upsertAgent(snapshot.agents, readAgentChanged(event)),
        });
      case EVENT_AGENT_DELETED:
        return handleDeleted(event);
      case EVENT_ADDITIONS_CHANGED: {
        const { agentId, additions } = readAdditionsChanged(event);
        return updateDetails(agentId, { additions });
      }
      case EVENT_ACTION_RECORDED: {
        const entry = readActionRecorded(event);
        const details = snapshot.details;
        if (details?.agentId !== entry.agent_id) return;
        return updateDetails(entry.agent_id, {
          actions: actionRecord.merge(details.actions, entry),
        });
      }
      case EVENT_CLIENT_RESYNC:
        return void load();
      case EVENT_STORAGE_ERROR:
        return update({
          notice: {
            lead: "The office cannot save.",
            message: readStorageError(event),
          },
        });
    }
  }

  async function load(): Promise<void> {
    try {
      const [agents, projects] = await Promise.all([
        service.listAgents(),
        service.listProjects(),
      ]);
      update({ phase: "ready", failure: null, agents, projects });
      const selected = snapshot.selectedAgentId;
      if (selected === null) return;
      if (agents.some((agent) => agent.id === selected))
        await loadDetails(selected);
      else update({ selectedAgentId: null, details: null });
    } catch (rejection) {
      update({ phase: "failed", failure: describe(rejection) });
    }
  }

  return {
    /** Listens for the server's events. Call it once. */
    async watch(): Promise<void> {
      try {
        await client.watchEvents(handleEvent);
      } catch (rejection) {
        update({
          notice: {
            lead: "Live updates are off.",
            message: describe(rejection),
          },
        });
      }
    },

    /** Loads the office. Call it each time the connection becomes ready. */
    load,

    select(agentId: string): void {
      if (snapshot.selectedAgentId === agentId) return;
      showDetails(agentId);
    },

    retryDetails(): void {
      const agentId = snapshot.selectedAgentId;
      if (agentId === null) return;
      updateDetails(agentId, { phase: "loading", failure: null });
      void loadDetails(agentId);
    },

    createAgent: (input: NewAgent): Promise<FailureText> =>
      submit(async () => {
        const agent = await service.createAgent(input);
        update({ agents: upsertAgent(snapshot.agents, agent) });
        showDetails(agent.id);
      }),

    createProject: (input: NewProject): Promise<FailureText> =>
      submit(async () => {
        const project = await service.createProject(input);
        update({ projects: [...snapshot.projects, project] });
      }),

    deleteProject: (projectId: string): Promise<void> =>
      perform("The project was not deleted.", async () => {
        await service.deleteProject(projectId);
        update({
          projects: snapshot.projects.filter(
            (project) => project.id !== projectId,
          ),
        });
      }),

    startAgent: (agentId: string): Promise<void> =>
      perform("The agent did not start.", async () => {
        await service.startAgent(agentId);
        await refreshSessions(agentId);
      }),

    stopAgent: async (agentId: string): Promise<void> => {
      if (snapshot.stoppingAgentIds.has(agentId)) return;
      update({
        stoppingAgentIds: new Set(snapshot.stoppingAgentIds).add(agentId),
      });
      await perform("The agent did not stop.", () =>
        service.stopAgent(agentId),
      );
      const stillStopping = new Set(snapshot.stoppingAgentIds);
      stillStopping.delete(agentId);
      update({ stoppingAgentIds: stillStopping });
    },

    startNewTask: (agentId: string): Promise<void> =>
      perform("The new task did not start.", async () => {
        await service.startNewTask(agentId);
        await refreshSessions(agentId);
      }),

    continueSession: (agentId: string, sessionId: string): Promise<void> =>
      perform("The session was not continued.", async () => {
        await service.continueSession(sessionId);
        await refreshSessions(agentId);
      }),

    deleteAgent: (agentId: string): Promise<void> =>
      perform("The agent was not deleted.", async () => {
        ownDeletes.add(agentId);
        try {
          await service.deleteAgent(agentId);
        } catch (rejection) {
          ownDeletes.delete(agentId);
          throw rejection;
        }
        handleDeleted({ type: EVENT_AGENT_DELETED, agent_id: agentId });
      }),

    saveAdditions: (
      agentId: string,
      additions: Additions,
    ): Promise<FailureText> =>
      submit(async () => {
        const saved = await service.setAdditions(agentId, additions);
        updateDetails(agentId, { additions: saved });
      }),
  };
}

export type OfficeController = ReturnType<typeof createOfficeController>;
