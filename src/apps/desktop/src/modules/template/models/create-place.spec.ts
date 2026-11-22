// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { HierarchyTree } from "../../hierarchy";
import { directPlaceOf, needsPlace } from "./create-place";

const TREE: HierarchyTree = {
  ceo: { id: "ceo", role: "ceo" },
  teams: [
    {
      id: "team-1",
      name: "api",
      project_id: null,
      lead: { id: "lead-1", role: "lead" },
      members: [],
    },
  ],
  unassigned: [],
};

describe("create place", () => {
  test("only a lead and a member need a place", () => {
    expect(needsPlace("lead")).toBe(true);
    expect(needsPlace("member")).toBe(true);
    expect(needsPlace("ceo")).toBe(false);
    expect(needsPlace(null)).toBe(false);
  });

  test("a member goes under its team's lead", () => {
    expect(directPlaceOf("member", TREE, "team-1")).toEqual({
      parent_agent_id: "lead-1",
      team_id: "team-1",
    });
  });

  test("a lead goes under the CEO", () => {
    expect(directPlaceOf("lead", TREE, "team-1")).toEqual({
      parent_agent_id: "ceo",
      team_id: "team-1",
    });
  });

  test("a CEO and a template with no role send no place", () => {
    expect(directPlaceOf("ceo", TREE, "team-1")).toBeNull();
    expect(directPlaceOf(null, TREE, "team-1")).toBeNull();
  });

  test("an unknown team sends no place", () => {
    expect(directPlaceOf("member", TREE, "gone")).toBeNull();
  });
});
