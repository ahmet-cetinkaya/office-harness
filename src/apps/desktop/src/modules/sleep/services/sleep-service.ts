// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { IDLE_SLEEP_KEY } from "../models/sleep";

/** The typed face of the idle sleep setting and the `wake.start` request. */
export function createSleepService(client: CoreClient) {
  return {
    readIdleSleep: async (): Promise<string> => {
      const setting = (await client.sendRequest("settings.get", {
        key: IDLE_SLEEP_KEY,
      })) as { value?: unknown } | null;
      return typeof setting?.value === "string" ? setting.value : "";
    },
    writeIdleSleep: async (value: string): Promise<void> => {
      await client.sendRequest("settings.set", { key: IDLE_SLEEP_KEY, value });
    },
    /** Resolves when the wake began; the results follow as events. */
    wake: async (agentId: string): Promise<void> => {
      await client.sendRequest("wake.start", { agent_id: agentId });
    },
  };
}
