// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient, ServerEvent } from "../../../core/api";
import { createResumeController } from "./resume-controller";

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

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

const PLAN = {
  agents: [
    { agent_id: "ada", can_continue: true, held: false },
    { agent_id: "bo", can_continue: true, held: true },
    {
      agent_id: "cy",
      can_continue: true,
      held: false,
      after_unclean_stop: true,
    },
  ],
};

async function opened(start: Handler = () => ({ order: [] })) {
  const fake = fakeClient((kind, fields) =>
    kind === "resume.plan" ? PLAN : start(kind, fields),
  );
  const controller = createResumeController(fake.client);
  await controller.watch();
  await controller.open();
  return { ...fake, controller };
}

describe("resume controller", () => {
  test("open checks every agent that is not on hold, in plan order", async () => {
    const { controller } = await opened();
    expect(controller.snapshot.phase).toBe("choosing");
    expect(controller.snapshot.entries.map((e) => e.agent_id)).toEqual([
      "ada",
      "bo",
      "cy",
    ]);
    expect(controller.snapshot.checkedIds).toEqual(["ada", "cy"]);
  });

  test("a failed plan read shows the reason", async () => {
    const fake = fakeClient(() => {
      throw { code: "internal", message: "boom" };
    });
    const controller = createResumeController(fake.client);
    await controller.open();
    expect(controller.snapshot.phase).toBe("failed");
    expect(controller.snapshot.failure).toBe("boom");
  });

  test("a marked agent that is checked needs the acknowledgement before a start", async () => {
    const { controller, requests } = await opened();
    await controller.confirm();
    expect(requests.some((r) => r.kind === "resume.start")).toBe(false);

    controller.acknowledge(true);
    await controller.confirm();
    expect(requests.find((r) => r.kind === "resume.start")?.fields).toEqual({
      agent_ids: ["ada", "cy"],
    });
  });

  test("unchecking the marked agent lifts the need for the acknowledgement", async () => {
    const { controller, requests } = await opened();
    controller.toggle("cy");
    await controller.confirm();
    expect(requests.find((r) => r.kind === "resume.start")?.fields).toEqual({
      agent_ids: ["ada"],
    });
  });

  test("nothing checked sends nothing", async () => {
    const { controller, requests } = await opened();
    controller.toggle("ada");
    controller.toggle("cy");
    await controller.confirm();
    expect(controller.snapshot.phase).toBe("choosing");
    expect(requests.some((r) => r.kind === "resume.start")).toBe(false);
  });

  test("results apply to the sent agents, and others are ignored", async () => {
    const { controller, push } = await opened(() => ({ order: ["ada"] }));
    controller.toggle("cy");
    await controller.confirm();
    expect(controller.snapshot.phase).toBe("running");

    push({
      type: "resume.result",
      agent_id: "ada",
      result: "checking",
      continued: true,
      is_final: false,
    });
    push({
      type: "resume.result",
      agent_id: "ada",
      result: "continued",
      is_final: true,
    });
    push({
      type: "resume.result",
      agent_id: "bo",
      result: "continued",
      is_final: true,
    });

    expect(Object.keys(controller.snapshot.results)).toEqual(["ada"]);
    expect(controller.snapshot.results.ada.result).toBe("continued");
  });

  test("a result that arrives before the answer is kept", async () => {
    const fake = fakeClient((kind) => {
      if (kind === "resume.plan") return PLAN;
      fake.push({
        type: "resume.result",
        agent_id: "ada",
        result: "checking",
        is_final: false,
      });
      return { order: ["ada", "cy"] };
    });
    const controller = createResumeController(fake.client);
    await controller.watch();
    await controller.open();
    controller.acknowledge(true);
    await controller.confirm();
    expect(controller.snapshot.results.ada.result).toBe("checking");
  });

  test("a refused start stays on the list and shows the reason", async () => {
    const { controller } = await opened(() => {
      throw { code: "invalid", message: "bad ids" };
    });
    controller.toggle("cy");
    await controller.confirm();
    expect(controller.snapshot.phase).toBe("choosing");
    expect(controller.snapshot.failure).toBe("bad ids");
    expect(controller.snapshot.isStarting).toBe(false);
  });
});

describe("resume controller during a sweep (0017, FR-007)", () => {
  test("a plan read while the sweep runs waits, and loads the plan on done", async () => {
    let isSweeping = true;
    const fake = fakeClient((kind) =>
      kind === "resume.plan"
        ? isSweeping
          ? { sweeping: true, agents: [] }
          : PLAN
        : undefined,
    );
    const controller = createResumeController(fake.client);
    await controller.watch();
    await controller.open();
    expect(controller.snapshot.phase).toBe("sweeping");
    expect(controller.snapshot.entries).toEqual([]);

    isSweeping = false;
    fake.push({ type: "office.sweep", state: "done" });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(controller.snapshot.phase).toBe("choosing");
    expect(controller.snapshot.entries).toHaveLength(3);
  });

  test("a running event does not reload, and a done event outside the wait does nothing", async () => {
    const { controller, requests, push } = await opened();
    const reads = () => requests.filter((r) => r.kind === "resume.plan").length;
    expect(reads()).toBe(1);

    push({ type: "office.sweep", state: "running" });
    push({ type: "office.sweep", state: "done" });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(reads()).toBe(1);
    expect(controller.snapshot.phase).toBe("choosing");
  });
});
