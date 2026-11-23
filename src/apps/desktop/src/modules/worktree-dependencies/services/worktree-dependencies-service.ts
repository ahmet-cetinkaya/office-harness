// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { WORKTREE_DEPENDENCIES_KEY } from "../models/worktree-dependencies";

/** The typed face of the worktree dependencies setting. */
export function createWorktreeDependenciesService(client: CoreClient) {
  return {
    read: async (): Promise<string> => {
      const setting = (await client.sendRequest("settings.get", {
        key: WORKTREE_DEPENDENCIES_KEY,
      })) as { value?: unknown } | null;
      return typeof setting?.value === "string" ? setting.value : "";
    },
    write: async (value: string): Promise<void> => {
      await client.sendRequest("settings.set", {
        key: WORKTREE_DEPENDENCIES_KEY,
        value,
      });
    },
  };
}
