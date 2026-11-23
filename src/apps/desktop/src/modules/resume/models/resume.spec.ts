// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  confirmBlock,
  endedLeftoversText,
  hasUncleanStopMark,
  isSweepingPlan,
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
    uncleanStop: { kind: "none" },
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
      uncleanStop: { kind: "may_run", reason: null },
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
    expect(
      marksOf(
        entry({ isHeld: true, uncleanStop: { kind: "may_run", reason: null } }),
      ),
    ).toEqual(["On hold", "Was running when the server stopped uncleanly"]);
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
    entry({ agent_id: "bo", uncleanStop: { kind: "may_run", reason: null } }),
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

describe("sweep finding (0017, D8, D9)", () => {
  const findingOf = (fields: Record<string, unknown>) =>
    readPlan({ agents: [{ agent_id: "ada", ...fields }] })[0].uncleanStop;

  test("reads the three findings", () => {
    expect(findingOf({ unclean_stop: null })).toEqual({ kind: "none" });
    expect(findingOf({ unclean_stop: { ended: 2 } })).toEqual({
      kind: "ended",
      count: 2,
    });
    expect(findingOf({ unclean_stop: { may_run: "could not end" } })).toEqual({
      kind: "may_run",
      reason: "could not end",
    });
  });

  test("an older server's boolean counts as may_run, a newer finding wins", () => {
    expect(findingOf({ after_unclean_stop: true })).toEqual({
      kind: "may_run",
      reason: null,
    });
    expect(findingOf({ after_unclean_stop: false })).toEqual({ kind: "none" });
    expect(
      findingOf({ unclean_stop: { ended: 1 }, after_unclean_stop: false }),
    ).toEqual({ kind: "ended", count: 1 });
  });

  test("the ended note has the right grammar and no warning", () => {
    expect(endedLeftoversText(1)).toBe(
      "Ended 1 process left from the last run",
    );
    expect(endedLeftoversText(3)).toBe(
      "Ended 3 processes left from the last run",
    );
    const ended = entry({ uncleanStop: { kind: "ended", count: 1 } });
    expect(marksOf(ended)).toEqual(["Ended 1 process left from the last run"]);
    expect(hasUncleanStopMark([ended])).toBe(false);
  });

  test("may_run gives the 0013 mark with the reason, and nothing gives no mark", () => {
    expect(
      marksOf(entry({ uncleanStop: { kind: "may_run", reason: "denied" } })),
    ).toEqual(["Was running when the server stopped uncleanly: denied"]);
    expect(marksOf(entry())).toEqual([]);
  });

  test("only may_run gates Confirm", () => {
    const ended = entry({
      agent_id: "a",
      uncleanStop: { kind: "ended", count: 1 },
    });
    const none = entry({ agent_id: "b" });
    const mayRun = entry({
      agent_id: "c",
      uncleanStop: { kind: "may_run", reason: "x" },
    });
    expect(confirmBlock([ended, none], ["a", "b"], false)).toBeNull();
    expect(confirmBlock([ended, none, mayRun], ["a", "c"], false)).toBe(
      "needs_acknowledgement",
    );
    expect(confirmBlock([ended, none, mayRun], ["a", "c"], true)).toBeNull();
    expect(confirmBlock([ended, none, mayRun], ["a", "b"], false)).toBeNull();
    expect(hasUncleanStopMark([ended, none, mayRun])).toBe(true);
  });

  test("a sweeping plan is told by its flag", () => {
    expect(isSweepingPlan({ sweeping: true, agents: [] })).toBe(true);
    expect(isSweepingPlan({ sweeping: false, agents: [] })).toBe(false);
    expect(isSweepingPlan({ agents: [] })).toBe(false);
    expect(isSweepingPlan(null)).toBe(false);
  });
});
