// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  confirmBlock,
  interruptedCount,
  marksOf,
  planText,
  readPlan,
  readResult,
  resultText,
  type PlanEntry,
  type ResumeResult,
} from "./resume";

function entry(overrides: Partial<PlanEntry> = {}): PlanEntry {
  return {
    agent_id: "ada",
    lastSessionStartedAt: 1_700_000_000_000,
    canContinue: true,
    reason: null,
    isHeld: false,
    isAfterUncleanStop: false,
    ...overrides,
  };
}

function result(overrides: Partial<ResumeResult> = {}): ResumeResult {
  return {
    agent_id: "ada",
    result: "continued",
    continued: false,
    reason: null,
    isFinal: true,
    ...overrides,
  };
}

describe("interrupted count", () => {
  test("counts only the interrupted agents", () => {
    const agents = [
      { status: "interrupted" },
      { status: "working" },
      { status: "interrupted" },
    ];
    expect(interruptedCount(agents)).toBe(2);
    expect(interruptedCount([])).toBe(0);
  });
});

describe("plan", () => {
  test("reads entries in the order sent and drops bad ones", () => {
    const plan = readPlan({
      agents: [
        {
          agent_id: "b",
          last_session: { id: "s1", started_at: 5 },
          can_continue: false,
          reason: "gone",
          held: true,
          after_unclean_stop: true,
        },
        { nope: 1 },
        { agent_id: "a", can_continue: true, held: false },
      ],
    });
    expect(plan.map((item) => item.agent_id)).toEqual(["b", "a"]);
    expect(plan[0]).toEqual({
      agent_id: "b",
      lastSessionStartedAt: 5,
      canContinue: false,
      reason: "gone",
      isHeld: true,
      isAfterUncleanStop: true,
    });
    expect(readPlan(null)).toEqual([]);
  });

  test("the plan text names a continue with its start time, or a new session with its reason", () => {
    expect(planText(entry())).toStartWith("Continues the session started ");
    expect(planText(entry({ canContinue: false, reason: "no id" }))).toBe(
      "Starts a new session: no id",
    );
    expect(planText(entry({ canContinue: false }))).toBe(
      "Starts a new session",
    );
  });

  test("marks say on hold and the unclean stop", () => {
    expect(marksOf(entry())).toEqual([]);
    expect(marksOf(entry({ isHeld: true, isAfterUncleanStop: true }))).toEqual([
      "On hold",
      "Was running when the server stopped uncleanly",
    ]);
  });
});

describe("result", () => {
  test("reads an event, and ignores one with an unknown result", () => {
    expect(
      readResult({
        type: "resume.result",
        agent_id: "ada",
        result: "held",
        reason: "x",
        is_final: true,
      }),
    ).toEqual(result({ result: "held", reason: "x" }));
    expect(readResult({ agent_id: "ada", result: "surprise" })).toBeNull();
  });

  test("each result is text", () => {
    expect(resultText(result({ result: "checking", continued: true }))).toBe(
      "Checking: continuing a session",
    );
    expect(resultText(result({ result: "checking" }))).toBe(
      "Checking: starting a new session",
    );
    expect(resultText(result())).toBe("Continued");
    expect(resultText(result({ result: "new_session", reason: "r" }))).toBe(
      "New session: r",
    );
    expect(resultText(result({ result: "held", reason: "r" }))).toBe(
      "On hold: r",
    );
    expect(resultText(result({ result: "skipped", reason: "r" }))).toBe(
      "Skipped: r",
    );
    expect(resultText(result({ result: "skipped" }))).toBe("Skipped");
  });
});

describe("confirm block", () => {
  const entries = [
    entry(),
    entry({ agent_id: "bo", isAfterUncleanStop: true }),
  ];

  test("nothing checked blocks", () => {
    expect(confirmBlock(entries, [], true)).toBe("nothing_checked");
  });

  test("a checked marked agent needs the acknowledgement", () => {
    expect(confirmBlock(entries, ["bo"], false)).toBe("needs_acknowledgement");
    expect(confirmBlock(entries, ["bo"], true)).toBeNull();
  });

  test("an unmarked selection needs no acknowledgement", () => {
    expect(confirmBlock(entries, ["ada"], false)).toBeNull();
  });
});
