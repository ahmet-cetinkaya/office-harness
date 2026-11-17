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

/** What a parent role allows under it, for the refusal's allowed list
 * (FR-003). */
const ALLOWED_UNDER: Record<RoleKind, string> = {
  ceo: "team lead",
  lead: "member",
  member: "nothing",
};

/** The refusal text the server sends for a role that cannot sit at a place
 * (the wording of `check_placement`, 0002): it names the roles allowed at
 * the parent (FR-003). The dialogs show it when the tree changed under an
 * open dialog, so the choice is no longer offered. */
export function roleRefusalText(role: RoleKind, parentRole: RoleKind): string {
  return `a ${ROLE_WORDS[role]} cannot sit under a ${ROLE_WORDS[parentRole]}; allowed under a ${ROLE_WORDS[parentRole]}: ${ALLOWED_UNDER[parentRole]}`;
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

/** The question a stop asks, with the counted descendants (FR-007). */
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
        : `The agent's CLI is ended and the session stops. ${below} below ${agentName} keep running; stop them too in the same action.`,
  };
}
