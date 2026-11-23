// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createTempsController } from "./temps-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

const EMPTY_LIST = { temps: [], finished: [] };

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

const listCount = (requests: { kind: string }[]) =>
  requests.filter((request) => request.kind === "temp.list").length;

async function opened(handler: Handler = () => EMPTY_LIST) {
  const fake = fakeClient(handler);
  const controller = createTempsController(fake.client);
  await controller.watch();
  await controller.open();
  return { ...fake, controller };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("temps controller list", () => {
  test("open reads the list and shows it", async () => {
    const { controller } = await opened(() => ({
      temps: [{ agent: { id: "a" } }],
      finished: [],
    }));
    expect(controller.snapshot.phase).toBe("ready");
    expect(controller.snapshot.temps).toHaveLength(1);
  });

  test("a refused read shows its text", async () => {
    const { controller } = await opened(() => {
      throw { code: "internal", message: "the list failed" };
    });
    expect(controller.snapshot.phase).toBe("failed");
    expect(controller.snapshot.failure).toBe("the list failed");
  });

  test("temp.changed and agent.changed read the list again", async () => {
    const { requests, push } = await opened();
    expect(listCount(requests)).toBe(1);
    push({ type: "temp.changed" });
    await settle();
    push({ type: "agent.changed", agent: {} });
    await settle();
    push({ type: "agent.status", agent_id: "a", status: "working" });
    await settle();
    expect(listCount(requests)).toBe(3);
  });

  test("events read nothing while the dialog is closed", async () => {
    const { controller, requests, push } = await opened();
    controller.close();
    push({ type: "temp.changed" });
    await settle();
    expect(listCount(requests)).toBe(1);
  });
});

describe("temps controller end and dismiss", () => {
  test("end sends temp.end, reads again, and resolves to null", async () => {
    const { controller, requests } = await opened();
    const failure = await controller.end("a1");
    expect(failure).toBeNull();
    expect(requests.map((request) => request.kind)).toEqual([
      "temp.list",
      "temp.end",
      "temp.list",
    ]);
    expect(requests[1].fields).toEqual({ agent_id: "a1" });
  });

  test("a refused end keeps its text and still reads again", async () => {
    const { controller, requests } = await opened((kind) => {
      if (kind === "temp.end") {
        throw { code: "conflict", message: "the temp already ended" };
      }
      return EMPTY_LIST;
    });
    const failure = await controller.end("a1");
    expect(failure).toBe("the temp already ended");
    expect(controller.snapshot.actionFailure).toBe("the temp already ended");
    expect(listCount(requests)).toBe(2);
  });

  test("dismiss sends temp.dismiss with the entry id", async () => {
    const { controller, requests } = await opened();
    expect(await controller.dismiss("f1")).toBeNull();
    expect(requests[1]).toEqual({
      kind: "temp.dismiss",
      fields: { entry_id: "f1" },
    });
  });

  test("a new action clears the last refusal", async () => {
    let isRefusing = true;
    const { controller } = await opened((kind) => {
      if (kind === "temp.dismiss" && isRefusing) {
        throw { code: "not_found", message: "no such entry" };
      }
      return EMPTY_LIST;
    });
    await controller.dismiss("f1");
    expect(controller.snapshot.actionFailure).toBe("no such entry");
    isRefusing = false;
    await controller.dismiss("f1");
    expect(controller.snapshot.actionFailure).toBeNull();
  });
});
