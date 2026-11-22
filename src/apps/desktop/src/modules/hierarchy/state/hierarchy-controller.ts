// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  createAgentService,
  KEEP_WORKTREES,
  failedOutcome,
  type DeleteOutcome,
  type NewAgent,
  type WorktreeChoice,
  type WorktreeQuery,
  type WorktreeStatus,
} from "../../agent";
import { EMPTY_TREE, type HierarchyTree } from "../models/hierarchy";
import { placeOf, type CreatePlace } from "../models/placement";
import {
  createHierarchyService,
  type NewTeam,
} from "../services/hierarchy-service";

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

/** A form's refusal, or null when the request went through. */
export type FailureText = string | null;

/**
 * Owns the tree snapshot and the create flows. It reads `hierarchy.tree` and
 * reads it again on `hierarchy.changed`, so agents that other clients or
 * agents created show up without a reload (D6).
 */
export function createHierarchyController(client: CoreClient) {
  const service = createHierarchyService(client);
  const agentService = createAgentService(client);
  let snapshot: HierarchySnapshot = INITIAL_HIERARCHY;
  const listeners = new Set<(snapshot: HierarchySnapshot) => void>();
  // Reloads can overlap when events arrive while a read is still out; the
  // counter tells which read is the newest, and only it may publish.
  let newestRead = 0;

  function publish(changes: Partial<HierarchySnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const tree = await service.fetchTree();
      if (readId !== newestRead) return;
      publish({ phase: "ready", failure: null, tree });
    } catch (rejection) {
      if (readId !== newestRead) return;
      publish({ phase: "failed", failure: describe(rejection) });
    }
  }

  /** Runs one form request, which shows the failure itself (FR-003). */
  async function submit(work: () => Promise<unknown>): Promise<FailureText> {
    try {
      await work();
      return null;
    } catch (rejection) {
      return describe(rejection);
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

    /** Creates the CEO from the create-CEO dialog. */
    createCeo: (input: NewAgent): Promise<FailureText> =>
      submit(() => service.createCeo(input)),

    /** Creates a member under the team lead the dialog names. */
    createMember: (
      input: NewAgent,
      leadAgentId: string,
    ): Promise<FailureText> =>
      submit(() => service.createMember(input, leadAgentId)),

    /** Creates a team with its lead, in one request (FR-002). */
    createTeam: (input: NewTeam): Promise<FailureText> =>
      submit(() => service.createTeam(input)),

    /** The agents that sit below this one (FR-006, FR-007). */
    countDescendants: (agentId: string): Promise<number> =>
      service.countDescendants(agentId),

    /** The server's reason a descendant count read failed, for the
     * confirmation to show instead of a silent zero (FR-006, FR-007). */
    describeCountFailure: (rejection: unknown): string => describe(rejection),

    /** Stops an agent and, when asked, its whole subtree (FR-007). */
    stopSubtree: (
      agentId: string,
      includeDescendants: boolean,
    ): Promise<FailureText> =>
      submit(() => service.stopSubtree(agentId, includeDescendants)),

    /** Deletes an agent and, when asked, its whole subtree (FR-006). The
     * count is what the confirmation showed; the server recounts it. A
     * worktree that changed after the question answers "changed" (0010). */
    deleteSubtree: async (
      agentId: string,
      includeDescendants: boolean,
      descendantCount: number,
      choice?: WorktreeChoice,
    ): Promise<DeleteOutcome> => {
      try {
        const result = await service.deleteSubtree(
          agentId,
          includeDescendants,
          descendantCount,
          choice,
        );
        return { kind: "deleted", result };
      } catch (rejection) {
        return failedOutcome(rejection, choice ?? KEEP_WORKTREES);
      }
    },

    /** The worktree request of a form (0010, FR-001). It rejects, and the
     * form hides the choice. */
    worktreeInfo: (query: WorktreeQuery) => agentService.worktreeInfo(query),

    /** The worktrees among these agents. It rejects when the read fails. */
    worktreeStatus: (agentIds: string[]): Promise<WorktreeStatus[]> =>
      agentService.worktreeStatus(agentIds),

    /** The place a create dialog would act at, from the tree as it is now. */
    placeOf: (selectedAgentId: string | null): CreatePlace =>
      placeOf(snapshot.tree, selectedAgentId),
  };
}

export type HierarchyController = ReturnType<typeof createHierarchyController>;
