// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import { EMPTY_TEMP_LIST, type TempList } from "../models/temps";
import { createTempService } from "../services/temp-service";

export interface TempsSnapshot extends TempList {
  phase: "closed" | "loading" | "ready" | "failed";
  /** Why the list did not load, or null. */
  failure: string | null;
  /** Why the last End or Dismiss was refused, or null. */
  actionFailure: string | null;
}

export const INITIAL_TEMPS: TempsSnapshot = {
  ...EMPTY_TEMP_LIST,
  phase: "closed",
  failure: null,
  actionFailure: null,
};

const EVENT_TEMP_CHANGED = "temp.changed";
const EVENT_AGENT_CHANGED = "agent.changed";
const REQUEST_FAILED = "The request failed.";

/**
 * Owns the list of temps and finished temps while the Temps dialog is open.
 * It reads the list again on each event that can change it.
 */
export function createTempsController(client: CoreClient) {
  const service = createTempService(client);
  let snapshot: TempsSnapshot = INITIAL_TEMPS;
  let latestRead = 0;
  const listeners = new Set<(snapshot: TempsSnapshot) => void>();

  function update(changes: Partial<TempsSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  const isClosed = (): boolean => snapshot.phase === "closed";

  async function refresh(): Promise<void> {
    if (isClosed()) return;
    const read = ++latestRead;
    try {
      const list = await service.list();
      if (read !== latestRead || isClosed()) return;
      update({ ...list, phase: "ready", failure: null });
    } catch (rejection) {
      if (read !== latestRead || isClosed()) return;
      update({ phase: "failed", failure: describe(rejection) });
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (
      event.type === EVENT_TEMP_CHANGED ||
      event.type === EVENT_AGENT_CHANGED
    ) {
      void refresh();
    }
  }

  /** Ends or dismisses, then reads the list again; resolves to the refusal. */
  async function change(request: () => Promise<void>): Promise<string | null> {
    update({ actionFailure: null });
    let failure: string | null = null;
    try {
      await request();
    } catch (rejection) {
      failure = describe(rejection);
      update({ actionFailure: failure });
    }
    await refresh();
    return failure;
  }

  return {
    get snapshot(): TempsSnapshot {
      return snapshot;
    },

    /** Calls back on each change, starting with the current snapshot. */
    subscribe(listener: (snapshot: TempsSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the dialog's own interval still reads the list.
      }
    },

    /** The dialog opened: events and reads count from now on. */
    open: async (): Promise<void> => {
      update({ ...INITIAL_TEMPS, phase: "loading" });
      await refresh();
    },

    /** The dialog closed: events no longer read the list. */
    close(): void {
      update(INITIAL_TEMPS);
    },

    refresh,

    end: (agentId: string): Promise<string | null> =>
      change(() => service.end(agentId)),

    dismiss: (entryId: string): Promise<string | null> =>
      change(() => service.dismiss(entryId)),
  };
}

export type TempsController = ReturnType<typeof createTempsController>;
