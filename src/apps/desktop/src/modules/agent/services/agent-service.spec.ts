// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../../../core/api";
import { createAgentService } from "./agent-service";

function fakeClient(answer: unknown) {
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
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
  };
  return { client, sent };
}

describe("agent service worktree calls", () => {
  test("worktree_info sends the form's folder, project, and name", async () => {
    const { client, sent } = fakeClient({ in_repository: false });

    const info = await createAgentService(client).worktreeInfo({
      folder: "/work",
      name: "Ada",
    });

    expect(info).toEqual({ in_repository: false });
    expect(sent).toEqual([
      {
        kind: "agent.worktree_info",
        fields: { folder: "/work", name: "Ada" },
      },
    ]);
  });

  test("worktree_status sends the agent ids and reads a list", async () => {
    const { client, sent } = fakeClient([{ agent_id: "a" }]);

    const statuses = await createAgentService(client).worktreeStatus(["a"]);

    expect(statuses).toHaveLength(1);
    expect(sent[0]).toEqual({
      kind: "agent.worktree_status",
      fields: { agent_ids: ["a"] },
    });
  });

  test("a plain delete sends no worktree fields and reads the result", async () => {
    const { client, sent } = fakeClient({
      deleted_branches: [],
      kept_branches: ["b"],
      kept_folders: ["/f"],
    });

    const result = await createAgentService(client).deleteAgent("a");

    expect(sent[0].fields).toEqual({ agent_id: "a" });
    expect(result.kept_branches).toEqual(["b"]);
  });

  test("remove sends the choice and the confirmed hashes", async () => {
    const { client, sent } = fakeClient({});

    await createAgentService(client).deleteAgent("a", {
      worktrees: "remove",
      confirmed_status: { a: "h" },
    });

    expect(sent[0].fields).toEqual({
      agent_id: "a",
      worktrees: "remove",
      confirmed_status: { a: "h" },
    });
  });
});
