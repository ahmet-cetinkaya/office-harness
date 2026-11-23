// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  formProblem,
  IDLE_MINUTES_INVALID_MESSAGE,
  isWakeBusy,
  minutesProblem,
  readIdleSleepForm,
  readWakeResult,
  settingValueOf,
  wakeText,
  type WakeResult,
} from "./sleep";

function result(overrides: Partial<WakeResult> = {}): WakeResult {
  return {
    agent_id: "ada",
    result: "continued",
    continued: true,
    reason: null,
    isFinal: true,
    ...overrides,
  };
}

describe("idle time validation", () => {
  test("a whole number from 5 to 1440 is accepted", () => {
    for (const text of ["5", "30", " 45 ", "1440"]) {
      expect(minutesProblem(text)).toBeNull();
    }
  });

  test("everything else is refused with the server's text", () => {
    for (const text of ["", "4", "1441", "0", "-5", "1.5", "abc", "1e3"]) {
      expect(minutesProblem(text)).toBe(IDLE_MINUTES_INVALID_MESSAGE);
    }
  });

  test("off needs no valid minutes", () => {
    expect(formProblem({ isOn: false, minutes: "abc" })).toBeNull();
    expect(formProblem({ isOn: true, minutes: "abc" })).toBe(
      IDLE_MINUTES_INVALID_MESSAGE,
    );
  });
});

describe("the form and the setting", () => {
  test("off is 0 and shows 30 minutes", () => {
    expect(readIdleSleepForm("0")).toEqual({ isOn: false, minutes: "30" });
    expect(readIdleSleepForm("")).toEqual({ isOn: false, minutes: "30" });
  });

  test("a stored number is on with that number", () => {
    expect(readIdleSleepForm("45")).toEqual({ isOn: true, minutes: "45" });
  });

  test("the stored value is 0 for off and the trimmed minutes for on", () => {
    expect(settingValueOf({ isOn: false, minutes: "45" })).toBe("0");
    expect(settingValueOf({ isOn: true, minutes: " 45 " })).toBe("45");
  });
});

describe("wake results", () => {
  test("each kind reads as text", () => {
    expect(wakeText({ kind: "pending" })).toBe("Waking");
    expect(
      wakeText({ kind: "result", result: result({ result: "checking" }) }),
    ).toBe("Checking");
    expect(wakeText({ kind: "result", result: result() })).toBe("Continued");
    expect(
      wakeText({
        kind: "result",
        result: result({ result: "new_session", reason: "no session id" }),
      }),
    ).toBe("New session: no session id");
    expect(
      wakeText({
        kind: "result",
        result: result({ result: "held", reason: "ended at once" }),
      }),
    ).toBe("On hold: ended at once");
    expect(wakeText({ kind: "failure", text: "refused" })).toBe("refused");
  });

  test("a reason that is missing leaves the lead alone", () => {
    expect(
      wakeText({ kind: "result", result: result({ result: "held" }) }),
    ).toBe("On hold");
  });

  test("only waking and checking are busy", () => {
    expect(isWakeBusy(undefined)).toBe(false);
    expect(isWakeBusy({ kind: "pending" })).toBe(true);
    expect(
      isWakeBusy({ kind: "result", result: result({ result: "checking" }) }),
    ).toBe(true);
    expect(isWakeBusy({ kind: "result", result: result() })).toBe(false);
    expect(isWakeBusy({ kind: "failure", text: "x" })).toBe(false);
  });

  test("the event reads, and a bad event is none", () => {
    expect(
      readWakeResult({
        type: "wake.result",
        agent_id: "ada",
        result: "new_session",
        reason: "why",
        is_final: true,
      }),
    ).toEqual(
      result({ result: "new_session", reason: "why", continued: false }),
    );
    expect(readWakeResult({ agent_id: "ada", result: "skipped" })).toBeNull();
    expect(readWakeResult(null)).toBeNull();
  });
});
