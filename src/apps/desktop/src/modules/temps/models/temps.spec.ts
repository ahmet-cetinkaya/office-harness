// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  durationText,
  keptText,
  NOTHING_KEPT_TEXT,
  quietText,
  readTempList,
  reasonText,
  reportLine,
  type FinishedTemp,
} from "./temps";

function finished(overrides: Partial<FinishedTemp> = {}): FinishedTemp {
  return {
    id: "f1",
    agent_id: "a1",
    name: "Scout",
    objective: "Find the bug",
    creator: "user",
    creator_name: "the person",
    parent_name: null,
    started_at: 1,
    ended_at: 2,
    reason: "done",
    report: null,
    kept_folder: null,
    kept_branch: null,
    removal_failure: null,
    ...overrides,
  };
}

describe("durationText", () => {
  test("seconds, minutes, and hours read as short text", () => {
    expect(durationText(0)).toBe("0 s");
    expect(durationText(59)).toBe("59 s");
    expect(durationText(60)).toBe("1 min");
    expect(durationText(12 * 60 + 30)).toBe("12 min");
    expect(durationText(3600)).toBe("1 h");
    expect(durationText(3600 + 5 * 60)).toBe("1 h 5 min");
  });

  test("a negative span reads as zero", () => {
    expect(durationText(-5)).toBe("0 s");
  });
});

describe("quietText", () => {
  test("a temp that does not run has no quiet time", () => {
    expect(quietText(null)).toBe("not running");
    expect(quietText(720)).toBe("12 min");
  });
});

describe("reasonText", () => {
  test("it keeps the server's words with a capital first letter", () => {
    expect(reasonText("done")).toBe("Done");
    expect(reasonText("no output for 30 minutes")).toBe(
      "No output for 30 minutes",
    );
    expect(reasonText("exited with code 2")).toBe("Exited with code 2");
    expect(reasonText("")).toBe("");
  });
});

describe("reportLine and keptText", () => {
  test("a report shows its kind and text; none shows nothing", () => {
    expect(reportLine(finished())).toBeNull();
    expect(
      reportLine(finished({ report: { kind: "done", text: "All fixed" } })),
    ).toBe("Done report: All fixed");
  });

  test("nothing kept says that git ignored files were removed", () => {
    expect(keptText(finished())).toBe(NOTHING_KEPT_TEXT);
    expect(NOTHING_KEPT_TEXT).toBe(
      "nothing kept; files that git ignores were removed with the worktree",
    );
  });

  test("a kept folder and branch are named, with a removal failure", () => {
    const kept = finished({ kept_folder: "/w/a", kept_branch: "oh/a" });
    expect(keptText(kept)).toBe("Kept folder /w/a, branch oh/a");
    expect(keptText({ ...kept, removal_failure: "file locked" })).toBe(
      "Kept folder /w/a, branch oh/a. The worktree was not removed: file locked",
    );
  });
});

describe("readTempList", () => {
  test("a malformed answer is an empty list", () => {
    expect(readTempList(null)).toEqual({ temps: [], finished: [] });
    expect(readTempList({ temps: 3 })).toEqual({ temps: [], finished: [] });
  });
});
