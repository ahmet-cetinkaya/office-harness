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
    last_line: null,
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

test("a failed tree read shows the failure, and Try again loads the tree", async ({
  page,
}) => {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: COMPANY_AGENTS,
      projects: [{ id: "project-site", name: "Site", folder: "/work/site" }],
      hierarchy: COMPANY_TREE,
    },
    hierarchyTreeFailure: {
      code: "internal",
      message: "the tree read broke",
    },
  });
  await page.goto("/");

  await expect(page.getByText("The hierarchy did not load.")).toBeVisible();
  await expect(page.getByText("the tree read broke")).toBeVisible();

  await page.getByRole("button", { name: "Try again" }).click();

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(tree).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Ada, CEO, working" }),
  ).toBeVisible();
});

/** The CEO-less tree the mock holds once another client deletes the CEO. */
const TREE_WITHOUT_CEO = {
  ceo: null,
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: "project-site" as const,
      lead: { id: "agent-lead-bo", role: "lead" as const },
      members: [
        { id: "agent-member-cy", role: "member" as const },
        { id: "agent-member-dee", role: "member" as const },
      ],
    },
  ],
  unassigned: [],
};

/** Another client deletes the CEO and tells this view the tree changed. */
async function deleteCeo(page: Page): Promise<void> {
  await page.evaluate((tree) => {
    window.__mockCore.setHierarchy(tree);
    window.__mockCore.pushEvent({
      type: "hierarchy.changed",
      cause: "subtree.deleted",
    });
  }, TREE_WITHOUT_CEO);
}

test("deleting the active row from elsewhere leaves outside focus alone", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const ceoRow = tree.getByRole("treeitem", { name: "Ada, CEO, working" });
  await ceoRow.click();

  // The user then moves on to the agents list, say to open its terminal.
  const agentsButton = page.getByRole("button", { name: "Ada, working" });
  await agentsButton.focus();
  await expect(agentsButton).toBeFocused();

  // Another client deletes the CEO; the teams remain, so a first row exists.
  await deleteCeo(page);

  const teamRow = tree.getByRole("treeitem", {
    name: "Team Backend, project Site, 2 members",
  });
  await expect(teamRow).toBeVisible();
  await expect(agentsButton).toBeFocused();
});

test("deleting the focused row hands focus back to the tree's first row", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const ceoRow = tree.getByRole("treeitem", { name: "Ada, CEO, working" });
  await ceoRow.focus();
  await expect(ceoRow).toBeFocused();

  // Another client deletes the CEO while the row holds keyboard focus.
  await deleteCeo(page);

  const teamRow = tree.getByRole("treeitem", {
    name: "Team Backend, project Site, 2 members",
  });
  await expect(teamRow).toBeVisible();
  await expect(teamRow).toBeFocused();
});

/** The tree the mock holds once another client moves Cy from Backend to Web. */
const TREE_WITH_CY_IN_WEB: HierarchyTree = {
  ceo: { id: "agent-ceo", role: "ceo" },
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: "project-site",
      lead: { id: "agent-lead-bo", role: "lead" },
      members: [{ id: "agent-member-dee", role: "member" }],
    },
    {
      id: "team-web",
      name: "Web",
      project_id: null,
      lead: { id: "agent-lead-el", role: "lead" },
      members: [
        { id: "agent-member-fay", role: "member" },
        { id: "agent-member-gus", role: "member" },
        { id: "agent-member-cy", role: "member" },
      ],
    },
  ],
  unassigned: [],
};

