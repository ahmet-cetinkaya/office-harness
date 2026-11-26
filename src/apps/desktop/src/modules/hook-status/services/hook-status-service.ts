// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { HOOK_STATUS_KEY } from "../models/hook-status";

/** The typed face of the hook status setting. */
export function createHookStatusService(client: CoreClient) {
  return {
    read: async (): Promise<string> => {
      const setting = (await client.sendRequest("settings.get", {
        key: HOOK_STATUS_KEY,
      })) as { value?: unknown } | null;
      return typeof setting?.value === "string" ? setting.value : "";
    },
    write: async (value: string): Promise<void> => {
      await client.sendRequest("settings.set", {
        key: HOOK_STATUS_KEY,
        value,
      });
    },
  };
}
