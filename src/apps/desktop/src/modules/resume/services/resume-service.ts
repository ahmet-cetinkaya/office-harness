// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { isSweepingPlan, readPlan, type PlanEntry } from "../models/resume";

/** The answer of `resume.plan`: no entries while the sweep runs (0017, D7). */
export interface ResumePlan {
  isSweeping: boolean;
  entries: PlanEntry[];
}

/** The typed face of the `resume.plan` and `resume.start` requests. */
export function createResumeService(client: CoreClient) {
  return {
    plan: async (): Promise<ResumePlan> => {
      const answer = await client.sendRequest("resume.plan");
      return { isSweeping: isSweepingPlan(answer), entries: readPlan(answer) };
    },
    /** Resolves to the ids in start order; results follow as events. */
    start: async (agentIds: string[]): Promise<string[]> => {
      const answer = (await client.sendRequest("resume.start", {
        agent_ids: agentIds,
      })) as { order?: unknown } | null;
      return Array.isArray(answer?.order)
        ? answer.order.filter((id) => typeof id === "string")
        : agentIds;
    },
  };
}
