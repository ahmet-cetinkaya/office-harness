// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  announcementText,
  eventText,
  isNeverReadOnArrival,
  isUnread,
  leftoverKeptBody,
  rowText,
  sortRows,
  unreadCount,
  type NotificationRow,
} from "./notification";

function row(overrides: Partial<NotificationRow> = {}): NotificationRow {
  return {
    id: "n-1",
    agent_id: "agent-ada",
    agent_name: "Ada",
    event: "exited",
    created_at: 1_000,
    read_at: null,
    ...overrides,
  };
}

describe("notification rows", () => {
  test("an event reads as the house voice words it", () => {
    expect(eventText("exited")).toBe("exited");
    expect(eventText("failed")).toBe("failed");
    expect(eventText("waiting_for_input")).toBe("waits for your input");
    expect(eventText("bell")).toBe("rang the bell");
    expect(eventText("surprise_kind")).toBe("surprise kind");
  });

  test("an automatic hold names the limit from the detail (0007 D7)", () => {
    const hold = row({
      event: "auto_hold",
      detail: "directives sent by one agent",
    });
    expect(announcementText(hold)).toBe(
      'Ada was put on hold after 3 refusals by the limit "directives sent by one agent"',
    );
    expect(rowText(hold, true)).toBe(
      'Ada, was put on hold after 3 refusals by the limit "directives sent by one agent"',
    );
  });

  test("an automatic hold without a detail still says it was put on hold", () => {
    expect(announcementText(row({ event: "auto_hold" }))).toBe(
      "Ada was put on hold after 3 refusals",
    );
  });

  test("a leftover that could not be ended names the OS reason (0017)", () => {
    expect(
      announcementText(row({ event: "leftover_kept", detail: "denied" })),
    ).toBe("Ada has a leftover process that could not be ended: denied");
    expect(announcementText(row({ event: "leftover_kept" }))).toBe(
      "Ada has a leftover process that could not be ended",
    );
    expect(isNeverReadOnArrival("leftover_kept")).toBe(true);
  });

  test("the one OS notification names each agent once", () => {
    expect(leftoverKeptBody(["Ada", "Bo", "Ada"])).toBe(
      "Could not end a leftover process of: Ada, Bo",
    );
  });

  test("a failed resume names the reason from the detail (0013 D7)", () => {
    expect(
      announcementText(row({ event: "resume_hold", detail: "ended at once" })),
    ).toBe("Ada was put on hold: ended at once");
    expect(announcementText(row({ event: "resume_hold" }))).toBe(
      "Ada was put on hold",
    );
  });

  test("a report names its kind from the detail (0015)", () => {
    expect(announcementText(row({ event: "report", detail: "blocked" }))).toBe(
      "Ada sent a report: blocked",
    );
    expect(announcementText(row({ event: "report" }))).toBe(
      "Ada sent a report",
    );
  });

  test("an ended temp names the end reason from the detail (0020)", () => {
    const ended = row({
      event: "temp_ended",
      detail: "no output for 30 minutes",
    });
    expect(announcementText(ended)).toBe("Ada ended: no output for 30 minutes");
    expect(rowText(ended, false)).toBe(
      "Ada, ended: no output for 30 minutes, agent deleted",
    );
    expect(announcementText(row({ event: "temp_ended" }))).toBe("Ada ended");
  });

  test("a row reads as one sentence in its reading order (D11)", () => {
    expect(rowText(row(), true)).toBe("Ada, exited");
    expect(rowText(row({ event: "bell" }), true)).toBe("Ada, rang the bell");
  });

  test("a row of a deleted agent says so from the kept name (edge case)", () => {
    expect(rowText(row(), false)).toBe("Ada, exited, agent deleted");
  });

  test("an announcement names the agent and its event once", () => {
    expect(announcementText(row({ event: "waiting_for_input" }))).toBe(
      "Ada waits for your input",
    );
  });

  test("a row without a read time is unread", () => {
    expect(isUnread(row())).toBe(true);
    expect(isUnread(row({ read_at: 2_000 }))).toBe(false);
  });

  test("the unread count counts the rows without a read time", () => {
    const rows = [
      row({ id: "a" }),
      row({ id: "b", read_at: 5 }),
      row({ id: "c" }),
    ];
    expect(unreadCount(rows)).toBe(2);
    expect(unreadCount([])).toBe(0);
  });

  test("the list sorts unread first, then newest first (D4)", () => {
    const sorted = sortRows([
      row({ id: "old-read", read_at: 5, created_at: 1_000 }),
      row({ id: "new-read", read_at: 9, created_at: 3_000 }),
      row({ id: "old-unread", created_at: 2_000 }),
      row({ id: "new-unread", created_at: 4_000 }),
    ]);
    expect(sorted.map((entry) => entry.id)).toEqual([
      "new-unread",
      "old-unread",
      "new-read",
      "old-read",
    ]);
  });
});
