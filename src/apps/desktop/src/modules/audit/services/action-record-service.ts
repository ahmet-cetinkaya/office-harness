// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { mergeEntry, type ActionEntry } from "../models/action-entry";

export const ACTIONS_REQUEST = "agent.actions";

export async function listActions(
  client: CoreClient,
  agentId: string,
): Promise<ActionEntry[]> {
  const entries = await client.sendRequest(ACTIONS_REQUEST, {
    agent_id: agentId,
  });
  return Array.isArray(entries) ? (entries as ActionEntry[]) : [];
}

/**
 * What another module needs to show an agent's action record. The other module
 * receives it instead of importing it, so that its own tests do not load the
 * record's view.
 */
export interface ActionRecordSource {
  list(agentId: string): Promise<ActionEntry[]>;
  merge(entries: ActionEntry[], entry: ActionEntry): ActionEntry[];
}

export function createActionRecordSource(
  client: CoreClient,
): ActionRecordSource {
  return {
    list: (agentId) => listActions(client, agentId),
    merge: mergeEntry,
  };
}
