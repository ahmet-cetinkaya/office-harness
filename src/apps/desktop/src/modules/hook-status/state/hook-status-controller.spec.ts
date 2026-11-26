// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../../../core/api";
import { createHookStatusController } from "./hook-status-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function fakeClient(handler: Handler) {
  const requests: { kind: string; fields?: Record<string, unknown> }[] = [];
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async () => {},
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
  return { client, requests };
}

describe("hook status controller", () => {
  test("it reads the setting, on by default", async () => {
    const { client, requests } = fakeClient(() => ({ value: "on" }));
    const controller = createHookStatusController(client);
    expect(await controller.readIsOn()).toBe(true);
    expect(requests).toEqual([
      { kind: "settings.get", fields: { key: "hook_status" } },
    ]);
  });

  test("off reads as off", async () => {
    const { client } = fakeClient(() => ({ value: "off" }));
    expect(await createHookStatusController(client).readIsOn()).toBe(false);
  });

  test("it stores the switch as on or off", async () => {
    const { client, requests } = fakeClient(() => null);
    const controller = createHookStatusController(client);
    expect(await controller.save(false)).toBeNull();
    expect(await controller.save(true)).toBeNull();
    expect(requests.map((request) => request.fields)).toEqual([
      { key: "hook_status", value: "off" },
      { key: "hook_status", value: "on" },
    ]);
  });

  test("the server's refusal is returned", async () => {
    const { client } = fakeClient(() => {
      throw { code: "invalid", message: "the server says no" };
    });
    expect(await createHookStatusController(client).save(true)).toBe(
      "the server says no",
    );
  });
});
