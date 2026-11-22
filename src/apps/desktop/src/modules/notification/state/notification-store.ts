// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  announcementText,
  EVENT_AUTO_HOLD,
  sortRows,
  type NotificationRow,
} from "../models/notification";
import { createNotificationService } from "../services/notification-service";
import type { OsNotificationService } from "../services/os-notification";

export interface NotificationSnapshot {
  /** Every notification, unread first, then newest first (D4). */
  rows: NotificationRow[];
  /** The newest notification's text, announced once by the live region. */
  announcement: string | null;
}

export const INITIAL_NOTIFICATIONS: NotificationSnapshot = {
  rows: [],
  announcement: null,
};

const EVENT_NOTIFICATION_CREATED = "notification.created";

/**
 * Owns the notification list and the OS-notification rule (FR-006, D5, D7).
 * It reads `notification.list` on load, adds each `notification.created` row,
 * and decides per event whether an OS notification is useful: an event for
 * the agent whose terminal is open while the window is focused is already on
 * screen, so it enters the list as read instead.
 */
export function createNotificationController(
  client: CoreClient,
  os: OsNotificationService,
  /** The agent whose terminal is open and selected, or null. */
  selectedAgentId: () => string | null,
  /** Whether the desktop app window holds the focus (D5). */
  isWindowFocused: () => boolean,
  /**
   * Called with the id of a notification that is marked read on arrival, so
   * that the Needs you item it made never shows (0008 D8).
   */
  onReadOnArrival: (notificationId: string) => void = () => {},
) {
  const service = createNotificationService(client);
  let snapshot: NotificationSnapshot = INITIAL_NOTIFICATIONS;
  // Plain state, not runes: the listeners are Svelte effects, not UI state.
  const listeners = new Set<(snapshot: NotificationSnapshot) => void>();

  function publish(changes: Partial<NotificationSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message;
  }

  function readNow(row: NotificationRow): NotificationRow {
    return { ...row, read_at: Date.now() };
  }

  /** Puts one row at the front in the list order, once (D4). */
  function withRow(
    rows: NotificationRow[],
    row: NotificationRow,
  ): NotificationRow[] {
    return sortRows([row, ...rows.filter((known) => known.id !== row.id)]);
  }

  function rowOf(event: ServerEvent): NotificationRow | null {
    const row = event.notification;
    if (typeof row !== "object" || row === null) return null;
    const fields = row as Record<string, unknown>;
    if (
      typeof fields.id !== "string" ||
      typeof fields.agent_id !== "string" ||
      typeof fields.agent_name !== "string" ||
      typeof fields.event !== "string" ||
      typeof fields.created_at !== "number"
    ) {
      return null;
    }
    return {
      id: fields.id,
      agent_id: fields.agent_id,
      agent_name: fields.agent_name,
      event: fields.event,
      detail: typeof fields.detail === "string" ? fields.detail : null,
      created_at: fields.created_at,
      read_at: typeof fields.read_at === "number" ? fields.read_at : null,
    };
  }

  async function handleCreated(event: ServerEvent): Promise<void> {
    const row = rowOf(event);
    if (row === null) return;
    // An automatic hold is never on screen: the owner must see it even while
    // watching that agent's terminal (0007, MODIFIED 0003/FR-006).
    const isOnScreen =
      row.event !== EVENT_AUTO_HOLD &&
      row.agent_id === selectedAgentId() &&
      isWindowFocused();
    if (isOnScreen) {
      onReadOnArrival(row.id);
      // The user watches this agent's terminal, so an OS notification adds
      // nothing (FR-006 exception b, A7): the row shows as read at once.
      publish({
        rows: withRow(snapshot.rows, readNow(row)),
        announcement: null,
      });
      try {
        await service.read(row.id);
      } catch {
        // The next list read tells the truth again.
      }
      return;
    }
    publish({
      rows: withRow(snapshot.rows, row),
      announcement: announcementText(row),
    });
    if (await os.isAllowed()) {
      os.show(row.agent_id, row.agent_name, announcementText(row));
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_NOTIFICATION_CREATED) void handleCreated(event);
  }

  function markReadRow(row: NotificationRow): NotificationRow {
    return row.read_at === null ? { ...row, read_at: Date.now() } : row;
  }

  /** The tap on an OS notification, as the OS service reports it (D7). */
  function onTap(onTapAgent: (agentId: string) => void): Promise<void> {
    return os.onTap(onTapAgent);
  }

  return {
    get snapshot(): NotificationSnapshot {
      return snapshot;
    },

    /**
     * Calls back on each change, starting with the current snapshot. Returns
     * the stop function, as Svelte effects expect.
     */
    subscribe(listener: (snapshot: NotificationSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the list still shows what the last load read.
      }
    },

    onTap,

    /** Reads the list, the server's truth (D4). Call it on each ready. */
    load: async (): Promise<void> => {
      try {
        publish({ rows: sortRows(await service.list()), announcement: null });
      } catch {
        // A failed read keeps the rows it has; the next load tells the truth.
      }
    },

    /** The read that opening one notification asks for (FR-009). */
    read: async (notificationId: string): Promise<void> => {
      publish({
        rows: snapshot.rows.map((row) =>
          row.id === notificationId ? markReadRow(row) : row,
        ),
      });
      try {
        await service.read(notificationId);
      } catch {
        // The next list read tells the truth again.
      }
    },

    /**
     * The read that opening any agent's terminal asks for (FR-009). It is
     * fire-and-forget: the terminal must not wait on it.
     */
    readAgent: async (agentId: string): Promise<void> => {
      publish({
        rows: snapshot.rows.map((row) =>
          row.agent_id === agentId ? markReadRow(row) : row,
        ),
      });
      try {
        await service.readAgent(agentId);
      } catch (failure) {
        console.warn(
          "office-harness could not mark the agent's notifications read:",
          describe(failure),
        );
      }
    },
  };
}

export type NotificationController = ReturnType<
  typeof createNotificationController
>;
