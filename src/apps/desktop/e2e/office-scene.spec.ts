// SPDX-License-Identifier: AGPL-3.0-or-later

// Scene e2e. Landed: floors, desks, switcher, search, empty states; the
// keyboard traversal, announcements, and deletion focus moves (T5 a, b);
// the reduced-motion scan (T5 c); the desk terminal embed (D5); and the
// review acceptance tests (SC-002 shape under a search, PB-1, PB-4).
// Still waiting for test/mock-core-extensions: departments from a seeded
// tree and the SC-004 100-agent measurement.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent, Project } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const DESKTOP = { width: 1280, height: 720 };
const PHONE = { width: 360, height: 800 };
// SC-004 (plan D11): the scene opens in under 2 seconds and a
// find-and-open lands in under 15 seconds. The limits are the spec's own,
// so a miss shows up here and can trigger the plan's fold fallback.
const SC004_OPEN_LIMIT_MS = 2_000;
const SC004_FIND_OPEN_LIMIT_MS = 15_000;

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ada",
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1_700_000_000_000,
    last_line: null,
    ...overrides,
  };
}

const PROJECTS = [
  { id: "project-site", name: "Site", folder: "/work/site" },
  { id: "project-api", name: "Api", folder: "/work/api" },
  { id: "project-empty", name: "Empty", folder: "/work/empty" },
];

/** Site: Ada, Bo. Api: Cy. No project: Dee. Empty floor: none. */
const OFFICE_AGENTS = [
  agent({ project_id: "project-site", last_line: "Building the site" }),
  agent({
    id: "agent-bo",
    name: "Bo",
    project_id: "project-site",
    status: "stopped",
  }),
  agent({
    id: "agent-cy",
    name: "Cy",
    project_id: "project-api",
    status: "waiting_for_input",
  }),
  agent({ id: "agent-dee", name: "Dee", status: "exited", last_line: "Bye." }),
];

async function openScene(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: OFFICE_AGENTS, projects: PROJECTS, ...office },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "To the scene" }).click();
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

/** Axe at the plan's two widths: desktop first, then phone. */
async function expectNoSeriousViolationsAtBothWidths(
  page: Page,
): Promise<void> {
  await expectNoSeriousViolations(page);
  await page.setViewportSize(PHONE);
  await expectNoSeriousViolations(page);
  await page.setViewportSize(DESKTOP);
}

/** The scene's own area; the shell's switch actions stay outside it. */
function scene(page: Page) {
  return page.getByRole("region", { name: "Office scene" });
}

/** The desks inside the scene, in reading order. */
function desks(page: Page) {
  return scene(page).getByRole("button");
}

/** The SC-004 office: 100 agents across 10 projects, ten per project. */
function scaleOffice(): { agents: Agent[]; projects: Project[] } {
  const projects = Array.from({ length: 10 }, (_, floor) => ({
    id: `project-scale-${floor}`,
    name: `Floor ${floor}`,
    folder: `/work/floor-${floor}`,
  }));
  const agents = Array.from({ length: 100 }, (_, desk) =>
    agent({
      id: `agent-scale-${desk}`,
      name: `Agent ${String(desk).padStart(3, "0")}`,
      project_id: projects[Math.floor(desk / 10)].id,
    }),
  );
  return { agents, projects };
}

test("the floors show their desks with name, status, and last line", async ({
  page,
}) => {
  await openScene(page);

  const ada = desks(page).filter({ hasText: "Ada" });
  await expect(ada).toHaveAttribute(
    "aria-label",
    "Ada, project Site, working, Building the site",
  );
  await expect(desks(page).filter({ hasText: "Dee" })).toHaveAttribute(
    "aria-label",
    "Dee, exited, Bye.",
  );
  // The "No project" floor holds Dee, and the agentless floor shows its text.
  await expect(
    scene(page).getByRole("heading", { name: "No project (1)" }),
  ).toBeVisible();
  await expect(
    scene(page).getByRole("heading", { name: "Empty (0)" }),
  ).toBeVisible();
  await expect(
    scene(page).getByText("This floor has no agents."),
  ).toBeVisible();
  // The desk text is the panel row's text for the same agent (SC-003 shape).
  await expect(desks(page).filter({ hasText: "Cy" })).toContainText(
    "waiting for input",
  );

  await expectNoSeriousViolationsAtBothWidths(page);
});

