// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  actorText,
  actionText,
  mergeEntry,
  resultText,
  type ActionEntry,
} from "./action-entry";

function entry(id: number): ActionEntry {
  return {
    id,
    actor: "user",
    agent_id: "a",
    action: "start",
    result: "ok",
    at: id,
    detail: null,
  };
}

describe("action entries", () => {
  test("an entry that is known is not added twice", () => {
    const entries = [entry(1)];
    expect(mergeEntry(entries, entry(1))).toBe(entries);
  });

  test("a new entry goes in by its id", () => {
    expect(mergeEntry([entry(1), entry(3)], entry(2)).map((e) => e.id)).toEqual(
      [1, 2, 3],
    );
  });

  test("results and actions read as text", () => {
    expect(resultText("ok")).toBe("Succeeded");
    expect(resultText("no such folder")).toBe("Failed: no such folder");
    expect(actionText("new_task")).toBe("Started a new task");
    expect(actionText("hold")).toBe("Put on hold");
    expect(actionText("release")).toBe("Released from hold");
    expect(actionText("report")).toBe("Sent a report");
    expect(actorText("office")).toBe("the office");
    expect(actorText("user")).toBe("user");
  });
});
