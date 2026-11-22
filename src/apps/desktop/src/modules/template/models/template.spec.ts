// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  agentCountOf,
  readImportPreview,
  readTemplates,
  type TeamBody,
} from "./template";

const AGENT_BODY = {
  body_version: 1,
  name_pattern: "reviewer-{n}",
  command: "claude",
  working_folder: null,
  additions: {
    flags: ["--verbose"],
    env: [
      { name: "API_KEY", value: null },
      { name: "MODE", value: "" },
    ],
    startup_prompt: null,
  },
  role: "member",
};

describe("reading templates", () => {
  test("an environment value of null stays null and an empty text stays empty", () => {
    const [template] = readTemplates([
      {
        id: "t1",
        name: "Reviewer",
        kind: "agent",
        body: AGENT_BODY,
        updated_at: 5,
      },
    ]);

    expect(template.kind === "agent" && template.body.additions.env).toEqual([
      { name: "API_KEY", value: null },
      { name: "MODE", value: "" },
    ]);
  });

  test("a body without the own worktree field reads as off, and true reads as on", () => {
    const read = (body: unknown) =>
      readTemplates([
        { id: "t1", name: "R", kind: "agent", body, updated_at: 1 },
      ]);

    const [without] = read(AGENT_BODY);
    const [chosen] = read({ ...AGENT_BODY, own_worktree: true });

    expect(without.kind === "agent" && without.body.own_worktree).toBe(false);
    expect(chosen.kind === "agent" && chosen.body.own_worktree).toBe(true);
  });

  test("an entry that is not a template is dropped", () => {
    expect(readTemplates([{ id: 1 }, null, "x"])).toEqual([]);
    expect(readTemplates(undefined)).toEqual([]);
  });

  test("a preview entry carries the save name", () => {
    const [entry] = readImportPreview([
      {
        name: "Reviewer",
        save_name: "Reviewer (2)",
        kind: "agent",
        body: AGENT_BODY,
      },
    ]);

    expect(entry.save_name).toBe("Reviewer (2)");
    expect(entry.name).toBe("Reviewer");
  });
});

describe("agent count", () => {
  test("an agent body makes one agent", () => {
    const [template] = readTemplates([
      { id: "t1", name: "A", kind: "agent", body: AGENT_BODY, updated_at: 1 },
    ]);
    expect(agentCountOf(template.body)).toBe(1);
  });

  test("a team makes its lead and every counted member", () => {
    const team: TeamBody = {
      lead: readTemplates([
        { id: "t1", name: "A", kind: "agent", body: AGENT_BODY, updated_at: 1 },
      ])[0].body as TeamBody["lead"],
      members: [],
    };
    team.members = [
      { entry: team.lead, count: 6 },
      { entry: team.lead, count: 2 },
    ];
    expect(agentCountOf(team)).toBe(9);
  });
});
