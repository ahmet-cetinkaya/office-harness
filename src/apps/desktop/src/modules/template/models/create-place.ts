// SPDX-License-Identifier: AGPL-3.0-or-later

import type { HierarchyTree } from "../../hierarchy";
import type { TemplateRole } from "./template";
import type { DirectPlace } from "../services/template-service";

/** One team the create dialog offers as the place of a lead or a member. */
export interface TeamChoice {
  teamId: string;
  name: string;
  leadId: string;
}

export function teamChoices(tree: HierarchyTree): TeamChoice[] {
  return tree.teams.map((team) => ({
    teamId: team.id,
    name: team.name,
    leadId: team.lead.id,
  }));
}

/** True when a direct creation of this role needs a team to go into (D4). */
export function needsPlace(role: TemplateRole | null): boolean {
  return role === "lead" || role === "member";
}

/**
 * The place a direct creation sends. A member sits under its team's lead and
 * a lead under the CEO; the server checks the rest with the 0002 rules and
 * refuses with their text. A role without a place sends none (D4).
 */
export function directPlaceOf(
  role: TemplateRole | null,
  tree: HierarchyTree,
  teamId: string,
): DirectPlace | null {
  if (!needsPlace(role)) return null;
  const team = tree.teams.find((known) => known.id === teamId);
  const parentId = role === "member" ? team?.lead.id : tree.ceo?.id;
  if (team === undefined || parentId === undefined) return null;
  return { parent_agent_id: parentId, team_id: team.id };
}
