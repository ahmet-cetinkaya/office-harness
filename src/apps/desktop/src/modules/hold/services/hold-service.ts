// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import type { ActionEntry } from "../../audit";
import { readHoldState, type HoldState } from "../models/hold";

/** The typed face of the `hold.*` and `office.actions` requests, and of the limit settings. */
export function createHoldService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);
  // An absent agent id means the office (contract: hold.set).
  const target = (agentId?: string) =>
    agentId === undefined ? undefined : { agent_id: agentId };

  return {
    list: async (): Promise<HoldState> =>
      readHoldState(await send("hold.list")),
    set: async (agentId?: string): Promise<void> => {
      await send("hold.set", target(agentId));
    },
    release: async (agentId?: string): Promise<void> => {
      await send("hold.release", target(agentId));
    },
    officeActions: async (): Promise<ActionEntry[]> => {
      const entries = await send("office.actions");
      return Array.isArray(entries) ? (entries as ActionEntry[]) : [];
    },
    readSetting: async (key: string): Promise<string> => {
      const setting = (await send("settings.get", { key })) as {
        value?: unknown;
      } | null;
      return typeof setting?.value === "string" ? setting.value : "";
    },
    writeSetting: async (key: string, value: string): Promise<void> => {
      await send("settings.set", { key, value });
    },
  };
}

export type HoldService = ReturnType<typeof createHoldService>;
