// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { createLastViewWriter } from "./last-view";

type Recorded = Array<{ key: string; value: string }>;

function harness() {
  let clock = 1_000;
  const recorded: Recorded = [];
  const client = {
    sendRequest: async (kind: string, fields: Record<string, unknown>) => {
      if (kind === "settings.set") {
        recorded.push({
          key: String(fields.key),
          value: String(fields.value),
        });
      }
      return null;
    },
  };
  const writer = createLastViewWriter(client as never, () => clock);
  const view = (
    overrides: Partial<{ view: "panel" | "scene" }> = {},
  ): Parameters<typeof writer.write>[0] => ({
    project: null,
    agent: null,
    terminal_open: false,
    view: "scene",
    ...overrides,
  });
  return { writer, recorded, view, advance: (ms: number) => (clock += ms) };
}

describe("last-view writer", () => {
  test("the first write goes out at once", () => {
    const { writer, recorded, view } = harness();
    writer.write(view());
    expect(recorded).toHaveLength(1);
    expect(recorded[0].key).toBe("last_view");
    expect(JSON.parse(recorded[0].value)).toEqual(view());
  });

  test("a write inside one second is held, not sent", () => {
    const { writer, recorded, view, advance } = harness();
    writer.write(view());
    advance(200);
    writer.write(view({ view: "panel" }));
    expect(recorded).toHaveLength(1);
    expect(JSON.parse(recorded[0].value).view).toBe("scene");
  });

  test("bursts coalesce into the newest value at close", () => {
    const { writer, recorded, view, advance } = harness();
    writer.write(view({ view: "scene" }));
    advance(400);
    writer.write(view({ view: "panel" }));
    advance(400);
    writer.write(view({ view: "scene" }));
    writer.close();
    expect(recorded).toHaveLength(2);
    expect(JSON.parse(recorded[1].value).view).toBe("scene");
  });

  test("close sends a held write at once and clears the hold", () => {
    const { writer, recorded, view } = harness();
    writer.write(view({ view: "scene" }));
    writer.write(view({ view: "panel" }));
    writer.close();
    expect(recorded).toHaveLength(2);
    expect(JSON.parse(recorded[1].value).view).toBe("panel");
    // After close the throttle window restarts from the flush.
    writer.write(view({ view: "panel" }));
    expect(recorded).toHaveLength(2);
  });

  test("after the window elapses the next write goes out at once", () => {
    const { writer, recorded, view, advance } = harness();
    writer.write(view());
    advance(1_000);
    writer.write(view({ view: "panel" }));
    expect(recorded).toHaveLength(2);
    expect(JSON.parse(recorded[1].value).view).toBe("panel");
  });
});
