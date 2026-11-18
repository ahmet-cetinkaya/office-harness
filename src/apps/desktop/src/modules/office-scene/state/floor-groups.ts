// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Agent, Project } from "../../agent";

/** The floor of the agents without a project, always last (0005 D3). */
export const NO_PROJECT_FLOOR_NAME = "No project";

/** The project choice the switcher holds: every project, or one project (D7). */
export type ProjectChoice =
  { kind: "all" } | { kind: "one"; projectId: string };

/**
 * One agent as the `hierarchy.tree` answer carries it (contract 0002). The
 * names mirror `modules/hierarchy/models/hierarchy.ts`, whose public face
 * does not export the types yet; the scene takes the read tree as its second
 * input because the agent snapshot carries no role or team (research R3).
 */
export interface HierarchyTreeAgent {
  id: string;
  role: string | null;
}

/** One team branch of the `hierarchy.tree` answer (contract 0002). */
export interface HierarchyTreeTeam {
  id: string;
  name: string;
  project_id: string | null;
  lead: HierarchyTreeAgent;
  members: HierarchyTreeAgent[];
}

/** The whole `hierarchy.tree` answer: the CEO, the teams, the agents without a role. */
export interface HierarchyTree {
  ceo: HierarchyTreeAgent | null;
  teams: HierarchyTreeTeam[];
  unassigned: HierarchyTreeAgent[];
}

/** One department of a floor, or the whole floor when no department splits it. */
export interface DeskGroup {
  /** The team name, or null for the group without a department. */
  name: string | null;
  /**
   * The agents in reading order: a department's lead first where it sits,
   * then by agent name; the CEO first in the group without a department.
   */
  agents: Agent[];
}

/** One floor: a project of the choice, or the no-project floor (D3). */
export interface Floor {
  /** The project name, or `NO_PROJECT_FLOOR_NAME` on the no-project floor. */
  name: string;
  /** The project id, or null on the no-project floor. */
  projectId: string | null;
  /** How many desks the floor holds. */
  agentCount: number;
  /** The group without a department first, then departments by team name. */
  groups: DeskGroup[];
}

function byName(first: Agent, second: Agent): number {
  return (
    first.name.localeCompare(second.name) || first.id.localeCompare(second.id)
  );
}

function byProjectName(first: Project, second: Project): number {
  return (
    first.name.localeCompare(second.name) || first.id.localeCompare(second.id)
  );
}

function byTeamName(
  first: HierarchyTreeTeam,
  second: HierarchyTreeTeam,
): number {
  return (
    first.name.localeCompare(second.name) || first.id.localeCompare(second.id)
  );
}

/** The team of every agent the tree places, the lead included. */
function teamOfEveryAgent(
  tree: HierarchyTree | undefined,
): Map<string, HierarchyTreeTeam> {
  const teamOf = new Map<string, HierarchyTreeTeam>();
  for (const team of tree?.teams ?? []) {
    for (const treeAgent of [team.lead, ...team.members]) {
      teamOf.set(treeAgent.id, team);
    }
  }
  return teamOf;
}

/** The group without a department: the CEO first when present, then by name (D3). */
function ceoFirstThenByName(agents: Agent[], ceoId: string | null): Agent[] {
  const ceo =
    ceoId === null ? undefined : agents.find((agent) => agent.id === ceoId);
  const rest = agents.filter((agent) => agent !== ceo).sort(byName);
  return ceo === undefined ? rest : [ceo, ...rest];
}

/** One department's agents on one floor: the lead first only where it sits (D3). */
function departmentAgents(
  team: HierarchyTreeTeam,
  floorAgents: Agent[],
  teamOf: ReadonlyMap<string, HierarchyTreeTeam>,
): Agent[] {
  const inTeam = floorAgents.filter((agent) => teamOf.get(agent.id) === team);
  const members = inTeam
    .filter((agent) => agent.id !== team.lead.id)
    .sort(byName);
  const lead = inTeam.find((agent) => agent.id === team.lead.id);
  return lead === undefined ? members : [lead, ...members];
}

