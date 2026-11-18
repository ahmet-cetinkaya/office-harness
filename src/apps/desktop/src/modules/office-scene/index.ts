// SPDX-License-Identifier: AGPL-3.0-or-later

export { default as SceneView } from "./views/scene-view.svelte";
export {
  deskLastLine,
  deskStatusText,
  floorGroups,
  NO_PROJECT_FLOOR_NAME,
  searchMatches,
  type DeskGroup,
  type Floor,
  type ProjectChoice,
} from "./state/floor-groups";
export type {
  HierarchyTree,
  HierarchyTreeAgent,
  HierarchyTreeTeam,
} from "../hierarchy";
