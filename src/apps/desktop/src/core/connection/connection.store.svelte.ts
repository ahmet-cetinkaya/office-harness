// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../api";
import {
  createConnectionController,
  type ConnectionSnapshot,
} from "./connection-controller";

/** The controller's snapshot as reactive state for the shell. */
export function createConnectionStore(client: CoreClient) {
  let snapshot = $state<ConnectionSnapshot>({
    state: { status: "loading" },
    action: { phase: "idle" },
    coreError: null,
  });
  const controller = createConnectionController(client, (next) => {
    snapshot = next;
  });

  return {
    get snapshot() {
      return snapshot;
    },
    start: controller.start,
    startServer: controller.startServer,
    restartServer: controller.restartServer,
    retry: controller.retry,
  };
}

export type ConnectionStore = ReturnType<typeof createConnectionStore>;
