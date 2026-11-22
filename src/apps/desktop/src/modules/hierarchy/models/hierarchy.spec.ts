// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { Agent, Project } from "../../agent";
import {
  buildHierarchy,
  descendantIdsOf,
  EMPTY_TREE,
  flattenVisible,
  readHierarchyTree,
  roleText,
  statusText,
  UNASSIGNED_GROUP_ID,
  type HierarchyTree,
} from "./hierarchy";

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ceo",
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

const CEO = { id: "agent-ceo", role: "ceo" };
const LEAD_A = { id: "agent-lead-a", role: "lead" };
const MEMBER_A1 = { id: "agent-member-a1", role: "member" };

function treeWithCeoAndTeam(): HierarchyTree {
  return {
    ceo: CEO,
    teams: [
      {
        id: "team-a",
        name: "Backend",
        project_id: "project-site",
        lead: LEAD_A,
        members: [MEMBER_A1],
      },
    ],
    unassigned: [{ id: "agent-float", role: null }],
  };
}

const AGENTS = [
  agent(),
  agent({ id: "agent-lead-a", name: "Bo", status: "stopped" }),
  agent({ id: "agent-member-a1", name: "Cy", status: "waiting_for_input" }),
  agent({ id: "agent-float", name: "Dee", status: "exited" }),
];

const PROJECTS: Project[] = [
  { id: "project-site", name: "Site", folder: "/s" },
];

const NO_COLLAPSED = new Set<string>();

describe("status and role text", () => {
  test("known statuses map to words, unknown pass through", () => {
    expect(statusText("waiting_for_input")).toBe("waiting for input");
    expect(statusText("mystery")).toBe("mystery");
  });

  test("roles read as words and a null role says no role", () => {
    expect(roleText("ceo")).toBe("CEO");
    expect(roleText("lead")).toBe("team lead");
    expect(roleText("member")).toBe("member");
    expect(roleText(null)).toBe("no role");
  });
});

describe("readHierarchyTree", () => {
  test("a well-formed answer is read whole", () => {
    const answer = {
      ceo: CEO,
      teams: [
        {
          id: "team-a",
          name: "Backend",
          project_id: "project-site",
          lead: LEAD_A,
          members: [MEMBER_A1],
        },
      ],
      unassigned: [{ id: "agent-float", role: null }],
    };
    expect(readHierarchyTree(answer)).toEqual(treeWithCeoAndTeam());
  });

  test("a malformed answer becomes the empty tree", () => {
    expect(readHierarchyTree(null)).toBe(EMPTY_TREE);
    expect(readHierarchyTree("no")).toBe(EMPTY_TREE);
    expect(readHierarchyTree({ teams: "no", ceo: 3 })).toEqual(EMPTY_TREE);
    expect(
      readHierarchyTree({ teams: [{ id: 4, lead: null, members: 9 }] }),
    ).toEqual(EMPTY_TREE);
  });
});

