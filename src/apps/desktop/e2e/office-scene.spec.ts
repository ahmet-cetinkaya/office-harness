// SPDX-License-Identifier: AGPL-3.0-or-later

// Scene e2e, part 1 (0005 T3): only what today's mock core supports. The
// part 2 run adds: departments from a seeded tree, a pushed status and line
// change updating a desk in place while the search text and focus hold
// (SC-002, SC-003 shapes), the PB-4 recorded-calls assertion on the switch,
// the terminal embed and its focus (D5), and the `view` field restore (D6).
// The role, team, and settings seeds themselves wait for
// test/mock-core-extensions, which is editing e2e/support/mock-core.ts.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const DESKTOP = { width: 1280, height: 720 };
const PHONE = { width: 360, height: 800 };

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
    scene(page).getByText("No desk matches the search."),
  ).toBeVisible();

  // Spaces-only is no search: every desk is back.
  await search.fill("   ");
  await expect(desks(page)).toHaveCount(4);
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
