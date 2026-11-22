// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  childPath,
  contentLines,
  decodeContent,
  NO_NEWLINE_MARKER,
  parseDiff,
  tooLargeText,
  visibleBlock,
  visibleLine,
} from "./file-view";

function base64(bytes: number[]): string {
  return btoa(String.fromCharCode(...bytes));
}

describe("decodeContent", () => {
  test("decodes UTF-8 text", () => {
    const bytes = new TextEncoder().encode("héllo 👍");
    expect(decodeContent(base64([...bytes]))).toBe("héllo 👍");
  });

  test("replaces each invalid sequence with U+FFFD", () => {
    expect(decodeContent(base64([0x61, 0xff, 0x62, 0xc3, 0x28]))).toBe("a�b�(");
  });

  test("decodes an empty file", () => {
    expect(decodeContent("")).toBe("");
  });
});

describe("visibleLine", () => {
  test("keeps tabs", () => {
    expect(visibleLine("\tif (a)\t{")).toBe("\tif (a)\t{");
  });

  test("shows ESC, DEL, and a C1 code as symbols", () => {
    expect(visibleLine("a\u001b[0mb")).toBe("a␛[0mb");
    expect(visibleLine("\u007f")).toBe("␡");
    expect(visibleLine("\u009b")).toBe("\\u009B");
  });

  test("shows a right-to-left override as an escape", () => {
    expect(visibleLine("a‮b")).toBe("a\\u202Eb");
  });

  test("drops one carriage return at the end of the line", () => {
    expect(visibleLine("text\r")).toBe("text");
  });

  test("shows a carriage return inside the line", () => {
    expect(visibleLine("a\rb")).toBe("a␍b");
  });
});

describe("contentLines", () => {
  test("splits on line feeds and drops the one after the last line", () => {
    expect(contentLines("a\nb\n")).toEqual(["a", "b"]);
    expect(contentLines("a\n\nb")).toEqual(["a", "", "b"]);
  });

  test("drops the carriage return of a Windows line break", () => {
    expect(contentLines("a\r\nb\r\n")).toEqual(["a", "b"]);
  });

  test("gives an empty file no line", () => {
    expect(contentLines("")).toEqual([]);
  });
});

describe("visibleBlock", () => {
  test("keeps the line breaks and shows control characters", () => {
    expect(visibleBlock("one\u001b\n\ntwo\n")).toBe("one␛\n\ntwo");
  });
});

describe("parseDiff", () => {
  const DIFF = [
    "@@ -1,3 +1,4 @@ fn main",
    " first",
    "-second",
    "+second changed",
    "+inserted",
    " third",
    "@@ -10,2 +11 @@",
    "-old tail",
    "\\ No newline at end of file",
    "+new tail",
    "\\ No newline at end of file",
    "",
  ].join("\n");

  test("numbers added, removed, and context lines", () => {
    const lines = parseDiff(DIFF);
    expect(
      lines.map((line) => [line.kind, line.oldNumber, line.newNumber]),
    ).toEqual([
      ["hunk", null, null],
      ["context", 1, 1],
      ["removed", 2, null],
      ["added", null, 2],
      ["added", null, 3],
      ["context", 3, 4],
      ["hunk", null, null],
      ["removed", 10, null],
      ["note", null, null],
      ["added", null, 11],
      ["note", null, null],
    ]);
  });

  test("keeps the hunk header and the no-newline marker as text", () => {
    const lines = parseDiff(DIFF);
    expect(lines[0].text).toBe("@@ -1,3 +1,4 @@ fn main");
    expect(lines[8].text).toBe(NO_NEWLINE_MARKER);
  });

  test("removes the sign from the text of a line", () => {
    const lines = parseDiff(DIFF);
    expect(lines[2].text).toBe("second");
    expect(lines[3].text).toBe("second changed");
  });

  test("reads a blank context line whose space was cut", () => {
    const lines = parseDiff("@@ -1,3 +1,3 @@\n a\n\n b\n");
    expect(lines.map((line) => line.kind)).toEqual([
      "hunk",
      "context",
      "context",
      "context",
    ]);
    expect(lines[3].oldNumber).toBe(3);
  });

  test("shows control characters of a line as symbols", () => {
    const [, added] = parseDiff("@@ -0,0 +1 @@\n+a\u001b‮\r\n");
    expect(added.text).toBe("a␛\\u202E");
  });

  test("gives an empty diff no line", () => {
    expect(parseDiff("")).toEqual([]);
  });
});

describe("tooLargeText", () => {
  test("names the size and the limit", () => {
    expect(tooLargeText("This file", 1_048_576)).toBe(
      "This file is 1,048,576 bytes. Content over 512 KiB is not shown.",
    );
  });

  test("names the limit when the size is unknown", () => {
    expect(tooLargeText("This change", null)).toBe(
      "This change is over the limit. Content over 512 KiB is not shown.",
    );
  });
});

describe("childPath", () => {
  test("joins with a slash and leaves the root out", () => {
    expect(childPath("", "src")).toBe("src");
    expect(childPath("src", "main.txt")).toBe("src/main.txt");
  });
});
