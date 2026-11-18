// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import type { NotificationRow } from "../models/notification";

function asList(result: unknown): NotificationRow[] {
  return Array.isArray(result) ? (result as NotificationRow[]) : [];
}

/** The typed face of the `notification.*` requests (contract). */
export function createNotificationService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    list: async () => asList(await send("notification.list")),
    read: async (notificationId: string) => {
      await send("notification.read", { notification_id: notificationId });
    },
    readAgent: async (agentId: string) => {
      await send("notification.read_agent", { agent_id: agentId });
    },
  };
}

export type NotificationService = ReturnType<typeof createNotificationService>;
