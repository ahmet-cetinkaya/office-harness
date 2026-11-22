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
};

const OFFICE_ACTOR = "office";

export function actionText(action: string): string {
  return ACTION_TEXT[action] ?? action.replaceAll("_", " ");
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