/** The tree with Cy in Web once another client deletes Cy. */
const TREE_WITHOUT_CY: HierarchyTree = {
  ceo: { id: "agent-ceo", role: "ceo" },
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: "project-site",
      lead: { id: "agent-lead-bo", role: "lead" },
      members: [{ id: "agent-member-dee", role: "member" }],
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

test("a moved focused row keeps focus, and its later delete leaves outside focus alone", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const cyRow = tree.getByRole("treeitem", {
    name: "Cy, member, waiting for input",
  });
  await cyRow.focus();
  await expect(cyRow).toBeFocused();

  // Another client moves Cy to another team; the row is rebuilt under the
  // new parent, and keyboard focus follows the rebuilt element. No emitter
  // sends this cause, and the view ignores it, so the real path still runs.
  await page.evaluate((movedTree) => {
    window.__mockCore.setHierarchy(movedTree);
    window.__mockCore.pushEvent({
      type: "hierarchy.changed",
      cause: "agent.moved",
    });
  }, TREE_WITH_CY_IN_WEB);

  await expect(cyRow).toBeVisible();
  await expect(cyRow).toBeFocused();

  // The user then moves on to the agents list, say to open its terminal.
  const agentsButton = page.getByRole("button", { name: "Ada, working" });
  await agentsButton.focus();
  await expect(agentsButton).toBeFocused();

  // Another client deletes Cy while the agents list holds focus.
  await page.evaluate((treeAfterDelete) => {
    window.__mockCore.setHierarchy(treeAfterDelete);
    window.__mockCore.pushEvent({
      type: "hierarchy.changed",
      cause: "subtree.deleted",
    });
  }, TREE_WITHOUT_CY);

  await expect(cyRow).toHaveCount(0);
  await expect(agentsButton).toBeFocused();
});

test("tabbing out of the tree keeps a later row delete from stealing focus", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const cyRow = tree.getByRole("treeitem", {
    name: "Cy, member, waiting for input",
  });
  await cyRow.focus();
  await expect(cyRow).toBeFocused();

  // Tab leaves the tree for the next tab stop, the agents list.
  await page.keyboard.press("Tab");
  const agentsButton = page.getByRole("button", { name: "Ada, working" });
  await expect(agentsButton).toBeFocused();

  // Another client deletes Cy, whose row held focus until the tab-out;
  // the tree does not pull focus back from the agents list.
  await page.evaluate((treeAfterDelete) => {
    window.__mockCore.setHierarchy(treeAfterDelete);
    window.__mockCore.pushEvent({
      type: "hierarchy.changed",
      cause: "subtree.deleted",
    });
  }, TREE_WITHOUT_CY);

  await expect(cyRow).toHaveCount(0);
  await expect(agentsButton).toBeFocused();
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

/** The seed for the create flows: an empty office with one project. */
const EMPTY_OFFICE_SEED: OfficeSeed = {
  projects: [{ id: "project-site", name: "Site", folder: "/work/site" }],
  hierarchy: { ceo: null, teams: [], unassigned: [] },
};

/** The kinds the webview asked the server for, in order. */
async function sentRequestKinds(page: Page): Promise<string[]> {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter((call) => call.command === "send_request")
    .map((call) => String(call.args.kind));
}

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

test("US1 builds a company from an empty office with the keyboard alone", async ({
  page,
}) => {
  await openOffice(page, EMPTY_OFFICE_SEED);

  const tree = page.getByRole("tree", { name: "Hierarchy" });

  // 1. The CEO, from the empty office's own create actions.
  await pressOn(page, "Create CEO");
  await expect(page.getByRole("dialog", { name: "New CEO" })).toBeVisible();
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.keyboard.type("Ada");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: "Create agent" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);

  const ceoRow = tree.getByRole("treeitem", { name: "Ada, CEO, stopped" });
  await expect(ceoRow).toBeVisible();

  // 2. Two teams with a lead each, one request per team (FR-002). The CEO
  // row is the active one, so the toolbar offers the team lead.
  await ceoRow.click();
  for (const teamName of ["Backend", "Web"]) {
    await pressOn(page, "Create team lead");
    await expect(page.getByRole("dialog", { name: "New team" })).toBeVisible();
    await expect(page.getByLabel("Team name")).toBeFocused();
    await page.keyboard.type(teamName);
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.type(teamName === "Backend" ? "Bo" : "El");
    await page.keyboard.press("Tab");
    await page.keyboard.type("claude");
    await page.keyboard.press("Tab");
    await expect(
      page.getByRole("button", { name: "Create team", exact: true }),
    ).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  }
  await expect(
    tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "El, team lead, stopped" }),
  ).toBeVisible();

  // 3. Two members under each lead. Selecting a lead row moves the place,
  // so the member dialog names the lead it sits under.
  const members = [
    {
      name: "Cy",
      leadRow: "Bo, team lead, stopped",
      leadId: null as string | null,
    },
    {
      name: "Dee",
      leadRow: "Bo, team lead, stopped",
      leadId: null as string | null,
    },
    {
      name: "Fay",
      leadRow: "El, team lead, stopped",
      leadId: null as string | null,
    },
    {
      name: "Gus",
      leadRow: "El, team lead, stopped",
      leadId: null as string | null,
    },
  ];
  for (const member of members) {
    await tree.getByRole("treeitem", { name: member.leadRow }).focus();
    await page.keyboard.press("Enter");
    await pressOn(page, "Create member");
    await expect(
      page.getByRole("dialog", { name: "New member" }),
    ).toBeVisible();
    await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
    await page.keyboard.type(member.name);
    await page.keyboard.press("Tab");
    await page.keyboard.type("claude");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  }

  // The company stands: the CEO, two teams with a lead each, two members
  // per team, under their correct parents.
  await expect(
    tree.getByRole("treeitem", { name: "Team Backend, 2 members" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, stopped" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Dee, member, stopped" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Team Web, 2 members" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Fay, member, stopped" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "Gus, member, stopped" }),
  ).toBeVisible();
  await expect(ceoRow).toBeVisible();

  // The member requests carried the lead as the parent (FR-001): the two
  // Backend members name one lead and the two Web members another.
  const calls = await page.evaluate(() => window.__mockCore.calls);
  const memberRoles = calls
    .filter(
      (call) =>
        call.command === "send_request" &&
        call.args.kind === "agent.create" &&
        (call.args.fields as { role?: { kind?: string } }).role?.kind ===
          "member",
    )
    .map(
      (call) =>
        (call.args.fields as { role: { parent_agent_id: string } }).role
          .parent_agent_id,
    );
  // Cy and Dee name one lead, Fay and Gus another (SC-002: correct parents).
  expect(memberRoles).toHaveLength(4);
  expect(memberRoles[0]).toBe(memberRoles[1]);
  expect(memberRoles[2]).toBe(memberRoles[3]);
  expect(memberRoles[0]).not.toBe(memberRoles[2]);
  expect(await sentRequestKinds(page)).toContain("team.create");
  await expectNoSeriousViolations(page);
});

