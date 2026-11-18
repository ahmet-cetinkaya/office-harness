// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Agent, Project } from "../../agent";
import { roleText, type HierarchyTree } from "../../hierarchy";

/** The project choice that shows the agents of every project. */
export const ALL_PROJECTS = null;

/**
 * The agents of the choice, in the server's order (FR-005). An agent without
 * a project shows under all projects only (edge case).
 */
export function visibleAgents(
  agents: Agent[],
  projectChoice: string | null,
): Agent[] {
  return projectChoice === ALL_PROJECTS
    ? agents
    : agents.filter((agent) => agent.project_id === projectChoice);
}

/** The choice as it applies: a project that no longer exists shows all projects. */
export function effectiveChoice(
  projects: Project[],
  projectChoice: string | null,
): string | null {
  if (projectChoice === ALL_PROJECTS) return ALL_PROJECTS;
  const exists = projects.some((project) => project.id === projectChoice);
  return exists ? projectChoice : ALL_PROJECTS;
}

/** True when the chosen project exists but has no agents (edge case). */
export function isEmptyProject(
  projects: Project[],
  agents: Agent[],
  projectChoice: string | null,
): boolean {
  return (
    projectChoice !== ALL_PROJECTS &&
    projects.some((project) => project.id === projectChoice) &&
    agents.length === 0
  );
}

/** The name of the agent's project, or null when it has none. */
export function projectNameOf(
  agent: Agent,
  projects: Project[],
): string | null {
  return (
    projects.find((project) => project.id === agent.project_id)?.name ?? null
  );
}

export interface AgentHierarchy {
  /** The role word from the tree, such as "ceo", or null outside the hierarchy. */
  role: string | null;
  /** The agent's team's name, or null outside every team. */
  teamName: string | null;
}

/**
 * The row's second line for the hierarchy: "CEO", "team lead, team Backend",
 * or empty outside the hierarchy. `roleText` turns a role word into text.
 */
export function hierarchyText(hierarchy: AgentHierarchy): string {
  const parts = [
    hierarchy.role === null ? null : roleText(hierarchy.role),
    hierarchy.teamName,
  ].filter((part) => part !== null);
  return parts.join(", ");
}

/** One agent's role and team from the hierarchy tree (FR-003, D10). */
export function agentHierarchy(
  tree: HierarchyTree,
  agentId: string,
): AgentHierarchy {
  if (tree.ceo !== null && tree.ceo.id === agentId) {
    return { role: tree.ceo.role, teamName: null };
  }
  for (const team of tree.teams) {
    const entry =
      team.lead.id === agentId
        ? team.lead
        : team.members.find((member) => member.id === agentId);
    if (entry !== undefined) return { role: entry.role, teamName: team.name };
  }
  return { role: null, teamName: null };
}
