// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { Agent, Project } from "../../agent";
import {
  deskLastLine,
  deskStatusText,
  floorGroups,
  NO_PROJECT_FLOOR_NAME,
  type ProjectChoice,
  searchMatches,
  type HierarchyTree,
} from "./floor-groups";

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-1",
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1,
    last_line: null,
    ...overrides,
  };
}

const ALL: ProjectChoice = { kind: "all" };
const ONE_SITE: ProjectChoice = { kind: "one", projectId: "project-site" };

const PROJECTS: Project[] = [
  { id: "project-site", name: "Site", folder: "/site" },
  { id: "project-api", name: "Api", folder: "/api" },
  { id: "project-empty", name: "Empty", folder: "/empty" },
];

function treeOf(overrides: Partial<HierarchyTree> = {}): HierarchyTree {
  return {
    ceo: null,
    teams: [],
    unassigned: [],
    ...overrides,
  };
}

describe("floorGroups", () => {
  test("12 agents in 3 projects give 3 floors with 12 desks", () => {
    const agents = [
      ...Array.from({ length: 4 }, (_, index) =>
        agent({
          id: `agent-site-${index}`,
          name: `Site ${index}`,
          project_id: "project-site",
        }),
      ),
      ...Array.from({ length: 4 }, (_, index) =>
        agent({
          id: `agent-api-${index}`,
          name: `Api ${index}`,
          project_id: "project-api",
        }),
      ),
      ...Array.from({ length: 4 }, (_, index) =>
        agent({
          id: `agent-no-${index}`,
          name: `No ${index}`,
          project_id: null,
        }),
      ),
    ];
    const floors = floorGroups(agents, PROJECTS, ALL);
    expect(floors.map((floor) => floor.name)).toEqual([
      "Api",
      "Empty",
      "Site",
      NO_PROJECT_FLOOR_NAME,
    ]);
    expect(floors.map((floor) => floor.agentCount)).toEqual([4, 0, 4, 4]);
    expect(floors.reduce((total, floor) => total + floor.agentCount, 0)).toBe(
      12,
    );
    expect(
      floors.flatMap((floor) => floor.groups.flatMap((group) => group.agents)),
    ).toHaveLength(12);
  });

  test("the no-project floor comes last and holds the agents without a project", () => {
    const floors = floorGroups(
      [
        agent({ id: "agent-a", name: "Bo", project_id: "project-site" }),
        agent({ id: "agent-b", name: "Cy", project_id: null }),
      ],
      PROJECTS,
      ALL,
    );
    expect(floors.at(-1)?.name).toBe(NO_PROJECT_FLOOR_NAME);
    expect(floors.at(-1)?.projectId).toBeNull();
    expect(floors.at(-1)?.groups[0].agents.map((desk) => desk.name)).toEqual([
      "Cy",
    ]);
  });

  test("a project without agents keeps its floor with count 0", () => {
    const floors = floorGroups(
      [agent({ project_id: "project-site" })],
      PROJECTS,
      ALL,
    );
    const empty = floors.find((floor) => floor.projectId === "project-empty");
    expect(empty?.name).toBe("Empty");
    expect(empty?.agentCount).toBe(0);
    expect(empty?.groups).toEqual([]);
  });

  test("one project choice shows only that project's floor", () => {
    const floors = floorGroups(
      [agent({ project_id: "project-site" })],
      PROJECTS,
      ONE_SITE,
    );
    expect(floors.map((floor) => floor.name)).toEqual(["Site"]);
  });

  test("100 agents in 10 projects keep a stable sorted order", () => {
    const projects = Array.from({ length: 10 }, (_, index) => ({
      id: `project-${index}`,
      name: `Project ${9 - index}`,
      folder: `/p${index}`,
    }));
    const agents = Array.from({ length: 100 }, (_, index) =>
      agent({
        id: `agent-${index}`,
        name: `Agent ${index}`,
        project_id: projects[index % 10].id,
      }),
    );
    const run = (order: Project[]) =>
      floorGroups(agents, order, ALL).map((floor) => floor.projectId);
    expect(run([...projects].reverse())).toEqual(run(projects));
    expect(run(projects)).toEqual(
      [...projects]
        .sort((first, second) => first.name.localeCompare(second.name))
        .map((project) => project.id),
    );
  });

  test("an agent whose project is missing from the choice sits on the no-project floor", () => {
    const floors = floorGroups(
      [agent({ project_id: "project-gone" })],
      PROJECTS,
      ALL,
    );
    expect(floors.map((floor) => floor.name)).toEqual([
      "Api",
      "Empty",
      "Site",
      NO_PROJECT_FLOOR_NAME,
    ]);
    expect(floors.at(-1)?.groups[0].agents.map((desk) => desk.name)).toEqual([
      "Ada",
    ]);
  });
});

