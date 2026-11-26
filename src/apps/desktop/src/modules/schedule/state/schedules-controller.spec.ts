// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { emptyDraft, type Schedule } from "../models/schedule";
import { createSchedulesController } from "./schedules-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function schedule(overrides: Partial<Schedule> = {}): Schedule {
  return {
    id: "s1",
    name: "nightly",
    text: "go",
    target_agent_id: "a1",
    repeat: { kind: "daily", time: "09:00" },
    on: true,
    next_due: 1,
    last_handled_due: null,
    last_missed: null,
    off_reason: null,
    ...overrides,
  };
}

function fakeClient(handler: Handler) {
  const requests: { kind: string; fields?: Record<string, unknown> }[] = [];
  let onEvent: (event: ServerEvent) => void = () => {};
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async (callback) => {
      onEvent = callback;
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind, fields) => {
      requests.push({ kind, fields });
      return handler(kind, fields);
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async () => "cancelled",
    importTemplatesFile: async () => null,
  };
  return { client, requests, push: (event: ServerEvent) => onEvent(event) };
}

const RUN = {
  id: 1,
  schedule_id: "s1",
  due: 5,
  started: 6,
  via: "due",
  result: "submitted",
  reason: null,
};

function standard(kind: string): unknown {
  if (kind === "schedule.list") return { schedules: [schedule()], zone: "UTC" };
  if (kind === "schedule.history") return { schedule_id: "s1", runs: [RUN] };
  return {};
}

async function opened(handler: Handler = standard) {
  const fake = fakeClient(handler);
  const controller = createSchedulesController(fake.client);
  await controller.watch();
  await controller.open();
  return { ...fake, controller };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("schedules controller list", () => {
  test("open reads the list, the zone, and the last run of each", async () => {
    const { controller } = await opened();
    expect(controller.snapshot.phase).toBe("ready");
    expect(controller.snapshot.zone).toBe("UTC");
    expect(controller.snapshot.schedules).toHaveLength(1);
    expect(controller.snapshot.lastRuns.s1.result).toBe("submitted");
  });

  test("a refused read shows its text", async () => {
    const { controller } = await opened(() => {
      throw { code: "internal", message: "the list failed" };
    });
    expect(controller.snapshot.phase).toBe("failed");
    expect(controller.snapshot.failure).toBe("the list failed");
  });
});

describe("schedules controller events", () => {
  test("schedule.changed replaces a row by its id and adds a new one", async () => {
    const { controller, push } = await opened();

    push({ type: "schedule.changed", schedule: schedule({ on: false }) });
    push({
      type: "schedule.changed",
      schedule: schedule({ id: "s2", name: "alpha" }),
    });
    await settle();

    expect(controller.snapshot.schedules.map((item) => item.id)).toEqual([
      "s2",
      "s1",
    ]);
    expect(controller.snapshot.schedules[1].on).toBe(false);
  });

  test("schedule.removed drops the row and its last run", async () => {
    const { controller, push } = await opened();

    push({ type: "schedule.removed", id: "s1" });

    expect(controller.snapshot.schedules).toEqual([]);
    expect(controller.snapshot.lastRuns).toEqual({});
  });

  test("events do nothing while the dialog is closed", async () => {
    const { controller, push } = await opened();
    controller.close();

    push({ type: "schedule.changed", schedule: schedule({ id: "s9" }) });

    expect(controller.snapshot.schedules).toEqual([]);
  });
});

describe("schedules controller save", () => {
  test("a save sends the fields and puts the saved row in the list", async () => {
    const saved = schedule({ id: "s3", name: "weekly" });
    const { controller, requests } = await opened((kind) =>
      kind === "schedule.save" ? { schedule: saved } : standard(kind),
    );

    const outcome = await controller.save(
      { ...emptyDraft("a1"), name: "weekly", text: "go" },
      null,
    );

    expect(outcome).toEqual({ saved: true });
    const sent = requests.find((request) => request.kind === "schedule.save");
    expect(sent?.fields?.name).toBe("weekly");
    expect(sent?.fields).not.toHaveProperty("schedule_id");
    expect(controller.snapshot.schedules.map((item) => item.id)).toContain(
      "s3",
    );
  });

  test("a refused save carries each failing field and its rule", async () => {
    const { controller } = await opened((kind) => {
      if (kind !== "schedule.save") return standard(kind);
      throw {
        code: "invalid",
        message: "name must not be blank",
        fields: [
          { field: "name", rule: "blank", message: "must not be blank" },
        ],
      };
    });

    const outcome = await controller.save(emptyDraft("a1"), "s1");

    expect(outcome.saved).toBe(false);
    if (outcome.saved) return;
    expect(outcome.problems).toEqual([
      { field: "name", rule: "blank", message: "must not be blank" },
    ]);
  });

  test("a refusal without fields, such as the cap, gives its message", async () => {
    const cap = "the office already has 50 schedules, the schedule cap";
    const { controller } = await opened((kind) => {
      if (kind !== "schedule.save") return standard(kind);
      throw { code: "refused", message: cap };
    });

    const outcome = await controller.save(emptyDraft("a1"), null);

    expect(outcome).toEqual({ saved: false, problems: [], message: cap });
  });
});

describe("schedules controller actions", () => {
  test("a switch refused keeps the row and shows the refusal", async () => {
    const { controller } = await opened((kind) => {
      if (kind !== "schedule.switch") return standard(kind);
      throw { code: "refused", message: "a switch on is refused: time" };
    });

    const failure = await controller.switchTo("s1", true);

    expect(failure).toBe("a switch on is refused: time");
    expect(controller.snapshot.actionFailure).toBe(failure);
    expect(controller.snapshot.schedules[0].on).toBe(true);
  });

  test("a switch puts the answered row in the list", async () => {
    const { controller } = await opened((kind) =>
      kind === "schedule.switch"
        ? { schedule: schedule({ on: false, next_due: null }) }
        : standard(kind),
    );

    expect(await controller.switchTo("s1", false)).toBeNull();
    expect(controller.snapshot.schedules[0].on).toBe(false);
  });

  test("run now and delete send the schedule id", async () => {
    const { controller, requests } = await opened();

    await controller.runNow("s1");
    await controller.remove("s1");

    expect(
      requests
        .filter((request) => request.fields?.schedule_id === "s1")
        .map((r) => r.kind),
    ).toEqual(expect.arrayContaining(["schedule.run_now", "schedule.delete"]));
    expect(controller.snapshot.schedules).toEqual([]);
  });

  test("the history loads newest first and closes", async () => {
    const { controller } = await opened();
    let seen: unknown = null;
    controller.subscribeHistory((history) => (seen = history));

    await controller.openHistory("s1");
    expect((seen as { runs: unknown[] }).runs).toHaveLength(1);

    controller.closeHistory();
    expect(seen).toBeNull();
  });
});