test("the delete confirmation states the counted descendants and cancel deletes 0", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const boRow = tree.getByRole("treeitem", { name: "Bo, team lead, stopped" });
  await boRow.click();

  // The lead has 2 members, so the question counts 3 agents in all.
  await pressOn(page, "Delete Bo");
  const dialog = page.getByRole("dialog", { name: "Delete Bo?" });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("2 agents below Bo");
  await expect(dialog).toContainText("3 agents in all");

  // Cancel keeps every agent.
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  for (const name of [
    "Bo, team lead, stopped",
    "Cy, member, waiting for input",
    "Dee, member, exited",
  ]) {
    await expect(tree.getByRole("treeitem", { name })).toBeVisible();
  }
  const deletes = await page.evaluate(() => window.__mockCore.calls);
  expect(
    deletes.filter(
      (call) =>
        call.command === "send_request" && call.args.kind === "agent.delete",
    ),
  ).toEqual([]);
});

test("a confirmed lead delete removes the lead and its 2 members", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  const boRow = tree.getByRole("treeitem", { name: "Bo, team lead, stopped" });
  await boRow.click();

  await pressOn(page, "Delete Bo");
  await page
    .getByRole("dialog", { name: "Delete Bo?" })
    .getByRole("button", { name: "Delete agent" })
    .click();

  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(boRow).toHaveCount(0);
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, waiting for input" }),
  ).toHaveCount(0);
  await expect(
    tree.getByRole("treeitem", { name: "Dee, member, exited" }),
  ).toHaveCount(0);
  // The team went with its lead: the tree lists a team only while its lead
  // row exists (SC-005 counts the agents, not the teams).
  await expect(
    tree.getByRole("treeitem", { name: /Team Backend/ }),
  ).toHaveCount(0);
  await expect(
    tree.getByRole("treeitem", { name: "Ada, CEO, working" }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: "El, team lead, working" }),
  ).toBeVisible();

  const deletes = await page.evaluate(() => window.__mockCore.calls);
  const sent = deletes.find(
    (call) =>
      call.command === "send_request" && call.args.kind === "agent.delete",
  );
  expect(sent?.args.fields).toMatchObject({
    agent_id: "agent-lead-bo",
    include_descendants: true,
    descendant_count: 2,
  });
});

