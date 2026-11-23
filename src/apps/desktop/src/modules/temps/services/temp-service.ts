// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { readTempList, type TempList } from "../models/temps";

/** The typed face of the `temp.*` requests (0020). */
export function createTempService(client: CoreClient) {
  return {
    list: async (): Promise<TempList> =>
      readTempList(await client.sendRequest("temp.list")),
    end: async (agentId: string): Promise<void> => {
      await client.sendRequest("temp.end", { agent_id: agentId });
    },
    dismiss: async (entryId: string): Promise<void> => {
      await client.sendRequest("temp.dismiss", { entry_id: entryId });
    },
  };
}
