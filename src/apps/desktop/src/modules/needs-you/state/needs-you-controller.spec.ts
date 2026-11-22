// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, setSystemTime, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import type { NeedsYouItem } from "../models/needs-you";
import { createNeedsYouController } from "./needs-you-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function fakeClient(handler: Handler = () => ({ items: [] })) {
  const requests: { kind: string; fields?: Record<string, unknown> }[] = [];
  let onEvent: (event: ServerEvent) => void = () => {};
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async (callback) => {
      onEvent = callback;
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind, fields) => {
      requests.push({ kind, fields });
      return handler(kind, fields);
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
  };
  return { client, requests, push: (event: ServerEvent) => onEvent(event) };
}

function item(agentId: string, origin: string | null = null): NeedsYouItem {
  return {
    agent_id: agentId,
    kind: "bell",
    detail: null,
    first_event_at: 1_000,
    latest_at: 1_000,
    origin_notification_id: origin,
  };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const allKnown = () => true;

function observe(client: CoreClient) {
  const controller = createNeedsYouController(client);
  let last = controller.snapshot;
  controller.subscribe((next) => (last = next));
  void controller.watch();
  return { controller, state: () => last };
}

describe("needs-you controller", () => {
  test("load reads needs_you.list in the server's order and announces nothing", async () => {
    const { client, requests } = fakeClient(() => ({
      items: [item("b"), item("a")],
    }));
    const { controller, state } = observe(client);

    await controller.load();

    expect(requests.map((sent) => sent.kind)).toEqual(["needs_you.list"]);
    expect(state().items.map((entry) => entry.agent_id)).toEqual(["b", "a"]);
    expect(state().announcement).toBeNull();
  });

  test("a changed event replaces the list and announces the count change once", async () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    await controller.load();

    push({ type: "needs_you.changed", items: [item("a")] });
    expect(state().announcement).toBe("Needs you: 1 agent");

    push({ type: "needs_you.changed", items: [item("a")] });
    expect(state().announcement).toBe("Needs you: 1 agent");
    expect(state().items).toHaveLength(1);
  });

  test("an item whose origin was read on arrival never shows or counts", async () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    await controller.load();

    controller.hideOrigin("n1");
    push({ type: "needs_you.changed", items: [item("a", "n1")] });

    expect(state().items).toEqual([]);
    expect(state().announcement).toBeNull();

    // The server removed the item on the read; the hide is forgotten.
    push({ type: "needs_you.changed", items: [] });
    push({ type: "needs_you.changed", items: [item("a", "n1")] });

    expect(state().items).toHaveLength(1);
  });

  test("a coalesced item with no origin always shows", async () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    controller.hideOrigin("n1");

    push({ type: "needs_you.changed", items: [item("a", null)] });

    expect(state().items).toHaveLength(1);
  });

  test("the top press takes the first item at once, so a second press takes the next", async () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    await controller.load();
    push({ type: "needs_you.changed", items: [item("a"), item("b")] });

    expect(controller.takeTop(allKnown)).toBe("a");
    expect(controller.takeTop(allKnown)).toBe("b");

    expect(state().items).toEqual([]);
    expect(controller.takeTop(allKnown)).toBeNull();
    expect(state().announcement).toBe("No agent needs you");
  });

  test("the top press skips an agent that no longer exists", async () => {
    const { client, push } = fakeClient();
    const { controller } = observe(client);
    await controller.load();
    push({ type: "needs_you.changed", items: [item("gone"), item("b")] });

    expect(controller.takeTop((agentId) => agentId !== "gone")).toBe("b");
  });

  test("a dismiss removes the item at once and sends the agent id", async () => {
    const { client, requests, push } = fakeClient();
    const { controller, state } = observe(client);
    await controller.load();
    push({ type: "needs_you.changed", items: [item("a"), item("b")] });

    const failure = await controller.dismiss("a");

    expect(failure).toBeNull();
    expect(state().items.map((entry) => entry.agent_id)).toEqual(["b"]);
    expect(requests.at(-1)).toEqual({
      kind: "needs_you.dismiss",
      fields: { agent_id: "a" },
    });
  });

  test("a refused dismiss reads the list again and returns the refusal", async () => {
    const list = { items: [item("a")] };
    const { client } = fakeClient((kind) => {
      if (kind === "needs_you.dismiss") {
        throw { code: "internal", message: "the item could not be removed" };
      }
      return list;
    });
    const { controller, state } = observe(client);
    await controller.load();

    const failure = await controller.dismiss("a");

    expect(failure).toBe("the item could not be removed");
    expect(state().items).toHaveLength(1);
  });

  test("a dismiss of an item that is already gone is no failure", async () => {
    const { client } = fakeClient((kind) => {
      if (kind === "needs_you.dismiss") {
        throw { code: "not_found", message: "no needs-you item" };
      }
      return { items: [] };
    });
    const { controller } = observe(client);

    expect(await controller.dismiss("a")).toBeNull();
  });

  test("the tick moves the clock and keeps the announcement", async () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    push({ type: "needs_you.changed", items: [item("a")] });
    const before = state().now;
    const stop = controller.startTicking(5);

    await new Promise((resolve) => setTimeout(resolve, 30));
    stop();

    expect(state().now).toBeGreaterThan(before);
    expect(state().announcement).toBe("Needs you: 1 agent");
  });

  test("a resync reads the list again", async () => {
    const { client, requests, push } = fakeClient();
    observe(client);

    push({ type: "client.resync" });
    await settle();

    expect(requests.map((sent) => sent.kind)).toEqual(["needs_you.list"]);
  });

  test("a resync from three items to one speaks no stale count", async () => {
    const { client, push } = fakeClient(() => ({ items: [item("a")] }));
    const { controller, state } = observe(client);
    push({
      type: "needs_you.changed",
      items: [item("a"), item("b"), item("c")],
    });
    expect(state().announcement).toBe("Needs you: 3 agents");

    await controller.load();

    expect(state().items).toHaveLength(1);
    expect(state().announcement).toBeNull();
  });

  test("an item that the origin hide covered shows again when the read failed", () => {
    const { client, push } = fakeClient();
    const { controller, state } = observe(client);
    controller.hideOrigin("n1");
    push({ type: "needs_you.changed", items: [item("a", "n1")] });
    expect(state().items).toEqual([]);

    controller.showOrigin("n1");

    expect(state().items).toHaveLength(1);
  });

  describe("a taken item", () => {
    test("stays out when a list that still has it arrives before the server handled the read", () => {
      const { client, push } = fakeClient();
      const { controller, state } = observe(client);
      push({ type: "needs_you.changed", items: [item("a"), item("b")] });

      controller.takeTop(allKnown);
      push({ type: "needs_you.changed", items: [item("a"), item("b")] });

      expect(state().items.map((entry) => entry.agent_id)).toEqual(["b"]);
      expect(controller.takeTop(allKnown)).toBe("b");
    });

    test("comes back as a new item once a server list had dropped it", () => {
      const { client, push } = fakeClient();
      const { controller, state } = observe(client);
      push({ type: "needs_you.changed", items: [item("a")] });
      controller.takeTop(allKnown);

      push({ type: "needs_you.changed", items: [] });
      push({ type: "needs_you.changed", items: [item("a")] });

      expect(state().items).toHaveLength(1);
    });

    test("shows again after the timeout when the server never dropped it", () => {
      const { client, push } = fakeClient();
      const { controller, state } = observe(client);
      push({ type: "needs_you.changed", items: [item("a")] });
      controller.takeTop(allKnown);
      push({ type: "needs_you.changed", items: [item("a")] });
      const stop = controller.startTicking(5);

      setSystemTime(Date.now() + 60_000);
      return new Promise<void>((resolve) =>
        setTimeout(() => {
          stop();
          setSystemTime();
          expect(state().items).toHaveLength(1);
          resolve();
        }, 30),
      );
    });
  });
});