test("the switcher shows one project's floor and All projects brings the rest back", async ({
  page,
}) => {
  await openScene(page);

  await page.getByLabel("Project", { exact: true }).selectOption({
    label: "Site",
  });
  await expect(
    scene(page).getByRole("heading", { name: "Site (2)" }),
  ).toBeVisible();
  await expect(
    scene(page).getByRole("heading", { name: "Api (1)" }),
  ).toHaveCount(0);
  await expect(desks(page)).toHaveCount(2);

  await page.getByLabel("Project", { exact: true }).selectOption({
    label: "All projects",
  });
  await expect(desks(page)).toHaveCount(4);
  await expect(
    scene(page).getByRole("heading", { name: "No project (1)" }),
  ).toBeVisible();
});

test("the floors name their departments and the roles order the desks (FR-003)", async ({
  page,
}) => {
  /** Ada is the CEO; Bo leads Backend and Cy sits in it; El leads Web with
   * Fi in it; Dee has no team. Cy sits on Api, so its department spans two
   * floors and shows there without its lead. */
  const agents = [
    agent({ project_id: "project-site" }),
    agent({
      id: "agent-bo",
      name: "Bo",
      project_id: "project-site",
      status: "stopped",
    }),
    agent({
      id: "agent-cy",
      name: "Cy",
      project_id: "project-api",
      status: "waiting_for_input",
    }),
    agent({ id: "agent-el", name: "El", project_id: "project-api" }),
    agent({ id: "agent-fi", name: "Fi", project_id: "project-api" }),
    agent({ id: "agent-dee", name: "Dee", status: "exited" }),
  ];
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents,
      projects: PROJECTS,
      hierarchy: {
        ceo: { id: "agent-ada", role: "ceo" },
        teams: [
          {
            id: "team-backend",
            name: "Backend",
            project_id: "project-site",
            lead: { id: "agent-bo", role: "lead" },
            members: [{ id: "agent-cy", role: "member" }],
          },
          {
            id: "team-web",
            name: "Web",
            project_id: "project-api",
            lead: { id: "agent-el", role: "lead" },
            members: [{ id: "agent-fi", role: "member" }],
          },
        ],
        unassigned: [],
      },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "To the scene" }).click();

  const floor = (name: string) => scene(page).getByRole("region", { name });
  // The desk's accessible name opens with the agent's name.
  const deskNames = (area: ReturnType<typeof scene>) =>
    area
      .getByRole("button")
      .evaluateAll((desks) =>
        desks.map(
          (desk) => desk.getAttribute("aria-label")?.split(",")[0] ?? "",
        ),
      );

  // Departments sort by team name after the unnamed group; a department
  // shows on every floor where one of its agents sits.
  const api = floor("Api (3)");
  await expect(api.getByRole("heading", { name: "Backend" })).toBeVisible();
  await expect(api.getByRole("heading", { name: "Web" })).toBeVisible();
  // Cy's department's lead sits on Site, so Cy is first on Api; El leads
  // Web here, so El comes before Fi.
  expect(await deskNames(api)).toEqual(["Cy", "El", "Fi"]);

  // The unnamed group goes first on its floor with the CEO at its head,
  // and the lead Bo heads his department alone (Cy is on Api). Dee has no
  // team and no project, so it sits on the No project floor instead.
  const site = floor("Site (2)");
  await expect(site.getByRole("heading", { name: "Backend" })).toBeVisible();
  expect(await deskNames(site)).toEqual(["Ada", "Bo"]);

  // The switcher narrows to one floor with its departments intact.
  await page.getByLabel("Project", { exact: true }).selectOption({
    label: "Api",
  });
  await expect(api.getByRole("heading", { name: "Web" })).toBeVisible();
  await expect(desks(page)).toHaveCount(3);
  expect(await deskNames(scene(page))).toEqual(["Cy", "El", "Fi"]);
});

