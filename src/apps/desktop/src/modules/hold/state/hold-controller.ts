// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import type { ActionEntry } from "../../audit";
import {
  announcementOfChange,
  NO_HOLDS,
  readHoldState,
  type HoldState,
} from "../models/hold";
import { LIMIT_SETTINGS } from "../models/limits";
import { createHoldService } from "../services/hold-service";

export interface HoldSnapshot {
  state: HoldState;
  /** The newest hold change as text, announced once by the live region. */
  announcement: string | null;
}

export const INITIAL_HOLD: HoldSnapshot = {
  state: NO_HOLDS,
  announcement: null,
};

const EVENT_HOLD_CHANGED = "hold.changed";
const EVENT_RESYNC = "client.resync";
const REQUEST_FAILED = "The request failed.";

/** A request's refusal text, or null when the request went through. */
export type FailureText = string | null;

/**
 * Owns the hold state. It reads `hold.list` on load and replaces its copy on
 * each `hold.changed`, which also carries holds the office set by itself.
 */
export function createHoldController(
  client: CoreClient,
  /** The agent's name for an announcement; the id when the agent is unknown. */
  nameOfAgent: (agentId: string) => string,
) {
  const service = createHoldService(client);
  let snapshot: HoldSnapshot = INITIAL_HOLD;
  const listeners = new Set<(snapshot: HoldSnapshot) => void>();
  // An event is newer than any read that is still out, so it ends that read's
  // right to publish.
  let newestRead = 0;

  function publish(changes: Partial<HoldSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function submit(work: () => Promise<unknown>): Promise<FailureText> {
    try {
      await work();
      return null;
    } catch (rejection) {
      return describe(rejection);
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_RESYNC) {
      void load();
      return;
    }
    if (event.type !== EVENT_HOLD_CHANGED) return;
    newestRead++;
    const state = readHoldState(event);
    const announcement = announcementOfChange(
      snapshot.state,
      state,
      nameOfAgent,
    );
    publish({ state, announcement });
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const state = await service.list();
      if (readId === newestRead) publish({ state, announcement: null });
    } catch {
      // A failed read keeps the state it has; the next load tells the truth.
    }
  }

  return {
    get snapshot(): HoldSnapshot {
      return snapshot;
    },

    /**
     * Calls back on each change, starting with the current snapshot. Returns
     * the stop function, as Svelte effects expect.
     */
    subscribe(listener: (snapshot: HoldSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the state stays as the last load read it.
      }
    },

    /** Reads `hold.list`. Call it on each ready connection. */
    load,

    holdOffice: (): Promise<FailureText> => submit(() => service.set()),
    releaseOffice: (): Promise<FailureText> => submit(() => service.release()),
    holdAgent: (agentId: string): Promise<FailureText> =>
      submit(() => service.set(agentId)),
    releaseAgent: (agentId: string): Promise<FailureText> =>
      submit(() => service.release(agentId)),

    /** The office-level action record entries, newest first (A11). */
    officeActions: (): Promise<ActionEntry[]> => service.officeActions(),

    /** Each limit's stored text; the default when the server holds none. */
    readLimits: async (): Promise<Record<string, string>> => {
      const values: Record<string, string> = {};
      for (const limit of LIMIT_SETTINGS) {
        const stored = await service.readSetting(limit.key);
        values[limit.key] = stored === "" ? String(limit.defaultValue) : stored;
      }
      return values;
    },

    /** Saves the limits in order; the first refusal stops the save. */
    saveLimits: (values: Record<string, string>): Promise<FailureText> =>
      submit(async () => {
        for (const limit of LIMIT_SETTINGS) {
          await service.writeSetting(limit.key, values[limit.key].trim());
        }
      }),

    /** The refusal text of a rejected read, for a dialog to show. */
    describeFailure: describe,
  };
}

export type HoldController = ReturnType<typeof createHoldController>;
