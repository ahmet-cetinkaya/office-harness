// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { sessionResultText } from "./session-end";

describe("session results", () => {
  test("a sign-in end and a start that never ran read as words", () => {
    expect(
      sessionResultText("not-signed-in: gemini is not signed in: its first"),
    ).toBe("not signed in. gemini is not signed in: its first");
    expect(sessionResultText("spawn-error: claude is not signed in")).toBe(
      "never started: claude is not signed in",
    );
  });

  test("the other results stay as the server sends them", () => {
    expect(sessionResultText("0")).toBe("0");
    expect(sessionResultText("signal")).toBe("signal");
    expect(sessionResultText("lost")).toBe(
      "lost: the server stopped uncleanly",
    );
    expect(sessionResultText(null)).toBe("unknown");
  });
});
