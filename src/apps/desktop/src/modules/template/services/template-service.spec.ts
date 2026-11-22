// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../../../core/api";
import type { AgentBody } from "../models/template";
import { createTemplateService } from "./template-service";

const BODY: AgentBody = {
  name_pattern: "a-{n}",
  command: "claude",
  working_folder: null,
  additions: { flags: [], env: [], startup_prompt: null },
  role: null,
  own_worktree: false,
};

function fakeClient(answer: unknown = {}) {
  const sent: { kind: string; fields?: Record<string, unknown> }[] = [];
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async () => {},
    watchTerminalFrames: async () => {},
    sendRequest: async (kind, fields) => {
      sent.push({ kind, fields });
      return answer;
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "saved",
    importTemplatesFile: async () => null,
  };
  return { client, sent };
}

describe("template service", () => {
  test("a save names the template with template_id, never id, and adds the body version", async () => {
    const { client, sent } = fakeClient({});

    await createTemplateService(client).save({
      id: "t1",
      name: "Reviewer",
      kind: "agent",
      body: BODY,
    });

    expect(sent[0].kind).toBe("template.save");
    expect(sent[0].fields).toEqual({
      template_id: "t1",
      name: "Reviewer",
      kind: "agent",
      body: { body_version: 1, ...BODY },
    });
    expect(sent[0].fields).not.toHaveProperty("id");
  });

  test("a new template sends no template_id", async () => {
    const { client, sent } = fakeClient({});

    await createTemplateService(client).save({
      name: "Reviewer",
      kind: "agent",
      body: BODY,
    });

    expect(sent[0].fields).not.toHaveProperty("template_id");
  });

  test("a direct creation sends the place and the project only when given", async () => {
    const { client, sent } = fakeClient({});
    const service = createTemplateService(client);

    await service.createAgent({
      templateId: "t1",
      place: null,
      projectId: null,
    });
    await service.createAgent({
      templateId: "t1",
      place: { parent_agent_id: "lead", team_id: "team" },
      projectId: "p1",
    });

    expect(sent[0].fields).toEqual({ template_id: "t1" });
    expect(sent[1].fields).toEqual({
      template_id: "t1",
      place: { parent_agent_id: "lead", team_id: "team" },
      project_id: "p1",
    });
  });

  test("a team creation sends the team name and the parent", async () => {
    const { client, sent } = fakeClient({});

    await createTemplateService(client).createTeam({
      templateId: "t2",
      teamName: "api",
      parentAgentId: "ceo",
      projectId: null,
    });

    expect(sent[0]).toEqual({
      kind: "template.create_team",
      fields: { template_id: "t2", team_name: "api", parent_agent_id: "ceo" },
    });
  });

  test("an import preview and confirm send the text", async () => {
    const { client, sent } = fakeClient({ templates: [] });
    const service = createTemplateService(client);

    await service.importPreview("file text");
    await service.importConfirm("file text");

    expect(sent.map((request) => request.kind)).toEqual([
      "template.import.preview",
      "template.import.confirm",
    ]);
    expect(sent[1].fields).toEqual({ text: "file text" });
  });
});
