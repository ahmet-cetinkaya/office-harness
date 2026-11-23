// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type { FinishedTemp } from "../src/modules/temps/models/temps";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const REPOSITORY = "/work/repo";
const OBJECTIVE = "Find the flaky test";
const REMOVAL_TEXT =
  "Scout will be removed. Work that is not merged stays in its worktree and branch. Files that git ignores are removed with a removed worktree.";
const NOTHING_KEPT =
  "nothing kept; files that git ignores were removed with the worktree";
const TEMP_CAP_REFUSAL = "temp_cap is a whole number from 1 to 100";
const TEMP_IDLE_REFUSAL =
  "temp_idle_minutes is a whole number of minutes from 5 to 1440";
const NARROW_VIEWPORT = { width: 360, height: 740 };
const FOCUS_OUTLINE_WIDTH = "2px";

const REPOSITORIES: OfficeSeed["repositories"] = {
  [REPOSITORY]: {
    base: "main",
    branches: ["main"],
    branchName: "office-harness/ada",
  },
};

function scout(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-scout",
    name: "Scout",
    command: "claude",
    working_folder: REPOSITORY,
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1_700_000_000_000,
    last_line: null,
    temp: { objective: OBJECTIVE, creator: "user" },
    ...overrides,
  };
}

function finishedEntry(overrides: Partial<FinishedTemp> = {}): FinishedTemp {
  return {
    id: "finished-1",
    agent_id: "agent-gone",
    name: "Courier",
    objective: "Check the release notes",
    creator: "user",
    creator_name: "the person",
    parent_name: null,
    started_at: 1_700_000_000_000,
    ended_at: 1_700_000_600_000,
    reason: "done",
    report: { kind: "done", text: "The notes are complete" },
    kept_folder: null,
    kept_branch: null,
    removal_failure: null,
    ...overrides,
  };
}

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { repositories: REPOSITORIES, ...office },
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Idle sleep" })).toBeVisible();
}

const seededTemps: OfficeSeed = {
  agents: [scout()],
  tempRows: {
    "agent-scout": { run_seconds: 12 * 60 + 20, quiet_seconds: 90 },
  },
  finishedTemps: [finishedEntry()],
};

