// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { AgentBody } from "./template";
import { agentRows, bodyGroups } from "./template-text";

const BODY: AgentBody = {
  name_pattern: "reviewer-{n}",
  command: "claude --model opus",
  working_folder: null,
  additions: {
    flags: ["--verbose"],
    env: [{ name: "API_KEY", value: null }],
    startup_prompt: "run \u001b[31mred\u001b[0m",
  },
  role: null,
  own_worktree: false,
};

function rowValue(label: string): string | undefined {
  return agentRows(BODY).find((row) => row.label === label)?.value;
}

describe("template text", () => {
  test("an escape code is shown as a visible symbol", () => {
    expect(rowValue("Startup prompt")).toBe("run ␛[31mred␛[0m");
  });

  test("the rows follow the reading order of the spec", () => {
    expect(agentRows(BODY).map((row) => row.label)).toEqual([
      "Name pattern",
      "Role",
      "Command",
      "Own worktree",
      "Working folder",
      "Flags",
      "Environment variables",
      "Startup prompt",
    ]);
  });

  test("the own worktree choice reads as no or yes", () => {
    expect(rowValue("Own worktree")).toBe("no");
    expect(
      agentRows({ ...BODY, own_worktree: true }).find(
        (row) => row.label === "Own worktree",
      )?.value,
    ).toBe("yes");
  });

  test("a body with no folder reads as the project folder", () => {
    expect(rowValue("Working folder")).toBe("the project folder");
  });

  test("a team lists its lead and each member entry with its count", () => {
    const groups = bodyGroups({
      lead: BODY,
      members: [{ entry: BODY, count: 6 }],
    });
    expect(groups.map((group) => group.title)).toEqual([
      "Lead",
      "Members, entry 1, 6 agents",
    ]);
  });
});
