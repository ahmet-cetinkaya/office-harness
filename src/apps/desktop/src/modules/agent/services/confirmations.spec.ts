// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { confirmationText, needsConfirmation } from "./confirmations";

describe("confirmations", () => {
  test("stop and delete always ask", () => {
    expect(needsConfirmation({ kind: "stop" })).toBe(true);
    expect(needsConfirmation({ kind: "delete" })).toBe(true);
  });

  test("a new task asks only when it ends a running session", () => {
    expect(needsConfirmation({ kind: "new_task", isRunning: true })).toBe(true);
    expect(needsConfirmation({ kind: "new_task", isRunning: false })).toBe(
      false,
    );
  });

  test("a continue asks only when it ends a running session", () => {
    const running = {
      kind: "continue",
      sessionId: "s",
      isRunning: true,
    } as const;
    expect(needsConfirmation(running)).toBe(true);
    expect(needsConfirmation({ ...running, isRunning: false })).toBe(false);
  });

  test("the text names the agent and the running session", () => {
    const text = confirmationText({ kind: "new_task", isRunning: true }, "Ada");
    expect(text.title).toContain("Ada");
    expect(text.body).toContain("running session ends");
    expect(text.isDestructive).toBe(true);
  });
});
