// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  CoreRequestError,
  type ConnectionState,
  type CoreClient,
} from "../api";
import {
  createConnectionController,
  type ConnectionSnapshot,
} from "./connection-controller";

function fakeClient(overrides: Partial<CoreClient> = {}): CoreClient & {
  push(state: ConnectionState): void;
} {
  let onState: (state: ConnectionState) => void = () => {};
  return {
    watchConnection: async (callback) => {
      onState = callback;
    },
    watchEvents: async () => {},
    watchTerminalFrames: async () => {},
    sendRequest: async () => null,
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
    ...overrides,
    push: (state) => onState(state),
  };
}

function observe(client: CoreClient) {
  const snapshots: ConnectionSnapshot[] = [];
  const controller = createConnectionController(client, (snapshot) =>
    snapshots.push(snapshot),
  );
  return { controller, snapshots, last: () => snapshots[snapshots.length - 1] };
}

describe("connection controller", () => {
  test("follows the states that the core pushes", async () => {
    const client = fakeClient();
    const { controller, last } = observe(client);

    await controller.start();
    client.push({ status: "not_reachable" });
    expect(last().state).toEqual({ status: "not_reachable" });
    client.push({ status: "connected" });

    expect(last().state).toEqual({ status: "connected" });
  });

  test("reports a core that cannot be reached", async () => {
    const client = fakeClient({
      watchConnection: async () => {
        throw new CoreRequestError("core_unavailable", "no desktop core");
      },
    });
    const { controller, last } = observe(client);

    await controller.start();

    expect(last().coreError).toBe("no desktop core");
  });

  test("start server shows progress and then returns to idle", async () => {
    const { controller, snapshots, last } = observe(fakeClient());

    await controller.startServer();

    expect(snapshots.map((snapshot) => snapshot.action.phase)).toEqual([
      "running",
      "idle",
    ]);
    expect(last().action).toEqual({ phase: "idle" });
  });

  test("start server shows why it failed", async () => {
    const client = fakeClient({
      startServer: async () => {
        throw new CoreRequestError("start_failed", "the program is missing");
      },
    });
    const { controller, last } = observe(client);

    await controller.startServer();

    expect(last().action).toEqual({
      phase: "failed",
      message: "the program is missing",
    });
  });

  test("a failed action is forgotten when the status changes", async () => {
    const client = fakeClient({
      startServer: async () => {
        throw new CoreRequestError("start_failed", "the program is missing");
      },
    });
    const { controller, last } = observe(client);
    await controller.start();
    await controller.startServer();

    client.push({ status: "not_authorized" });

    expect(last().action).toEqual({ phase: "idle" });
  });

  test("restart asks the server to restart", async () => {
    const kinds: string[] = [];
    const client = fakeClient({
      sendRequest: async (kind) => {
        kinds.push(kind);
        return null;
      },
    });
    const { controller, last } = observe(client);

    await controller.restartServer();

    expect(kinds).toEqual(["server.restart"]);
    expect(last().action).toEqual({ phase: "idle" });
  });

  test("restart counts a closed connection as the restart beginning", async () => {
    const client = fakeClient({
      sendRequest: async () => {
        throw new CoreRequestError("not_connected", "closed");
      },
    });
    const { controller, last } = observe(client);

    await controller.restartServer();

    expect(last().action).toEqual({ phase: "idle" });
  });

  test("restart shows a refusal from the server", async () => {
    const client = fakeClient({
      sendRequest: async () => {
        throw new CoreRequestError("refused", "not now");
      },
    });
    const { controller, last } = observe(client);

    await controller.restartServer();

    expect(last().action).toEqual({ phase: "failed", message: "not now" });
  });

  test("retry asks the core to try again at once", async () => {
    let retries = 0;
    const client = fakeClient({
      retryConnection: async () => {
        retries += 1;
      },
    });
    const { controller } = observe(client);

    await controller.retry();

    expect(retries).toBe(1);
  });
});
