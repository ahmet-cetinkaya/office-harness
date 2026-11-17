// SPDX-License-Identifier: AGPL-3.0-or-later

import type { HierarchyTree } from "./hierarchy";

/** The three roles of the company (FR-001). A team lead is created with its
 * team in one request (`team.create`), so the lead role's dialog is the team
 * dialog (FR-002). */
export type RoleKind = "ceo" | "lead" | "member";

/**
 * The place a new agent would take. It comes from the tree row the user has
 * selected: the CEO takes team leads, a team lead takes members, and with no
 * selection the office rules decide (FR-003).
 */
export type CreatePlace =
  | { kind: "office" }
  | { kind: "under-ceo" }
  | { kind: "under-lead"; teamId: string };

/** The place the create dialog starts from, read from the selection. */
export function placeOf(
  tree: HierarchyTree,
  selectedAgentId: string | null,
): CreatePlace {
  if (selectedAgentId === null) return { kind: "office" };
  if (tree.ceo?.id === selectedAgentId) return { kind: "under-ceo" };
  const team = tree.teams.find((team) => team.lead.id === selectedAgentId);
  if (team !== undefined) return { kind: "under-lead", teamId: team.id };
  return { kind: "office" };
}

/** The roles the create dialogs list at a place, in dialog order (FR-003).
 * The office offers the CEO while the top is free; a team lead needs the
 * team flow, so it is offered at the CEO's row and on a team; a member is
 * offered at its lead's row. */
export function allowedRolesAt(
  place: CreatePlace,
  tree: HierarchyTree,
): RoleKind[] {
  if (place.kind === "under-ceo") return ["lead"];
  if (place.kind === "under-lead") return ["member"];
  return tree.ceo === null ? ["ceo"] : [];
}
