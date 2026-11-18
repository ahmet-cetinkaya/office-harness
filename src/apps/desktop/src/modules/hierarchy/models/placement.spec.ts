// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { allowedRolesAt, placeOf } from "./placement";
import { EMPTY_TREE, type HierarchyTree } from "./hierarchy";

const CEO = { id: "ceo-1", role: "ceo" as const };
const BACKEND = {
  id: "team-a",
  name: "Backend",
  project_id: null,
  lead: { id: "lead-1", role: "lead" as const },
  members: [{ id: "member-1", role: "member" as const }],
};

function treeWithCeo(): HierarchyTree {
  return { ceo: CEO, teams: [BACKEND], unassigned: [] };
}

describe("placeOf", () => {
  test("no selection asks the office rules", () => {
    expect(placeOf(EMPTY_TREE, null)).toEqual({ kind: "office" });
  });

  test("the CEO row places under the CEO", () => {
    expect(placeOf(treeWithCeo(), "ceo-1")).toEqual({ kind: "under-ceo" });
  });

  test("a lead row places in its team", () => {
    expect(placeOf(treeWithCeo(), "lead-1")).toEqual({
      kind: "under-lead",
      teamId: "team-a",
    });
  });

  test("a member or unknown row asks the office rules", () => {
    expect(placeOf(treeWithCeo(), "member-1")).toEqual({ kind: "office" });
    expect(placeOf(treeWithCeo(), "gone")).toEqual({ kind: "office" });
  });
});

describe("allowedRolesAt", () => {
  test("an empty office offers the CEO", () => {
    expect(allowedRolesAt({ kind: "office" }, EMPTY_TREE)).toEqual(["ceo"]);
  });

  test("an office with a CEO offers nothing from the toolbar", () => {
    expect(allowedRolesAt({ kind: "office" }, treeWithCeo())).toEqual([]);
  });

  test("under the CEO only a team lead may sit", () => {
    expect(allowedRolesAt({ kind: "under-ceo" }, treeWithCeo())).toEqual([
      "lead",
    ]);
  });

  test("under a lead only a member may sit", () => {
    expect(
      allowedRolesAt({ kind: "under-lead", teamId: "team-a" }, treeWithCeo()),
    ).toEqual(["member"]);
  });
});
