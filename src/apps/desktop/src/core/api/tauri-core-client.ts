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

/**
 * One `watch_events` channel for every listener (0008 D8): separate channels
 * have no order between them, and the Needs you rule needs the notification
 * event to reach its listener before the needs_you event. The channel opens
 * on the first call and each event goes to the listeners in arrival order.
 */
function createEventFanOut() {
  const listeners = new Set<(event: ServerEvent) => void>();
  let opening: Promise<void> | null = null;

  function open(): Promise<void> {
    return call("watch_events", {
      onEvent: new Channel<ServerEvent>((event) => {
        for (const listener of listeners) listener(event);
      }),
    });
  }

  return (onEvent: (event: ServerEvent) => void): Promise<void> => {
    listeners.add(onEvent);
    opening ??= open();
    // A failed open is retried by the next call, and its listener leaves.
    return opening.catch((failure) => {
      listeners.delete(onEvent);
      opening = null;
      throw failure;
    });
  };
}

export function createTauriCoreClient(): CoreClient {
  return {
    watchConnection: (onState) =>
      call("watch_connection", {
        onState: new Channel<ConnectionState>(onState),
      }),
    watchEvents: createEventFanOut(),
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
    exportTemplatesFile: (contents) =>
      call("export_templates_file", { contents }),
    importTemplatesFile: () => call("import_templates_file"),
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
