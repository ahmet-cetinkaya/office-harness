// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import { EMPTY_TREE, type HierarchyTree } from "../models/hierarchy";
import { createHierarchyService } from "../services/hierarchy-service";

export interface HierarchySnapshot {
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  tree: HierarchyTree;
}

export const INITIAL_HIERARCHY: HierarchySnapshot = {
  phase: "loading",
  failure: null,
  tree: EMPTY_TREE,
};

const REQUEST_FAILED = "The request failed.";

/** The server's tree event: any change to the tree shape or a role (D6). */
const EVENT_TREE_CHANGED = "hierarchy.changed";

/**
 * Owns the tree snapshot. It reads `hierarchy.tree` and reads it again on
 * `hierarchy.changed`, so agents that other clients or agents created show up
 * without a reload (D6).
 */
export function createHierarchyController(client: CoreClient) {
  const service = createHierarchyService(client);
  let snapshot: HierarchySnapshot = INITIAL_HIERARCHY;
  const listeners = new Set<(snapshot: HierarchySnapshot) => void>();

  function publish(changes: Partial<HierarchySnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function load(): Promise<void> {
    try {
      publish({
        phase: "ready",
        failure: null,
        tree: await service.fetchTree(),
      });
    } catch (rejection) {
      publish({ phase: "failed", failure: describe(rejection) });
    }
  }

  function handleEvent(event: ServerEvent): void {
    const isTreeChange =
      event.type === EVENT_TREE_CHANGED || event.type === "client.resync";
    if (isTreeChange) void load();
  }

  return {
    /** The snapshot of the last change. Reading it inside a `$derived` tracks it. */
    get snapshot(): HierarchySnapshot {
      return snapshot;
    },

    /**
     * Calls back on each change, starting with the current snapshot. Returns
     * the stop function, as Svelte effects expect.
     */
    subscribe(listener: (snapshot: HierarchySnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /**
     * Listens for the server's events. Call it once. A core that cannot be
     * reached turns into a failed read, as with the office store.
     */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch (rejection) {
        publish({ phase: "failed", failure: describe(rejection) });
      }
    },

    /** Loads the tree. Call it each time the connection becomes ready. */
    load,
  };
}

export type HierarchyController = ReturnType<typeof createHierarchyController>;
