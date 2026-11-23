// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createSecretsController } from "./secrets-controller";

const MARKER = "unique-marker-value-0026";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function fakeClient(handler: Handler = () => undefined) {
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

const LIST = {
  store: { state: "ready", custody: "os_keyring" },
  secrets: [
    {
      name: "API_KEY",
      version: 1,
      updated_at: 1,
      unreadable: false,
      grants: [],
    },
  ],
};
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function observe(client: CoreClient) {
  const controller = createSecretsController(client);
  let last = controller.snapshot;
  controller.subscribe((next) => (last = next));
  void controller.watch();
  return { controller, state: () => last };
}

describe("secrets controller", () => {
  test("load reads secret.list and marks the list as loaded", async () => {
    const { client, requests } = fakeClient(() => LIST);
    const { controller, state } = observe(client);

    await controller.load();

    expect(requests.map((sent) => sent.kind)).toEqual(["secret.list"]);
    expect(state().isLoaded).toBe(true);
    expect(state().view.secrets[0].name).toBe("API_KEY");
  });

  test("a secret.changed event and a resync read the list again", async () => {
    const { client, requests, push } = fakeClient(() => LIST);
    observe(client);

    push({ type: "secret.changed", cause: "saved", name: "API_KEY" });
    push({ type: "client.resync" });
    await settle();

    expect(requests.map((sent) => sent.kind)).toEqual([
      "secret.list",
      "secret.list",
    ]);
  });

  test("a failed read tells the person and keeps the list", async () => {
    const { client } = fakeClient((kind) => {
      if (kind === "secret.list") {
        throw { code: "internal", message: "the store cannot be read" };
      }
      return undefined;
    });
    const { controller, state } = observe(client);

    await controller.load();

    expect(state().loadFailure).toBe("the store cannot be read");
    expect(state().isLoaded).toBe(false);
  });

  test("a save with a short value or a bad name sends nothing", async () => {
    const { client, requests } = fakeClient();
    const { controller } = observe(client);

    const short = await controller.save("API_KEY", "short");
    const badName = await controller.save("api key", MARKER);

    expect(short).toContain("at least 8 bytes");
    expect(badName).toContain("The name is");
    expect(requests).toEqual([]);
  });

  test("a save sends the value once and the snapshot never holds it", async () => {
    const { client, requests } = fakeClient(() => LIST);
    const { controller, state } = observe(client);

    const outcome = await controller.save("API_KEY", MARKER);
    await controller.load();

    expect(outcome).toBeNull();
    expect(requests[0]).toEqual({
      kind: "secret.set",
      fields: { name: "API_KEY", value: MARKER },
    });
    expect(JSON.stringify(state())).not.toContain(MARKER);
  });

  test("a refusal of the server comes back as text without the value", async () => {
    const { client } = fakeClient(() => {
      throw {
        code: "refused",
        message: "the secret store is locked: the keyring is locked",
      };
    });
    const { controller } = observe(client);

    const outcome = await controller.save("API_KEY", MARKER);

    expect(outcome).toBe("the secret store is locked: the keyring is locked");
    expect(outcome).not.toContain(MARKER);
  });

  test("grant, revoke, delete, and reset send their fields", async () => {
    const { client, requests } = fakeClient();
    const { controller } = observe(client);

    await controller.grant("API_KEY", "ada");
    await controller.revoke("API_KEY", "ada");
    await controller.remove("API_KEY");
    await controller.reset(3);

    expect(requests).toEqual([
      { kind: "secret.grant", fields: { name: "API_KEY", agent_id: "ada" } },
      { kind: "secret.revoke", fields: { name: "API_KEY", agent_id: "ada" } },
      { kind: "secret.delete", fields: { name: "API_KEY" } },
      { kind: "secret.reset", fields: { confirm_count: 3 } },
    ]);
  });
});
