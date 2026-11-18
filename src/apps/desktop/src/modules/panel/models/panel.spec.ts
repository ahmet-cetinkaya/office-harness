// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { Agent } from "../../agent";
import type { HierarchyTree } from "../../hierarchy";
import {
  agentHierarchy,
  hierarchyText,
  isEmptyProject,
  projectNameOf,
  visibleAgents,
} from "./panel";

function agent(id: string, overrides: Partial<Agent> = {}): Agent {
  return {
    id,
    name: `Agent ${id}`,
    command: "claude",
    working_folder: "/work",
    project_id: null,
    machine_id: "m",
    status: "working",
    created_at: 1,
    last_line: null,
    ...overrides,
  };
}

const PROJECTS = [
  { id: "p-site", name: "Site", folder: "/work/site" },
  { id: "p-api", name: "Api", folder: "/work/api" },
];

describe("panel list", () => {
  test("all projects shows the agents of every project and the no-project agents", () => {
    const agents = [
      agent("a", { project_id: "p-site" }),
      agent("b", { project_id: null }),
      agent("c", { project_id: "p-api" }),
    ];
    expect(visibleAgents(agents, null).map((entry) => entry.id)).toEqual([
      "a",
      "b",
      "c",
    ]);
  });

  test("one project shows only its own agents", () => {
    const agents = [
      agent("a", { project_id: "p-site" }),
      agent("b", { project_id: null }),
      agent("c", { project_id: "p-api" }),
    ];
    expect(visibleAgents(agents, "p-site").map((entry) => entry.id)).toEqual([
      "a",
    ]);
  });

  test("a no-project agent never shows under one project", () => {
    const agents = [agent("b", { project_id: null })];
    expect(visibleAgents(agents, "p-site")).toEqual([]);
    expect(visibleAgents(agents, null)).toHaveLength(1);
  });

  test("an empty project is named, an empty choice under all projects is not", () => {
    expect(isEmptyProject(PROJECTS, [], "p-site")).toBe(true);
    expect(isEmptyProject(PROJECTS, [agent("a")], "p-site")).toBe(false);
    expect(isEmptyProject(PROJECTS, [], null)).toBe(false);
    expect(isEmptyProject(PROJECTS, [], "p-gone")).toBe(false);
  });

  test("the project name comes from the agent's project, or not at all", () => {
    expect(projectNameOf(agent("a", { project_id: "p-api" }), PROJECTS)).toBe(
      "Api",
    );
    expect(projectNameOf(agent("b"), PROJECTS)).toBeNull();
  });

  test("the hierarchy gives the role, and the team name for a team agent", () => {
    const tree: HierarchyTree = {
      ceo: { id: "ceo", role: "ceo" },
      teams: [
        {
          id: "t",
          name: "Backend",
          project_id: null,
          lead: { id: "lead", role: "lead" },
          members: [{ id: "member", role: "member" }],
        },
      ],
      unassigned: [{ id: "outside", role: null }],
    };
    expect(agentHierarchy(tree, "ceo")).toEqual({
      role: "ceo",
      teamName: null,
    });
    expect(agentHierarchy(tree, "lead")).toEqual({
      role: "lead",
      teamName: "Backend",
    });
    expect(agentHierarchy(tree, "member")).toEqual({
      role: "member",
      teamName: "Backend",
    });
    expect(agentHierarchy(tree, "outside")).toEqual({
      role: null,
      teamName: null,
    });
    expect(agentHierarchy(tree, "nobody")).toEqual({
      role: null,
      teamName: null,
    });
  });

  test("the hierarchy line reads as text, or is empty outside the hierarchy", () => {
    expect(hierarchyText({ role: "ceo", teamName: null })).toBe("CEO");
    expect(hierarchyText({ role: "lead", teamName: "Backend" })).toBe(
      "team lead, Backend",
    );
    expect(hierarchyText({ role: null, teamName: null })).toBe("");
  });
});
