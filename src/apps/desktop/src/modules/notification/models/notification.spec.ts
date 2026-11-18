// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  announcementText,
  eventText,
  isUnread,
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
