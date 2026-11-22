// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  readDeleteResult,
  type DeleteResult,
  type NewAgent,
  type WorktreeChoice,
} from "../../agent";
import { readHierarchyTree, type HierarchyTree } from "../models/hierarchy";
import type { RoleKind } from "../models/placement";

export const TREE_REQUEST = "hierarchy.tree";

/** The `role` object of `agent.create` (contract 0002, FR-003). */
export interface RolePlacement {
  kind: RoleKind;
  parent_agent_id?: string;
  team_id?: string;
}

/** The `lead` object of `team.create`: an `agent.create` body plus nothing. */
export interface NewTeamLead {
  name: string;
  command: string;
  working_folder?: string;
}

/** The fields of `team.create` (contract 0002, FR-002). */
export interface NewTeam {
  name: string;
  project_id?: string;
  lead: NewTeamLead;
}

function readTeamAnswer(answer: unknown): string {
  const id = (answer as { id?: unknown })?.id;
  return typeof id === "string" ? id : "";
}

/**
 * The typed face of the `hierarchy.*` and `team.*` requests, and of the
 * `agent.create` places (contract 0002).
 */
export function createHierarchyService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    fetchTree: async (): Promise<HierarchyTree> =>
      readHierarchyTree(await send(TREE_REQUEST)),

    /** Creates the CEO: an agent with `role.kind` "ceo" (FR-001, FR-003). */
    createCeo: async (input: NewAgent) => {
      const role: RolePlacement = { kind: "ceo" };
      return await send("agent.create", { ...input, role });
    },

    /** Creates a member under a team lead; its team is the lead's (A6). */
    createMember: async (input: NewAgent, leadAgentId: string) => {
      const role: RolePlacement = {
        kind: "member",
        parent_agent_id: leadAgentId,
      };
      return await send("agent.create", { ...input, role });
    },

    /** Creates a team with its lead, in one request (FR-002). */
    createTeam: async (input: NewTeam): Promise<string> =>
      readTeamAnswer(await send("team.create", { ...input })),

    /** The agents that sit below this one (FR-006, FR-007). */
    countDescendants: async (agentId: string): Promise<number> => {
      const answer = await send("hierarchy.descendants", {
        agent_id: agentId,
      });
      const count = (answer as { count?: unknown })?.count;
      return typeof count === "number" ? count : 0;
    },

    /** Stops an agent and, when asked, its whole subtree (FR-007). */
    stopSubtree: async (agentId: string, includeDescendants: boolean) => {
      await send("agent.stop", {
        agent_id: agentId,
        include_descendants: includeDescendants,
      });
    },

    /** Deletes an agent and, when asked, its whole subtree; the server
     * recounts `descendant_count` (FR-006). The worktree choice is the one
     * the question gave (0010, FR-005). */
    deleteSubtree: async (
      agentId: string,
      includeDescendants: boolean,
      descendantCount: number,
      choice?: WorktreeChoice,
    ): Promise<DeleteResult> =>
      readDeleteResult(
        await send("agent.delete", {
          agent_id: agentId,
          include_descendants: includeDescendants,
          descendant_count: descendantCount,
          ...choice,
        }),
      ),
  };
}

export type HierarchyService = ReturnType<typeof createHierarchyService>;
