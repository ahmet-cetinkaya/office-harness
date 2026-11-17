// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { readHierarchyTree, type HierarchyTree } from "../models/hierarchy";

export const TREE_REQUEST = "hierarchy.tree";

/** The typed face of the `hierarchy.*` requests (contract 0002). */
export function createHierarchyService(client: CoreClient) {
  return {
    fetchTree: async (): Promise<HierarchyTree> =>
      readHierarchyTree(await client.sendRequest(TREE_REQUEST)),
  };
}

export type HierarchyService = ReturnType<typeof createHierarchyService>;
