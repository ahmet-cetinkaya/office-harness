// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { readSecretsView, type SecretsView } from "../models/secrets";

/** The typed face of the `secret.*` requests of the person (0026). */
export function createSecretsService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    list: async (): Promise<SecretsView> =>
      readSecretsView(await send("secret.list")),
    /** The value goes to the server and nowhere else. */
    save: async (name: string, value: string): Promise<void> => {
      await send("secret.set", { name, value });
    },
    remove: async (name: string): Promise<void> => {
      await send("secret.delete", { name });
    },
    grant: async (name: string, agentId: string): Promise<void> => {
      await send("secret.grant", { name, agent_id: agentId });
    },
    revoke: async (name: string, agentId: string): Promise<void> => {
      await send("secret.revoke", { name, agent_id: agentId });
    },
    reset: async (confirmCount: number): Promise<void> => {
      await send("secret.reset", { confirm_count: confirmCount });
    },
  };
}

export type SecretsService = ReturnType<typeof createSecretsService>;
