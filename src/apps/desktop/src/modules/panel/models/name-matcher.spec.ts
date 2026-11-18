// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { Agent } from "../../agent";
import { matchesAgentName } from "./name-matcher";

function agent(name: string): Agent {
  return {
    id: "a",
    name,
    command: "claude",
    working_folder: "/work",
    project_id: null,
    machine_id: "m",
    status: "working",
    created_at: 1,
    last_line: null,
  };
}

describe("agent name matcher", () => {
  test("a mixed-case query matches the name", () => {
    expect(matchesAgentName(agent("Backend Lead"), "BACKEND")).toBe(true);
  });

  test("a substring of the name matches", () => {
    expect(matchesAgentName(agent("Backend Lead"), "Lead")).toBe(true);
  });

  test("the query's surrounding spaces are ignored", () => {
    expect(matchesAgentName(agent("Backend Lead"), "  lead  ")).toBe(true);
  });

  test("a name without the query does not match", () => {
    expect(matchesAgentName(agent("Backend Lead"), "Frontend")).toBe(false);
  });
});