test("the search filters desks, focuses the match, and says when nothing matches", async ({
  page,
}) => {
  await openScene(page);

  const search = page.getByLabel("Search agents");
  await search.fill("bo");
  await expect(desks(page)).toHaveCount(1);
  await expect(desks(page).filter({ hasText: "Bo" })).toBeFocused();

  await search.fill("zzz");
  await expect(desks(page)).toHaveCount(0);
  await expect(
    scene(page).getByRole("paragraph").filter({
      hasText: "No desk matches the search.",
    }),
  ).toBeVisible();
  // The zero-match case is heard through the one live region as well (D9).
  await expect(scene(page).getByRole("status")).toContainText(
    "No desk matches the search.",
  );

  // Spaces-only is no search: every desk is back.
  await search.fill("   ");
  await expect(desks(page)).toHaveCount(4);

  // Mixed case goes through the panel's matchesAgentName, so the scene
  // search agrees with the panel switcher's semantics (D7): "dE" matches
  // only Dee, case-insensitively.
  await search.fill("dE");
  await expect(desks(page)).toHaveCount(1);
  await expect(desks(page).filter({ hasText: "Dee" })).toBeFocused();
});

test("a pushed status and line change update the desk under an active search (SC-002 shape)", async ({
  page,
}) => {
  await openScene(page);

  const search = page.getByLabel("Search agents");
  // Only Bo matches, so the update must land on the one rendered desk.
  await search.fill("bo");
  await expect(desks(page)).toHaveCount(1);
  await expect(desks(page).filter({ hasText: "Bo" })).toBeFocused();

  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-bo",
      status: "starting",
    });
    window.__mockCore.pushEvent({
      type: "agent.line",
      agent_id: "agent-bo",
      line: "Rebuilt from clean.",
    });
  });

  // The desk updates in place; the search text and the focus hold.
  const updated = desks(page).filter({ hasText: "Bo" });
  await expect(updated).toHaveAttribute(
    "aria-label",
    "Bo, project Site, starting, Rebuilt from clean.",
  );
  await expect(desks(page)).toHaveCount(1);
  await expect(search).toHaveValue("bo");
  await expect(updated).toBeFocused();
});

test("the scene exposes no create, start, stop, or delete control (PB-1)", async ({
  page,
}) => {
  await openScene(page);

  for (const label of [
    "New agent",
    "New project",
    "Start agent",
    "Stop agent",
    "Delete agent",
    "Delete project",
  ]) {
    await expect(scene(page).getByRole("button", { name: label })).toHaveCount(
      0,
    );
  }
  // The only desk buttons are desks; nothing acts on an agent beyond the
  // (D5) desk open itself.
  await expect(desks(page)).toHaveCount(4);
});

test("switching views sends no agent request (PB-4, recorded calls)", async ({
  page,
}) => {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: OFFICE_AGENTS, projects: PROJECTS },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "To the scene" }).click();

  const agentRequests = async () =>
    page.evaluate(() =>
      window.__mockCore.calls
        .filter((call) => call.command === "send_request")
        .map((call) => String(call.args.kind))
        .filter((kind) => kind.startsWith("agent.")),
    );

  const before = await agentRequests();
  await page.getByRole("button", { name: "To the panel" }).click();
  await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
  await page.getByRole("button", { name: "To the scene" }).click();
  await expect(
    page.getByRole("region", { name: "Office scene" }),
  ).toBeVisible();

  // Only settings.set rides the switch (last_view); agent.list and friends
  // are the initial loads, unchanged by the switches (PB-4).
  const after = await agentRequests();
  expect(after).toEqual(before);
  const settings = await page.evaluate(() =>
    (
      window.__mockCore.calls.filter(
        (call) =>
          call.command === "send_request" && call.args.kind === "settings.set",
      ) as Array<{ args: { fields?: { key?: string } } }>
    ).map((call) => String(call.args.fields?.key)),
  );
  expect(settings.length).toBeGreaterThan(0);
  expect(settings.every((key) => key === "last_view")).toBe(true);
});

test("the empty office says so and the shell returns to the panel", async ({
  page,
}) => {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: [], projects: [] },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "To the scene" }).click();

  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
  // The create action lives in the panel; the scene offers only the way back.
  await expect(
    page.getByRole("button", { name: "Create the first agent" }),
  ).toHaveCount(0);

  await expectNoSeriousViolationsAtBothWidths(page);

  await page.getByRole("button", { name: "To the panel" }).click();
  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create the first agent" }),
  ).toBeVisible();
});

