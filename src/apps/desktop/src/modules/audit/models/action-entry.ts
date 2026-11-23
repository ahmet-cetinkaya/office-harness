// SPDX-License-Identifier: AGPL-3.0-or-later

/** One entry of the action record (FR-011). `result` is "ok" or error text. */
export interface ActionEntry {
  id: number;
  actor: string;
  /** Null for an office-level entry, such as an office hold. */
  agent_id: string | null;
  action: string;
  result: string;
  at: number;
  /** Why a start began a new session instead of a continue; null otherwise. */
  detail: string | null;
}

export const ACTION_OK = "ok";

const ACTION_TEXT: Record<string, string> = {
  create: "Created",
  start: "Started",
  stop: "Stopped",
  delete: "Deleted",
  new_task: "Started a new task",
  continue: "Continued a session",
  task_text: "Changed the task text",
  hold: "Put on hold",
  release: "Released from hold",
  report: "Sent a report",
  end_leftover: "Checked a leftover process",
  helper_limit: "Stopped storing helpers",
  sleep: "Put to sleep",
  wake: "Woke",
  end: "Ended",
  secret_read: "Read a secret",
  secret_change: "Changed a secret",
};

/**
 * Entries whose detail is not a reason but a record of what happened, so it
 * is labelled "Detail" (0017, D9).
 */
const DETAIL_ACTIONS: readonly string[] = [
  "end_leftover",
  "helper_limit",
  "secret_read",
  "secret_change",
];

/**
 * Control and format characters (which include the zero-width and the
 * bidirectional controls) and the line and paragraph separators. The server
 * escapes the same set.
 */
const UNSAFE_CHARACTERS = /[\p{Cc}\p{Cf}\u2028\u2029]/gu;

const OFFICE_ACTOR = "office";

export function actionText(action: string): string {
  return ACTION_TEXT[action] ?? action.replaceAll("_", " ");
}

/**
 * The detail as safe text. The server already writes such characters as
 * `\u{..}`; the desktop does it again for any that remain, so a command line
 * cannot reorder or hide text (0017, D9).
 */
export function visibleDetail(detail: string): string {
  return detail.replace(
    UNSAFE_CHARACTERS,
    (character) => `\\u{${(character.codePointAt(0) ?? 0).toString(16)}}`,
  );
}

/** The detail line of an entry: "Reason: ..." or "Detail: ...", or null. */
export function detailText(entry: ActionEntry): string | null {
  if (entry.detail === null) return null;
  const label = DETAIL_ACTIONS.includes(entry.action) ? "Detail" : "Reason";
  return `${label}: ${visibleDetail(entry.detail)}`;
}

export function actorText(actor: string): string {
  return actor === OFFICE_ACTOR ? "the office" : actor;
}

export function resultText(result: string): string {
  return result === ACTION_OK ? "Succeeded" : `Failed: ${result}`;
}

/** Adds an entry that is not in the list yet, newest last. */
export function mergeEntry(
  entries: ActionEntry[],
  entry: ActionEntry,
): ActionEntry[] {
  if (entries.some((known) => known.id === entry.id)) return entries;
  return [...entries, entry].sort((first, second) => first.id - second.id);
}
