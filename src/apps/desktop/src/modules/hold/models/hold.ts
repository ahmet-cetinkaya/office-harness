// SPDX-License-Identifier: AGPL-3.0-or-later

/** One hold of the contract: the office's, or one agent's (hold.list). */
export interface Hold {
  /** Absent for the office hold. */
  agent_id?: string | null;
  set_by: "user" | "office";
  reason?: string | null;
  /** Unix time in milliseconds. */
  since: number;
}

/** The full hold state, as `hold.list` and `hold.changed` carry it. */
export interface HoldState {
  office: Hold | null;
  agents: Hold[];
}

export const NO_HOLDS: HoldState = { office: null, agents: [] };

export const OFFICE_ON_HOLD_TEXT = "Office on hold";
export const AGENT_ON_HOLD_TEXT = "on hold";
const OFFICE_RELEASED_TEXT = "Office released from hold";
const AUTOMATIC_HOLD_TEXT = "Held automatically";

function isHold(value: unknown): value is Hold {
  if (typeof value !== "object" || value === null) return false;
  const fields = value as Record<string, unknown>;
  return typeof fields.set_by === "string" && typeof fields.since === "number";
}

/** The state of a `hold.list` answer or a `hold.changed` event; bad input is no hold. */
export function readHoldState(value: unknown): HoldState {
  if (typeof value !== "object" || value === null) return NO_HOLDS;
  const fields = value as Record<string, unknown>;
  return {
    office: isHold(fields.office) ? fields.office : null,
    agents: Array.isArray(fields.agents) ? fields.agents.filter(isHold) : [],
  };
}

export function holdOfAgent(state: HoldState, agentId: string): Hold | null {
  return state.agents.find((hold) => hold.agent_id === agentId) ?? null;
}

/** What an automatic hold adds to "on hold": that the office set it, and why. */
export function automaticHoldText(hold: Hold | null): string | null {
  if (hold === null || hold.set_by !== "office") return null;
  return hold.reason
    ? `${AUTOMATIC_HOLD_TEXT}: ${hold.reason}`
    : AUTOMATIC_HOLD_TEXT;
}

function agentIdsOf(state: HoldState): Set<string> {
  return new Set(state.agents.map((hold) => hold.agent_id ?? ""));
}

/**
 * What the live region says about the change from one state to the next, or
 * null when no hold or release happened. `nameOf` gives an agent's name, or
 * null for an agent the app does not know; that agent is not announced.
 */
export function announcementOfChange(
  before: HoldState,
  after: HoldState,
  nameOf: (agentId: string) => string | null,
): string | null {
  const sentences: string[] = [];
  if (before.office === null && after.office !== null) {
    sentences.push(OFFICE_ON_HOLD_TEXT);
  }
  if (before.office !== null && after.office === null) {
    sentences.push(OFFICE_RELEASED_TEXT);
  }
  const heldBefore = agentIdsOf(before);
  const heldAfter = agentIdsOf(after);
  for (const agentId of heldAfter) {
    const name = nameOf(agentId);
    if (name !== null && !heldBefore.has(agentId)) {
      sentences.push(`${name} ${AGENT_ON_HOLD_TEXT}`);
    }
  }
  // A hold that the office pruned with its deleted agent has no name to say.
  for (const agentId of heldBefore) {
    const name = nameOf(agentId);
    if (name !== null && !heldAfter.has(agentId)) {
      sentences.push(`${name} released from hold`);
    }
  }
  return sentences.length === 0 ? null : sentences.join(". ");
}
