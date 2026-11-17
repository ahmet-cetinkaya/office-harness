// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createHierarchyController } from "./hierarchy-controller";

interface FakeClient extends CoreClient {
  requests: string[];
  sendEvent(event: ServerEvent): void;
  /** Parks the next tree read until releaseTreeRead answers it. */
  holdNextTreeRead(): void;
  /** Answers a parked tree read. */
  releaseTreeRead(answer: unknown): void;
}

/** A core client that records the requests and replays events by hand. */
function fakeClient(treeAnswer: unknown, failTree = false): FakeClient {
  const listeners: Array<(event: ServerEvent) => void> = [];
  const requests: string[] = [];
  let parkNextRead = false;
  let parkedRead: ((answer: unknown) => void) | null = null;
  return {
    requests,
    watchConnection: async () => {},
    watchEvents: async (onEvent) => {
      listeners.push(onEvent);
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind) => {
      requests.push(kind);
      if (kind === "hierarchy.tree") {
        if (failTree) {
          throw { code: "internal", message: "the tree read broke" };
        }
        if (parkNextRead) {
          parkNextRead = false;
          return await new Promise((resolve) => (parkedRead = resolve));
        }
      }
      return treeAnswer;
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
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
