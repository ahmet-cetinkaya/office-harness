// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { Agent, AgentTask } from "../models/agent";
import { withTask } from "./office-events";

function agent(id: string, task?: AgentTask | null): Agent {
  return {
    id,
    name: `Agent ${id}`,
    command: "claude",
    working_folder: "/work",
    project_id: null,
    machine_id: "m",
    status: "working",
    created_at: 1,
    last_line: "building",
    task,
  };
}

const TASK: AgentTask = { text: "Fix the build", set_at: 5, source: "user" };

describe("withTask", () => {
  test("patches only the task of the named agent", () => {
    const agents = [agent("a"), agent("b")];

    const patched = withTask(agents, "b", TASK);

    expect(patched[1]).toEqual({ ...agents[1], task: TASK });
    expect(patched[1]?.last_line).toBe("building");
    expect(patched[0]).toBe(agents[0]);
  });

  test("a null task clears the task", () => {
    const patched = withTask([agent("a", TASK)], "a", null);

    expect(patched[0]?.task).toBeNull();
  });

  test("an unknown agent changes nothing and the input is not mutated", () => {
    const agents = [agent("a", TASK)];

    const patched = withTask(agents, "missing", null);

    expect(patched).toEqual(agents);
    expect(agents[0]?.task).toBe(TASK);
  });
});
