// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { CoreRequestError } from "../../../core/api";
import {
  baseOptions,
  changeStateText,
  deleteResultText,
  failedOutcome,
  isWorktreeChanged,
  readDeleteResult,
  removeChoice,
  worktreesWithChanges,
  type RepositoryInfo,
  type WorktreeStatus,
} from "./worktree";

function status(overrides: Partial<WorktreeStatus> = {}): WorktreeStatus {
  return {
    agent_id: "a",
    name: "Ada",
    folder: "/data/worktrees/a",
    branch: "office-harness/ada",
    base: "main",
    folder_exists: true,
    reachable: true,
    changed_files: 0,
    status_hash: "h-a",
    own_commits: 0,
    base_exists: true,
    ...overrides,
  };
}

const INFO: RepositoryInfo = {
  in_repository: true,
  base: "main",
  base_is_branch: true,
  branches: ["main", "develop"],
  branch_name: "office-harness/ada",
  subfolder: "",
};

describe("worktree models", () => {
  test("the change state reads as text", () => {
    expect(changeStateText(status({ changed_files: 3 }))).toBe(
      "3 changed files",
    );
    expect(changeStateText(status({ changed_files: 1 }))).toBe(
      "1 changed file",
    );
    expect(changeStateText(status())).toBe("no changes");
    expect(changeStateText(status({ reachable: false }))).toBe(
      "cannot be read, will be kept",
    );
  });

  test("only a readable worktree with changes asks the second question", () => {
    const listed = [
      status({ agent_id: "a", changed_files: 2 }),
      status({ agent_id: "b" }),
      status({ agent_id: "c", reachable: false, changed_files: 5 }),
    ];

    expect(worktreesWithChanges(listed).map((one) => one.agent_id)).toEqual([
      "a",
    ]);
  });

  test("remove carries the hash of every listed worktree", () => {
    const listed = [
      status({ agent_id: "a", status_hash: "h-a" }),
      status({ agent_id: "b", status_hash: "h-b" }),
    ];

    expect(removeChoice(listed)).toEqual({
      worktrees: "remove",
      confirmed_status: { a: "h-a", b: "h-b" },
    });
  });

  test("a detached commit is offered by its short id", () => {
    const detached = {
      ...INFO,
      base: "0123456789abcdef",
      base_is_branch: false,
    };

    expect(baseOptions(INFO).map((option) => option.value)).toEqual([
      "main",
      "develop",
    ]);
    expect(baseOptions(detached)[0]).toEqual({
      value: "0123456789abcdef",
      label: "0123456 (detached commit)",
    });
  });

  test("the result lists deleted and kept branches and kept folders", () => {
    const result = {
      deleted_branches: ["b1"],
      kept_branches: ["b2", "b3"],
      kept_folders: ["/f"],
    };

    expect(deleteResultText(result)).toBe(
      "Deleted branches: b1. Kept branches: b2, b3. Kept folders: /f.",
    );
    expect(deleteResultText(readDeleteResult(null))).toBeNull();
  });

  test("only the confirm-again conflict of a remove asks again", () => {
    const changed = new CoreRequestError(
      "conflict",
      "the worktree of Ada changed; confirm again",
    );
    const other = new CoreRequestError("conflict", "the agent has 3 members");

    expect(isWorktreeChanged(changed)).toBe(true);
    expect(isWorktreeChanged(other)).toBe(false);
    expect(failedOutcome(changed, removeChoice([])).kind).toBe("changed");
    expect(failedOutcome(changed, { worktrees: "keep" }).kind).toBe("refused");
  });
});