/** The groups of one floor: the group without a department first, then departments sorted by team name (D3). */
function groupsOf(
  floorAgents: Agent[],
  teamOf: ReadonlyMap<string, HierarchyTreeTeam>,
  ceoId: string | null,
): DeskGroup[] {
  const groups: DeskGroup[] = [];
  const outsideTeams = floorAgents.filter((agent) => !teamOf.has(agent.id));
  if (outsideTeams.length > 0) {
    groups.push({
      name: null,
      agents: ceoFirstThenByName(outsideTeams, ceoId),
    });
  }
  const teamsOfFloor = [
    ...new Set(
      floorAgents
        .map((agent) => teamOf.get(agent.id))
        .filter((team) => team !== undefined),
    ),
  ].sort(byTeamName);
  for (const team of teamsOfFloor) {
    groups.push({
      name: team.name,
      agents: departmentAgents(team, floorAgents, teamOf),
    });
  }
  return groups;
}

function floorOf(
  name: string,
  projectId: string | null,
  floorAgents: Agent[],
  teamOf: ReadonlyMap<string, HierarchyTreeTeam>,
  ceoId: string | null,
): Floor {
  return {
    name,
    projectId,
    agentCount: floorAgents.length,
    groups: groupsOf(floorAgents, teamOf, ceoId),
  };
}

/**
 * The floors of the office scene (0005 D3): the projects of the choice
 * sorted by name, then the no-project floor when a chosen agent lacks one.
 * An agent whose project is gone from the choice sits on the no-project
 * floor too, so no desk is lost. An undefined or empty tree groups by
 * floors only: every agent sits in the group without a department.
 */
export function floorGroups(
  agents: Agent[],
  projects: Project[],
  choice: ProjectChoice,
  tree?: HierarchyTree,
): Floor[] {
  const teamOf = teamOfEveryAgent(tree);
  const ceoId = tree?.ceo?.id ?? null;
  const chosenProjects = projects
    .filter(
      (project) => choice.kind === "all" || project.id === choice.projectId,
    )
    .sort(byProjectName);
  const floors = chosenProjects.map((project) =>
    floorOf(
      project.name,
      project.id,
      agents.filter((agent) => agent.project_id === project.id),
      teamOf,
      ceoId,
    ),
  );
  if (choice.kind === "all") {
    const projectIds = new Set(chosenProjects.map((project) => project.id));
    const withoutFloor = agents.filter(
      (agent) => agent.project_id === null || !projectIds.has(agent.project_id),
    );
    if (withoutFloor.length > 0) {
      floors.push(
        floorOf(NO_PROJECT_FLOOR_NAME, null, withoutFloor, teamOf, ceoId),
      );
    }
  }
  return floors;
}

/**
 * The agents whose name holds the text, in the given reading order. Text
 * that is empty or spaces only is no search, so every agent matches (D7).
 * The case handling is a case-insensitive substring for now: the panel's
 * switcher match function does not exist yet (research R2), and T3 swaps it
 * in here when it lands.
 */
export function searchMatches(agents: Agent[], text: string): Agent[] {
  const needle = text.trim().toLowerCase();
  if (needle === "") return [...agents];
  return agents.filter((agent) => agent.name.toLowerCase().includes(needle));
}

/**
 * The status text of a desk. It mirrors `shownStatusText` from
 * `modules/agent/models/agent.ts` — "Stopping" while the agent's stop
 * request is open — because the agent module's public face does not export
 * that function yet; a change there must be mirrored here.
 */
export function deskStatusText(agent: Agent, isStopping: boolean): string {
  if (isStopping) return "Stopping";
  return agent.status.replaceAll("_", " ");
}

/** The last output line of a desk, empty before any output (D4). */
export function deskLastLine(agent: Agent): string {
  return agent.last_line ?? "";
}
