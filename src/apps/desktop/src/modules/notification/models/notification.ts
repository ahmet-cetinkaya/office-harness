// SPDX-License-Identifier: AGPL-3.0-or-later

/** One notification of the list (contract `notification.list`, D4). */
export interface NotificationRow {
  id: string;
  agent_id: string;
  /** The agent's name at creation, kept for the rows of a deleted agent. */
  agent_name: string;
  event: string;
  created_at: number;
  /** Null while unread (FR-007). */
  read_at: number | null;
}

const EVENT_TEXT: Record<string, string> = {
  exited: "exited",
  failed: "failed",
  waiting_for_input: "waits for your input",
  bell: "rang the bell",
};

/** An event as text; a kind this app does not know is shown as sent. */
export function eventText(event: string): string {
  return EVENT_TEXT[event] ?? event.replaceAll("_", " ");
}

/** What a row of a deleted agent says instead of opening (edge case). */
export const DELETED_TEXT = "agent deleted";

/** The row's name as one sentence, in the row's reading order (D11). */
export function rowText(row: NotificationRow, agentExists: boolean): string {
  const parts = [row.agent_name, eventText(row.event)];
  if (!agentExists) parts.push(DELETED_TEXT);
  return parts.join(", ");
}

/** What the live region announces for one new notification. */
export function announcementText(row: NotificationRow): string {
  return `${row.agent_name} ${eventText(row.event)}`;
}

/** True while the user has not opened the notification (FR-007). */
export function isUnread(row: NotificationRow): boolean {
  return row.read_at === null;
}

export function unreadCount(rows: NotificationRow[]): number {
  return rows.filter(isUnread).length;
}

/** The list order: unread first, then newest first (D4). */
export function sortRows(rows: NotificationRow[]): NotificationRow[] {
  return [...rows].sort((first, second) => {
    if (isUnread(first) !== isUnread(second)) return isUnread(first) ? -1 : 1;
    return second.created_at - first.created_at;
  });
}
