// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type { HierarchyTree } from "../src/modules/hierarchy/models/hierarchy";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ceo",
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1_700_000_000_000,
    ...overrides,
  };
}

/** The US1 tree: a CEO, two teams with a lead each, two members per team. */
const COMPANY_TREE: HierarchyTree = {
  ceo: { id: "agent-ceo", role: "ceo" },
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: "project-site",
      lead: { id: "agent-lead-bo", role: "lead" },
      members: [
        { id: "agent-member-cy", role: "member" },
        { id: "agent-member-dee", role: "member" },
      ],
    },
    {
      id: "team-web",
      name: "Web",
      project_id: null,
      lead: { id: "agent-lead-el", role: "lead" },
      members: [
        { id: "agent-member-fay", role: "member" },
        { id: "agent-member-gus", role: "member" },
      ],
    },
  ],
  unassigned: [],
};

const COMPANY_AGENTS = [
  agent({ status: "working" }),
  agent({ id: "agent-lead-bo", name: "Bo", status: "stopped" }),
  agent({ id: "agent-member-cy", name: "Cy", status: "waiting_for_input" }),
  agent({ id: "agent-member-dee", name: "Dee", status: "exited" }),
  agent({ id: "agent-lead-el", name: "El", status: "working" }),
  agent({ id: "agent-member-fay", name: "Fay", status: "stopped" }),
  agent({ id: "agent-member-gus", name: "Gus", status: "working" }),
];

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
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

async function openCompany(page: Page): Promise<void> {
  await openOffice(page, {
    agents: COMPANY_AGENTS,
    projects: [{ id: "project-site", name: "Site", folder: "/work/site" }],
    hierarchy: COMPANY_TREE,
  });
}

test("the tree shows each agent under its parent with role and status as text", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(tree).toBeVisible();

  const ceo = tree.getByRole("treeitem", { name: "Ada, CEO, working" });
  await expect(ceo).toBeVisible();
  await expect(ceo).toHaveAttribute("aria-level", "1");
  await expect(ceo).toHaveAttribute("aria-expanded", "true");

  const team = tree.getByRole("treeitem", {
    name: "Team Backend, project Site, 2 members",
  });
  await expect(team).toBeVisible();
  await expect(team).toHaveAttribute("aria-level", "2");
  await expect(
    tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, waiting for input" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Gus, member, working" }),
  ).toBeVisible();

  const group = tree.getByRole("group").first();
  await expect(group).toBeAttached();
  await expectNoSeriousViolations(page);
});

test("an office without a CEO shows the teams at the top", async ({ page }) => {
  await openOffice(page, {
    agents: [agent({ id: "agent-lead-bo", name: "Bo", status: "stopped" })],
    hierarchy: {
      ceo: null,
      teams: [
        {
          id: "team-backend",
          name: "Backend",
          project_id: null,
          lead: { id: "agent-lead-bo", role: "lead" },
          members: [],
        },
      ],
      unassigned: [],
    },
  });

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const team = tree.getByRole("treeitem", { name: "Team Backend, no members" });
  await expect(team).toBeVisible();
  await expect(team).toHaveAttribute("aria-level", "1");
  await expect(
    tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }),
  ).toBeVisible();
});

test("an empty hierarchy says so", async ({ page }) => {
  await openOffice(page, {
    agents: [agent({ id: "agent-float", name: "Dee", status: "stopped" })],
    hierarchy: { ceo: null, teams: [], unassigned: [] },
  });

  await expect(
    page.getByText("The office has no CEO and no team."),
  ).toBeVisible();
});

