// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Agent } from "../../agent";

/**
 * True when the agent's name contains the query, ignoring case and the
 * query's surrounding spaces.
 */
export function matchesAgentName(agent: Agent, query: string): boolean {
  return agent.name.toLowerCase().includes(query.trim().toLowerCase());
}