test("the stop confirmation offers only-this-agent and with-descendants choices", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).click();

  await pressOn(page, "Stop Bo");
  const dialog = page.getByRole("dialog", { name: "Stop Bo?" });
  await expect(dialog).toBeVisible();
  // Both request shapes are named, and only-this-agent is the default.
  await expect(dialog).toContainText("include_descendants false");
  await expect(dialog).toContainText("2 agents below Bo keep running");
  await expect(dialog).toContainText("include_descendants true");
  await expect(
    dialog.getByRole("radio", { name: /Stop only Bo/ }),
  ).toBeChecked();
  await expect(
    dialog.getByRole("radio", { name: /Stop Bo and 2 agents/ }),
  ).not.toBeChecked();

  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const stops = await page.evaluate(() => window.__mockCore.calls);
  expect(
    stops.filter(
      (call) =>
        call.command === "send_request" && call.args.kind === "agent.stop",
    ),
  ).toEqual([]);
});

test("a confirmed only-this-agent stop sends include_descendants false", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).click();

  await pressOn(page, "Stop Bo");
  await page
    .getByRole("dialog", { name: "Stop Bo?" })
    .getByRole("button", { name: "Stop only Bo" })
    .click();

  await expect(page.getByRole("dialog")).toHaveCount(0);
  const stops = await page.evaluate(() => window.__mockCore.calls);
  const sent = stops.find(
    (call) =>
      call.command === "send_request" && call.args.kind === "agent.stop",
  );
  expect(sent?.args.fields).toMatchObject({
    agent_id: "agent-lead-bo",
    include_descendants: false,
  });
  // The members keep running.
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, waiting for input" }),
  ).toBeVisible();
});

test("a confirmed stop with descendants sends include_descendants", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).click();

  await pressOn(page, "Stop Bo");
  const dialog = page.getByRole("dialog", { name: "Stop Bo?" });
  await dialog.getByRole("radio", { name: /Stop Bo and 2 agents/ }).check();
  await dialog.getByRole("button", { name: "Stop Bo and 2 agents" }).click();

  await expect(page.getByRole("dialog")).toHaveCount(0);
  const stops = await page.evaluate(() => window.__mockCore.calls);
  const sent = stops.find(
    (call) =>
      call.command === "send_request" && call.args.kind === "agent.stop",
  );
  expect(sent?.args.fields).toMatchObject({
    agent_id: "agent-lead-bo",
    include_descendants: true,
  });
});

test("the create actions list only the roles the place allows (FR-003)", async ({
  page,
}) => {
  await openCompany(page);

  // With a CEO, no second CEO is offered; the tree offers no toolbar role
  // while the CEO row is not the active one.
  await expect(page.getByRole("button", { name: "Create CEO" })).toHaveCount(0);

  // The office without a CEO offers the CEO again.
  await openOffice(page, {
    agents: [],
    hierarchy: { ceo: null, teams: [], unassigned: [] },
  });
  await pressOn(page, "Create CEO");
  await expect(page.getByRole("dialog", { name: "New CEO" })).toBeVisible();
  await page.keyboard.press("Escape");

  // An office without a CEO but with a team offers a member once the lead's
  // row is the active one.
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
  await page
    .getByRole("tree", { name: "Hierarchy" })
    .getByRole("treeitem", { name: "Bo, team lead, stopped" })
    .click();
  await pressOn(page, "Create member");
  await expect(page.getByRole("dialog", { name: "New member" })).toBeVisible();
});

