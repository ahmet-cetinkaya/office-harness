// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Page } from "@playwright/test";
import type { ConnectionState } from "../../src/core/api";

export interface MockCoreOptions {
  initialState: ConnectionState;
  /** What "start_server" answers with. When it succeeds the state becomes
   * `stateAfterStart`. */
  startServer?:
    | { outcome: "ok"; stateAfterStart: ConnectionState }
    | { outcome: "fail"; code: string; message: string };
}

export interface RecordedCall {
  command: string;
  args: Record<string, unknown>;
}

declare global {
  interface Window {
    __mockCore: {
      push(state: ConnectionState): void;
      calls: RecordedCall[];
    };
  }
}

/**
 * Stands in for the Tauri IPC the webview talks to, so the real client code
 * runs against a scripted core. The mock never holds a token, as the real
 * webview does not either.
 */
export async function installMockCore(
  page: Page,
  options: MockCoreOptions,
): Promise<void> {
  await page.addInitScript((mock: MockCoreOptions) => {
    const callbacks = new Map<number, (message: unknown) => void>();
    const nextIndex = new Map<number, number>();
    const stateChannels: number[] = [];
    const calls: RecordedCall[] = [];
    let nextCallbackId = 1;
    let current = mock.initialState;

    const send = (channelId: number, message: unknown) => {
      const index = nextIndex.get(channelId) ?? 0;
      nextIndex.set(channelId, index + 1);
      callbacks.get(channelId)?.({ index, message });
    };
    const push = (state: ConnectionState) => {
      current = state;
      stateChannels.forEach((id) => send(id, state));
    };

    const internals = {
      transformCallback(callback: (message: unknown) => void) {
        const id = nextCallbackId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id: number) {
        callbacks.delete(id);
      },
      async invoke(command: string, args: Record<string, unknown> = {}) {
        calls.push({ command, args });
        switch (command) {
          case "watch_connection": {
            const { id } = args.onState as { id: number };
            stateChannels.push(id);
            send(id, current);
            return undefined;
          }
          case "start_server":
            if (mock.startServer?.outcome === "fail") {
              throw {
                code: mock.startServer.code,
                message: mock.startServer.message,
              };
            }
            if (mock.startServer?.outcome === "ok") {
              push(mock.startServer.stateAfterStart);
            }
            return undefined;
          default:
            return undefined;
        }
      },
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: internals,
      __mockCore: { push, calls },
    });
  }, options);
}
