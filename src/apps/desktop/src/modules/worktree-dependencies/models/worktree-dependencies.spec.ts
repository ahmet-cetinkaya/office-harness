// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { isSwitchOn, switchValueOf } from "./worktree-dependencies";

describe("worktree dependencies setting", () => {
  test("only off reads as off; an empty value is the default, on", () => {
    expect(isSwitchOn("off")).toBe(false);
    expect(isSwitchOn("on")).toBe(true);
    expect(isSwitchOn("")).toBe(true);
  });

  test("the switch stores on or off", () => {
    expect(switchValueOf(true)).toBe("on");
    expect(switchValueOf(false)).toBe("off");
  });
});
