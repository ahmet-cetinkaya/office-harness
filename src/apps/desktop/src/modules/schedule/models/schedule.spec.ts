// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  dateText,
  draftOf,
  draftProblems,
  dueText,
  emptyDraft,
  offReasonText,
  problemText,
  readHistory,
  readScheduleList,
  repeatWords,
  rowDescription,
  rowName,
  saveFields,
  switchText,
  type RunRecord,
  type Schedule,
} from "./schedule";

const ZONE = "UTC";
// Monday, 24 November 2025, 09:30 UTC.
const MONDAY_MORNING = Date.UTC(2025, 10, 24, 9, 30);

function schedule(overrides: Partial<Schedule> = {}): Schedule {
  return {
    id: "s1",
    name: "nightly",
    text: "run the tests",
    target_agent_id: "a1",
    repeat: { kind: "daily", time: "09:00" },
    on: true,
    next_due: MONDAY_MORNING,
    last_handled_due: null,
    last_missed: null,
    off_reason: null,
    ...overrides,
  };
}

describe("repeat in words", () => {
  test("each kind reads as a sentence", () => {
    expect(repeatWords({ kind: "daily", time: "09:00" })).toBe(
      "every day at 09:00",
    );
    expect(repeatWords({ kind: "every_hours", hours: 6 })).toBe(
      "every 6 hours",
    );
    expect(repeatWords({ kind: "every_hours", hours: 1 })).toBe("every hour");
    expect(
      repeatWords({ kind: "weekly", days: ["fri", "mon"], time: "17:30" }),
    ).toBe("every Monday, Friday at 17:30");
    expect(
      repeatWords({ kind: "once", date: "2025-11-24", time: "08:00" }, "en-US"),
    ).toBe("once on Mon, Nov 24, 2025 at 08:00");
  });

  test("a date that is not a date stays as it is", () => {
    expect(dateText("soon")).toBe("soon");
  });
});

describe("due times", () => {
  test("a due time shows weekday, date, and time in the given zone", () => {
    const text = dueText(MONDAY_MORNING, ZONE, "en-US");
    expect(text).toContain("Mon");
    expect(text).toContain("Nov 24, 2025");
    expect(text).toContain("09:30");
  });

  test("the same instant moves to another zone's day and time", () => {
    const text = dueText(
      Date.UTC(2025, 10, 24, 11, 0),
      "Pacific/Kiritimati",
      "en-US",
    );
    expect(text).toContain("Tue");
    expect(text).toContain("01:00");
  });

  test("no due time reads as none", () => {
    expect(dueText(null)).toBe("none");
  });
});

describe("rows", () => {
  const last: RunRecord = {
    id: 1,
    schedule_id: "s1",
    due: MONDAY_MORNING,
    started: MONDAY_MORNING,
    via: "due",
    result: "skipped",
    reason: "stopped",
  };

  test("the name holds the name, the switch as text, and the next run", () => {
    const name = rowName(schedule({ on: false }), ZONE);
    expect(name.startsWith("nightly, off, next run ")).toBe(true);
    expect(switchText(schedule())).toBe("on");
  });

  test("the description holds the action, the repeat, and the last result", () => {
    const description = rowDescription(schedule(), "Ana", last, ZONE);
    expect(description).toContain('"run the tests" to Ana');
    expect(description).toContain("every day at 09:00");
    expect(description).toContain("Skipped, due");
  });

  test("an off reason reads as a sentence", () => {
    expect(offReasonText(schedule())).toBeNull();
    expect(offReasonText(schedule({ off_reason: "time_missed" }))).toContain(
      "time was missed",
    );
    expect(offReasonText(schedule({ off_reason: "target_deleted" }))).toContain(
      "target agent was deleted",
    );
  });
});

describe("answers", () => {
  test("a malformed list reads as empty", () => {
    expect(readScheduleList(null)).toEqual({ schedules: [], zone: "" });
    expect(
      readScheduleList({ schedules: [schedule()], zone: "UTC" }).zone,
    ).toBe("UTC");
  });

  test("the history keeps the newest 20", () => {
    const runs = Array.from({ length: 30 }, (_, id) => ({ id }));
    expect(readHistory({ runs })).toHaveLength(20);
    expect(readHistory({})).toEqual([]);
  });
});

describe("form", () => {
  const valid = { ...emptyDraft("a1"), name: "nightly", text: "go" };

  test("a valid draft passes", () => {
    expect(draftProblems(valid)).toEqual([]);
  });

  test("each field names its rule as the server words it", () => {
    const problems = draftProblems({
      ...valid,
      name: " ",
      text: "line\nbreak",
      targetAgentId: "",
      kind: "weekly",
    });
    expect(problems.map(problemText)).toEqual([
      "Name must not be blank",
      "Text must not have control characters",
      "Repeat a weekly repeat needs at least one day",
      "Target must name an agent that exists",
    ]);
  });

  test("a name over 100 characters is too long", () => {
    const [problem] = draftProblems({ ...valid, name: "x".repeat(101) });
    expect(problem.rule).toBe("too_long");
    expect(problem.message).toBe("must have at most 100 characters");
  });

  test("repeat rules: hours, time, and date", () => {
    const rules = (changes: Partial<typeof valid>) =>
      draftProblems({ ...valid, ...changes }).map((item) => item.rule);
    expect(rules({ kind: "every_hours", hours: "0" })).toEqual([
      "hours_out_of_range",
    ]);
    expect(rules({ kind: "every_hours", hours: "169" })).toEqual([
      "hours_out_of_range",
    ]);
    expect(rules({ kind: "every_hours", hours: "168" })).toEqual([]);
    expect(rules({ time: "9:00" })).toEqual(["bad_time"]);
    expect(rules({ time: "24:00" })).toEqual(["bad_time"]);
    expect(rules({ kind: "once", date: "2025-02-30" })).toEqual(["bad_date"]);
    expect(rules({ kind: "once", date: "2025-02-28" })).toEqual([]);
  });

  test("a save names the schedule only for an edit and sends the repeat of its kind", () => {
    const daily = saveFields(valid, null);
    expect(daily).toEqual({
      name: "nightly",
      text: "go",
      target_agent_id: "a1",
      repeat: { kind: "daily", time: "09:00" },
      on: true,
    });
    const edit = saveFields(
      { ...valid, kind: "weekly", days: ["mon"], time: "08:15" },
      "s1",
    );
    expect(edit.schedule_id).toBe("s1");
    expect(edit.repeat).toEqual({
      kind: "weekly",
      days: ["mon"],
      time: "08:15",
    });
    expect(
      saveFields({ ...valid, kind: "every_hours", hours: "6" }, null).repeat,
    ).toEqual({ kind: "every_hours", hours: 6 });
  });

  test("a draft of a schedule copies its values", () => {
    const draft = draftOf(
      schedule({ repeat: { kind: "weekly", days: ["tue"], time: "07:00" } }),
    );
    expect(draft.kind).toBe("weekly");
    expect(draft.days).toEqual(["tue"]);
    expect(draft.time).toBe("07:00");
  });
});
