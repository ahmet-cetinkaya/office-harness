// SPDX-License-Identifier: AGPL-3.0-or-later

import type { RoleKind } from "./placement";

/** The words a role reads as, in dialogs and refusals (FR-001, FR-003). */
export const ROLE_WORDS: Record<RoleKind, string> = {
  ceo: "CEO",
  lead: "team lead",
  member: "member",
};

/** A role as text. */
export function roleKindText(role: RoleKind): string {
  return ROLE_WORDS[role];
}

/** A count as spoken text: "1 agent", "3 agents". */
export function countAgentsText(count: number): string {
  return count === 1 ? "1 agent" : `${count} agents`;
}

/** The question a delete asks, with the counted descendants (FR-006). */
export function deleteQuestionText(
  agentName: string,
  descendantCount: number,
): { title: string; body: string } {
  const below = countAgentsText(descendantCount);
  const inAll = countAgentsText(descendantCount + 1);
  return {
    title: `Delete ${agentName}?`,
    body:
      descendantCount === 0
        ? `This deletes ${agentName}. The action record keeps the entries.`
        : `This deletes ${agentName} and ${below} below ${agentName}, ${inAll} in all. The action record keeps the entries.`,
  };
}

/** The question a stop asks, with the counted descendants (FR-007). When the
 * agent has descendants the answer is a choice: stop only this agent, or
 * stop it with the agents below it. Each body says what happens, not which
 * wire field carries it. */
export function stopQuestionText(
  agentName: string,
  descendantCount: number,
): { title: string; body: string } {
  const below = countAgentsText(descendantCount);
  return {
    title: `Stop ${agentName}?`,
    body:
      descendantCount === 0
        ? "The agent's CLI is ended and the session stops."
        : `Only ${agentName} stops, and ${below} below ${agentName} keep running. To stop them too in the same action, choose stopping them with it.`,
  };
}
