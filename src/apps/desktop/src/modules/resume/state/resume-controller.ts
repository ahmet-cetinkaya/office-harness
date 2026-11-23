// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  confirmBlock,
  readResult,
  type PlanEntry,
  type ResumeResult,
} from "../models/resume";
import { createResumeService } from "../services/resume-service";

/**
 * Where the dialog is: reading the plan, waiting for the sweep that must end
 * first (0017), choosing, or showing the results.
 */
export type ResumePhase =
  "loading" | "sweeping" | "failed" | "choosing" | "running";

export interface ResumeSnapshot {
  phase: ResumePhase;
  /** The refusal text of the last failed request, or null. */
  failure: string | null;
  entries: PlanEntry[];
  checkedIds: string[];
  isAcknowledged: boolean;
  /** True while `resume.start` waits for the server. */
  isStarting: boolean;
  /** The ids that were sent, in start order. */
  sentIds: string[];
  /** The newest result of each sent agent, by agent id. */
  results: Record<string, ResumeResult>;
}

export const INITIAL_RESUME: ResumeSnapshot = {
  phase: "loading",
  failure: null,
  entries: [],
  checkedIds: [],
  isAcknowledged: false,
  isStarting: false,
  sentIds: [],
  results: {},
};

const EVENT_RESUME_RESULT = "resume.result";
const EVENT_SWEEP = "office.sweep";
const REQUEST_FAILED = "The request failed.";

/**
 * Owns the state of the resume dialog. The server keeps resuming when the
 * dialog closes, so the controller lives with the panel and `open` starts a
 * fresh pass.
 */
export function createResumeController(client: CoreClient) {
  const service = createResumeService(client);
  let snapshot: ResumeSnapshot = INITIAL_RESUME;
  const listeners = new Set<(snapshot: ResumeSnapshot) => void>();
  // A newer open ends the right of an older plan read to publish.
  let newestOpen = 0;

  function publish(changes: Partial<ResumeSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_SWEEP) {
      // A plan that waits for the sweep loads once the sweep is done.
      const isWaiting =
        snapshot.phase === "sweeping" || snapshot.phase === "loading";
      if (event.state === "done" && isWaiting) void open();
      return;
    }
    if (event.type !== EVENT_RESUME_RESULT) return;
    const result = readResult(event);
    if (result === null || !snapshot.sentIds.includes(result.agent_id)) return;
    publish({ results: { ...snapshot.results, [result.agent_id]: result } });
  }

  /**
   * Reads `resume.plan` and checks every agent that is not on hold (FR-002).
   * While the sweep runs the plan is empty, so the dialog waits for `done`.
   */
  async function open(): Promise<void> {
    const openId = ++newestOpen;
    publish({ ...INITIAL_RESUME });
    try {
      const { isSweeping, entries } = await service.plan();
      if (openId !== newestOpen) return;
      if (isSweeping) {
        publish({ phase: "sweeping" });
        return;
      }
      publish({
        phase: "choosing",
        entries,
        checkedIds: entries
          .filter((entry) => !entry.isHeld)
          .map((entry) => entry.agent_id),
      });
    } catch (rejection) {
      if (openId !== newestOpen) return;
      publish({ phase: "failed", failure: describe(rejection) });
    }
  }

  return {
    get snapshot(): ResumeSnapshot {
      return snapshot;
    },

    /** Calls back on each change, starting with the current snapshot. */
    subscribe(listener: (snapshot: ResumeSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the results list stays at "waiting"; the agents'
        // own statuses still move.
      }
    },

    open,

    toggle(agentId: string): void {
      const isChecked = snapshot.checkedIds.includes(agentId);
      publish({
        checkedIds: isChecked
          ? snapshot.checkedIds.filter((id) => id !== agentId)
          : [...snapshot.checkedIds, agentId],
      });
    },

    acknowledge(isAcknowledged: boolean): void {
      publish({ isAcknowledged });
    },

    /** Sends the checked agents. Does nothing while Confirm is unavailable. */
    confirm: async (): Promise<void> => {
      const { entries, checkedIds, isAcknowledged } = snapshot;
      if (snapshot.isStarting || snapshot.phase !== "choosing") return;
      if (confirmBlock(entries, checkedIds, isAcknowledged) !== null) return;
      // In plan order, and known before the answer, since a result may
      // arrive first.
      const sentIds = entries
        .map((entry) => entry.agent_id)
        .filter((id) => checkedIds.includes(id));
      publish({ isStarting: true, failure: null, sentIds, results: {} });
      try {
        const order = await service.start(sentIds);
        publish({ phase: "running", isStarting: false, sentIds: order });
      } catch (rejection) {
        publish({
          isStarting: false,
          failure: describe(rejection),
          sentIds: [],
        });
      }
    },
  };
}

export type ResumeController = ReturnType<typeof createResumeController>;
