// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createSleepController } from "./sleep-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function fakeClient(handler: Handler) {
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

async function watched(handler: Handler = () => null) {
  const fake = fakeClient(handler);
  const controller = createSleepController(fake.client);
  await controller.watch();
  return { ...fake, controller };
}

function wakeEvent(result: string, extra: Record<string, unknown> = {}) {
  return {
    type: "wake.result",
    agent_id: "ada",
    result,
    is_final: result !== "checking",
    ...extra,
  } as ServerEvent;
}

describe("sleep controller wake", () => {
  test("it shows Waking while wake.start waits", async () => {
    let answer: (value: unknown) => void = () => {};
    const { controller, requests } = await watched(
      () => new Promise((resolve) => (answer = resolve)),
    );

    const sent = controller.wake("ada");

    expect(controller.snapshot.wakes.ada).toEqual({ kind: "pending" });
    expect(requests).toEqual([
      { kind: "wake.start", fields: { agent_id: "ada" } },
    ]);
    answer({});
    await sent;
    expect(controller.snapshot.wakes.ada).toEqual({ kind: "pending" });
  });

  test("a second wake while one runs sends nothing", async () => {
    const { controller, requests } = await watched(() => new Promise(() => {}));
    void controller.wake("ada");
    await controller.wake("ada");
    expect(requests).toHaveLength(1);
  });

  test("results replace the entry of their agent only", async () => {
    const { controller, push } = await watched();
    push(wakeEvent("checking", { continued: true }));
    expect(controller.snapshot.wakes.ada).toMatchObject({
      kind: "result",
      result: { result: "checking" },
    });
    push(wakeEvent("held", { reason: "ended at once" }));
    expect(controller.snapshot.wakes.ada).toMatchObject({
      result: { result: "held", reason: "ended at once" },
    });
    expect(controller.snapshot.wakes.bo).toBeUndefined();
  });

  test("a result that arrives before the answer is kept", async () => {
    let answer: (value: unknown) => void = () => {};
    const { controller, push } = await watched(
      () => new Promise((resolve) => (answer = resolve)),
    );
    const sent = controller.wake("ada");
    push(wakeEvent("checking", { continued: true }));
    answer({});
    await sent;
    expect(controller.snapshot.wakes.ada).toMatchObject({ kind: "result" });
  });

  test("the conflict answer reads as no longer asleep", async () => {
    const { controller } = await watched(() => {
      throw { code: "conflict", message: "the agent is no longer asleep" };
    });
    await controller.wake("ada");
    expect(controller.snapshot.wakes.ada).toEqual({
      kind: "failure",
      text: "This agent is no longer asleep",
    });
  });

  test("another refusal shows the server's text", async () => {
    const { controller } = await watched(() => {
      throw { code: "refused", message: "the server is stopping" };
    });
    await controller.wake("ada");
    expect(controller.snapshot.wakes.ada).toEqual({
      kind: "failure",
      text: "the server is stopping",
    });
  });
});

describe("sleep controller reset", () => {
  test("the next status change to asleep clears the entry", async () => {
    const { controller, push } = await watched();
    push(wakeEvent("continued"));
    push({ type: "agent.status", agent_id: "ada", status: "working" });
    expect(controller.snapshot.wakes.ada).toBeDefined();

    push({ type: "agent.status", agent_id: "ada", status: "asleep" });

    expect(controller.snapshot.wakes.ada).toBeUndefined();
  });

  test("a new wake replaces the older result", async () => {
    const { controller, push } = await watched();
    push(wakeEvent("held", { reason: "ended at once" }));
    const sent = controller.wake("ada");
    expect(controller.snapshot.wakes.ada).toEqual({ kind: "pending" });
    await sent;
  });
});

describe("sleep controller setting", () => {
  test("it reads off as 30 minutes", async () => {
    const { controller, requests } = await watched(() => ({
      key: "idle_sleep_minutes",
      value: "0",
    }));
    expect(await controller.readForm()).toEqual({ isOn: false, minutes: "30" });
    expect(requests[0]).toEqual({
      kind: "settings.get",
      fields: { key: "idle_sleep_minutes" },
    });
  });

  test("a bad value is refused before anything is sent", async () => {
    const { controller, requests } = await watched();
    const refusal = await controller.saveForm({ isOn: true, minutes: "4" });
    expect(refusal).toContain("from 5 to 1440");
    expect(requests).toEqual([]);
  });

  test("a good value is stored as text, off as 0", async () => {
    const { controller, requests } = await watched();
    expect(await controller.saveForm({ isOn: true, minutes: "30" })).toBeNull();
    expect(
      await controller.saveForm({ isOn: false, minutes: "30" }),
    ).toBeNull();
    expect(requests.map((request) => request.fields)).toEqual([
      { key: "idle_sleep_minutes", value: "30" },
      { key: "idle_sleep_minutes", value: "0" },
    ]);
  });

  test("the server's refusal is returned and nothing else changes", async () => {
    const { controller } = await watched(() => {
      throw { code: "invalid", message: "the server says no" };
    });
    expect(await controller.saveForm({ isOn: true, minutes: "30" })).toBe(
      "the server says no",
    );
  });
});