describe("floorGroups with a tree", () => {
  const TREE = treeOf({
    ceo: { id: "agent-ceo", role: "ceo" },
    teams: [
      {
        id: "team-backend",
        name: "Backend",
        project_id: "project-site",
        lead: { id: "agent-lead-backend", role: "lead" },
        members: [
          { id: "agent-backend-1", role: "member" },
          { id: "agent-backend-2", role: "member" },
        ],
      },
      {
        id: "team-frontend",
        name: "Frontend",
        project_id: "project-site",
        lead: { id: "agent-lead-frontend", role: "lead" },
        members: [{ id: "agent-api-1", role: "member" }],
      },
    ],
    unassigned: [],
  });

  /** Ada (CEO) and Bo (Backend lead) work on Site, Dee (Backend member) on Api. */
  const TREE_AGENTS = [
    agent({ id: "agent-ceo", name: "Ada", project_id: "project-site" }),
    agent({
      id: "agent-lead-backend",
      name: "Bo",
      project_id: "project-site",
    }),
    agent({ id: "agent-backend-1", name: "Cy", project_id: "project-site" }),
    agent({ id: "agent-backend-2", name: "Dee", project_id: "project-api" }),
    agent({
      id: "agent-lead-frontend",
      name: "Eli",
      project_id: "project-api",
    }),
    agent({ id: "agent-api-1", name: "Fay", project_id: "project-site" }),
    agent({ id: "agent-plain", name: "Gus", project_id: "project-site" }),
    agent({ id: "agent-none", name: "Hal", project_id: null }),
  ];

  test("a department lists its lead first, then members by name", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const site = floors.find((floor) => floor.projectId === "project-site");
    const backend = site?.groups.find((group) => group.name === "Backend");
    expect(backend?.agents.map((desk) => desk.name)).toEqual(["Bo", "Cy"]);
  });

  test("a cross-floor department leads only on its lead's floor", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const frontend = floors.flatMap((floor) =>
      floor.groups
        .filter((group) => group.name === "Frontend")
        .map((group) => ({
          floor: floor.projectId,
          names: group.agents.map((desk) => desk.name),
        })),
    );
    expect(frontend).toEqual([
      { floor: "project-api", names: ["Eli"] },
      { floor: "project-site", names: ["Fay"] },
    ]);
  });

  test("departments sit after the unnamed group, sorted by team name", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const site = floors.find((floor) => floor.projectId === "project-site");
    expect(site?.groups.map((group) => group.name)).toEqual([
      null,
      "Backend",
      "Frontend",
    ]);
    expect(site?.groups[0].agents.map((desk) => desk.name)).toEqual([
      "Ada",
      "Gus",
    ]);
  });

  test("the CEO sits first in the unnamed group of its floor", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const site = floors.find((floor) => floor.projectId === "project-site");
    const unnamed = site?.groups.find((group) => group.name === null);
    expect(unnamed?.agents.map((desk) => desk.name)).toEqual(["Ada", "Gus"]);
  });

  test("an agent with a project but no team sits in the unnamed group", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const site = floors.find((floor) => floor.projectId === "project-site");
    const unnamed = site?.groups.find((group) => group.name === null);
    expect(unnamed?.agents.map((desk) => desk.name)).toContain("Gus");
  });

  test("an agent without a project sits on the no-project floor outside the teams", () => {
    const floors = floorGroups(TREE_AGENTS, PROJECTS, ALL, TREE);
    const noProject = floors.find(
      (floor) => floor.name === NO_PROJECT_FLOOR_NAME,
    );
    expect(noProject?.groups.map((group) => group.name)).toEqual([null]);
    expect(noProject?.groups[0].agents.map((desk) => desk.name)).toEqual([
      "Hal",
    ]);
  });

  test("an agent absent from the tree sits in the unnamed group of its floor", () => {
    const floors = floorGroups(
      [agent({ id: "agent-out", name: "Ivy", project_id: "project-site" })],
      PROJECTS,
      ALL,
      TREE,
    );
    const site = floors.find((floor) => floor.projectId === "project-site");
    expect(site?.groups.map((group) => group.name)).toEqual([null]);
    expect(site?.groups[0].agents.map((desk) => desk.name)).toEqual(["Ivy"]);
  });

  test("a department appears only on the floors where one of its agents sits", () => {
    const floors = floorGroups(
      [
        agent({ id: "agent-plain", name: "Gus", project_id: "project-site" }),
        agent({ id: "agent-backend-1", name: "Cy", project_id: "project-api" }),
      ],
      PROJECTS,
      ALL,
      TREE,
    );
    expect(
      floors
        .filter((floor) => floor.agentCount > 0)
        .map((floor) => ({
          name: floor.name,
          groups: floor.groups.map((group) => group.name),
        })),
    ).toEqual([
      { name: "Api", groups: ["Backend"] },
      { name: "Site", groups: [null] },
    ]);
  });
});

