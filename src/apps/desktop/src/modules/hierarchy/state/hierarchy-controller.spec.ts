// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createHierarchyController } from "./hierarchy-controller";

interface FakeClient extends CoreClient {
  requests: string[];
  sendEvent(event: ServerEvent): void;
  /** The fields of the last request of one type, when it was sent. */
  sentFields(kind: string): Record<string, unknown> | undefined;
  /** Parks the next tree read until releaseTreeRead answers it. */
  holdNextTreeRead(): void;
  /** Answers a parked tree read. */
  releaseTreeRead(answer: unknown): void;
}

/** A core client that records the requests and replays events by hand. The
 * tree answer is a value, or a function of the request type for the tests
 * that stage several answers at once. */
function fakeClient(
  treeAnswer: unknown | ((kind: string) => unknown),
  failTree = false,
): FakeClient {
  const listeners: Array<(event: ServerEvent) => void> = [];
  const requests: string[] = [];
  const sent = new Map<string, Record<string, unknown>>();
  let parkNextRead = false;
  let parkedRead: ((answer: unknown) => void) | null = null;
  const answerOf = (kind: string): unknown =>
    typeof treeAnswer === "function"
      ? (treeAnswer as (kind: string) => unknown)(kind)
      : treeAnswer;
  return {
    requests,
    watchConnection: async () => {},
    watchEvents: async (onEvent) => {
      listeners.push(onEvent);
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind, fields) => {
      requests.push(kind);
      sent.set(kind, fields ?? {});
      if (kind === "hierarchy.tree") {
        if (failTree) {
          throw { code: "internal", message: "the tree read broke" };
        }
        if (parkNextRead) {
          parkNextRead = false;
          return await new Promise((resolve) => (parkedRead = resolve));
        }
      }
      return answerOf(kind);
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
    sentFields: (kind) => sent.get(kind),
    sendEvent: (event) => listeners.forEach((listener) => listener(event)),
    holdNextTreeRead: () => {
      parkNextRead = true;
    },
    releaseTreeRead: (answer) => {
      const resolve = parkedRead;
      parkedRead = null;
      resolve?.(answer);
    },
  };
}

const TREE = {
  ceo: { id: "ceo-1", role: "ceo" },
  teams: [],
  unassigned: [],
};

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe("hierarchy controller", () => {
  test("it reads the tree on load", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);
    await controller.load();
    expect(controller.snapshot.phase).toBe("ready");
    expect(controller.snapshot.tree.ceo).toEqual({ id: "ceo-1", role: "ceo" });
    expect(client.requests).toEqual(["hierarchy.tree"]);
  });

  test("a failed read names the failure and keeps the empty tree", async () => {
    const client = fakeClient(null, true);
    const controller = createHierarchyController(client);
    await controller.load();
    expect(controller.snapshot.phase).toBe("failed");
    expect(controller.snapshot.failure).toBe("the tree read broke");
    expect(controller.snapshot.tree.ceo).toBeNull();
  });

  test("a hierarchy.changed event reads the tree again", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);
    await controller.watch();
    await controller.load();
    expect(client.requests).toEqual(["hierarchy.tree"]);

    client.sendEvent({ type: "hierarchy.changed", cause: "team.created" });
    await settle();

    expect(client.requests).toEqual(["hierarchy.tree", "hierarchy.tree"]);
  });

  test("an unrelated event does not read the tree again", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);
    await controller.watch();
    await controller.load();

    client.sendEvent({
      type: "agent.status",
      agent_id: "ceo-1",
      status: "working",
    });
    await settle();

    expect(client.requests).toEqual(["hierarchy.tree"]);
  });

  test("an older tree reply never overwrites a newer one", async () => {
    const NEW_TREE = { ceo: { id: "ceo-2", role: "ceo" } };
    const client = fakeClient(NEW_TREE);
    client.holdNextTreeRead();
    const controller = createHierarchyController(client);
    await controller.watch();
    void controller.load();

    client.sendEvent({ type: "hierarchy.changed", cause: "team.created" });
    await settle();
    expect(controller.snapshot.tree.ceo).toEqual({ id: "ceo-2", role: "ceo" });

    // The first read answers after the reload, as an overlapped read can.
    client.releaseTreeRead(TREE);
    await settle();
    expect(controller.snapshot.tree.ceo).toEqual({ id: "ceo-2", role: "ceo" });
    expect(client.requests).toEqual(["hierarchy.tree", "hierarchy.tree"]);
  });

  test("subscribe calls back at once and on each change until stopped", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);
    const seen: string[] = [];
    const stop = controller.subscribe((snapshot) => seen.push(snapshot.phase));
    expect(seen).toEqual(["loading"]);
    await controller.load();
    expect(seen).toEqual(["loading", "ready"]);
    stop();
    client.sendEvent({ type: "hierarchy.changed", cause: "team.created" });
    await settle();
    expect(seen).toEqual(["loading", "ready"]);
  });

  test("creating the CEO sends the role object and reports no failure", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);
    await controller.load();

    const failure = await controller.createCeo({
      name: "Ada",
      command: "claude",
    });

    expect(failure).toBeNull();
    const sent = client.sentFields("agent.create");
    expect(sent?.role).toEqual({ kind: "ceo" });
    expect(sent?.name).toBe("Ada");
  });

  test("a refused create names the server's reason (FR-003)", async () => {
    const client = fakeClient((kind: string) => {
      if (kind === "agent.create") {
        throw { code: "refused", message: "the office already has a CEO: Bo" };
      }
      return TREE;
    });
    const controller = createHierarchyController(client);

    const failure = await controller.createCeo({ name: "Ada", command: "n" });

    expect(failure).toBe("the office already has a CEO: Bo");
  });

  test("creating a member carries the lead as its parent (FR-001)", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);

    await controller.createMember({ name: "Cy", command: "n" }, "lead-9");

    expect(client.sentFields("agent.create")?.role).toEqual({
      kind: "member",
      parent_agent_id: "lead-9",
    });
  });

  test("creating a team sends the lead object and answers the team id", async () => {
    const client = fakeClient((kind: string) =>
      kind === "team.create" ? { id: "team-new" } : TREE,
    );
    const controller = createHierarchyController(client);

    const failure = await controller.createTeam({
      name: "Backend",
      lead: { name: "Bo", command: "claude" },
    });

    expect(failure).toBeNull();
    const sent = client.sentFields("team.create");
    expect(sent?.name).toBe("Backend");
    expect(sent?.lead).toEqual({ name: "Bo", command: "claude" });
  });

  test("the descendant count reads the server's count", async () => {
    const client = fakeClient((kind: string) =>
      kind === "hierarchy.descendants" ? { count: 2 } : TREE,
    );
    const controller = createHierarchyController(client);

    expect(await controller.countDescendants("lead-1")).toBe(2);
  });

  test("a subtree delete sends the counted confirmation (FR-006)", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);

    await controller.deleteSubtree("lead-1", true, 2);

    expect(client.sentFields("agent.delete")).toEqual({
      agent_id: "lead-1",
      include_descendants: true,
      descendant_count: 2,
    });
  });

  test("a plain stop sends no descendant fields", async () => {
    const client = fakeClient(TREE);
    const controller = createHierarchyController(client);

    await controller.stopSubtree("lead-1", false);

    expect(client.sentFields("agent.stop")).toEqual({
      agent_id: "lead-1",
      include_descendants: false,
    });
  });

  test("a stale counted delete shows the server's conflict", async () => {
    const client = fakeClient((kind: string) => {
      if (kind === "agent.delete") {
        throw { code: "conflict", message: "the agent has 3 members now" };
      }
      return TREE;
    });
    const controller = createHierarchyController(client);

    const failure = await controller.deleteSubtree("lead-1", true, 2);

    expect(failure).toBe("the agent has 3 members now");
  });

  test("placeOf reads the tree the snapshot holds", async () => {
    const TREE_WITH_TEAM = {
      ceo: { id: "ceo-1", role: "ceo" },
      teams: [
        {
          id: "team-a",
          name: "Backend",
          project_id: null,
          lead: { id: "lead-1", role: "lead" },
          members: [],
        },
      ],
      unassigned: [],
    };
    const client = fakeClient(TREE_WITH_TEAM);
    const controller = createHierarchyController(client);
    await controller.load();

    expect(controller.placeOf("ceo-1")).toEqual({ kind: "under-ceo" });
    expect(controller.placeOf("lead-1")).toEqual({
      kind: "under-lead",
      teamId: "team-a",
    });
    expect(controller.placeOf("member-x")).toEqual({ kind: "office" });
    expect(controller.placeOf(null)).toEqual({ kind: "office" });
  });

  test("a watch on a core that cannot be reached fails the snapshot", async () => {
    const client = fakeClient(TREE);
    client.watchEvents = async () => {
      throw { code: "core_unavailable", message: "no desktop core" };
    };
    const controller = createHierarchyController(client);
    await controller.watch();
    expect(controller.snapshot.phase).toBe("failed");
    expect(controller.snapshot.failure).toBe("no desktop core");
  });
});