test("the keyboard alone moves over every desk in reading order", async ({
  page,
}) => {
  await openScene(page);

  // Reading order: floors sort by name, so Api (Cy) comes first, then
  // Site (Ada, Bo), then No project (Dee). Tab walks all four desks in
  // DOM order across floor edges; the arrows stop inside one floor.
  const cy = desks(page).filter({ hasText: "Cy" });
  await cy.focus();
  await expect(cy).toBeFocused();

  await page.keyboard.press("Tab");
  await expect(desks(page).filter({ hasText: "Ada" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(desks(page).filter({ hasText: "Bo" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(desks(page).filter({ hasText: "Dee" })).toBeFocused();

  // The arrows move inside one floor and stop at its edge, no wrap. The
  // Site floor has two desks (Ada, Bo); the Api and No project floors
  // have one each, so arrows hold there.
  await desks(page).filter({ hasText: "Ada" }).focus();
  await page.keyboard.press("ArrowDown");
  await expect(desks(page).filter({ hasText: "Bo" })).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(desks(page).filter({ hasText: "Bo" })).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(desks(page).filter({ hasText: "Ada" })).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(desks(page).filter({ hasText: "Ada" })).toBeFocused();

  // The focus mark is the shared focus-visible outline, always there.
  const outline = await desks(page)
    .filter({ hasText: "Ada" })
    .evaluate((element) => {
      const style = getComputedStyle(element);
      return { width: style.outlineWidth, style: style.outlineStyle };
    });
  expect(outline.style).not.toBe("none");
  expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(2);

  await expectNoSeriousViolations(page);
});

test("a pushed status change is announced with the agent's name", async ({
  page,
}) => {
  await openScene(page);

  const region = page
    .locator('[role="region"][aria-labelledby="scene-heading"]')
    .getByRole("status")
    .first();
  await expect(region).toHaveText("");

  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-ada",
      status: "waiting_for_input",
    });
  });

  await expect(region).toHaveText("Ada, waiting for input");
});

test("deleting the focused desk's agent moves focus forward, back, or to the empty state", async ({
  page,
}) => {
  await openScene(page);

  // Reading order: Cy (Api), Dee (Site), Ada (Site), Bo (Site).
  const ada = desks(page).filter({ hasText: "Ada" });
  const bo = desks(page).filter({ hasText: "Bo" });
  await ada.focus();

  // The next desk in reading order takes the focus.
  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.deleted",
      agent_id: "agent-ada",
    });
  });
  await expect(bo).toBeFocused();

  // The last desk's deletion falls back to the previous one.
  const boAgain = desks(page).filter({ hasText: "Bo" });
  await boAgain.focus();
  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.deleted",
      agent_id: "agent-bo",
    });
  });
  await expect(desks(page).filter({ hasText: "Dee" })).toBeFocused();

  // The last desk left drops the scene into its empty state heading.
  await desks(page).filter({ hasText: "Dee" }).focus();
  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.deleted",
      agent_id: "agent-dee",
    });
    window.__mockCore.pushEvent({
      type: "agent.deleted",
      agent_id: "agent-cy",
    });
    window.__mockCore.pushEvent({
      type: "agent.deleted",
      agent_id: "agent-bo",
    });
  });
  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeFocused();
});

