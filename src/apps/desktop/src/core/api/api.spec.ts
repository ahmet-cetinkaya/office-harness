// SPDX-License-Identifier: AGPL-3.0-or-later

import { afterEach, describe, expect, test } from "bun:test";
import {
  CORE_UNAVAILABLE,
  CoreRequestError,
  toCoreRequestError,
} from "./core-request-error";
import { createTauriCoreClient } from "./tauri-core-client";

type Invocation = { command: string; arguments_: Record<string, unknown> };

function installTauri(answer: (invocation: Invocation) => unknown) {
  const invocations: Invocation[] = [];
  const callbacks = new Map<number, (message: unknown) => void>();
  let nextCallbackId = 1;
  const internals = {
    transformCallback(callback: (message: unknown) => void) {
      const id = nextCallbackId++;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback(id: number) {
      callbacks.delete(id);
    },
    async invoke(command: string, arguments_: Record<string, unknown>) {
      const invocation = { command, arguments_ };
      invocations.push(invocation);
      return answer(invocation);
    },
  };
  Object.assign(globalThis, { window: globalThis });
  Object.assign(globalThis, { __TAURI_INTERNALS__: internals });
  return { invocations, callbacks };
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, "__TAURI_INTERNALS__");
});

describe("toCoreRequestError", () => {
  test("keeps the code and message of a Rust error", () => {
    const error = toCoreRequestError({ code: "conflict", message: "taken" });

    expect(error).toBeInstanceOf(CoreRequestError);
    expect(error.code).toBe("conflict");
    expect(error.message).toBe("taken");
  });

  test("wraps text rejections as core_unavailable", () => {
    expect(toCoreRequestError("command not found").code).toBe(CORE_UNAVAILABLE);
    expect(toCoreRequestError(new Error("boom")).message).toBe("boom");
  });
});

describe("createTauriCoreClient", () => {
  test("sends a request with its kind and fields", async () => {
    const { invocations } = installTauri(() => ({ ok: true }));

    const result = await createTauriCoreClient().sendRequest("agent.start", {
      agent_id: "a",
    });

    expect(result).toEqual({ ok: true });
    expect(invocations[0]).toEqual({
      command: "send_request",
      arguments_: { kind: "agent.start", fields: { agent_id: "a" } },
    });
  });

  test("turns a rejected command into a CoreRequestError", async () => {
    installTauri(() => {
      throw {
        code: "not_connected",
        message: "The office server is not connected.",
      };
    });

    const failure = createTauriCoreClient().startServer();

    await expect(failure).rejects.toMatchObject({ code: "not_connected" });
  });

  test("delivers the states that the core pushes to the channel", async () => {
    const { invocations, callbacks } = installTauri(() => undefined);
    const received: unknown[] = [];

    await createTauriCoreClient().watchConnection((state) =>
      received.push(state),
    );
    const channelId = (invocations[0].arguments_.onState as { id: number }).id;
    callbacks.get(channelId)?.({ index: 0, message: { status: "loading" } });
    callbacks.get(channelId)?.({ index: 1, message: { status: "connected" } });

    expect(received).toEqual([{ status: "loading" }, { status: "connected" }]);
  });
});