describe("floorGroups without a tree", () => {
  test("an undefined tree groups by floors only", () => {
    const floors = floorGroups(
      [
        agent({ id: "agent-a", name: "Bo", project_id: "project-site" }),
        agent({ id: "agent-b", name: "Cy", project_id: "project-site" }),
      ],
      PROJECTS,
      ONE_SITE,
    );
    expect(floors).toHaveLength(1);
    expect(floors[0].groups).toHaveLength(1);
    expect(floors[0].groups[0].name).toBeNull();
    expect(floors[0].groups[0].agents.map((desk) => desk.name)).toEqual([
      "Bo",
      "Cy",
    ]);
  });

  test("an empty tree groups by floors only", () => {
    const floors = floorGroups(
      [agent({ project_id: "project-site" })],
      PROJECTS,
      ONE_SITE,
      treeOf(),
    );
    expect(floors[0].groups).toEqual([
      { name: null, agents: [expect.objectContaining({ name: "Ada" })] },
    ]);
  });
});

describe("searchMatches", () => {
  const AGENTS = [
    agent({ id: "agent-a", name: "Ada" }),
    agent({ id: "agent-b", name: "Badu" }),
    agent({ id: "agent-c", name: "Cy" }),
  ];

  test("one agent matches a name", () => {
    expect(searchMatches(AGENTS, "cy").map((desk) => desk.name)).toEqual([
      "Cy",
    ]);
  });

  test("several matches keep the reading order", () => {
    expect(searchMatches(AGENTS, "A").map((desk) => desk.name)).toEqual([
      "Ada",
      "Badu",
    ]);
  });

  test("no match is an empty list", () => {
    expect(searchMatches(AGENTS, "zzz")).toEqual([]);
  });

  test("empty text and spaces-only text match every agent", () => {
    expect(searchMatches(AGENTS, "")).toEqual(AGENTS);
    expect(searchMatches(AGENTS, "   ")).toEqual(AGENTS);
  });
});

describe("desk text", () => {
  test("the status text mirrors shownStatusText", () => {
    expect(deskStatusText(agent({ status: "waiting_for_input" }), false)).toBe(
      "waiting for input",
    );
    expect(deskStatusText(agent({ status: "working" }), false)).toBe("working");
    expect(deskStatusText(agent({ status: "working" }), true)).toBe("Stopping");
    expect(deskStatusText(agent({ status: "mystery" }), false)).toBe("mystery");
  });

  test("the last line is empty before any output", () => {
    expect(deskLastLine(agent({ last_line: "done" }))).toBe("done");
    expect(deskLastLine(agent({ last_line: null }))).toBe("");
  });
});