test("a refused create shows the server's refusal text (FR-003)", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [],
    projects: [],
    hierarchy: { ceo: null, teams: [], unassigned: [] },
    refusals: {
      "agent.create": {
        code: "invalid",
        message: "invalid: the office already has a CEO: agent-ceo",
      },
    },
  });

  // The office offers the CEO; the server refuses because another client
  // took the top while the dialog was open.
  await pressOn(page, "Create CEO");
  await expect(page.getByRole("dialog", { name: "New CEO" })).toBeVisible();
  await page.keyboard.type("Bo");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");

  await expect(page.getByRole("alert")).toContainText(
    "invalid: the office already has a CEO: agent-ceo",
  );
  // The dialog stays open, so the user can correct or cancel.
  await expect(page.getByRole("dialog", { name: "New CEO" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("a stale counted delete shows the server's recount refusal", async ({
  page,
}) => {
  await openOffice(page, {
    agents: COMPANY_AGENTS.slice(0, 4),
    hierarchy: {
      ceo: { id: "agent-ceo", role: "ceo" },
      teams: [
        {
          id: "team-backend",
          name: "Backend",
          project_id: null,
          lead: { id: "agent-lead-bo", role: "lead" },
          members: [
            { id: "agent-member-cy", role: "member" },
            { id: "agent-member-dee", role: "member" },
          ],
        },
      ],
      unassigned: [],
    },
    refusals: {
      "agent.delete": {
        code: "conflict",
        message: "the confirmation counted 2 descendants, but there are 3",
      },
    },
  });

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).click();
  await pressOn(page, "Delete Bo");
  await page
    .getByRole("dialog", { name: "Delete Bo?" })
    .getByRole("button", { name: "Delete agent" })
    .click();

  await expect(page.getByRole("alert")).toContainText(
    "the confirmation counted 2 descendants, but there are 3",
  );
  // Nothing was deleted; the tree stands.
  await expect(
    tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }),
  ).toBeVisible();
});

test("the create flows run with the keyboard alone and axe stays clean", async ({
  page,
}) => {
  await openOffice(page, EMPTY_OFFICE_SEED);

  const tree = page.getByRole("tree", { name: "Hierarchy" });

  // CEO by keyboard.
  await pressOn(page, "Create CEO");
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await expectNoSeriousViolations(page);
  await page.keyboard.type("Ada");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    tree.getByRole("treeitem", { name: "Ada, CEO, stopped" }),
  ).toBeVisible();

  // Team with its lead by keyboard: the CEO row active, Shift+Tab back to
  // the toolbar above the tree, and the team lead's dialog is the team
  // dialog (FR-002).
  await tree.getByRole("treeitem", { name: "Ada, CEO, stopped" }).focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Shift+Tab");
  await expect(
    page.getByRole("button", { name: "Create team lead" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: "New team" })).toBeVisible();
  await expect(page.getByLabel("Team name")).toBeFocused();
  await expectNoSeriousViolations(page);
  await page.keyboard.type("Backend");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await page.keyboard.type("Bo");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);

  // Member under the lead by keyboard: the lead row active, Shift+Tab back
  // to the toolbar above the tree, Enter on the create action.
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Shift+Tab");
  await expect(
    page.getByRole("button", { name: "Create member" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.keyboard.type("Cy");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    tree.getByRole("treeitem", { name: "Cy, member, stopped" }),
  ).toBeVisible();

  await expectNoSeriousViolations(page);
});

test("the stop and delete confirmations run axe clean while open", async ({
  page,
}) => {
  await openCompany(page);

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: "Bo, team lead, stopped" }).click();

  // The two-choice stop confirmation.
  await pressOn(page, "Stop Bo");
  await expect(page.getByRole("dialog", { name: "Stop Bo?" })).toBeVisible();
  await expectNoSeriousViolations(page);
  await page.keyboard.press("Escape");

  // The counted delete confirmation.
  await pressOn(page, "Delete Bo");
  await expect(page.getByRole("dialog", { name: "Delete Bo?" })).toBeVisible();
  await expectNoSeriousViolations(page);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
