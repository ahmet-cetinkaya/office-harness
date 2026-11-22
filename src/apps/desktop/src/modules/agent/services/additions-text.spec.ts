// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  formatEnvironment,
  formatFlags,
  parseEnvironment,
  parseFlags,
  parseStartupPrompt,
} from "./additions-text";

describe("additions text", () => {
  test("a flag is one non-empty line", () => {
    expect(parseFlags("--model\n  opus \n\n")).toEqual(["--model", "opus"]);
  });

  test("environment lines split at the first equals sign", () => {
    expect(parseEnvironment("A=1\nURL=a=b")).toEqual({
      ok: true,
      entries: [
        { name: "A", value: "1" },
        { name: "URL", value: "a=b" },
      ],
    });
  });

  test("a bare NAME is no value and NAME= is an empty value", () => {
    expect(parseEnvironment("API_KEY\nEMPTY=")).toEqual({
      ok: true,
      entries: [
        { name: "API_KEY", value: null },
        { name: "EMPTY", value: "" },
      ],
    });
  });

  test("a line without a name is named in the message", () => {
    const parsed = parseEnvironment("=value");
    expect(parsed.ok).toBe(false);
  });

  test("formatting then parsing returns the same lists", () => {
    const entries = [
      { name: "A", value: "x y" },
      { name: "NO_VALUE", value: null },
      { name: "EMPTY", value: "" },
    ];
    expect(parseEnvironment(formatEnvironment(entries))).toEqual({
      ok: true,
      entries,
    });
    expect(parseFlags(formatFlags(["-a", "b"]))).toEqual(["-a", "b"]);
  });

  test("an empty prompt means no prompt", () => {
    expect(parseStartupPrompt("  \n")).toBeNull();
    expect(parseStartupPrompt("Be brief.\n")).toBe("Be brief.\n");
  });
});
