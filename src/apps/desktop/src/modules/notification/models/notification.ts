// SPDX-License-Identifier: AGPL-3.0-or-later

/** One notification of the list (contract `notification.list`, D4). */
export interface NotificationRow {
  id: string;
  /** Null for a row that no agent owns: the missed schedule runs (0025). */
  agent_id: string | null;
  /** The agent's name at creation, kept for the rows of a deleted agent. */
  agent_name: string;
  event: string;
  /** The limit's name for an `auto_hold`, the reason for a `resume_hold` or a `temp_ended`, the report kind for a `report`, the OS reason for a `leftover_kept`; null otherwise. */
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
const EVENT_TEMP_ENDED = "temp_ended";
/** A schedule run's result, or the missed runs of schedules (0025). */
export const EVENT_SCHEDULE = "schedule";
/** An agent's leftover process that the sweep could not end (0017, FR-006). */
export const EVENT_LEFTOVER_KEPT = "leftover_kept";
/**
 * Events the agent's terminal does not show, so they stay unread and ping
 * the OS even while that terminal is open and focused: the holds (0007,
 * 0013/PB-6), a report for the person (0015, MODIFIED 0003/FR-006), and a
 * leftover that could not be ended (0017).
 */
const EVENTS_NEVER_READ_ON_ARRIVAL: readonly string[] = [
  EVENT_AUTO_HOLD,
  EVENT_RESUME_HOLD,
  EVENT_REPORT,
  EVENT_LEFTOVER_KEPT,
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
  if (event === EVENT_LEFTOVER_KEPT) {
    const base = "has a leftover process that could not be ended";
    return detail === null ? base : `${base}: ${detail}`;
  }
  if (event === EVENT_REPORT) {
    return detail === null ? "sent a report" : `sent a report: ${detail}`;
  }
  if (event === EVENT_SCHEDULE) return detail ?? "";
  if (event === EVENT_TEMP_ENDED) {
    return detail === null ? "ended" : `ended: ${detail}`;
  }
  return EVENT_TEXT[event] ?? event.replaceAll("_", " ");
}

/** Whether opening the row opens the schedules view, not an agent (0025). */
export function opensSchedules(row: NotificationRow): boolean {
  return row.event === EVENT_SCHEDULE;
}

/** What a row of a deleted agent says instead of opening (edge case). */
export const DELETED_TEXT = "agent deleted";

/** The row's name as one sentence, in the row's reading order (D11). */
export function rowText(row: NotificationRow, agentExists: boolean): string {
  const parts = [row.agent_name, eventText(row.event, row.detail)];
  if (row.agent_id !== null && !agentExists) parts.push(DELETED_TEXT);
  return parts.filter((part) => part !== "").join(", ");
}

/** What the live region announces for one new notification. */
export function announcementText(row: NotificationRow): string {
  return `${row.agent_name} ${eventText(row.event, row.detail)}`.trim();
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

/**
 * The one OS notification for the agents whose leftovers could not be ended
 * (0017, FR-006): it names each agent once, in the order the rows came.
 */
export function leftoverKeptBody(agentNames: string[]): string {
  const names = [...new Set(agentNames)].join(", ");
  return `Could not end a leftover process of: ${names}`;
}
