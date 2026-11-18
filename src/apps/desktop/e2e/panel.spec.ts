// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type { HierarchyTree } from "../src/modules/hierarchy/models/hierarchy";
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
];

/** One agent per project choice: Site, Api, and no project. */
const OFFICE_AGENTS = [
  agent({ project_id: "project-site" }),
  agent({
    id: "agent-bo",
    name: "Bo",
    project_id: "project-api",
    status: "stopped",
  }),
  agent({ id: "agent-cy", name: "Cy", status: "waiting_for_input" }),
];

/** The 0002 tree that gives every seeded agent a role, and two of them a team. */
const TEAM_TREE: HierarchyTree = {
  ceo: { id: "agent-ada", role: "ceo" },
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: null,
      lead: { id: "agent-bo", role: "lead" },
      members: [{ id: "agent-cy", role: "member" }],
    },
  ],
  unassigned: [],
};

async function openPanel(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: OFFICE_AGENTS, projects: PROJECTS, ...office },
  });
  await page.goto("/");
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

async function chooseProject(page: Page, label: string): Promise<void> {
  await page.getByLabel("Project", { exact: true }).selectOption({ label });
}

/** The focus ring that the shared classes give every control. */
async function expectFocusVisible(row: ReturnType<Page["getByRole"]>) {
  const outline = await row.evaluate((element) => {
    const style = getComputedStyle(element);
    return { width: style.outlineWidth, style: style.outlineStyle };
  });
  expect(outline.style).not.toBe("none");
  expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(2);
}

test("All projects lists the agents of every project and the agent without one", async ({
  page,
}) => {
  await openPanel(page);

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Bo, project Api, stopped" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toBeVisible();
  await expect(list.getByRole("listitem")).toHaveCount(3);

  await expectNoSeriousViolationsAtBothWidths(page);
});

test("one project shows only its own agents, and All projects brings the rest back", async ({
  page,
}) => {
  await openPanel(page);

  await chooseProject(page, "Site");
  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Bo, project Api, stopped" }),
  ).toHaveCount(0);
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toHaveCount(0);

  await chooseProject(page, "All projects");
  await expect(list.getByRole("listitem")).toHaveCount(3);
});

test("an empty project says so and offers to create an agent", async ({
  page,
}) => {
  await openPanel(page, { agents: [agent()] });

  await chooseProject(page, "Api");

  await expect(
    page.getByRole("heading", { name: "This project has no agents" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create an agent in this project" }),
  ).toBeVisible();
  await expectNoSeriousViolationsAtBothWidths(page);
});

test("the empty office keeps its offer at both widths", async ({ page }) => {
  await openPanel(page, { agents: [], projects: [] });

  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
  await expectNoSeriousViolationsAtBothWidths(page);
});

test("a row names the agent, its project, its status, and its role and team", async ({
  page,
}) => {
  await openPanel(page, { hierarchy: TEAM_TREE });

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working, CEO" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", {
      name: "Bo, project Api, stopped, team lead, Backend",
    }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", {
      name: "Cy, waiting for input, member, Backend",
    }),
  ).toBeVisible();

  // The same names hold when the panel lays out in one column.
  await page.setViewportSize(PHONE);
  await expect(
    list.getByRole("button", {
      name: "Bo, project Api, stopped, team lead, Backend",
    }),
  ).toBeVisible();
  await page.setViewportSize(DESKTOP);
});

test("a row without a role and team leaves them out of the name", async ({
  page,
}) => {
  await openPanel(page);

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toHaveAttribute("aria-label", "Ada, project Site, working");
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toHaveAttribute("aria-label", "Cy, waiting for input");
});

test("the last output line shows on the row", async ({ page }) => {
  await openPanel(page, {
    agents: [agent({ last_line: "Building the project" })],
  });

  await expect(page.getByText("Building the project")).toBeVisible();
});

test("a status and a line change update the row in place and keep the selection and focus", async ({
  page,
}) => {
  await openPanel(page);

  const row = page.getByRole("button", {
    name: "Ada, project Site, working",
  });
  await row.click();
  await expect(row).toHaveAttribute("aria-current", "true");
  // The opened terminal takes focus; the user returns to the list.
  await row.focus();

  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-ada",
      status: "waiting_for_input",
    });
    window.__mockCore.pushEvent({
      type: "agent.line",
      agent_id: "agent-ada",
      line: "Ready for input.",
    });
  });

  const updated = page.getByRole("button", {
    name: "Ada, project Site, waiting for input",
  });
  await expect(updated).toBeVisible();
  await expect(updated).toHaveAttribute("aria-current", "true");
  await expect(updated).toBeFocused();
  await expect(page.getByText("Ready for input.")).toBeVisible();
});

test("keyboard focus is visible on a row at both widths", async ({ page }) => {
  await openPanel(page);

  const row = page.getByRole("button", {
    name: "Ada, project Site, working",
  });
  await row.focus();
  await expectFocusVisible(row);

  await page.setViewportSize(PHONE);
  await expectFocusVisible(row);
  await page.setViewportSize(DESKTOP);
});
