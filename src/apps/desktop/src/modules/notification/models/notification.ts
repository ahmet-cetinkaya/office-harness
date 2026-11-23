// SPDX-License-Identifier: AGPL-3.0-or-later

/** One notification of the list (contract `notification.list`, D4). */
export interface NotificationRow {
  id: string;
  agent_id: string;
  /** The agent's name at creation, kept for the rows of a deleted agent. */
  agent_name: string;
  event: string;
  /** The limit's name for an `auto_hold`, the reason for a `resume_hold`, the report kind for a `report`; null otherwise. */
  detail?: string | null;
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

const EVENT_AUTO_HOLD = "auto_hold";
const EVENT_RESUME_HOLD = "resume_hold";
const EVENT_REPORT = "report";
/**
 * Events the agent's terminal does not show, so they stay unread and ping
 * the OS even while that terminal is open and focused: the holds (0007,
 * 0013/PB-6) and a report for the person (0015, MODIFIED 0003/FR-006).
 */
const EVENTS_NEVER_READ_ON_ARRIVAL: readonly string[] = [
  EVENT_AUTO_HOLD,
  EVENT_RESUME_HOLD,
  EVENT_REPORT,
];

/** Whether the focused-terminal exception must not mark this event read. */
export function isNeverReadOnArrival(event: string): boolean {
  return EVENTS_NEVER_READ_ON_ARRIVAL.includes(event);
}

/** How many refusals by one limit put an agent on hold (0007 D7). */
const AUTO_HOLD_REFUSALS = 3;

/**
 * An event as text; a kind this app does not know is shown as sent. An
 * automatic hold names the limit that it came from, when the row has one.
 */
export function eventText(event: string, detail: string | null = null): string {
  if (event === EVENT_AUTO_HOLD) {
    const base = `was put on hold after ${AUTO_HOLD_REFUSALS} refusals`;
    return detail === null ? base : `${base} by the limit "${detail}"`;
  }
  if (event === EVENT_RESUME_HOLD) {
    const base = "was put on hold";
    return detail === null ? base : `${base}: ${detail}`;
  }
  if (event === EVENT_REPORT) {
    return detail === null ? "sent a report" : `sent a report: ${detail}`;
  }
  return EVENT_TEXT[event] ?? event.replaceAll("_", " ");
}

/** What a row of a deleted agent says instead of opening (edge case). */
export const DELETED_TEXT = "agent deleted";

/** The row's name as one sentence, in the row's reading order (D11). */
export function rowText(row: NotificationRow, agentExists: boolean): string {
  const parts = [row.agent_name, eventText(row.event, row.detail)];
  if (!agentExists) parts.push(DELETED_TEXT);
  return parts.join(", ");
}

/** What the live region announces for one new notification. */
export function announcementText(row: NotificationRow): string {
  return `${row.agent_name} ${eventText(row.event, row.detail)}`;
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
