// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  creatorText,
  OBJECTIVE_MAX_CHARACTERS,
  OBJECTIVE_ONE_LINE_TEXT,
  OBJECTIVE_REQUIRED_TEXT,
  objectiveProblem,
} from "./agent";

describe("objectiveProblem", () => {
  test("a blank objective is refused with the required text", () => {
    expect(objectiveProblem("")).toBe(OBJECTIVE_REQUIRED_TEXT);
    expect(objectiveProblem("  \n ")).toBe(OBJECTIVE_REQUIRED_TEXT);
    expect(OBJECTIVE_REQUIRED_TEXT).toBe("A temp needs an objective");
  });

  test("a line break inside the objective is refused, one around it is not", () => {
    expect(objectiveProblem("one\ntwo")).toBe(OBJECTIVE_ONE_LINE_TEXT);
    expect(objectiveProblem("one\r\ntwo")).toBe(OBJECTIVE_ONE_LINE_TEXT);
    expect(objectiveProblem("one\n")).toBeNull();
    expect(OBJECTIVE_ONE_LINE_TEXT).toBe("The objective must be one line.");
  });

  test("1 and 8,192 characters pass, 8,193 is refused", () => {
    expect(objectiveProblem("x")).toBeNull();
    expect(objectiveProblem("x".repeat(OBJECTIVE_MAX_CHARACTERS))).toBeNull();
    expect(
      objectiveProblem("x".repeat(OBJECTIVE_MAX_CHARACTERS + 1)),
    ).not.toBeNull();
  });
});

describe("creatorText", () => {
  const agents = [{ id: "a1", name: "Lead" }];

  test("the person, a known agent, and an agent that is gone", () => {
    expect(creatorText("user", agents)).toBe("the person");
    expect(creatorText("a1", agents)).toBe("Lead");
    expect(creatorText("a9", agents)).toBe("a9");
  });
});