test.describe("reduced motion (SC-007, research R5)", () => {
  test.use({ reducedMotion: "reduce" });

  test("12 pushed status changes move or flash no element", async ({
    page,
  }) => {
    await openScene(page);
    expect(
      await page.evaluate(
        () => matchMedia("(prefers-reduced-motion: reduce)").matches,
      ),
    ).toBe(true);

    // 12 statuses change in the same second (the plan's SC-007 fixture).
    await page.evaluate(() => {
      const agents = ["agent-ada", "agent-bo", "agent-cy", "agent-dee"];
      for (let round = 0; round < 3; round += 1) {
        for (const agentId of agents) {
          window.__mockCore.pushEvent({
            type: "agent.status",
            agent_id: agentId,
            status: round % 2 === 0 ? "waiting_for_input" : "working",
          });
        }
      }
    });
    await expect(desks(page).filter({ hasText: "Ada" })).toContainText(
      "waiting for input",
    );

    // Pass 1 (deterministic): no motion-producing computed style anywhere
    // in the scene. Color/opacity-only changes are not motion (R4/R5).
    const motion = await page.evaluate(() => {
      const sceneRoot = document.querySelector(
        '[role="region"][aria-labelledby="scene-heading"]',
      );
      const offenders: Array<Record<string, string>> = [];
      if (sceneRoot === null) return offenders;
      for (const element of sceneRoot.querySelectorAll("*")) {
        const style = getComputedStyle(element);
        const transitionActive =
          style.transitionProperty !== "none" &&
          style.transitionDuration.split(",").some((d) => d.trim() !== "0s");
        if (
          style.animationName !== "none" ||
          transitionActive ||
          style.transform !== "none"
        ) {
          offenders.push({
            tag: element.tagName,
            cls: String(element.className).slice(0, 40),
            animation: style.animationName,
            transition: style.transitionProperty,
            transform: style.transform,
          });
        }
      }
      return offenders;
    });
    expect(motion).toEqual([]);

    // Pass 2 (runtime): two animation frames with no scroll input — every
    // desk rect must hold still, and the scroll position must not move.
    const frames = await page.evaluate(async () => {
      const raf = () =>
        new Promise((resolve) => requestAnimationFrame(resolve));
      const snapshot = () => {
        const desks = [...document.querySelectorAll("[data-agent-id]")].map(
          (element) => element.getBoundingClientRect().toJSON(),
        );
        return {
          scrollTop: document.scrollingElement?.scrollTop ?? 0,
          desks,
        };
      };
      await raf();
      const first = snapshot();
      await raf();
      const second = snapshot();
      return { first, second };
    });
    expect(frames.first.scrollTop).toBe(frames.second.scrollTop);
    expect(frames.first.desks).toEqual(frames.second.desks);
  });
});

test("view switches write last_view with the view field, throttled", async ({
  page,
}) => {
  await openScene(page);

  // The scene is open: the write carries view "scene" plus the 0003 fields.
  // The switch happens inside the 1s throttle window, so the scene write
  // rides the held tail and lands within about a second.
  const writes = async () =>
    page.evaluate(() =>
      (
        window.__mockCore.calls.filter(
          (call) =>
            call.command === "send_request" &&
            call.args.kind === "settings.set",
        ) as Array<{ args: { fields?: { value?: string } } }>
      ).map((call) => JSON.parse(String(call.args.fields?.value))),
    );

  await expect
    .poll(async () => (await writes()).at(-1))
    .toMatchObject({ view: "scene", project: null, terminal_open: false });

  // Back to the panel: the newest write carries view "panel".
  await page.getByRole("button", { name: "To the panel" }).click();
  await expect
    .poll(async () => (await writes()).at(-1))
    .toMatchObject({ view: "panel" });

  // One step each way, so at most a handful of writes ever go out (D6's
  // at-most-once-per-second throttle; the whole test runs in well under 1s
  // apart from polling, and the held tail never exceeds one write).
  const sceneWrites = (await writes()).filter(
    (value) => value.view === "scene",
  );
  expect(sceneWrites.length).toBeLessThanOrEqual(2);
});

test("100 agents in 10 projects: the scene opens and answers a find-and-open (SC-004)", async ({
  page,
}) => {
  const { agents, projects } = scaleOffice();
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents, projects },
  });
  // The plan (T5 d) measures at 360 width.
  await page.setViewportSize(PHONE);
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "To the scene" }),
  ).toBeVisible();

  // The clock runs in the browser from the scene click to the last desk in
  // the DOM, so the Playwright round-trip does not count in.
  const openMs = await page.evaluate(async (deskCount) => {
    const toScene = [...document.querySelectorAll("button")].find(
      (button) => button.textContent?.trim() === "To the scene",
    );
    if (toScene === undefined) return null;
    const startedAt = performance.now();
    toScene.click();
    while (
      document.querySelectorAll("[data-agent-id]").length < deskCount &&
      performance.now() - startedAt < 10_000
    ) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    return performance.now() - startedAt;
  }, 100);
  await expect(desks(page)).toHaveCount(100);
  expect(openMs).not.toBeNull();
  expect(openMs as number).toBeLessThan(SC004_OPEN_LIMIT_MS);

  // The find-and-open half: the search locates one named desk and its
  // click embeds the terminal, inside the 15-second bound.
  const findStarted = Date.now();
  await page.getByLabel("Search agents").fill("Agent 087");
  const desk = desks(page).filter({ hasText: "Agent 087" });
  await expect(desk).toHaveCount(1);
  await desk.click();
  await expect(
    page.getByLabel("Terminal of Agent 087", { exact: true }),
  ).toBeFocused();
  const findOpenMs = Date.now() - findStarted;
  expect(findOpenMs).toBeLessThan(SC004_FIND_OPEN_LIMIT_MS);

  // The scale edge case holds too: every desk keeps its readable size and
  // the page scrolls instead of shrinking anything.
  const deskHeight = await desks(page)
    .first()
    .evaluate((element) => element.getBoundingClientRect().height);
  expect(deskHeight).toBeGreaterThanOrEqual(44);
  expect(
    await page.evaluate(
      () => (document.scrollingElement?.scrollHeight ?? 0) > window.innerHeight,
    ),
  ).toBe(true);
});

