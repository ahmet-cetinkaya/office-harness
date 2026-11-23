// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const KEY = "worktree_dependency_links";
const REPOSITORY = "/work/repo";
const SHARED_NOTE = "Installed Node.js packages are shared from this checkout.";
const UNSHARED_NOTE =
  "Installed packages are not shared, so the agent installs them.";

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      repositories: {
        [REPOSITORY]: {
          base: "main",
          branches: ["main"],
          branchName: "office-harness/ada",
        },
      },
      ...office,
    },
  });
  await page.goto("/");
}

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

async function openDialog(page: Page) {
  await pressOn(page, "Worktree dependencies");
  const dialog = page.getByRole("dialog", { name: "Worktree dependencies" });
  await expect(dialog.getByRole("switch")).toBeFocused();
  return dialog;
}

async function storedSettings(page: Page) {
  const settings = await page.evaluate(() => window.__mockCore.settings);
  return settings.filter((setting) => setting.key === KEY);
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

async function openCreateForm(page: Page): Promise<void> {
  await pressOn(page, "New agent");
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.keyboard.type("Ada");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.type(REPOSITORY);
}

test("the setting is on by default, and turning it off by keyboard stores off", async ({
  page,
}) => {
  await openOffice(page);
  const dialog = await openDialog(page);

  const toggle = dialog.getByRole("switch", { name: "Worktree dependencies" });
  await expect(toggle).toBeChecked();
  await expect(dialog).toContainText("installed Node.js packages");
  await expect(dialog).toContainText(
    "On: new own worktrees share the packages.",
  );
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Space");
  await expect(dialog).toContainText("Off: each agent installs the packages");
  await pressOn(page, "Save");

  await expect(dialog).toHaveCount(0);
  expect(await storedSettings(page)).toEqual([{ key: KEY, value: "off" }]);
  const reopened = await openDialog(page);
  await expect(reopened.getByRole("switch")).not.toBeChecked();
});

test("Escape and Cancel close without storing anything", async ({ page }) => {
  await openOffice(page);
  await openDialog(page);
  await page.keyboard.press("Space");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);

  const dialog = await openDialog(page);
  await expect(dialog.getByRole("switch")).toBeChecked();
  await page.keyboard.press("Space");
  await pressOn(page, "Cancel");

  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await storedSettings(page)).toEqual([]);
});

test("a setting that cannot be saved shows the reason and keeps the dialog", async ({
  page,
}) => {
  await openOffice(page, {
    refusals: {
      "settings.set": {
        code: "internal",
        message: "storage failure: disk full",
      },
    },
  });
  const dialog = await openDialog(page);

  await page.keyboard.press("Space");
  await pressOn(page, "Save");

  await expect(dialog).toContainText("Worktree dependencies was not saved.");
  await expect(dialog).toContainText("storage failure: disk full");
  await expect(dialog.getByRole("switch")).not.toBeChecked();
  await expectNoSeriousViolations(page);
});

test("the create form note says whether the packages are shared", async ({
  page,
}) => {
  await openOffice(page);
  await openCreateForm(page);
  await expect(page.getByText(SHARED_NOTE)).toBeVisible();
  await page.keyboard.press("Escape");

  await openDialog(page);
  await page.keyboard.press("Space");
  await pressOn(page, "Save");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await openCreateForm(page);

  await expect(page.getByText(UNSHARED_NOTE)).toBeVisible();
  await expect(page.getByText(SHARED_NOTE)).toHaveCount(0);
});
