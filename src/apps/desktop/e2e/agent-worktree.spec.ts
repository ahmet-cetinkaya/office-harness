// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type { HierarchyTree } from "../src/modules/hierarchy/models/hierarchy";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const REPOSITORY = "/work/repo";
const IGNORED_FILES_NOTE =
  "Files that git ignores are removed with the worktree.";

const REPOSITORIES: OfficeSeed["repositories"] = {
  [REPOSITORY]: {
    base: "main",
    branches: ["main", "develop"],
    branchName: "office-harness/ada",
  },
};

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ada",
    name: "Ada",
    command: "claude",
    working_folder: REPOSITORY,
    project_id: null,
    machine_id: "machine",
    status: "stopped",
    created_at: 1_700_000_000_000,
    last_line: null,
    ...overrides,
  };
}

function withWorktree(id: string, branch: string): Partial<Agent> {
  return {
    worktree: {
      repository: REPOSITORY,
      base: "main",
      branch,
      folder: `/data/worktrees/${id}`,
    },
  };
}

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { repositories: REPOSITORIES, ...office },
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

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

async function sentFields(
  page: Page,
  kind: string,
): Promise<Record<string, unknown>[]> {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter(
      (call) => call.command === "send_request" && call.args.kind === kind,
    )
    .map((call) => call.args.fields as Record<string, unknown>);
}

async function fillFormByKeyboard(page: Page, folder: string): Promise<void> {
  await pressOn(page, "New agent");
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.keyboard.type("Ada");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.type(folder);
}

test("the choice is not offered outside a repository", async ({ page }) => {
  await openOffice(page);
  await fillFormByKeyboard(page, "/work/plain");

  await expect
    .poll(async () => (await sentFields(page, "agent.worktree_info")).length)
    .toBeGreaterThan(0);
  await expect(page.getByLabel("Own worktree")).toHaveCount(0);
  await expect(page.getByText("Untracked files such as")).toHaveCount(0);
});

test("inside a repository the choice is off and shows base, branch name, and the note", async ({
  page,
}) => {
  await openOffice(page);
  await fillFormByKeyboard(page, REPOSITORY);

  const checkbox = page.getByLabel("Own worktree");
  await expect(checkbox).not.toBeChecked();
  await expect(page.getByLabel("Base")).toHaveValue("main");
  await expect(page.getByText("office-harness/ada")).toBeVisible();
  await expect(
    page.getByText(
      "Untracked files such as local environment files are not copied.",
    ),
  ).toBeVisible();
  await expectNoSeriousViolations(page);

  // Off sends the old create without the new fields.
  await pressOn(page, "Create agent");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const [created] = await sentFields(page, "agent.create");
  expect(created).not.toHaveProperty("own_worktree");
});

test("turning the choice on sends the base and the branch name, and the details show the rows", async ({
  page,
}) => {
  await openOffice(page);
  await fillFormByKeyboard(page, REPOSITORY);

  await page.getByLabel("Own worktree").focus();
  await page.keyboard.press("Space");
  await page.getByLabel("Base").selectOption("develop");
  await pressOn(page, "Create agent");

  await expect(page.getByRole("dialog")).toHaveCount(0);
  const [created] = await sentFields(page, "agent.create");
  expect(created).toMatchObject({
    own_worktree: true,
    base: "develop",
    branch_name: "office-harness/ada",
  });
  const details = page.getByRole("region", { name: "Ada" });
  await expect(details.getByText("Branch", { exact: true })).toBeVisible();
  await expect(details.getByText("office-harness/ada")).toBeVisible();
  await expect(details.getByText("develop", { exact: true })).toBeVisible();
  await expect(details.getByText("Worktree folder")).toBeVisible();
  await expectNoSeriousViolations(page);
});