test.describe("desk opens the terminal in the scene (D5)", () => {
  test("a click embeds the terminal with focus, no second window", async ({
    page,
  }) => {
    await openScene(page);

    await desks(page).filter({ hasText: "Cy" }).click();
    const terminal = page.getByLabel("Terminal of Cy", { exact: true });
    await expect(terminal).toBeFocused();
    await expect(page.getByText("Every key goes to the agent")).toBeVisible();
    // One page only: the desk click never spawns a second window (D5).
    // Bo is stopped, so its readonly embed proves itself via its note.
    const contexts = page.context().pages().length;
    await desks(page).filter({ hasText: "Bo" }).click();
    await expect(page.getByText("The agent is stopped.")).toBeVisible();
    expect(page.context().pages().length).toBe(contexts);
  });

  test("Enter on a focused desk opens the same terminal", async ({ page }) => {
    await openScene(page);

    await desks(page).filter({ hasText: "Bo" }).focus();
    await page.keyboard.press("Enter");
    // Bo is stopped: the embed shows recent output and takes no stdin, so
    // the focus mark sits on the terminal heading region instead of the
    // textarea; the embed itself is proven by the stopped-terminal note.
    await expect(page.getByText("The agent is stopped.")).toBeVisible();
    await expect(
      page.getByLabel("Terminal of Bo", { exact: true }),
    ).toHaveAttribute("readonly", "");
  });

  test("the focus-leave shortcut returns focus to the desk", async ({
    page,
  }) => {
    await openScene(page);

    await desks(page).filter({ hasText: "Cy" }).click();
    await expect(
      page.getByLabel("Terminal of Cy", { exact: true }),
    ).toBeFocused();

    await page.keyboard.press("Control+Shift+F12");
    await expect(desks(page).filter({ hasText: "Cy" })).toBeFocused();
  });

  test("a stopped agent's desk opens with its recent output", async ({
    page,
  }) => {
    // The terminal frame format encodes a 16-byte UUID, so the seeded id
    // must be one (terminal.spec's convention), or the snapshot drops.
    const agentId = "11111111-2222-4333-8444-555555555555";
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: [
          agent({ id: agentId, project_id: "project-site", status: "stopped" }),
        ],
        projects: PROJECTS,
        snapshots: { [agentId]: "recent saved output" },
      },
    });
    await page.goto("/");
    await page.getByRole("button", { name: "To the scene" }).click();

    await desks(page).filter({ hasText: "Ada" }).click();
    const input = page.getByLabel("Terminal of Ada", { exact: true });
    // A stopped terminal takes no input (isRunning false; readonly) and the
    // snapshot reaches the screen-reader buffer (terminal.spec convention).
    await expect(page.getByText(/The agent is stopped/)).toBeVisible();
    await expect(page.locator(".xterm-accessibility-tree")).toContainText(
      "recent saved output",
    );
    await input.press("x");
    const inputs = await page.evaluate(() =>
      window.__mockCore.calls.filter(
        (call) =>
          call.command === "send_request" &&
          call.args.kind === "terminal.input",
      ),
    );
    expect(inputs).toEqual([]);
  });
});

