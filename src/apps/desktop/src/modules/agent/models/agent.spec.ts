// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  type Agent,
  type AgentHook,
  creatorText,
  listStatusText,
  readHook,
  statusSourceText,
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
    expect(creatorText("office", agents)).toBe("the office");
    expect(creatorText("schedule:0a1b2c3d-1111", agents)).toBe(
      "schedule 0a1b2c3d",
    );
  });
});

const EXACT: AgentHook = {
  source: "exact",
  reason: null,
  ask_open: false,
  compacting: false,
};

function agentWith(hook: unknown): Agent {
  return {
    id: "a",
    name: "Ada",
    command: "claude",
    working_folder: null,
    project_id: null,
    machine_id: "m",
    status: "working",
    created_at: 1,
    last_line: null,
    hook: hook as AgentHook | null,
  };
}

describe("readHook", () => {
  test("it reads a good hook and treats a bad shape as none", () => {
    expect(readHook(EXACT)).toEqual(EXACT);
    expect(readHook(undefined)).toBeNull();
    expect(readHook({ source: "bogus" })).toBeNull();
  });
});

describe("listStatusText", () => {
  test("the ask wins over compacting, and no hook leaves the status alone", () => {
    const both = { ...EXACT, ask_open: true, compacting: true };
    expect(listStatusText(agentWith(both), false)).toBe(
      "working, asks permission",
    );
    expect(
      listStatusText(agentWith({ ...EXACT, compacting: true }), false),
    ).toBe("working, compacting");
    expect(listStatusText(agentWith(EXACT), false)).toBe("working");
    expect(listStatusText(agentWith(null), false)).toBe("working");
  });
});

describe("statusSourceText", () => {
  test("it names the source, with the reason when not exact", () => {
    expect(statusSourceText(agentWith(null))).toBeNull();
    expect(statusSourceText(agentWith(EXACT))).toBe("exact");
    const late = {
      ...EXACT,
      source: "late",
      reason: "no report for a stalled turn",
    };
    expect(statusSourceText(agentWith(late))).toBe(
      "late (no report for a stalled turn)",
    );
    expect(statusSourceText(agentWith({ ...EXACT, source: "estimated" }))).toBe(
      "estimated",
    );
  });
});
