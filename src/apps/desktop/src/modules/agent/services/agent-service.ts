// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import type {
  Additions,
  AdditionsView,
  Agent,
  NewAgent,
  NewProject,
  Project,
  Session,
} from "../models/agent";

function asList<Item>(result: unknown): Item[] {
  return Array.isArray(result) ? (result as Item[]) : [];
}

/** The typed face of the `agent.*`, `project.*`, and `session.*` requests. */
export function createAgentService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    listAgents: async () => asList<Agent>(await send("agent.list")),
    listProjects: async () => asList<Project>(await send("project.list")),
    createAgent: async (agent: NewAgent) =>
      (await send("agent.create", { ...agent })) as Agent,
    createProject: async (project: NewProject) =>
      (await send("project.create", { ...project })) as Project,
    deleteProject: async (projectId: string) => {
      await send("project.delete", { project_id: projectId });
    },
    startAgent: async (agentId: string) => {
      await send("agent.start", { agent_id: agentId });
    },
    stopAgent: async (agentId: string) => {
      await send("agent.stop", { agent_id: agentId });
    },
    startNewTask: async (agentId: string) => {
      await send("agent.new_task", { agent_id: agentId });
    },
    deleteAgent: async (agentId: string) => {
      await send("agent.delete", { agent_id: agentId });
    },
    getAdditions: async (agentId: string) =>
      (await send("agent.additions.get", {
        agent_id: agentId,
      })) as AdditionsView,
    setAdditions: async (agentId: string, additions: Additions) =>
      (await send("agent.additions.set", {
        agent_id: agentId,
        additions,
      })) as AdditionsView,
    setTask: async (agentId: string, text: string) =>
      (await send("agent.task.set", { agent_id: agentId, text })) as Agent,
    listSessions: async (agentId: string) =>
      asList<Session>(await send("session.list", { agent_id: agentId })),
    continueSession: async (sessionId: string) => {
      await send("session.continue", { session_id: sessionId });
    },
  };
}

export type AgentService = ReturnType<typeof createAgentService>;
