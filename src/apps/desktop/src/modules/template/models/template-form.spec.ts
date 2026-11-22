// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  agentBodyOfDraft,
  draftOfAgentBody,
  emptyAgentDraft,
  emptyTeamDraft,
  teamBodyOfDraft,
} from "./template-form";
import type { AgentBody } from "./template";

const BODY: AgentBody = {
  name_pattern: "reviewer-{n}",
  command: "claude",
  working_folder: "/work",
  additions: {
    flags: ["--verbose"],
    env: [
      { name: "API_KEY", value: null },
      { name: "MODE", value: "" },
    ],
    startup_prompt: "go",
  },
  role: "member",
};

describe("template form", () => {
  test("a body round-trips through the editor text without turning no value into text", () => {
    const result = agentBodyOfDraft(draftOfAgentBody(BODY), "body");

    expect(result).toEqual({ ok: true, body: BODY });
  });

  test("an empty folder and an empty prompt mean none", () => {
    const draft = { ...emptyAgentDraft("a-{n}"), command: " claude " };

    const result = agentBodyOfDraft(draft, "body");

    expect(result.ok && result.body.command).toBe("claude");
    expect(result.ok && result.body.working_folder).toBeNull();
    expect(result.ok && result.body.additions.startup_prompt).toBeNull();
  });

  test("an environment line with no name is refused and names where", () => {
    const draft = { ...emptyAgentDraft("a"), environmentText: "=x" };

    const result = agentBodyOfDraft(draft, "lead");

    expect(!result.ok && result.message).toContain("lead:");
  });

  test("a team starts with the defaults of A2", () => {
    const team = emptyTeamDraft();

    expect(team.lead.namePattern).toBe("{team}-lead");
    expect(team.members[0].entry.namePattern).toBe("{team}-member-{n}");
  });

  test("a member count that is not a whole number is refused and names the entry", () => {
    const team = emptyTeamDraft();
    team.members[0].count = "six";

    const result = teamBodyOfDraft(team);

    expect(!result.ok && result.message).toBe(
      "members[0]: the count is not a whole number",
    );
  });

  test("a valid count becomes a number", () => {
    const team = emptyTeamDraft();
    team.members[0].count = " 6 ";

    const result = teamBodyOfDraft(team);

    expect(result.ok && result.body.members[0].count).toBe(6);
  });
});
