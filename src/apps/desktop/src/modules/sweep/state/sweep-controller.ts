// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient, ServerEvent } from "../../../core/api";
import { readSweepState, type SweepState } from "../models/sweep";
import { createSweepService } from "../services/sweep-service";

/**
 * "finished" is a sweep that this app saw running; it is spoken once and
 * never shown, so an app that connects after the sweep says nothing.
 */
export type SweepPhase = "idle" | "running" | "finished";

export interface SweepSnapshot {
  phase: SweepPhase;
}

export const INITIAL_SWEEP: SweepSnapshot = { phase: "idle" };

const EVENT_SWEEP = "office.sweep";
const EVENT_RESYNC = "client.resync";

/**
 * Owns what the panel knows of the sweep (0017, D9): it reads `sweep.state`
 * on each connection and follows `office.sweep`.
 */
export function createSweepController(client: CoreClient) {
  const service = createSweepService(client);
  let snapshot: SweepSnapshot = INITIAL_SWEEP;
  const listeners = new Set<(snapshot: SweepSnapshot) => void>();
  // An event is newer than any read that is still out.
  let newestRead = 0;

  function apply(state: SweepState): void {
    const phase: SweepPhase =
      state === "running"
        ? "running"
        : snapshot.phase === "running"
          ? "finished"
          : snapshot.phase;
    if (phase === snapshot.phase) return;
    snapshot = { phase };
    for (const listener of listeners) listener(snapshot);
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const state = await service.state();
      if (state !== null && readId === newestRead) apply(state);
    } catch {
      // An older server has no sweep, and a failed read says nothing; the
      // next connection reads again.
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_RESYNC) {
      void load();
      return;
    }
    if (event.type !== EVENT_SWEEP) return;
    const state = readSweepState(event);
    if (state === null) return;
    newestRead++;
    apply(state);
  }

  return {
    get snapshot(): SweepSnapshot {
      return snapshot;
    },

    subscribe(listener: (snapshot: SweepSnapshot) => void): () => void {
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

    /** Reads `sweep.state`. Call it on each ready connection. */
    load,
  };
}

export type SweepController = ReturnType<typeof createSweepController>;
