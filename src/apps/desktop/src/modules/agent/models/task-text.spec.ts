// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  TASK_LINE_MAX_GRAPHEMES,
  taskAge,
  taskLine,
  taskRowText,
  visibleText,
} from "./task-text";

const MINUTE = 60_000;

describe("visibleText", () => {
  test("shows ESC, BEL, and DEL as control pictures", () => {
    expect(visibleText("a\u001b[31mb")).toBe("a␛[31mb");
    expect(visibleText("\u0007")).toBe("␇");
    expect(visibleText("\u007f")).toBe("␡");
  });

  test("shows a C1 code as an escape", () => {
    expect(visibleText("\u009b")).toBe("\\u009B");
  });

  test("shows bidirectional format characters as escapes", () => {
    expect(visibleText("a‮b")).toBe("a\\u202Eb");
    expect(visibleText("⁦x⁩")).toBe("\\u2066x\\u2069");
  });

  test("keeps plain text, emoji, and line feeds", () => {
    expect(visibleText("Fix it 👍\nnext")).toBe("Fix it 👍\nnext");
  });
});

describe("taskLine", () => {
  test("takes the first line of a multi-line text and marks the cut", () => {
    expect(taskLine("first\nsecond")).toBe("first…");
    expect(taskLine("first\r\nsecond")).toBe("first…");
    expect(taskLine("first\rsecond")).toBe("first…");
  });

  test("adds no ellipsis when only blank lines follow", () => {
    expect(taskLine("first\n")).toBe("first");
    expect(taskLine("first\n  \n")).toBe("first");
  });

  test("adds one ellipsis to a long first line with more lines", () => {
    const text = "a".repeat(TASK_LINE_MAX_GRAPHEMES + 1) + "\nmore";
    expect(taskLine(text)).toBe("a".repeat(TASK_LINE_MAX_GRAPHEMES) + "…");
  });

  test("shows exactly 80 graphemes whole", () => {
    const text = "a".repeat(TASK_LINE_MAX_GRAPHEMES);
    expect(taskLine(text)).toBe(text);
  });

  test("cuts 81 graphemes to 80 and adds an ellipsis", () => {
    const text = "a".repeat(TASK_LINE_MAX_GRAPHEMES + 1);
    expect(taskLine(text)).toBe("a".repeat(TASK_LINE_MAX_GRAPHEMES) + "…");
  });

  test("counts an emoji as one grapheme", () => {
    const text = "👨‍👩‍👧".repeat(TASK_LINE_MAX_GRAPHEMES);
    expect(taskLine(text)).toBe(text);
  });

  test("shows an escape sequence as one visible line", () => {
    expect(taskLine("\u001b[31mred\nmore")).toBe("␛[31mred…");
  });
});

describe("taskAge", () => {
  test("counts minutes up to 59", () => {
    expect(taskAge(0, 0)).toBe("0 min ago");
    expect(taskAge(0, 59 * MINUTE)).toBe("59 min ago");
  });

  test("counts hours from 60 minutes", () => {
    expect(taskAge(0, 60 * MINUTE)).toBe("1 h ago");
    expect(taskAge(0, 125 * MINUTE)).toBe("2 h ago");
  });

  test("reads a time in the future as 0", () => {
    expect(taskAge(5 * MINUTE, 0)).toBe("0 min ago");
  });
});

describe("taskRowText", () => {
  test("joins the age and the line", () => {
    expect(taskRowText({ text: "Fix login", set_at: 0 }, 3 * MINUTE)).toBe(
      "Task given 3 min ago: Fix login",
    );
  });
});
