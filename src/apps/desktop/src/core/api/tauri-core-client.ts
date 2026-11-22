// SPDX-License-Identifier: AGPL-3.0-or-later

import { Channel, invoke } from "@tauri-apps/api/core";
import type { CoreClient, ServerEvent } from "./core-client";
import { toCoreRequestError } from "./core-request-error";
import type { ConnectionState } from "./generated/ConnectionState";

async function call<Result>(
  command: string,
  arguments_: Record<string, unknown> = {},
): Promise<Result> {
  try {
    return await invoke<Result>(command, arguments_);
  } catch (rejection) {
    throw toCoreRequestError(rejection);
  }
}

export function createTauriCoreClient(): CoreClient {
  return {
    watchConnection: (onState) =>
      call("watch_connection", {
        onState: new Channel<ConnectionState>(onState),
      }),
    watchEvents: (onEvent) =>
      call("watch_events", { onEvent: new Channel<ServerEvent>(onEvent) }),
    watchTerminalFrames: (onFrame) =>
      call("watch_terminal_frames", {
        onFrame: new Channel<ArrayBuffer | number[]>((frame) =>
          onFrame(new Uint8Array(frame)),
        ),
      }),
    sendRequest: (kind, fields) => call("send_request", { kind, fields }),
    startServer: () => call("start_server"),
    registerLoginStart: () => call("register_login_start"),
    loginStartStatus: () => call("login_start_status"),
    retryConnection: () => call("retry_connection"),
  };
}

/**
 * Shows the count in the window title (0008 D7). A failed call is dropped:
 * the count also shows as text in the panel, and some desktops show no title.
 */
export async function setWindowTitleCount(count: number): Promise<void> {
  try {
    await call("set_window_title", { count });
  } catch {
    // The count stays visible as text beside Needs you.
  }
}