test("agents without a role sit in their own group", async ({ page }) => {
  await openOffice(page, {
    agents: [agent({ id: "agent-float", name: "Dee", status: "stopped" })],
    hierarchy: {
      ceo: null,
      teams: [],
      unassigned: [{ id: "agent-float", role: null }],
    },
  });

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(
    tree.getByRole("treeitem", { name: "Outside the hierarchy, 1 agent" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Dee, no role, stopped" }),
  ).toBeVisible();
});

test("Arrow keys walk the tree and Enter opens the agent", async ({ page }) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const first = tree.getByRole("treeitem", { name: "Ada, CEO, working" });
  await first.focus();
  await expect(first).toBeFocused();

  await page.keyboard.press("ArrowDown");
  const team = tree.getByRole("treeitem", {
    name: "Team Backend, project Site, 2 members",
  });
  await expect(team).toBeFocused();

  // The tree starts open, so Right moves into the branch's first child.
  await page.keyboard.press("ArrowRight");
  const lead = tree.getByRole("treeitem", { name: "Bo, team lead, stopped" });
  await expect(lead).toBeFocused();

  await page.keyboard.press("ArrowRight");
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, waiting for input" }),
  ).toBeFocused();

  // Left on a leaf moves to its parent; Left on an open branch folds it.
  await page.keyboard.press("ArrowLeft");
  await expect(lead).toBeFocused();
  await page.keyboard.press("ArrowLeft");
  await expect(lead).toHaveAttribute("aria-expanded", "false");

  // The branch is folded: its members are gone from the walk.
  await page.keyboard.press("ArrowDown");
  const web = tree.getByRole("treeitem", { name: "Team Web, 2 members" });
  await expect(web).toBeFocused();

  // Left folds the open branch in place.
  await page.keyboard.press("ArrowLeft");
  await expect(web).toHaveAttribute("aria-expanded", "false");

  // Right unfolds it and keeps the focus.
  await page.keyboard.press("ArrowRight");
  await expect(web).toBeFocused();
  await expect(web).toHaveAttribute("aria-expanded", "true");

  // Enter on an agent opens its details.
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "El", exact: true }),
  ).toBeVisible();

  await expectNoSeriousViolations(page);
});

test("Home and End jump to the first and last visible rows", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem").first().focus();
  await page.keyboard.press("End");

  await expect(
    tree.getByRole("treeitem", { name: "Gus, member, working" }),
  ).toBeFocused();

  await page.keyboard.press("Home");
  await expect(
    tree.getByRole("treeitem", { name: "Ada, CEO, working" }),
  ).toBeFocused();
});

test("a collapsed branch hides its members until it is expanded again", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const team = tree.getByRole("treeitem", {
    name: "Team Backend, project Site, 2 members",
  });
  await team.click();
  await expect(team).toHaveAttribute("aria-expanded", "false");
  await expect(
    tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }),
  ).toHaveCount(0);

  await team.click();
  await expect(team).toHaveAttribute("aria-expanded", "true");
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, waiting for input" }),
  ).toBeVisible();
});

test("a hierarchy.changed event re-renders the tree", async ({ page }) => {
  await openOffice(page, {
    agents: [agent()],
    hierarchy: { ceo: null, teams: [], unassigned: [] },
  });

  await expect(
    page.getByText("The office has no CEO and no team."),
  ).toBeVisible();

  // Another client created a CEO: the mock now holds it, and the event
  // tells this view to read the tree again.
  await page.evaluate(() => {
    window.__mockCore.setHierarchy({
      ceo: { id: "agent-ceo", role: "ceo" },
      teams: [],
      unassigned: [],
    });
    window.__mockCore.pushEvent({
      type: "hierarchy.changed",
      cause: "agent.placed",
    });
  });

  await expect(
    page
      .getByRole("tree", { name: "Hierarchy" })
      .getByRole("treeitem", { name: "Ada, CEO, working" }),
  ).toBeVisible();
});

test("a status event updates the tree name without a reload", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(
    tree.getByRole("treeitem", { name: "Fay, member, stopped" }),
  ).toBeVisible();

  await page.evaluate(() =>
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-member-fay",
      status: "working",
    }),
  );

  await expect(
    tree.getByRole("treeitem", { name: "Fay, member, working" }),
  ).toBeVisible();
  await expectNoSeriousViolations(page);
});
