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
    sendRequest: (kind, fields) => call("send_request", { kind, fields }),
    startServer: () => call("start_server"),
    retryConnection: () => call("retry_connection"),
  };
}
