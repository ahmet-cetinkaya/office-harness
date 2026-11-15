// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  CoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import type { ActionEntry } from "../../audit";
import type { Agent, AdditionsView } from "../models/agent";
import {
  createOfficeController,
  type OfficeSnapshot,
} from "./office-controller";

function agent(id: string, status = "stopped"): Agent {
  return {
    id,
    name: `Agent ${id}`,
    command: "claude",
    working_folder: "/work",
    project_id: null,
    machine_id: "m",
    status,
    created_at: 1,
  };
}

const ADDITIONS: AdditionsView = {
  flags: [],
  env: [],
  startup_prompt: null,
  updated_at: 1,
  applies_next_session: false,
};

type Handler = (kind: string, fields?: Record<string, unknown>) => unknown;

function fakeClient(handler: Handler = () => undefined) {
  const requests: { kind: string; fields?: Record<string, unknown> }[] = [];
  let onEvent: (event: ServerEvent) => void = () => {};
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async (callback) => {
      onEvent = callback;
    },
    sendRequest: async (kind, fields) => {
      requests.push({ kind, fields });
      return handler(kind, fields);
    },
    startServer: async () => {},
    retryConnection: async () => {},
  };
  return { client, requests, push: (event: ServerEvent) => onEvent(event) };
}

function officeHandler(agents: Agent[]): Handler {
  return (kind) => {
    switch (kind) {
      case "agent.list":
        return agents;
      case "agent.additions.get":
        return ADDITIONS;
      case "agent.additions.set":
        return { ...ADDITIONS, applies_next_session: true };
      default:
        return [];
    }
  };
}

function observe(client: CoreClient) {
  let last: OfficeSnapshot | undefined;
  const actionRecord = {
    list: async (agentId: string) =>
      (await client.sendRequest("agent.actions", {
        agent_id: agentId,
      })) as ActionEntry[],
    merge: (entries: ActionEntry[], entry: ActionEntry) =>
      entries.some((known) => known.id === entry.id)
        ? entries
        : [...entries, entry],
  };
  const controller = createOfficeController(
    client,
    (next) => {
      last = next;
    },
    actionRecord,
  );
  return { controller, state: () => last as OfficeSnapshot };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("office controller", () => {
  test("loads the agents and the projects", async () => {
    const { client } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);

    await controller.load();

    expect(state().phase).toBe("ready");
    expect(state().agents.map((known) => known.id)).toEqual(["a"]);
  });

  test("says that the office did not load", async () => {
    const { client } = fakeClient(() => {
      throw new CoreRequestError("not_connected", "no server");
    });
    const { controller, state } = observe(client);

    await controller.load();

    expect(state().phase).toBe("failed");
    expect(state().failure).toBe("no server");
  });

  test("a status event changes the status and keeps its detail", async () => {
    const { client, push } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();

    push({
      type: "agent.status",
      agent_id: "a",
      status: "failed",
      detail: "command not found",
    });

    expect(state().agents[0].status).toBe("failed");
    expect(state().statusDetails.a).toBe("command not found");
    push({ type: "agent.status", agent_id: "a", status: "starting" });
    expect(state().statusDetails.a).toBeUndefined();
  });

  test("an agent that another client creates joins the list", async () => {
    const { client, push } = fakeClient(officeHandler([]));
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();

    push({ type: "agent.changed", agent: agent("b") });
    push({ type: "agent.changed", agent: agent("b", "working") });

    expect(state().agents).toHaveLength(1);
    expect(state().agents[0].status).toBe("working");
  });

  test("a deleted agent that is open closes its details and says why", async () => {
    const { client, push } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();
    controller.select("a");
    await settle();

    push({ type: "agent.deleted", agent_id: "a" });

    expect(state().agents).toEqual([]);
    expect(state().details).toBeNull();
    expect(state().notice?.lead).toBe("The agent was deleted.");
  });

  test("an own delete closes the details without a notice", async () => {
    const { client, push } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();
    controller.select("a");
    await settle();

    await controller.deleteAgent("a");
    push({ type: "agent.deleted", agent_id: "a" });

    expect(state().selectedAgentId).toBeNull();
    expect(state().notice).toBeNull();
  });

  test("an additions event refreshes the open agent only", async () => {
    const { client, push } = fakeClient(
      officeHandler([agent("a"), agent("b")]),
    );
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();
    controller.select("a");
    await settle();

    push({
      type: "agent.additions.changed",
      agent_id: "b",
      additions: { ...ADDITIONS, flags: ["--other"] },
    });
    expect(state().details?.additions?.flags).toEqual([]);
    push({
      type: "agent.additions.changed",
      agent_id: "a",
      additions: { ...ADDITIONS, flags: ["--new"] },
    });

    expect(state().details?.additions?.flags).toEqual(["--new"]);
  });

  test("an action record event adds its entry once", async () => {
    const { client, push } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);
    await controller.watch();
    await controller.load();
    controller.select("a");
    await settle();
    const entry = {
      id: 1,
      actor: "user",
      agent_id: "a",
      action: "start",
      result: "ok",
      at: 1,
    };

    push({ type: "action.recorded", entry });
    push({ type: "action.recorded", entry });

    expect(state().details?.actions).toEqual([entry]);
  });

  test("a refused request shows as a notice, not as an exception", async () => {
    const { client } = fakeClient((kind) => {
      if (kind === "agent.start") {
        throw new CoreRequestError("refused", "the folder /x does not exist");
      }
      return [];
    });
    const { controller, state } = observe(client);

    await controller.startAgent("a");

    expect(state().notice).toEqual({
      lead: "The agent did not start.",
      message: "the folder /x does not exist",
    });
    expect(state().isBusy).toBe(false);
  });

  test("a form gets the failure text and the list stays as it was", async () => {
    const { client } = fakeClient((kind) => {
      if (kind === "agent.create") {
        throw new CoreRequestError("conflict", "the name Ada is taken");
      }
      return [];
    });
    const { controller, state } = observe(client);

    const failure = await controller.createAgent({
      name: "Ada",
      command: "claude",
    });

    expect(failure).toBe("the name Ada is taken");
    expect(state().agents).toEqual([]);
  });

  test("saving additions stores what the server answers", async () => {
    const { client, requests } = fakeClient(officeHandler([agent("a")]));
    const { controller, state } = observe(client);
    await controller.load();
    controller.select("a");
    await settle();

    const failure = await controller.saveAdditions("a", {
      flags: ["--x"],
      env: [],
      startup_prompt: null,
    });

    expect(failure).toBeNull();
    expect(state().details?.additions?.applies_next_session).toBe(true);
    expect(
      requests.find((request) => request.kind === "agent.additions.set")
        ?.fields,
    ).toMatchObject({
      agent_id: "a",
    });
  });
});