describe("buildHierarchy", () => {
  test("the CEO row sits at level 1 with the team under it at 2 and 3", () => {
    const [ceo] = buildHierarchy(
      treeWithCeoAndTeam(),
      AGENTS,
      PROJECTS,
      NO_COLLAPSED,
    );
    expect(ceo.name).toBe("Ada, CEO, working");
    expect(ceo.level).toBe(1);
    expect(ceo.expanded).toBe(true);
    expect(ceo.children[0].name).toBe("Team Backend, project Site, 1 member");
    expect(ceo.children[0].level).toBe(2);
    expect(ceo.children[0].children[0].name).toBe("Bo, team lead, stopped");
    expect(ceo.children[0].children[0].children[0].name).toBe(
      "Cy, member, waiting for input",
    );
    expect(ceo.children[0].children[0].level).toBe(3);
    expect(ceo.children[0].children[0].children[0].level).toBe(4);
  });

  test("without a CEO the teams sit at level 1", () => {
    const [team] = buildHierarchy(
      { ceo: null, teams: treeWithCeoAndTeam().teams, unassigned: [] },
      AGENTS,
      [],
      NO_COLLAPSED,
    );
    expect(team.level).toBe(1);
  });

  test("an agent the office list does not know still shows, named as unknown", () => {
    const [ceo] = buildHierarchy(
      { ceo: CEO, teams: [], unassigned: [] },
      [],
      [],
      NO_COLLAPSED,
    );
    expect(ceo.name).toBe("Unknown agent, CEO");
  });

  test("a team with no members says so", () => {
    const [ceo] = buildHierarchy(
      {
        ceo: CEO,
        teams: [
          {
            id: "team-a",
            name: "Backend",
            project_id: null,
            lead: LEAD_A,
            members: [],
          },
        ],
        unassigned: [],
      },
      AGENTS,
      [],
      NO_COLLAPSED,
    );
    expect(ceo.children[0].name).toBe("Team Backend, no members");
  });

  test("the no-role group exists only when agents sit outside the hierarchy", () => {
    const withGroup = buildHierarchy(
      treeWithCeoAndTeam(),
      AGENTS,
      [],
      NO_COLLAPSED,
    );
    const group = withGroup[withGroup.length - 1];
    expect(group.id).toBe(UNASSIGNED_GROUP_ID);
    expect(group.name).toBe("Outside the hierarchy, 1 agent");
    expect(group.children[0].name).toBe("Dee, no role, exited");

    const withoutGroup = buildHierarchy(
      { ceo: CEO, teams: [], unassigned: [] },
      AGENTS,
      [],
      NO_COLLAPSED,
    );
    expect(withoutGroup.map((node) => node.id)).toEqual(["agent-ceo"]);
  });

  test("a collapsed branch keeps its built children and reports itself folded", () => {
    const [ceo] = buildHierarchy(
      treeWithCeoAndTeam(),
      AGENTS,
      [],
      new Set(["agent-ceo"]),
    );
    expect(ceo.expanded).toBe(false);
    expect(ceo.children).toHaveLength(1);
  });
});

describe("flattenVisible", () => {
  const nodes = buildHierarchy(
    treeWithCeoAndTeam(),
    AGENTS,
    PROJECTS,
    NO_COLLAPSED,
  );

  test("an open tree lists parents before children with their branch", () => {
    const rows = flattenVisible(nodes);
    expect(rows.map((row) => row.node.id)).toEqual([
      "agent-ceo",
      "team:team-a",
      "agent-lead-a",
      "agent-member-a1",
      UNASSIGNED_GROUP_ID,
      "agent-float",
    ]);
    expect(rows[2].parentId).toBe("team:team-a");
    expect(rows[0].parentId).toBeNull();
  });

  test("a collapsed branch hides its children but keeps its parent line", () => {
    const collapsed = buildHierarchy(
      treeWithCeoAndTeam(),
      AGENTS,
      PROJECTS,
      new Set(["team:team-a"]),
    );
    const rows = flattenVisible(collapsed);
    expect(rows.map((row) => row.node.id)).toEqual([
      "agent-ceo",
      "team:team-a",
      UNASSIGNED_GROUP_ID,
      "agent-float",
    ]);
  });

  test("the empty office gives no rows", () => {
    expect(
      flattenVisible(buildHierarchy(EMPTY_TREE, [], [], NO_COLLAPSED)),
    ).toEqual([]);
  });
});

describe("descendantIdsOf", () => {
  const tree: HierarchyTree = {
    ceo: { id: "ceo", role: "ceo" },
    teams: [
      {
        id: "t",
        name: "T",
        project_id: null,
        lead: { id: "lead", role: "lead" },
        members: [{ id: "m1", role: "member" }],
      },
    ],
    unassigned: [{ id: "free", role: null }],
  };

  test("the CEO holds every lead and member, a lead its members", () => {
    expect(descendantIdsOf(tree, "ceo")).toEqual(["lead", "m1"]);
    expect(descendantIdsOf(tree, "lead")).toEqual(["m1"]);
    expect(descendantIdsOf(tree, "m1")).toEqual([]);
    expect(descendantIdsOf(tree, "free")).toEqual([]);
  });
});
