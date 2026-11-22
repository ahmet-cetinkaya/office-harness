// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ServerEvent } from "../../../core/api";
import type { ActionEntry } from "../../audit";
import type { AdditionsView, Agent, AgentTask } from "../models/agent";

export const EVENT_AGENT_STATUS = "agent.status";
export const EVENT_AGENT_CHANGED = "agent.changed";
export const EVENT_AGENT_LINE = "agent.line";
export const EVENT_AGENT_TASK = "agent.task";
export const EVENT_AGENT_DELETED = "agent.deleted";
export const EVENT_PROJECT_CHANGED = "project.changed";
export const EVENT_ADDITIONS_CHANGED = "agent.additions.changed";
export const EVENT_ACTION_RECORDED = "action.recorded";
export const EVENT_STORAGE_ERROR = "storage.error";
/**
 * Made by the desktop core, never by the server: the webview missed events
 * and must reload. Its `client.` prefix cannot clash with a contract event.
 */
export const EVENT_CLIENT_RESYNC = "client.resync";

export function upsertAgent(agents: Agent[], agent: Agent): Agent[] {
  const exists = agents.some((known) => known.id === agent.id);
  return exists
    ? agents.map((known) => (known.id === agent.id ? agent : known))
    : [...agents, agent];
}

export function withStatus(
  agents: Agent[],
  agentId: string,
  status: string,
): Agent[] {
  return agents.map((agent) =>
    agent.id === agentId ? { ...agent, status } : agent,
  );
}

export function withLine(
  agents: Agent[],
  agentId: string,
  line: string,
): Agent[] {
  return agents.map((agent) =>
    agent.id === agentId ? { ...agent, last_line: line } : agent,
  );
}

/**
 * Patches only the task. `agent.changed` replaces the whole agent with a null
 * last line, so a task change never travels on it.
 */
export function withTask(
  agents: Agent[],
  agentId: string,
  task: AgentTask | null,
): Agent[] {
  return agents.map((agent) =>
    agent.id === agentId ? { ...agent, task } : agent,
  );
}

export function readTaskEvent(event: ServerEvent) {
  return {
    agentId: event.agent_id as string,
    task: (event.task ?? null) as AgentTask | null,
  };
}

export function readLineEvent(event: ServerEvent): string {
  return typeof event.line === "string" ? event.line : "";
}

export function withoutAgent(agents: Agent[], agentId: string): Agent[] {
  return agents.filter((agent) => agent.id !== agentId);
}

export function readStatusEvent(event: ServerEvent) {
  return {
    agentId: event.agent_id as string,
    status: event.status as string,
    detail: typeof event.detail === "string" ? event.detail : null,
  };
}

export function readAgentChanged(event: ServerEvent): Agent {
  return event.agent as Agent;
}

export function readAgentDeleted(event: ServerEvent): string {
  return event.agent_id as string;
}

export function readAdditionsChanged(event: ServerEvent) {
  return {
    agentId: event.agent_id as string,
    additions: event.additions as AdditionsView,
  };
}

export function readActionRecorded(event: ServerEvent): ActionEntry {
  return event.entry as ActionEntry;
}

export function readStorageError(event: ServerEvent): string {
  return typeof event.message === "string"
    ? event.message
    : "The office could not save its state.";
}

/** True for the bridge's lag text, which names the lost notifications. */
export function namesLostNotifications(message: string): boolean {
  return (
    message.includes("missed") && message.includes("notifications are missing")
  );
}
