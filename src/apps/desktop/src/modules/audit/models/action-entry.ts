// SPDX-License-Identifier: AGPL-3.0-or-later

/** One entry of the action record (FR-011). `result` is "ok" or error text. */
export interface ActionEntry {
  id: number;
  actor: string;
  agent_id: string;
  action: string;
  result: string;
  at: number;
}

export const ACTION_OK = "ok";

const ACTION_TEXT: Record<string, string> = {
  create: "Created",
  start: "Started",
  stop: "Stopped",
  delete: "Deleted",
  new_task: "Started a new task",
  continue: "Continued a session",
};

export function actionText(action: string): string {
  return ACTION_TEXT[action] ?? action.replaceAll("_", " ");
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