test.describe("T4: return and restore", () => {
  test("a 20-trial round trip keeps the project choice and the selected agent (SC-006)", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: { agents: OFFICE_AGENTS, projects: PROJECTS },
    });
    await page.goto("/");

    for (let trial = 0; trial < 20; trial += 1) {
      // Alternate the project and the agent inside it (SC-006's mixed runs).
      const [projectLabel, rowName] =
        trial % 2 === 0
          ? ["Site", "Ada, project Site, working"]
          : ["Api", "Cy, project Api, waiting for input"];
      await page.getByLabel("Project", { exact: true }).selectOption({
        label: projectLabel,
      });
      const row = page
        .getByRole("region", { name: "Agents" })
        .getByRole("button", { name: rowName });
      await row.click();

      await page.getByRole("button", { name: "To the scene" }).click();
      await expect(
        page.getByRole("region", { name: "Office scene" }),
      ).toBeVisible();

      await page.getByRole("button", { name: "To the panel" }).click();
      await expect(
        page.getByRole("region", { name: "Agents" }).getByRole("list"),
      ).toBeVisible();

      // The panel's choice and selection are exactly the scene-open ones.
      expect(
        await page.getByLabel("Project", { exact: true }).inputValue(),
      ).toBe(projectLabel === "Site" ? "project-site" : "project-api");
      await expect(
        page
          .getByRole("region", { name: "Agents" })
          .getByRole("button", { name: rowName }),
      ).toHaveAttribute("aria-current", "true");
    }
  });

  test("a start with a stored view:scene reopens the scene", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: null,
          agent: null,
          terminal_open: false,
          view: "scene",
        }),
      },
    });
    await page.goto("/");

    await expect(
      page.getByRole("region", { name: "Office scene" }),
    ).toBeVisible();
    await expect(page.getByRole("region", { name: "Agents" })).toHaveCount(0);
  });

  test("a switch after a scene restore writes the new view", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: null,
          agent: null,
          terminal_open: false,
          view: "scene",
        }),
      },
    });
    await page.goto("/");
    await expect(
      page.getByRole("region", { name: "Office scene" }),
    ).toBeVisible();

    await page.getByRole("button", { name: "To the panel" }).click();
    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(() => {
          const writes = window.__mockCore.settings;
          return writes.length === 0
            ? null
            : JSON.parse(String(writes.at(-1)?.value));
        }),
      )
      .toMatchObject({ view: "panel" });
  });

  test("a stored last_view without the view field (a 0003-era write) opens the panel", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: "project-site",
          agent: "agent-ada",
          terminal_open: true,
        }),
      },
    });
    await page.goto("/");

    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    expect(
      await page.getByRole("region", { name: "Office scene" }).count(),
    ).toBe(0);
    // The panel-side restore of the stored project/agent is 0003's reader;
    // this branch proves only the view: the panel opens.
  });

  test("an unknown view word opens the panel", async ({ page }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: null,
          agent: null,
          terminal_open: false,
          view: "galaxy",
        }),
      },
    });
    await page.goto("/");

    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    expect(
      await page.getByRole("region", { name: "Office scene" }).count(),
    ).toBe(0);
  });

  test("last_view's terminal_open follows the view that owns the terminal", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: { agents: OFFICE_AGENTS, projects: PROJECTS },
    });
    await page.goto("/");
    const lastWrite = async () =>
      page.evaluate(() => {
        const writes = window.__mockCore.calls.filter(
          (call) =>
            call.command === "send_request" &&
            call.args.kind === "settings.set",
        ) as Array<{ args: { fields?: { value?: string } } }>;
        return JSON.parse(String(writes.at(-1)?.args.fields?.value ?? "{}"));
      });

    // Open Cy's desk in the scene: the scene owns the open terminal.
    await page.getByRole("button", { name: "To the scene" }).click();
    await desks(page).filter({ hasText: "Cy" }).click();
    await expect(
      page.getByLabel("Terminal of Cy", { exact: true }),
    ).toBeFocused();
    await expect
      .poll(lastWrite)
      .toMatchObject({ view: "scene", terminal_open: true });

    // Back in the panel with no selection: the scene's desk must not leak
    // into the write; terminal_open is false.
    await page.getByRole("button", { name: "To the panel" }).click();
    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    await expect
      .poll(lastWrite)
      .toMatchObject({ view: "panel", terminal_open: false });

    // The panel opens Ada's terminal through its selection: true again.
    await page
      .getByRole("region", { name: "Agents" })
      .getByRole("button", { name: "Ada, project Site, working" })
      .click();
    await expect(
      page.getByLabel("Terminal of Ada", { exact: true }),
    ).toBeFocused();
    await expect.poll(lastWrite).toMatchObject({
      view: "panel",
      terminal_open: true,
      agent: "agent-ada",
    });
  });

  test("a delayed restore never disables the writes after an early switch", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: null,
          agent: null,
          terminal_open: false,
          view: "scene",
        }),
        lastViewDelayMs: 3_000,
      },
    });
    await page.goto("/");

    // The user switches before the slow restore answers: the early choice
    // wins the view...
    await page.getByRole("button", { name: "To the scene" }).click();
    await expect(
      page.getByRole("region", { name: "Office scene" }),
    ).toBeVisible();

    // ...and the writes keep flowing afterwards (the restore gate opens
    // even when the stored view is discarded).
    const writes = async () =>
      page.evaluate(() =>
        (
          window.__mockCore.calls.filter(
            (call) =>
              call.command === "send_request" &&
              call.args.kind === "settings.set",
          ) as Array<{ args: { fields?: { value?: string } } }>
        ).map((call) => JSON.parse(String(call.args.fields?.value))),
      );
    await page.getByRole("button", { name: "To the panel" }).click();
    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    await expect
      .poll(async () => (await writes()).at(-1))
      .toMatchObject({ view: "panel" });

    // Once the delayed answer lands, the user's view still stands.
    await page.waitForTimeout(3_500);
    await expect(page.getByRole("region", { name: "Agents" })).toBeVisible();
    expect(
      await page.getByRole("region", { name: "Office scene" }).count(),
    ).toBe(0);
    await expect
      .poll(async () => (await writes()).at(-1))
      .toMatchObject({ view: "panel" });
  });

  test("a scene round trip during the restore read starts no second restore", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        lastView: JSON.stringify({
          project: null,
          agent: "agent-bo",
          terminal_open: false,
          view: "panel",
        }),
        lastViewDelayMs: 1_500,
      },
    });
    await page.goto("/");

    // The panel's restore read is in flight; the panel unmounts and mounts
    // again, and the second mount must find the restore already under way.
    await page.getByRole("button", { name: "To the scene" }).click();
    await expect(
      page.getByRole("region", { name: "Office scene" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "To the panel" }).click();

    await expect(
      page.getByRole("button", { name: "Bo, project Site, stopped" }),
    ).toHaveAttribute("aria-current", "true");
    const reads = () =>
      page.evaluate(
        () =>
          window.__mockCore.calls.filter(
            (call) =>
              call.command === "send_request" &&
              call.args.kind === "settings.get" &&
              (call.args.fields as { key?: string })?.key === "last_view",
          ).length,
      );
    await page.waitForTimeout(500);
    expect(await reads()).toBe(1);
  });

  test("departments group the floors from the seeded tree (D2, FR-003)", async ({
    page,
  }) => {
    /** Ada is the CEO on Site; Bo leads Backend and sits on Site; Cy is a
     * Backend member on Api, so Backend spans two floors; Dee has no team. */
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: OFFICE_AGENTS,
        projects: PROJECTS,
        hierarchy: {
          ceo: { id: "agent-ada", role: "ceo" },
          teams: [
            {
              id: "team-backend",
              name: "Backend",
              project_id: "project-site",
              lead: { id: "agent-bo", role: "lead" },
              members: [{ id: "agent-cy", role: "member" }],
            },
          ],
          unassigned: [],
        },
      },
    });
    await page.goto("/");
    await page.getByRole("button", { name: "To the scene" }).click();

    const floor = (name: string) => scene(page).getByRole("region", { name });

    // Api floor: the cross-floor department's member sits here without Bo.
    const api = floor("Api (1)");
    await expect(api.getByRole("heading", { name: "Backend" })).toBeVisible();
    await expect(api.getByRole("button", { name: /Cy/ })).toBeVisible();

    // Site floor: the unnamed group first (CEO Ada, no-team Dee), then the
    // department with its lead Bo before Cy... Cy is on Api, so Bo alone.
    const site = floor("Site (2)");
    const siteDesks = site.getByRole("button");
    await expect(siteDesks).toHaveCount(2);
    expect(await siteDesks.first().getAttribute("aria-label")).toContain("Ada");
    await expect(site.getByRole("heading", { name: "Backend" })).toBeVisible();

    // Dee (no team) sits in the unnamed group on the No project floor.
    await expect(
      floor("No project (1)").getByRole("button", { name: /Dee/ }),
    ).toBeVisible();
  });
});
