// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  countAnnouncement,
  eventText,
  readItems,
  rowOf,
  waitText,
  type NeedsYouItem,
} from "./needs-you";

function item(overrides: Partial<NeedsYouItem> = {}): NeedsYouItem {
  return {
    agent_id: "ada",
    kind: "bell",
    detail: null,
    first_event_at: 1_000,
    latest_at: 1_000,
    origin_notification_id: null,
    report: null,
    ...overrides,
  };
}

describe("needs-you items", () => {
  test("the items of an answer keep the server's order", () => {
    const answer = {
      items: [
        { agent_id: "b", kind: "failed", first_event_at: 5 },
        { agent_id: "a", kind: "exited", first_event_at: 1 },
      ],
    };

    expect(readItems(answer).map((read) => read.agent_id)).toEqual(["b", "a"]);
    expect(readItems(answer)[0].detail).toBeNull();
  });

  test("bad input is no items", () => {
    expect(readItems(undefined)).toEqual([]);
    expect(readItems({ items: "no" })).toEqual([]);
    expect(
      readItems({ items: [{ agent_id: "a", kind: "unknown" }, 3] }),
    ).toEqual([]);
  });
});

describe("event text", () => {
  test("a waiting agent shows the quiet period in seconds", () => {
    expect(eventText(item({ kind: "waiting_for_input", detail: "5000" }))).toBe(
      "quiet for 5 s",
    );
  });

  test("a waiting agent without a usable period still says it waits", () => {
    expect(eventText(item({ kind: "waiting_for_input" }))).toBe(
      "waits for your input",
    );
    expect(eventText(item({ kind: "waiting_for_input", detail: "soon" }))).toBe(
      "waits for your input",
    );
  });

  test("a failed agent shows the server's text as sent", () => {
    expect(
      eventText(
        item({
          kind: "failed",
          detail: "the command ended with exit code 2",
        }),
      ),
    ).toBe("failed: the command ended with exit code 2");
    expect(eventText(item({ kind: "failed" }))).toBe("failed");
  });

  test("the other kinds are named", () => {
    expect(eventText(item({ kind: "exited" }))).toBe("exited");
    expect(eventText(item({ kind: "bell" }))).toBe("bell");
    expect(eventText(item({ kind: "resume_hold", detail: "ended" }))).toBe(
      "was put on hold after its resume failed: ended",
    );
    expect(
      eventText(item({ kind: "leftover_kept", detail: "permission denied" })),
    ).toBe("has a leftover process that could not be ended: permission denied");
    expect(eventText(item({ kind: "leftover_kept" }))).toBe(
      "has a leftover process that could not be ended",
    );
    expect(
      readItems({
        items: [{ agent_id: "a", kind: "leftover_kept", first_event_at: 1 }],
      }),
    ).toHaveLength(1);
    expect(eventText(item({ kind: "auto_hold", detail: "rate" }))).toBe(
      "put on hold by the office, limit: rate",
    );
  });
});

describe("wait time", () => {
  const first = 1_000_000;

  test("counts from the first event, in the largest fitting unit", () => {
    expect(waitText(first, first + 12_000)).toBe("waiting 12 s");
    expect(waitText(first, first + 3 * 60_000 + 5_000)).toBe("waiting 3 min");
    expect(waitText(first, first + (2 * 60 + 5) * 60_000)).toBe(
      "waiting 2 h 5 min",
    );
  });

  test("a clock behind the event shows no negative time", () => {
    expect(waitText(first, first - 5_000)).toBe("waiting 0 s");
  });
});

describe("reports", () => {
  const report = { kind: "done", text: "All tests pass", at: 2_000 };
  const base = { agent_id: "a", kind: "report", first_event_at: 1 };

  test("an item reads its report, and an item without one reads null", () => {
    const [withReport, without] = readItems({
      items: [{ ...base, detail: "done", report }, base],
    });

    expect(withReport.report).toEqual(report);
    expect(without.report).toBeNull();
  });

  test("a malformed report reads as null", () => {
    const malformed = [
      "text",
      { kind: "done", text: "x" },
      { kind: 1, text: "x", at: 2 },
      { kind: "done", text: 2, at: 2 },
    ];

    for (const bad of malformed) {
      expect(
        readItems({ items: [{ ...base, report: bad }] })[0].report,
      ).toBeNull();
    }
  });

  test("the event says the agent sent a report of its kind", () => {
    expect(eventText(item({ kind: "report", detail: "blocked" }))).toBe(
      "sent a report: blocked",
    );
    expect(eventText(item({ kind: "report" }))).toBe("sent a report");
  });

  test("a row shows the report as text and keeps the label to the event", () => {
    const row = rowOf(
      item({ kind: "report", detail: "done", report }),
      "Ada",
      null,
      1_000,
    );

    expect(row.reportText).toBe("done: All tests pass");
    expect(row.isReportLong).toBe(false);
    expect(row.label).toBe("Ada, sent a report: done, waiting 0 s");
  });

  test("a report over 160 characters is long, 160 is not", () => {
    const rowWith = (length: number) =>
      rowOf(
        item({ report: { ...report, text: "x".repeat(length) } }),
        "Ada",
        null,
        1_000,
      );

    expect(rowWith(160).isReportLong).toBe(false);
    expect(rowWith(161).isReportLong).toBe(true);
  });

  test("a row without a report has no report text", () => {
    expect(rowOf(item(), "Ada", null, 1_000).reportText).toBeNull();
  });

  test("the report stays on the row when a later event changes the kind", () => {
    const row = rowOf(
      item({ kind: "failed", detail: "exit 2", report }),
      "Ada",
      null,
      1_000,
    );

    expect(row.event).toBe("failed: exit 2");
    expect(row.reportText).toBe("done: All tests pass");
  });
});

describe("row", () => {
  test("the accessible name holds name, project, event, and wait time", () => {
    const row = rowOf(
      item({ kind: "waiting_for_input", detail: "5000" }),
      "Ada",
      "Site",
      4_000,
    );

    expect(row.label).toBe("Ada, project Site, quiet for 5 s, waiting 3 s");
  });

  test("an agent without a project has no project part", () => {
    expect(rowOf(item(), "Ada", null, 1_000).label).toBe(
      "Ada, bell, waiting 0 s",
    );
  });
});

describe("count announcement", () => {
  test("says the count, or that the list is empty", () => {
    expect(countAnnouncement(1)).toBe("Needs you: 1 agent");
    expect(countAnnouncement(3)).toBe("Needs you: 3 agents");
    expect(countAnnouncement(0)).toBe("Needs you is empty");
  });
});
