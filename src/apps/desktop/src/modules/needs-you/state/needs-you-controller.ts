// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  countAnnouncement,
  EMPTY_TEXT,
  readItems,
  type NeedsYouItem,
} from "../models/needs-you";
import { createNeedsYouService } from "../services/needs-you-service";

export interface NeedsYouSnapshot {
  /** The items the user sees and the title counts, in the server's order. */
  items: NeedsYouItem[];
  /** The newest count change as text, announced once by the live region. */
  announcement: string | null;
  /** The clock of the wait times, in Unix ms; the tick moves it. */
  now: number;
}

export const INITIAL_NEEDS_YOU: NeedsYouSnapshot = {
  items: [],
  announcement: null,
  now: 0,
};

/** The refusal text of a dismiss, or null when the item left. */
export type DismissFailure = string | null;

const EVENT_NEEDS_YOU_CHANGED = "needs_you.changed";
const EVENT_RESYNC = "client.resync";
const NOT_FOUND = "not_found";
const TICK_MS = 1_000;
/** How long a taken item stays hidden while the server has not answered. */
const PENDING_TAKE_MS = 5_000;

/**
 * Owns the Needs you list. It reads `needs_you.list` on load and replaces its
 * copy on each `needs_you.changed`. An item whose origin notification the
 * desktop marked read on arrival is never shown, counted, or announced
 * (FR-001, D8): the server removes it on that read, and the event that
 * carries it comes first.
 */
export function createNeedsYouController(client: CoreClient) {
  const service = createNeedsYouService(client);
  let snapshot: NeedsYouSnapshot = INITIAL_NEEDS_YOU;
  // Everything the server sent, including items that are hidden.
  let received: NeedsYouItem[] = [];
  const readOnArrival = new Set<string>();
  // Agents the jump took, with the time the hiding ends. A list that comes
  // before the server handled the read still holds them (D5, fast presses).
  const pendingTakes = new Map<string, number>();
  const listeners = new Set<(snapshot: NeedsYouSnapshot) => void>();
  // An event is newer than any read that is still out, so it ends that read's
  // right to publish.
  let newestRead = 0;

  function publish(changes: Partial<NeedsYouSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function isShown(item: NeedsYouItem): boolean {
    if (pendingTakes.has(item.agent_id)) return false;
    return (
      item.origin_notification_id === null ||
      !readOnArrival.has(item.origin_notification_id)
    );
  }

  /** Ends the hiding of an agent once the server list no longer has it. */
  function forgetTaken(serverItems: NeedsYouItem[]): void {
    const agentIds = new Set(serverItems.map((item) => item.agent_id));
    for (const agentId of pendingTakes.keys()) {
      if (!agentIds.has(agentId)) pendingTakes.delete(agentId);
    }
  }

  /** Ends the hiding that timed out, so a failed read cannot hide for good. */
  function expireTakes(): void {
    const now = Date.now();
    let hasExpired = false;
    for (const [agentId, until] of pendingTakes) {
      if (until > now) continue;
      pendingTakes.delete(agentId);
      hasExpired = true;
    }
    if (hasExpired) replace(received, false);
  }

  function originsOf(items: NeedsYouItem[]): Set<string> {
    return new Set(
      items.flatMap((item) =>
        item.origin_notification_id === null
          ? []
          : [item.origin_notification_id],
      ),
    );
  }

  /** Forgets an origin once the server has removed its item. */
  function forgetCleared(next: NeedsYouItem[]): void {
    const before = originsOf(received);
    const after = originsOf(next);
    for (const origin of before) {
      if (!after.has(origin)) readOnArrival.delete(origin);
    }
  }

  /** Makes `next` the list. A count change is announced only when asked. */
  function replace(next: NeedsYouItem[], isAnnounced: boolean): void {
    forgetCleared(next);
    received = next;
    const items = received.filter(isShown);
    const isCountChanged = items.length !== snapshot.items.length;
    publish({
      items,
      now: Date.now(),
      ...(isAnnounced && isCountChanged
        ? { announcement: countAnnouncement(items.length) }
        : {}),
    });
  }

  /** Makes the server's list the list; its items end the taken hiding. */
  function adopt(serverItems: NeedsYouItem[], isAnnounced: boolean): void {
    forgetTaken(serverItems);
    replace(serverItems, isAnnounced);
  }

  function withoutAgent(agentId: string): NeedsYouItem[] {
    return received.filter((item) => item.agent_id !== agentId);
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_RESYNC) {
      void load();
      return;
    }
    if (event.type !== EVENT_NEEDS_YOU_CHANGED) return;
    newestRead++;
    adopt(readItems(event), true);
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const items = await service.list();
      if (readId !== newestRead) return;
      // A resync is not a count change: the standing announcement is stale.
      publish({ announcement: null });
      adopt(items, false);
    } catch {
      // A failed read keeps the list it has; the next load tells the truth.
    }
  }

  return {
    get snapshot(): NeedsYouSnapshot {
      return snapshot;
    },

    /**
     * Calls back on each change, starting with the current snapshot. Returns
     * the stop function, as Svelte effects expect.
     */
    subscribe(listener: (snapshot: NeedsYouSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the list stays as the last load read it.
      }
    },

    /** Reads `needs_you.list`. Call it on each ready connection. */
    load,

    /** Moves the wait times on; returns the stop function. */
    startTicking(intervalMs = TICK_MS): () => void {
      const timer = setInterval(() => {
        expireTakes();
        publish({ now: Date.now() });
      }, intervalMs);
      return () => clearInterval(timer);
    },

    /**
     * The notification that made an item was marked read on arrival, so
     * the item never shows (D8). Call it before the item's event comes.
     */
    hideOrigin(notificationId: string): void {
      readOnArrival.add(notificationId);
      replace(received, false);
    },

    /**
     * The read on arrival failed, so the server keeps the item: it shows
     * again, because the user has not seen that agent's event.
     */
    showOrigin(notificationId: string): void {
      readOnArrival.delete(notificationId);
      replace(received, false);
    },

    /**
     * Takes the item at the top whose agent still exists, for the jump
     * (FR-007). The item leaves the local list at once and stays out until a
     * server list no longer has it, so a second press before the server's
     * event comes takes the next one (D5). With no such
     * item the view stays and the live region says so (FR-008).
     */
    takeTop(isKnownAgent: (agentId: string) => boolean): string | null {
      const top = snapshot.items.find((item) => isKnownAgent(item.agent_id));
      if (top === undefined) {
        publish({ announcement: EMPTY_TEXT });
        return null;
      }
      replace(withoutAgent(top.agent_id), true);
      pendingTakes.set(top.agent_id, Date.now() + PENDING_TAKE_MS);
      return top.agent_id;
    },

    /**
     * Dismisses one item (FR-004). It leaves the list at once; when the
     * server refuses, the list is read again and the refusal is returned.
     */
    dismiss: async (agentId: string): Promise<DismissFailure> => {
      replace(withoutAgent(agentId), true);
      try {
        await service.dismiss(agentId);
        return null;
      } catch (rejection) {
        const failure = toCoreRequestError(rejection);
        await load();
        return failure.code === NOT_FOUND ? null : failure.message;
      }
    },
  };
}

export type NeedsYouController = ReturnType<typeof createNeedsYouController>;
