// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { parseLastView, resolveLastView } from "./last-view";

describe("last view parse", () => {
  test("the documented shape parses (D6)", () => {
    expect(
      parseLastView(
        JSON.stringify({ project: "p-site", agent: "a", terminal_open: true }),
      ),
    ).toEqual({
      project: "p-site",
      agent: "a",
      terminal_open: true,
      view: "panel",
    });
    expect(
      parseLastView(
        JSON.stringify({ project: null, agent: null, terminal_open: false }),
      ),
    ).toEqual({
      project: null,
      agent: null,
      terminal_open: false,
      view: "panel",
    });
  });

  test("the scene view parses, and an unknown view word means the panel", () => {
    const stored = (view: unknown) =>
      parseLastView(
        JSON.stringify({
          project: null,
          agent: null,
          terminal_open: false,
          view,
        }),
      )?.view;
    expect(stored("scene")).toBe("scene");
    expect(stored("panel")).toBe("panel");
    expect(stored("galaxy")).toBe("panel");
  });

  test("absent or unreadable text is ignored, so the default view stands", () => {
    expect(parseLastView(undefined)).toBeNull();
    expect(parseLastView("")).toBeNull();
    expect(parseLastView(null)).toBeNull();
    expect(parseLastView("not json")).toBeNull();
    expect(parseLastView("42")).toBeNull();
    expect(parseLastView('{"project":"p"}')).toBeNull();
    expect(
      parseLastView('{"project":7,"agent":null,"terminal_open":true}'),
    ).toBeNull();
    expect(
      parseLastView('{"project":null,"agent":"a","terminal_open":"yes"}'),
    ).toBeNull();
  });
});

describe("last view resolve", () => {
  const PROJECTS = ["p-site", "p-api"];
  const AGENTS = ["agent-ada"];

  test("a present project and agent restore as they were", () => {
    const plan = resolveLastView(
      {
        project: "p-site",
        agent: "agent-ada",
        terminal_open: true,
        view: "panel",
      },
      PROJECTS,
      AGENTS,
    );
    expect(plan.choice).toBe("p-site");
    expect(plan.agent).toEqual({ kind: "present", agentId: "agent-ada" });
  });

  test("a missing project falls back to all projects", () => {
    const plan = resolveLastView(
      { project: "p-gone", agent: null, terminal_open: false, view: "panel" },
      PROJECTS,
      AGENTS,
    );
    expect(plan.choice).toBeNull();
    expect(plan.agent).toEqual({ kind: "none" });
  });

  test("a deleted agent is named and restores no selection (US3)", () => {
    const plan = resolveLastView(
      {
        project: "p-site",
        agent: "agent-gone",
        terminal_open: true,
        view: "panel",
      },
      PROJECTS,
      AGENTS,
    );
    expect(plan.choice).toBe("p-site");
    expect(plan.agent).toEqual({ kind: "missing" });
  });

  test("no stored agent restores the project alone", () => {
    const plan = resolveLastView(
      { project: "p-api", agent: null, terminal_open: false, view: "panel" },
      PROJECTS,
      AGENTS,
    );
    expect(plan.choice).toBe("p-api");
    expect(plan.agent).toEqual({ kind: "none" });
  });
});