async function openTemps(page: Page) {
  await page.getByRole("button", { name: /^Temps \(\d+\)$/ }).click();
  const dialog = page.getByRole("dialog", { name: "Temps" });
  await expect(dialog).toBeVisible();
  return dialog;
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
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

async function fillAgentForm(page: Page, folder: string): Promise<void> {
  await page.getByRole("button", { name: "New agent" }).first().click();
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.getByLabel("Command").fill("claude");
  await page.getByLabel("Working folder").fill(folder);
}

test.describe("create form", () => {
  test("a temp with an objective sends temp and objective and starts own worktree on", async ({
    page,
  }) => {
    await openOffice(page);
    await fillAgentForm(page, REPOSITORY);

    await page.getByLabel("Temporary agent (one job)").check();
    await expect(page.getByLabel("Own worktree")).toBeChecked();
    const objective = page.getByLabel("Objective");
    await expect(objective).toBeVisible();
    await expect(page.getByRole("dialog")).toContainText(
      "must not be blank and has at most 8,192 characters",
    );
    await objective.fill(OBJECTIVE);
    await page.getByRole("button", { name: "Create agent" }).click();

    await expect(page.getByRole("dialog")).toHaveCount(0);
    const [created] = await sentFields(page, "agent.create");
    expect(created).toMatchObject({
      temp: true,
      objective: OBJECTIVE,
      own_worktree: true,
      branch_name: "office-harness/ada",
    });
  });

  test("the person can turn the own worktree off for a temp", async ({
    page,
  }) => {
    await openOffice(page);
    await fillAgentForm(page, REPOSITORY);
    await page.getByLabel("Temporary agent (one job)").check();
    await expect(page.getByLabel("Own worktree")).toBeChecked();

    await page.getByLabel("Own worktree").uncheck();
    await page.getByLabel("Objective").fill(OBJECTIVE);
    await page.getByRole("button", { name: "Create agent" }).click();

    await expect(page.getByRole("dialog")).toHaveCount(0);
    const [created] = await sentFields(page, "agent.create");
    expect(created).toMatchObject({ temp: true, objective: OBJECTIVE });
    expect(created).not.toHaveProperty("own_worktree");
  });

  test("a blank objective blocks the create with the text", async ({
    page,
  }) => {
    await openOffice(page);
    await fillAgentForm(page, "/work/plain");
    await page.getByLabel("Temporary agent (one job)").check();
    await page.getByLabel("Objective").fill("   ");
    await page.getByRole("button", { name: "Create agent" }).click();

    await expect(page.getByRole("alert")).toContainText(
      "A temp needs an objective",
    );
    await expect(page.getByLabel("Objective")).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    expect(await sentFields(page, "agent.create")).toEqual([]);
  });

  test("an objective with a line break blocks the create with the text", async ({
    page,
  }) => {
    await openOffice(page);
    await fillAgentForm(page, "/work/plain");
    await page.getByLabel("Temporary agent (one job)").check();
    await page.getByLabel("Objective").fill("First line\nSecond line");
    await page.getByRole("button", { name: "Create agent" }).click();

    await expect(page.getByRole("alert")).toContainText(
      "The objective must be one line.",
    );
    expect(await sentFields(page, "agent.create")).toEqual([]);
  });

  test("an agent that is not a temp sends neither field", async ({ page }) => {
    await openOffice(page);
    await fillAgentForm(page, "/work/plain");
    await page.getByRole("button", { name: "Create agent" }).click();

    await expect(page.getByRole("dialog")).toHaveCount(0);
    const [created] = await sentFields(page, "agent.create");
    expect(created).not.toHaveProperty("temp");
    expect(created).not.toHaveProperty("objective");
  });
});

test.describe("temps dialog", () => {
  test("it lists temps and finished entries with every field", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    const dialog = await openTemps(page);

    const row = dialog.getByRole("article", {
      name: "Scout, working, created by the person",
    });
    await expect(row).toContainText("the person");
    await expect(row).toContainText("none");
    await expect(row).toContainText(OBJECTIVE);
    await expect(row).toContainText("working");
    await expect(row).toContainText("12 min");
    await expect(row).toContainText("1 min");
    await expect(row.getByRole("button", { name: "End Scout" })).toBeVisible();

    const entry = dialog.getByRole("article", {
      name: "Courier, finished, created by the person",
    });
    await expect(entry).toContainText("Check the release notes");
    await expect(entry).toContainText("Done");
    await expect(entry).toContainText("Done report: The notes are complete");
    await expect(entry).toContainText(NOTHING_KEPT);
    await expect(entry).toContainText(/2023/);
    await expect(
      entry.getByRole("button", { name: "Dismiss Courier" }),
    ).toBeVisible();
    await expect(entry).toHaveAccessibleDescription(
      /Check the release notes.*The notes are complete/,
    );
  });

  test("a finished temp that kept its work names the folder and branch", async ({
    page,
  }) => {
    await openOffice(page, {
      finishedTemps: [
        finishedEntry({
          kept_folder: "/data/worktrees/courier",
          kept_branch: "office-harness/courier",
          report: null,
        }),
      ],
    });
    const dialog = await openTemps(page);
    await expect(dialog).toContainText("folder /data/worktrees/courier");
    await expect(dialog).toContainText("branch office-harness/courier");
    await expect(dialog).not.toContainText(NOTHING_KEPT);
  });

  test("both lists say when they are empty", async ({ page }) => {
    await openOffice(page);
    const dialog = await openTemps(page);
    await expect(dialog.getByText("No temps.")).toBeVisible();
    await expect(dialog.getByText("No finished temps.")).toBeVisible();
  });

  test("End asks, Cancel and Escape end nothing, and confirming sends temp.end", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    const dialog = await openTemps(page);
    const end = dialog.getByRole("button", { name: "End Scout" });

    await end.click();
    const confirm = page.getByRole("dialog", { name: "End Scout?" });
    await expect(confirm).toContainText(REMOVAL_TEXT);
    await expect(confirm.getByRole("button", { name: "Cancel" })).toBeFocused();
    await confirm.getByRole("button", { name: "Cancel" }).click();
    await expect(confirm).toHaveCount(0);

    await end.click();
    await expect(confirm).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(confirm).toHaveCount(0);
    await expect(dialog).toBeVisible();
    expect(await sentFields(page, "temp.end")).toEqual([]);

    await end.click();
    await confirm.getByRole("button", { name: "End temp" }).click();

    await expect(dialog.getByText("No temps.")).toBeVisible();
    expect(await sentFields(page, "temp.end")).toEqual([
      { agent_id: "agent-scout" },
    ]);
    await expect(
      dialog.getByRole("article", { name: /^Scout, finished/ }),
    ).toContainText("Ended by the person");
  });

  test("Dismiss sends temp.dismiss and the entry leaves the list", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    const dialog = await openTemps(page);

    await dialog.getByRole("button", { name: "Dismiss Courier" }).click();

    await expect(dialog.getByText("No finished temps.")).toBeVisible();
    expect(await sentFields(page, "temp.dismiss")).toEqual([
      { entry_id: "finished-1" },
    ]);
  });

  test("the header button counts the temps", async ({ page }) => {
    await openOffice(page, seededTemps);
    await expect(page.getByRole("button", { name: "Temps (1)" })).toBeVisible();
  });

  test("it works by keyboard alone, with a visible focus", async ({ page }) => {
    await openOffice(page, seededTemps);
    await page.getByRole("button", { name: "Temps (1)" }).focus();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "Temps" });
    const end = dialog.getByRole("button", { name: "End Scout" });
    await expect(end).toBeFocused();
    await expect(end).toHaveCSS("outline-width", FOCUS_OUTLINE_WIDTH);

    await page.keyboard.press("Enter");
    const confirm = page.getByRole("dialog", { name: "End Scout?" });
    await expect(confirm.getByRole("button", { name: "Cancel" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(confirm).toHaveCount(0);
    await expect(end).toBeFocused();

    await page.keyboard.press("Tab");
    const dismiss = dialog.getByRole("button", { name: "Dismiss Courier" });
    await expect(dismiss).toBeFocused();
    await expect(dismiss).toHaveCSS("outline-width", FOCUS_OUTLINE_WIDTH);
    await page.keyboard.press("Tab");
    await expect(dialog.getByRole("button", { name: "Close" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });

  test("it has no horizontal scroll at 360 px", async ({ page }) => {
    await page.setViewportSize(NARROW_VIEWPORT);
    await openOffice(page, seededTemps);
    const dialog = await openTemps(page);

    const overflow = await page.evaluate(() => {
      const dialogElement = document.querySelector("dialog[open]");
      return {
        page:
          document.documentElement.scrollWidth -
          document.documentElement.clientWidth,
        dialog:
          (dialogElement?.scrollWidth ?? 0) - (dialogElement?.clientWidth ?? 0),
      };
    });
    expect(overflow.page).toBeLessThanOrEqual(0);
    expect(overflow.dialog).toBeLessThanOrEqual(0);
    await expect(
      dialog.getByRole("button", { name: "End Scout" }),
    ).toBeVisible();
  });

  test("an axe scan finds no serious or critical violation", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    await openTemps(page);
    await expectNoSeriousViolations(page);
  });
});

test.describe("temp details", () => {
  test("the details show the Temp row with objective and creator", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    await page.getByRole("button", { name: /^Scout, working/ }).click();

    const details = page.getByRole("region", { name: "Scout" });
    await expect(details.getByText("Temp", { exact: true })).toBeVisible();
    await expect(details).toContainText(OBJECTIVE);
    await expect(details).toContainText("Created by the person");
  });

  test("a stop of a temp asks with the removal text, and Cancel stops nothing", async ({
    page,
  }) => {
    await openOffice(page, seededTemps);
    await page.getByRole("button", { name: /^Scout, working/ }).click();
    const details = page.getByRole("region", { name: "Scout" });

    await details.getByRole("button", { name: "Stop agent" }).click();
    const confirm = page.getByRole("dialog", { name: "End Scout?" });
    await expect(confirm).toContainText(REMOVAL_TEXT);
    await expect(confirm.getByRole("button", { name: "Cancel" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(confirm).toHaveCount(0);
    expect(await sentFields(page, "agent.stop")).toEqual([]);

    await details.getByRole("button", { name: "Stop agent" }).click();
    await confirm.getByRole("button", { name: "End temp" }).click();
    await expect
      .poll(async () => (await sentFields(page, "agent.stop")).length)
      .toBe(1);
  });
});

test.describe("temp limits", () => {
  async function openLimits(page: Page) {
    await page.getByRole("button", { name: "Limits" }).click();
    const dialog = page.getByRole("dialog", { name: "Limits" });
    await expect(dialog.getByLabel("Temp cap")).toBeVisible();
    return dialog;
  }

  test("the cap and the idle limit show their defaults", async ({ page }) => {
    await openOffice(page);
    const dialog = await openLimits(page);
    await expect(dialog.getByLabel("Temp cap")).toHaveValue("8");
    await expect(dialog.getByLabel("Temp idle limit, minutes")).toHaveValue(
      "30",
    );
  });

  test("a cap of 0 or 101 and an idle limit of 4 or 1441 are refused", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openLimits(page);
    const cap = dialog.getByLabel("Temp cap");
    const idle = dialog.getByLabel("Temp idle limit, minutes");

    for (const text of ["0", "101"]) {
      await cap.fill(text);
      await dialog.getByRole("button", { name: "Save limits" }).click();
      await expect(dialog.getByRole("alert")).toContainText(TEMP_CAP_REFUSAL);
      await expect(cap).toHaveAttribute("aria-invalid", "true");
    }
    await cap.fill("8");
    for (const text of ["4", "1441"]) {
      await idle.fill(text);
      await dialog.getByRole("button", { name: "Save limits" }).click();
      await expect(dialog.getByRole("alert")).toContainText(TEMP_IDLE_REFUSAL);
      await expect(idle).toHaveAttribute("aria-invalid", "true");
    }
    const saved = await page.evaluate(() =>
      window.__mockCore.settings.filter((setting) =>
        setting.key.startsWith("temp_"),
      ),
    );
    expect(saved).toEqual([]);
  });

  test("valid values are saved under their keys", async ({ page }) => {
    await openOffice(page);
    const dialog = await openLimits(page);
    await dialog.getByLabel("Temp cap").fill("100");
    await dialog.getByLabel("Temp idle limit, minutes").fill("5");
    await dialog.getByRole("button", { name: "Save limits" }).click();

    await expect(dialog).toHaveCount(0);
    const saved = await page.evaluate(() => window.__mockCore.settings);
    expect(saved).toContainEqual({ key: "temp_cap", value: "100" });
    expect(saved).toContainEqual({ key: "temp_idle_minutes", value: "5" });
  });
});
