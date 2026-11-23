// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  readSweepState,
  SWEEP_FINISHED_TEXT,
  SWEEP_RUNNING_TEXT,
  sweepStatusText,
} from "./sweep";

describe("sweep state", () => {
  test("reads running and done, and nothing else", () => {
    expect(readSweepState({ state: "running" })).toBe("running");
    expect(readSweepState({ state: "done" })).toBe("done");
    expect(readSweepState({ state: "later" })).toBeNull();
    expect(readSweepState(null)).toBeNull();
  });

  test("the status text is shown only while running, and spoken after it", () => {
    expect(sweepStatusText("idle")).toBe("");
    expect(sweepStatusText("running")).toBe(SWEEP_RUNNING_TEXT);
    expect(sweepStatusText("finished")).toBe(SWEEP_FINISHED_TEXT);
    expect(SWEEP_RUNNING_TEXT).toBe("Ending processes left from the last run");
  });
});
