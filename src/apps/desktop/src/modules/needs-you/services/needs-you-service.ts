// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { readItems, type NeedsYouItem } from "../models/needs-you";

/** The typed face of the `needs_you.*` requests (contract). */
export function createNeedsYouService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    list: async (): Promise<NeedsYouItem[]> =>
      readItems(await send("needs_you.list")),
    dismiss: async (agentId: string): Promise<NeedsYouItem[]> =>
      readItems(await send("needs_you.dismiss", { agent_id: agentId })),
  };
}

export type NeedsYouService = ReturnType<typeof createNeedsYouService>;
