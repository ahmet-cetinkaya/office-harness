// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import type { NotificationRow } from "../models/notification";
import type { OsNotificationService } from "../services/os-notification";
import { createNotificationController } from "./notification-store";

function row(overrides: Partial<NotificationRow> = {}): NotificationRow {
  return {
    id: "n-1",
    agent_id: "agent-ada",
    agent_name: "Ada",
    event: "exited",
    created_at: 1_000,
    read_at: null,
    ...overrides,
  };
}

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

/** The OS service stand-in: it records what it was asked to show. */
function fakeOs(granted = true) {
  const shown: { agentId: string; title: string; body: string }[] = [];
  let asked = false;
  const os: OsNotificationService = {
    isAllowed: async () => {
      asked = true;
      return granted;
    },
    show: (agentId, title, body) => shown.push({ agentId, title, body }),
    onTap: async () => {},
  };
  return { os, shown, wasAsked: () => asked };
}

function observe(
  client: CoreClient,
  os: OsNotificationService,
  selected: () => string | null = () => null,
  windowFocused: () => boolean = () => false,
  onReadOnArrival: (notificationId: string) => void = () => {},
  onReadOnArrivalFailed: (notificationId: string) => void = () => {},
) {
  const controller = createNotificationController(
    client,
    os,
    selected,
    windowFocused,
    onReadOnArrival,
    onReadOnArrivalFailed,
  );
  let last = controller.snapshot;
  controller.subscribe((next) => (last = next));
  // The event door is the watch, so every test listens through it.
  void controller.watch();
  return { controller, state: () => last };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("notification controller", () => {
  test("the load brings the server's rows (D4)", async () => {
    const { client } = fakeClient(() => [
      row({ id: "a" }),
      row({ id: "b", read_at: 5 }),
    ]);
    const { os } = fakeOs();
    const { controller, state } = observe(client, os);

    await controller.load();

    expect(state().rows.map((entry) => entry.id)).toEqual(["a", "b"]);
    expect(state().announcement).toBeNull();
  });

  test("a created notification joins the front and is announced once", async () => {
    const { client, push } = fakeClient(() => []);
    const { os, shown } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "n9" }) });
    await settle();

    expect(state().rows.map((entry) => entry.id)).toEqual(["n9"]);
    expect(state().announcement).toBe("Ada exited");
    expect(shown).toEqual([
      { agentId: "agent-ada", title: "Ada", body: "Ada exited" },
    ]);
  });

  test("a notification read on arrival is reported, and one that is not is not (0008 D8)", async () => {
    const { client, push } = fakeClient(() => []);
    const { os } = fakeOs();
    const arrived: string[] = [];
    const { controller } = observe(
      client,
      os,
      () => "agent-ada",
      () => true,
      (notificationId) => arrived.push(notificationId),
    );
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "on" }) });
    push({
      type: "notification.created",
      notification: row({ id: "off", agent_id: "agent-bo" }),
    });
    push({
      type: "notification.created",
      notification: row({ id: "hold", event: "auto_hold" }),
    });
    await settle();

    expect(arrived).toEqual(["on"]);
  });

  test("a failed read on arrival reports the notification, so its item shows again", async () => {
    const { client, push } = fakeClient((kind) => {
      if (kind === "notification.read") {
        throw { code: "internal", message: "no answer" };
      }
      return [];
    });
    const { os } = fakeOs();
    const failed: string[] = [];
    const { controller } = observe(
      client,
      os,
      () => "agent-ada",
      () => true,
      () => {},
      (notificationId) => failed.push(notificationId),
    );
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "on" }) });
    await settle();

    expect(failed).toEqual(["on"]);
  });

  test("the announcement clears on the next load, so each one is spoken once", async () => {
    const { client, push } = fakeClient(() => []);
    const { os } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "n9" }) });
    await settle();
    await controller.load();

    expect(state().announcement).toBeNull();
  });

  test("an event for the open and focused agent shows read, with no OS ping (D5)", async () => {
    const { client, push, requests } = fakeClient(() => []);
    const { os, shown, wasAsked } = fakeOs();
    const { controller, state } = observe(
      client,
      os,
      () => "agent-ada",
      () => true,
    );
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "n9" }) });
    await settle();

    expect(state().rows.map((entry) => entry.id)).toEqual(["n9"]);
    expect(state().rows[0].read_at).not.toBeNull();
    expect(state().announcement).toBeNull();
    expect(shown).toEqual([]);
    expect(wasAsked()).toBe(false);
    expect(
      requests.find((sent) => sent.kind === "notification.read")?.fields,
    ).toEqual({ notification_id: "n9" });
  });

  test("an automatic hold stays unread and pings the OS even for the open and focused agent (0007)", async () => {
    const { client, push } = fakeClient(() => []);
    const { os, shown } = fakeOs();
    const { controller, state } = observe(
      client,
      os,
      () => "agent-ada",
      () => true,
    );
    await controller.load();

    push({
      type: "notification.created",
      notification: row({ id: "n9", event: "auto_hold" }),
    });
    await settle();

    expect(state().rows[0].read_at).toBeNull();
    expect(shown).toHaveLength(1);
  });

  test("an automatic hold carries its limit name into the text (0007 D7)", async () => {
    const { client, push } = fakeClient(() => []);
    const { os, shown } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    push({
      type: "notification.created",
      notification: row({
        id: "n9",
        event: "auto_hold",
        detail: "agents created by one agent",
      }),
    });
    await settle();

    expect(state().rows[0].detail).toBe("agents created by one agent");
    expect(state().announcement).toBe(
      'Ada was put on hold after 3 refusals by the limit "agents created by one agent"',
    );
    expect(shown[0].body).toBe(state().announcement ?? "");
  });

  test("an unfocused window still pings the OS for the selected agent (D5)", async () => {
    const { client, push } = fakeClient(() => []);
    const { os, shown } = fakeOs();
    const { controller, state } = observe(
      client,
      os,
      () => "agent-ada",
      () => false,
    );
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "n9" }) });
    await settle();

    expect(state().rows[0].read_at).toBeNull();
    expect(shown).toHaveLength(1);
  });

  test("a denied OS permission stays silent, and the list still works (D7)", async () => {
    const { client, push } = fakeClient(() => []);
    const { os, shown } = fakeOs(false);
    const { controller, state } = observe(client, os);
    await controller.load();

    push({ type: "notification.created", notification: row({ id: "n9" }) });
    await settle();

    expect(state().rows.map((entry) => entry.id)).toEqual(["n9"]);
    expect(shown).toEqual([]);
  });

  test("opening a notification reads it (FR-009)", async () => {
    const { client, requests } = fakeClient(() => [row({ id: "n1" })]);
    const { os } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    await controller.read("n1");

    expect(state().rows[0].read_at).not.toBeNull();
    expect(
      requests.find((sent) => sent.kind === "notification.read")?.fields,
    ).toEqual({ notification_id: "n1" });
  });

  test("opening an agent's terminal reads all of that agent's rows (FR-009)", async () => {
    const { client, requests } = fakeClient(() => [
      row({ id: "n1" }),
      row({ id: "n2", agent_id: "agent-bo" }),
      row({ id: "n3", read_at: 5 }),
    ]);
    const { os } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    await controller.readAgent("agent-ada");

    expect(
      state()
        .rows.filter((entry) => entry.agent_id === "agent-ada")
        .every((entry) => entry.read_at !== null),
    ).toBe(true);
    expect(state().rows.find((entry) => entry.id === "n2")?.read_at).toBeNull();
    expect(
      requests.find((sent) => sent.kind === "notification.read_agent")?.fields,
    ).toEqual({ agent_id: "agent-ada" });
  });

  test("a failed read_agent warns and keeps the panel working", async () => {
    const warnings: unknown[][] = [];
    const originalWarn = console.warn;
    console.warn = (...parts: unknown[]) => warnings.push(parts);
    try {
      const { client } = fakeClient((kind) => {
        if (kind === "notification.read_agent") {
          throw new Error("the server is gone");
        }
        return [];
      });
      const { os } = fakeOs();
      const { controller } = observe(client, os);

      await controller.readAgent("agent-ada");

      expect(warnings).toHaveLength(1);
    } finally {
      console.warn = originalWarn;
    }
  });

  test("a frame this app cannot read changes nothing", async () => {
    const { client, push } = fakeClient(() => []);
    const { os } = fakeOs();
    const { controller, state } = observe(client, os);
    await controller.load();

    push({ type: "notification.created", notification: "junk" });
    push({ type: "notification.created" });
    await settle();

    expect(state().rows).toEqual([]);
  });
});
