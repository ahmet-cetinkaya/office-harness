// SPDX-License-Identifier: AGPL-3.0-or-later

/** The event kinds of an item (contract `needs_you.list`). */
export type NeedsYouKind =
  | "failed"
  | "waiting_for_input"
  | "bell"
  | "auto_hold"
  | "resume_hold"
  | "exited"
  | "report";

/** One item of the list, as the server sends it (contract `needs_you.list`). */
export interface NeedsYouItem {
  agent_id: string;
  kind: NeedsYouKind;
  /** The server's text for the kind, shown as sent; null when the kind has none. */
  detail: string | null;
  /** Unix ms; the wait time counts from it. */
  first_event_at: number;
  latest_at: number;
  /** The notification that made the item; null for a coalesced bell. */
  origin_notification_id: string | null;
  /** The last report the agent sent; stays while a later event changes the kind. */
  report: ItemReport | null;
}

/** A report on an item (contract `report-protocol`); the text is already visible text. */
export interface ItemReport {
  kind: string;
  text: string;
  /** Unix ms. */
  at: number;
}

/** One row of the section: the item with the words the desktop writes. */
export interface NeedsYouRow {
  agentId: string;
  agentName: string;
  projectName: string | null;
  event: string;
  /** "<report kind>: <report text>" when the item holds a report. */
  reportText: string | null;
  /** True when the report is long enough to offer the whole-report button. */
  isReportLong: boolean;
  wait: string;
  /** The row's accessible name: name, project, event, and wait time. */
  label: string;
}

const KINDS: readonly string[] = [
  "failed",
  "waiting_for_input",
  "bell",
  "auto_hold",
  "resume_hold",
  "exited",
  "report",
];
/** A report longer than this many characters gets the whole-report button. */
export const REPORT_CLAMP_CHARACTERS = 160;
const MS_PER_SECOND = 1_000;
const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;

export const EMPTY_TEXT = "No agent needs you";
const EMPTY_COUNT_TEXT = "Needs you is empty";

function isItem(value: unknown): value is NeedsYouItem {
  if (typeof value !== "object" || value === null) return false;
  const fields = value as Record<string, unknown>;
  return (
    typeof fields.agent_id === "string" &&
    typeof fields.kind === "string" &&
    KINDS.includes(fields.kind) &&
    typeof fields.first_event_at === "number"
  );
}

function textOrNull(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function readReport(value: unknown): ItemReport | null {
  if (typeof value !== "object" || value === null) return null;
  const fields = value as Record<string, unknown>;
  if (
    typeof fields.kind !== "string" ||
    typeof fields.text !== "string" ||
    typeof fields.at !== "number"
  ) {
    return null;
  }
  return { kind: fields.kind, text: fields.text, at: fields.at };
}

/**
 * The items of a `needs_you.list` answer or a `needs_you.changed` event, in
 * the server's order; bad input is no items.
 */
export function readItems(value: unknown): NeedsYouItem[] {
  if (typeof value !== "object" || value === null) return [];
  const { items } = value as { items?: unknown };
  if (!Array.isArray(items)) return [];
  return items.filter(isItem).map((item) => ({
    agent_id: item.agent_id,
    kind: item.kind,
    detail: textOrNull(item.detail),
    first_event_at: item.first_event_at,
    latest_at: typeof item.latest_at === "number" ? item.latest_at : 0,
    origin_notification_id: textOrNull(item.origin_notification_id),
    report: readReport((item as { report?: unknown }).report),
  }));
}

function quietText(quietPeriodMs: string | null): string {
  const seconds = Number(quietPeriodMs) / MS_PER_SECOND;
  if (quietPeriodMs === null || !Number.isFinite(seconds) || seconds <= 0) {
    return "waits for your input";
  }
  return `quiet for ${seconds} s`;
}

/**
 * The event as the office saw it, as text. The desktop writes the words from
 * the kind and shows the server's detail as sent; it never reads the detail
 * for meaning (data model, "Stored detail per kind"). The waiting detail is
 * the one number, the quiet period in ms.
 */
export function eventText(item: NeedsYouItem): string {
  switch (item.kind) {
    case "waiting_for_input":
      return quietText(item.detail);
    case "failed":
      return item.detail === null ? "failed" : `failed: ${item.detail}`;
    case "auto_hold":
      return item.detail === null
        ? "put on hold by the office"
        : `put on hold by the office, limit: ${item.detail}`;
    case "resume_hold":
      return item.detail === null
        ? "was put on hold"
        : `was put on hold: ${item.detail}`;
    case "bell":
      return "bell";
    case "exited":
      return "exited";
    case "report":
      return item.detail === null
        ? "sent a report"
        : `sent a report: ${item.detail}`;
  }
}

/** The wait time as text, counted from the first event. */
export function waitText(firstEventAt: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - firstEventAt) / MS_PER_SECOND));
  if (seconds < SECONDS_PER_MINUTE) return `waiting ${seconds} s`;
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  if (minutes < MINUTES_PER_HOUR) return `waiting ${minutes} min`;
  const hours = Math.floor(minutes / MINUTES_PER_HOUR);
  return `waiting ${hours} h ${minutes % MINUTES_PER_HOUR} min`;
}

export function rowOf(
  item: NeedsYouItem,
  agentName: string,
  projectName: string | null,
  now: number,
): NeedsYouRow {
  const event = eventText(item);
  const wait = waitText(item.first_event_at, now);
  const { report } = item;
  const label = [
    agentName,
    ...(projectName === null ? [] : [`project ${projectName}`]),
    event,
    wait,
  ].join(", ");
  return {
    agentId: item.agent_id,
    agentName,
    projectName,
    event,
    reportText: report === null ? null : `${report.kind}: ${report.text}`,
    isReportLong:
      report !== null && report.text.length > REPORT_CLAMP_CHARACTERS,
    wait,
    label,
  };
}

/** What the live region says when the count changes. */
export function countAnnouncement(count: number): string {
  if (count === 0) return EMPTY_COUNT_TEXT;
  return `Needs you: ${count} ${count === 1 ? "agent" : "agents"}`;
}
