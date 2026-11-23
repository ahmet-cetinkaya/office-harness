// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createSweepController } from "./sweep-controller";

function fakeClient(answer: () => unknown) {
  let onEvent: (event: ServerEvent) => void = () => {};
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async (callback) => {
      onEvent = callback;
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind) =>
      kind === "sweep.state" ? answer() : undefined,
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
  };
  return { client, push: (event: ServerEvent) => onEvent(event) };
}

describe("sweep controller", () => {
  test("reads sweep.state on connect and follows the event to the end", async () => {
    const { client, push } = fakeClient(() => ({ state: "running" }));
    const controller = createSweepController(client);
    await controller.watch();
    expect(controller.snapshot.phase).toBe("idle");

    await controller.load();
    expect(controller.snapshot.phase).toBe("running");

    push({ type: "office.sweep", state: "done" });
    expect(controller.snapshot.phase).toBe("finished");
  });

  test("an app that connects after the sweep says nothing", async () => {
    const { client } = fakeClient(() => ({ state: "done" }));
    const controller = createSweepController(client);
    await controller.load();
    expect(controller.snapshot.phase).toBe("idle");
  });

  test("a running event starts a sweep, and each change is published once", async () => {
    const { client, push } = fakeClient(() => ({ state: "done" }));
    const controller = createSweepController(client);
    await controller.watch();
    const phases: string[] = [];
    controller.subscribe((snapshot) => phases.push(snapshot.phase));

    push({ type: "office.sweep", state: "running" });
    push({ type: "office.sweep", state: "running" });
    push({ type: "office.sweep", state: "done" });
    push({ type: "office.sweep", state: "bad" });

    expect(phases).toEqual(["idle", "running", "finished"]);
  });

  test("a read that is older than an event does not overwrite it", async () => {
    let release: (value: unknown) => void = () => {};
    const { client, push } = fakeClient(
      () => new Promise((resolve) => (release = resolve)),
    );
    const controller = createSweepController(client);
    await controller.watch();
    const loading = controller.load();

    push({ type: "office.sweep", state: "running" });
    release({ state: "done" });
    await loading;

    expect(controller.snapshot.phase).toBe("running");
  });

  test("a failed read leaves the state as it was", async () => {
    const { client } = fakeClient(() => {
      throw { code: "unknown_request", message: "no such request" };
    });
    const controller = createSweepController(client);
    await controller.load();
    expect(controller.snapshot.phase).toBe("idle");
  });
});
