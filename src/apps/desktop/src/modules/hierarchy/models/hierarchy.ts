// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Agent, Project } from "../../agent";

/** The status words the server sends (0001 FR-014). */
const STATUS_TEXT: Record<string, string> = {
  starting: "starting",
  working: "working",
  waiting_for_input: "waiting for input",
  exited: "exited",
  failed: "failed",
  stopped: "stopped",
  interrupted: "interrupted",
};

/** The role words the tree answer carries (FR-001). */
const ROLE_TEXT: Record<string, string> = {
  ceo: "CEO",
  lead: "team lead",
  member: "member",
};

/** A status as text. A word that this app does not know is shown as sent. */
export function statusText(status: string): string {
  return STATUS_TEXT[status] ?? status.replaceAll("_", " ");
}

/** A role as text. An agent without a role sits outside the hierarchy. */
export function roleText(role: string | null): string {
  if (role === null) return "no role";
  return ROLE_TEXT[role] ?? role.replaceAll("_", " ");
}

/** One agent as the `hierarchy.tree` answer carries it (contract 0002). */
export interface HierarchyTreeAgent {
  id: string;
  role: string | null;
}

/** One team branch of the `hierarchy.tree` answer. */
export interface HierarchyTreeTeam {
  id: string;
  name: string;
  project_id: string | null;
  lead: HierarchyTreeAgent;
  members: HierarchyTreeAgent[];
}

/** The whole `hierarchy.tree` answer: the CEO, the teams, and the agents without a role. */
export interface HierarchyTree {
  ceo: HierarchyTreeAgent | null;
  teams: HierarchyTreeTeam[];
  unassigned: HierarchyTreeAgent[];
}

export const EMPTY_TREE: HierarchyTree = {
  ceo: null,
  teams: [],
  unassigned: [],
};

/** One row of the tree as the view shows it. */
export interface HierarchyNode {
  /** Stable key: the agent id, `team:<id>`, or the no-role group's id. */
  id: string;
  title: string;
  meta: string;
  /** The accessible name: title and meta as one spoken line. */
  name: string;
  /** Depth from the top, starting at 1. Indentation only reinforces it. */
  level: number;
  /** Set when the row is an agent; Enter opens it. */
  agentId: string | null;
  /** Whether a branch shows its children. Only a branch with children uses it. */
  expanded: boolean;
  children: HierarchyNode[];
}

/** One visible row, with the branch it sits in. */
export interface FlatNode {
  node: HierarchyNode;
  parentId: string | null;
}

const UNKNOWN_AGENT_NAME = "Unknown agent";
export const UNASSIGNED_GROUP_ID = "unassigned";

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function readAgent(value: unknown): HierarchyTreeAgent | null {
  if (typeof value !== "object" || value === null) return null;
  const fields = value as Record<string, unknown>;
  if (typeof fields.id !== "string") return null;
  return {
    id: fields.id,
    role: typeof fields.role === "string" ? fields.role : null,
  };
}

function readTeam(value: unknown): HierarchyTreeTeam | null {
  if (typeof value !== "object" || value === null) return null;
  const fields = value as Record<string, unknown>;
  const lead = readAgent(fields.lead);
  if (
    typeof fields.id !== "string" ||
    typeof fields.name !== "string" ||
    lead === null
  ) {
    return null;
  }
  return {
    id: fields.id,
    name: fields.name,
    project_id:
      typeof fields.project_id === "string" ? fields.project_id : null,
    lead,
    members: asArray(fields.members)
      .map(readAgent)
      .filter((member) => member !== null),
  };
}

/** Reads the tree from the answer. A malformed answer becomes the empty tree, so the view never renders a guess. */
export function readHierarchyTree(answer: unknown): HierarchyTree {
  if (typeof answer !== "object" || answer === null) return EMPTY_TREE;
  const fields = answer as Record<string, unknown>;
  return {
    ceo: readAgent(fields.ceo),
    teams: asArray(fields.teams)
      .map(readTeam)
      .filter((team) => team !== null),
    unassigned: asArray(fields.unassigned)
      .map(readAgent)
      .filter((agent) => agent !== null),
  };
}

