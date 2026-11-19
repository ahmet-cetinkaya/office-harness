// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import type { Hold } from "../models/hold";
import { createHoldController } from "./hold-controller";

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
  };
  return { client, requests, push: (event: ServerEvent) => onEvent(event) };
}

const OFFICE: Hold = { set_by: "user", since: 3 };
const ADA: Hold = { agent_id: "ada", set_by: "user", since: 1 };
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function observe(client: CoreClient) {
  const controller = createHoldController(client, (id) => id.toUpperCase());
  let last = controller.snapshot;
  controller.subscribe((next) => (last = next));
  void controller.watch();
  return { controller, state: () => last };
}

describe("hold controller", () => {
  test("load reads hold.list and announces nothing", async () => {
    const { client, requests } = fakeClient(() => ({
      office: OFFICE,
      agents: [ADA],
    }));
    const { controller, state } = observe(client);

    await controller.load();

    expect(requests.map((sent) => sent.kind)).toEqual(["hold.list"]);
    expect(state().state).toEqual({ office: OFFICE, agents: [ADA] });
    expect(state().announcement).toBeNull();
  });

  test("a hold.changed event replaces the state and announces the change", async () => {
    const { client, push } = fakeClient(() => ({ agents: [] }));
    const { controller, state } = observe(client);
    await controller.load();

    push({ type: "hold.changed", office: OFFICE, agents: [ADA] });

    expect(state().state).toEqual({ office: OFFICE, agents: [ADA] });
    expect(state().announcement).toBe("Office on hold. ADA on hold");

    push({ type: "hold.changed", agents: [] });

    expect(state().state).toEqual({ office: null, agents: [] });
    expect(state().announcement).toBe(
      "Office released from hold. ADA released from hold",
    );
  });

  test("a resync reads the list again", async () => {
    const { client, requests, push } = fakeClient(() => ({ agents: [ADA] }));
    const { state } = observe(client);

    push({ type: "client.resync" });
    await settle();

    expect(requests.map((sent) => sent.kind)).toEqual(["hold.list"]);
    expect(state().state.agents).toEqual([ADA]);
  });

  test("an older list answer never overwrites a newer event", async () => {
    let answer: (value: unknown) => void = () => {};
    const { client, push } = fakeClient(
      () => new Promise((resolve) => (answer = resolve)),
    );
    const { controller, state } = observe(client);
    const reading = controller.load();

    push({ type: "hold.changed", office: OFFICE, agents: [] });
    answer({ agents: [] });
    await reading;

    expect(state().state.office).toEqual(OFFICE);
  });

  test("an unrelated event changes nothing", async () => {
    const { client, push } = fakeClient();
    const { state } = observe(client);

    push({ type: "agent.status", agent_id: "ada", status: "working" });

    expect(state().state).toEqual({ office: null, agents: [] });
  });

  test("the office hold sends no agent id and the agent hold sends one", async () => {
    const { client, requests } = fakeClient();
    const { controller } = observe(client);

    expect(await controller.holdOffice()).toBeNull();
    expect(await controller.releaseOffice()).toBeNull();
    expect(await controller.holdAgent("ada")).toBeNull();
    expect(await controller.releaseAgent("ada")).toBeNull();

    expect(requests).toEqual([
      { kind: "hold.set", fields: undefined },
      { kind: "hold.release", fields: undefined },
      { kind: "hold.set", fields: { agent_id: "ada" } },
      { kind: "hold.release", fields: { agent_id: "ada" } },
    ]);
  });

  test("a conflict shows the server's message", async () => {
    const { client } = fakeClient(() => {
      throw { code: "conflict", message: "the office is already on hold" };
    });
    const { controller } = observe(client);

    expect(await controller.holdOffice()).toBe("the office is already on hold");
  });

  test("the office actions are the answer's entries, or none for a bad answer", async () => {
    const entry = {
      id: 1,
      actor: "office",
      agent_id: null,
      action: "hold",
      result: "ok",
      at: 5,
    };
    const good = observe(fakeClient(() => [entry]).client);
    expect(await good.controller.officeActions()).toEqual([entry]);
    const bad = observe(fakeClient(() => null).client);
    expect(await bad.controller.officeActions()).toEqual([]);
  });

  test("the limits read the stored text and fall back to the defaults", async () => {
    const { client } = fakeClient((_kind, fields) => ({
      key: fields?.key,
      value: fields?.key === "limit_creates_per_agent" ? "7" : "",
    }));
    const { controller } = observe(client);

    expect(await controller.readLimits()).toEqual({
      limit_directives_per_agent: "10",
      limit_directives_per_target: "3",
      limit_creates_per_agent: "7",
      limit_lifecycle_per_agent: "10",
    });
  });

  test("saving the limits writes each key, trimmed, and stops at the first refusal", async () => {
    const written: string[] = [];
    const { client } = fakeClient((kind, fields) => {
      if (kind !== "settings.set") return undefined;
      if (fields?.key === "limit_creates_per_agent") {
        throw {
          code: "invalid",
          message: "a limit is a whole number of at least 1 and at most 1000",
        };
      }
      written.push(`${fields?.key}=${fields?.value}`);
      return undefined;
    });
    const { controller } = observe(client);

    const failure = await controller.saveLimits({
      limit_directives_per_agent: " 12 ",
      limit_directives_per_target: "4",
      limit_creates_per_agent: "0",
      limit_lifecycle_per_agent: "9",
    });

    expect(failure).toBe(
      "a limit is a whole number of at least 1 and at most 1000",
    );
    expect(written).toEqual([
      "limit_directives_per_agent=12",
      "limit_directives_per_target=4",
    ]);
  });
});
