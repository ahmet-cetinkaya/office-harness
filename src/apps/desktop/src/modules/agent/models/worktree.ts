// SPDX-License-Identifier: AGPL-3.0-or-later

import { toCoreRequestError } from "../../../core/api";

/** What the form holds when it asks `agent.worktree_info` (0010, FR-001). */
export interface WorktreeQuery {
  folder?: string;
  project_id?: string;
  name: string;
}

/** The answer of `agent.worktree_info` for a folder inside a repository. */
export interface RepositoryInfo {
  in_repository: true;
  base: string;
  base_is_branch: boolean;
  branches: string[];
  branch_name: string;
  subfolder: string;
}

export type WorktreeInfo = { in_repository: false } | RepositoryInfo;

/** The create fields the form adds when the own worktree box is on. */
export interface WorktreeSelection {
  own_worktree: true;
  base: string;
  branch_name: string;
}

/** One entry of the `agent.worktree_status` answer (0010, FR-005). */
export interface WorktreeStatus {
  agent_id: string;
  name: string;
  folder: string;
  branch: string;
  base: string;
  folder_exists: boolean;
  reachable: boolean;
  changed_files: number;
  status_hash: string;
  own_commits: number | null;
  base_exists: boolean;
}

/** The answer of `agent.delete`: what the office left in the repository. */
export interface DeleteResult {
  deleted_branches: string[];
  kept_branches: string[];
  kept_folders: string[];
}

/** What the user chose for the worktrees of the agents being deleted. */
export type WorktreeChoice =
  | { worktrees: "keep" }
  | { worktrees: "remove"; confirmed_status: Record<string, string> };

/** How a delete ended, as the controllers tell the views. */
export type DeleteOutcome =
  | { kind: "deleted"; result: DeleteResult }
  | { kind: "changed"; message: string }
  | { kind: "refused"; message: string };

export const KEEP_WORKTREES: WorktreeChoice = { worktrees: "keep" };

/** The server's words for a worktree that changed after the question. */
const CONFIRM_AGAIN = "confirm again";
const SHORT_COMMIT_LENGTH = 7;
const REQUEST_FAILED = "The request failed.";

function asStrings(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

export function readDeleteResult(answer: unknown): DeleteResult {
  const fields = (answer ?? {}) as Record<string, unknown>;
  return {
    deleted_branches: asStrings(fields.deleted_branches),
    kept_branches: asStrings(fields.kept_branches),
    kept_folders: asStrings(fields.kept_folders),
  };
}

/** The base shown in the select: a branch by name, a detached commit by its
 * short id. The value stays what `agent.create` takes. */
export function baseOptions(
  info: RepositoryInfo,
): { value: string; label: string }[] {
  const options = info.branches.map((branch) => ({
    value: branch,
    label: branch,
  }));
  if (info.base_is_branch) return options;
  const short = info.base.slice(0, SHORT_COMMIT_LENGTH);
  return [
    { value: info.base, label: `${short} (detached commit)` },
    ...options,
  ];
}

/** The change state of one worktree, as text (FR-005). */
export function changeStateText(status: WorktreeStatus): string {
  if (!status.reachable) return "cannot be read, will be kept";
  if (status.changed_files === 0) return "no changes";
  return status.changed_files === 1
    ? "1 changed file"
    : `${status.changed_files} changed files`;
}

/** The worktrees that hold changes a removal would lose (FR-006). */
export function worktreesWithChanges(
  statuses: WorktreeStatus[],
): WorktreeStatus[] {
  return statuses.filter(
    (status) => status.reachable && status.changed_files > 0,
  );
}

/** The `remove` choice: the hash of every worktree the question showed. */
export function removeChoice(statuses: WorktreeStatus[]): WorktreeChoice {
  return {
    worktrees: "remove",
    confirmed_status: Object.fromEntries(
      statuses.map((status) => [status.agent_id, status.status_hash]),
    ),
  };
}

/** True for the `conflict` that asks the user to confirm again. */
export function isWorktreeChanged(rejection: unknown): boolean {
  const error = toCoreRequestError(rejection);
  return error.code === "conflict" && error.message.includes(CONFIRM_AGAIN);
}

/** The result of a delete as one message, or null when nothing was left. */
export function deleteResultText(result: DeleteResult): string | null {
  const lists: [string, string[]][] = [
    ["Deleted branches", result.deleted_branches],
    ["Kept branches", result.kept_branches],
    ["Kept folders", result.kept_folders],
  ];
  const parts = lists
    .filter(([, names]) => names.length > 0)
    .map(([label, names]) => `${label}: ${names.join(", ")}.`);
  return parts.length === 0 ? null : parts.join(" ");
}

/** Maps a rejection of `agent.delete` to the outcome the views act on. */
export function failedOutcome(
  rejection: unknown,
  choice: WorktreeChoice,
): Exclude<DeleteOutcome, { kind: "deleted" }> {
  const message = toCoreRequestError(rejection).message || REQUEST_FAILED;
  const kind =
    choice.worktrees === "remove" && isWorktreeChanged(rejection)
      ? "changed"
      : "refused";
  return { kind, message };
}