function nodeName(title: string, meta: string): string {
  return meta === "" ? title : `${title}, ${meta}`;
}

/** A count as spoken text: "1 member", "2 members". */
function countText(count: number, word: string): string {
  return count === 1 ? `1 ${word}` : `${count} ${word}s`;
}

function agentNode(
  treeAgent: HierarchyTreeAgent,
  level: number,
  children: HierarchyNode[],
  agents: ReadonlyMap<string, Agent>,
  collapsedIds: ReadonlySet<string>,
): HierarchyNode {
  const known = agents.get(treeAgent.id);
  const role = roleText(treeAgent.role);
  const title = known ? known.name : UNKNOWN_AGENT_NAME;
  const meta = known ? `${role}, ${statusText(known.status)}` : role;
  return {
    id: treeAgent.id,
    title,
    meta,
    name: nodeName(title, meta),
    level,
    agentId: treeAgent.id,
    expanded: !collapsedIds.has(treeAgent.id),
    children,
  };
}

function teamNode(
  team: HierarchyTreeTeam,
  level: number,
  agents: ReadonlyMap<string, Agent>,
  projects: Project[],
  collapsedIds: ReadonlySet<string>,
): HierarchyNode {
  const members = team.members.map((member) =>
    agentNode(member, level + 2, [], agents, collapsedIds),
  );
  const lead = agentNode(team.lead, level + 1, members, agents, collapsedIds);
  const projectName = projects.find(
    (project) => project.id === team.project_id,
  )?.name;
  const meta = [
    ...(projectName === undefined ? [] : [`project ${projectName}`]),
    team.members.length === 0
      ? "no members"
      : countText(team.members.length, "member"),
  ].join(", ");
  const title = `Team ${team.name}`;
  const id = `team:${team.id}`;
  return {
    id,
    title,
    meta,
    name: nodeName(title, meta),
    level,
    agentId: null,
    expanded: !collapsedIds.has(id),
    children: [lead],
  };
}

function unassignedGroup(
  unassigned: HierarchyTreeAgent[],
  agents: ReadonlyMap<string, Agent>,
  collapsedIds: ReadonlySet<string>,
): HierarchyNode | null {
  if (unassigned.length === 0) return null;
  const title = "Outside the hierarchy";
  const meta = countText(unassigned.length, "agent");
  return {
    id: UNASSIGNED_GROUP_ID,
    title,
    meta,
    name: nodeName(title, meta),
    level: 1,
    agentId: null,
    expanded: !collapsedIds.has(UNASSIGNED_GROUP_ID),
    children: unassigned.map((agent) =>
      agentNode(agent, 2, [], agents, collapsedIds),
    ),
  };
}

/**
 * Builds the rows of the hierarchy view: the CEO with the teams under it (or
 * the teams alone when no CEO exists), then the no-role group. The server's
 * order is kept. Rows the user collapsed stay built; `flattenVisible` hides
 * their children.
 */
export function buildHierarchy(
  tree: HierarchyTree,
  agents: Agent[],
  projects: Project[],
  collapsedIds: ReadonlySet<string>,
): HierarchyNode[] {
  const byId = new Map(agents.map((agent) => [agent.id, agent]));
  const nodes: HierarchyNode[] = [];
  const teamLevel = tree.ceo === null ? 1 : 2;
  const teamNodes = tree.teams.map((team) =>
    teamNode(team, teamLevel, byId, projects, collapsedIds),
  );
  if (tree.ceo !== null) {
    nodes.push(agentNode(tree.ceo, 1, teamNodes, byId, collapsedIds));
  } else {
    nodes.push(...teamNodes);
  }
  const group = unassignedGroup(tree.unassigned, byId, collapsedIds);
  if (group !== null) nodes.push(group);
  return nodes;
}

/** The rows a user can see now, parents first, each with its branch. */
export function flattenVisible(nodes: HierarchyNode[]): FlatNode[] {
  const rows: FlatNode[] = [];
  const walk = (branch: HierarchyNode[], parentId: string | null): void => {
    for (const node of branch) {
      rows.push({ node, parentId });
      if (node.expanded && node.children.length > 0) {
        walk(node.children, node.id);
      }
    }
  };
  walk(nodes, null);
  return rows;
}