test("a delete asks keep or remove, keep is focused, and keeping lists the kept branch and folder", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent(withWorktree("agent-ada", "office-harness/ada"))],
    worktreeStates: { "agent-ada": { changedFiles: 3 } },
  });
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await pressOn(page, "Delete agent");
  const dialog = page.getByRole("dialog", { name: "Delete Ada?" });
  await expect(
    dialog.getByRole("button", { name: "Keep worktrees" }),
  ).toBeFocused();
  await expect(dialog).toContainText("3 changed files");
  await expect(dialog).toContainText(IGNORED_FILES_NOTE);
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const [sent] = await sentFields(page, "agent.delete");
  expect(sent).toMatchObject({ worktrees: "keep" });
  await expect(
    page.getByText("Kept branches: office-harness/ada."),
  ).toBeVisible();
  await expect(
    page.getByText("Kept folders: /data/worktrees/agent-ada."),
  ).toBeVisible();
});

test("removing a worktree with changes asks again with the counts, and cancel keeps it", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent(withWorktree("agent-ada", "office-harness/ada"))],
    worktreeStates: { "agent-ada": { changedFiles: 3 } },
  });
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await pressOn(page, "Delete agent");
  await pressOn(page, "Remove worktrees");
  const dialog = page.getByRole("dialog", {
    name: "Remove worktrees with changes?",
  });
  await expect(dialog.getByRole("alert")).toContainText("Ada: 3 changed files");
  await expect(
    dialog.getByRole("button", { name: "Cancel and keep everything" }),
  ).toBeFocused();
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await sentFields(page, "agent.delete")).toEqual([]);
  await expect(
    page.getByRole("heading", { name: "Ada", exact: true }),
  ).toBeVisible();

  await pressOn(page, "Delete agent");
  await pressOn(page, "Remove worktrees");
  await pressOn(page, "Remove worktrees and changes");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const [sent] = await sentFields(page, "agent.delete");
  expect(sent).toMatchObject({
    worktrees: "remove",
    confirmed_status: { "agent-ada": "hash-agent-ada-3" },
  });
  await expect(
    page.getByText("Deleted branches: office-harness/ada."),
  ).toBeVisible();
});

const COMPANY_TREE: HierarchyTree = {
  ceo: null,
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: null,
      lead: { id: "agent-bo", role: "lead" },
      members: [
        { id: "agent-cy", role: "member" },
        { id: "agent-dee", role: "member" },
      ],
    },
  ],
  unassigned: [{ id: "agent-eve", role: null }],
};

test("a subtree delete shows one question listing every worktree", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [
      agent({ id: "agent-eve", name: "Eve" }),
      agent({
        id: "agent-bo",
        name: "Bo",
        ...withWorktree("agent-bo", "b-bo"),
      }),
      agent({
        id: "agent-cy",
        name: "Cy",
        ...withWorktree("agent-cy", "b-cy"),
      }),
      agent({
        id: "agent-dee",
        name: "Dee",
        ...withWorktree("agent-dee", "b-dee"),
      }),
    ],
    // Eve stays, so the hierarchy view is not rebuilt for an empty office.
    hierarchy: COMPANY_TREE,
    worktreeStates: {
      "agent-bo": { changedFiles: 2 },
      "agent-cy": { changedFiles: 0 },
      "agent-dee": { reachable: false },
    },
  });
  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await tree.getByRole("treeitem", { name: /^Bo, team lead/ }).click();

  await pressOn(page, "Delete Bo");
  await expect(page.getByRole("dialog")).toHaveCount(1);
  const dialog = page.getByRole("dialog", { name: "Delete Bo?" });
  await expect(dialog).toContainText("2 agents below Bo");
  await expect(dialog).toContainText("Bo, branch b-bo: 2 changed files");
  await expect(dialog).toContainText("Cy, branch b-cy: no changes");
  await expect(dialog).toContainText(
    "Dee, branch b-dee: cannot be read, will be kept",
  );
  await expect(
    dialog.getByRole("button", { name: "Keep worktrees" }),
  ).toBeFocused();
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const [sent] = await sentFields(page, "agent.delete");
  expect(sent).toMatchObject({
    agent_id: "agent-bo",
    include_descendants: true,
    descendant_count: 2,
    worktrees: "keep",
  });
  await expect(
    page.getByRole("status").filter({ hasText: "Kept branches" }),
  ).toContainText("b-dee, b-cy, b-bo");
});
