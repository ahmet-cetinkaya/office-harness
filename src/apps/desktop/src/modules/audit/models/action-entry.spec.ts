// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  actorText,
  actionText,
  detailText,
  mergeEntry,
  resultText,
  visibleDetail,
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

describe("sweep entries", () => {
  const LEFTOVER_DETAIL =
    "identity 12: ended by force: pid 4242, session s1, node server.js";

  test("the sweep actions read as text", () => {
    expect(actionText("end_leftover")).toBe("Checked a leftover process");
    expect(actionText("helper_limit")).toBe("Stopped storing helpers");
  });

  test("the detail of a sweep action is labelled Detail, any other Reason", () => {
    const sweep = {
      ...entry(1),
      action: "end_leftover",
      detail: LEFTOVER_DETAIL,
    };
    expect(detailText(sweep)).toBe(`Detail: ${LEFTOVER_DETAIL}`);
    expect(detailText({ ...entry(2), detail: "why" })).toBe("Reason: why");
    expect(detailText(entry(3))).toBeNull();
  });

  test("a control or bidirectional-control character is written as an escape", () => {
    expect(visibleDetail("run\u001b[2Jx")).toBe("run\\u{1b}[2Jx");
    expect(visibleDetail("a\u202Eb\u2066c")).toBe("a\\u{202e}b\\u{2066}c");
    expect(visibleDetail("tab\there")).toBe("tab\\u{9}here");
    expect(visibleDetail("a\u200Db\uFEFFc\u00ADd\u2028e\u2029f")).toBe(
      "a\\u{200d}b\\u{feff}c\\u{ad}d\\u{2028}e\\u{2029}f",
    );
  });

  test("text the server already escaped stays as it is", () => {
    const escaped = "cmd \\u{1b}[0m and an ellipsis... 1a2b3c4d";
    expect(visibleDetail(escaped)).toBe(escaped);
  });
});

describe("actorText", () => {
  const scheduleId = "0a1b2c3d-1111-2222-3333-444455556666";

  test("the four kinds read as text", () => {
    expect(actorText("user")).toBe("user");
    expect(actorText("office")).toBe("the office");
    expect(actorText("agent:a1")).toBe("agent:a1");
    expect(actorText(`schedule:${scheduleId}`)).toBe("schedule 0a1b2c3d");
  });

  test("a schedule with a label shows the label and who made it", () => {
    expect(actorText(`schedule:${scheduleId}`, "Morning sweep")).toBe(
      "Morning sweep (schedule, made by the person)",
    );
  });

  test("a schedule id is never read as an agent, even one that no agent has", () => {
    expect(actorText("schedule:unknown-id-0000")).toBe("schedule unknown-");
  });
});
