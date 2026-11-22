// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Locator, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import {
  AGENT_CLEAN,
  AGENT_NOT_REPOSITORY,
  HOSTILE_COMMIT_IMAGE,
  HOSTILE_FILE_IMAGE,
  SCRIPT_FLAG,
  SEEDED_COMMIT_COUNT,
} from "./support/file-fixture";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const DESKTOP = { width: 1280, height: 720 };
const ZOOMED_200_PERCENT = { width: 640, height: 720 };
const FIRST_PAGE = 50;
const SLOW_OFFICE_MS = 800;

function agent(id = "agent-ada"): Agent {
  return {
    id,
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1_700_000_000_000,
    last_line: null,
  };
}

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

/** Opens the file view of the agent from its details, by keyboard. */
async function openFileView(
  page: Page,
  agentId = "agent-ada",
  office: OfficeSeed = {},
): Promise<Locator> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: [agent(agentId)], ...office },
  });
  await page.setViewportSize(DESKTOP);
  await page.goto("/");
  await page.getByRole("button", { name: /^Ada/ }).click();
  await pressOn(page, "Browse files");
  const dialog = page.getByRole("dialog", { name: "Files of Ada" });
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

function treeItem(page: Page, path: string): Locator {
  return page.locator(`[role="treeitem"][data-path="${path}"]`);
}

async function openTreeItem(page: Page, path: string): Promise<void> {
  await treeItem(page, path).focus();
  await page.keyboard.press("Enter");
}

async function openTab(page: Page, name: string): Promise<void> {
  await page.getByRole("tab", { name, exact: true }).click();
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

async function lastRevealFlag(page: Page, path: string): Promise<unknown> {
  const reads = await sentFields(page, "agent.files.read");
  return reads.filter((fields) => fields.path === path).at(-1)?.reveal;
}

/**
 * Whether the dialog sticks out of the viewport or scrolls sideways itself.
 * The panel behind it is not measured: it already scrolls sideways at this
 * width, and the dialog covers it.
 */
async function overflowsSideways(dialog: Locator): Promise<boolean> {
  return dialog.evaluate((element) => {
    const box = element.getBoundingClientRect();
    return (
      element.scrollWidth > element.clientWidth ||
      box.left < 0 ||
      box.right > window.innerWidth
    );
  });
}

test.describe("opening and closing", () => {
  test("opens from the details by keyboard, shows the header, and closes with Escape", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await expect(dialog.getByText("/work/ada")).toBeVisible();
    await expect(dialog.getByText(/^Loaded at /)).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Refresh", exact: true }),
    ).toBeVisible();
    await expect(dialog.getByRole("tab", { name: "Files" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(treeItem(page, "src")).toBeVisible();
    await expectNoSeriousViolations(page);

    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(
      page.getByRole("button", { name: "Browse files", exact: true }),
    ).toBeFocused();
  });

  test("moves between the parts with the arrow keys, Home, and End", async ({
    page,
  }) => {
    await openFileView(page);
    await page.getByRole("tab", { name: "Files" }).focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("tab", { name: "Changes" })).toBeFocused();
    await expect(page.getByRole("tab", { name: "Changes" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await page.keyboard.press("End");
    await expect(page.getByRole("tab", { name: "History" })).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("tab", { name: "Files" })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(page.getByRole("tab", { name: "History" })).toBeFocused();
    await page.keyboard.press("Home");
    await expect(page.getByRole("tab", { name: "Files" })).toBeFocused();
  });
});

test.describe("the tree", () => {
  test("lists folders first and follows the tree keyboard pattern", async ({
    page,
  }) => {
    await openFileView(page);
    const paths = await page
      .locator('[role="treeitem"]')
      .evaluateAll((items) =>
        items.map((item) => item.getAttribute("data-path")),
      );
    expect(paths.slice(0, 3)).toEqual(["empty", "src", ".env"]);

    await treeItem(page, "empty").focus();
    await page.keyboard.press("ArrowDown");
    await expect(treeItem(page, "src")).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(treeItem(page, "src")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(treeItem(page, "src/main.txt")).toBeVisible();
    await page.keyboard.press("ArrowRight");
    await expect(treeItem(page, "src/main.txt")).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(treeItem(page, "src")).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(treeItem(page, "src")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await page.keyboard.press("End");
    await expect(treeItem(page, "outside-link")).toBeFocused();
    await page.keyboard.press("Home");
    await expect(treeItem(page, "empty")).toBeFocused();

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(treeItem(page, "src")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await openTreeItem(page, "src/main.txt");
    const content = page.getByRole("region", { name: "File content" });
    await expect(content).toContainText("second line");
    await expectNoSeriousViolations(page);
  });
});

test.describe("every visible state", () => {
  test("shows a loading text while the office answers", async ({ page }) => {
    const dialog = await openFileView(page, "agent-ada", {
      fileViewDelayMs: SLOW_OFFICE_MS,
    });
    await expect(
      dialog.getByRole("status").filter({ hasText: "Loading the folder." }),
    ).toBeVisible();
    await expect(treeItem(page, "src")).toBeVisible();
  });

  test("shows an empty folder", async ({ page }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, "empty");
    await expect(dialog.getByText("This folder is empty.")).toBeVisible();
  });

  test("shows a text file with line numbers, read only", async ({ page }) => {
    await openFileView(page);
    await openTreeItem(page, "src");
    await openTreeItem(page, "src/main.txt");
    const content = page.getByRole("region", { name: "File content" });
    await expect(content.getByRole("row")).toHaveCount(3);
    await expect(
      content.getByRole("row").nth(1).getByRole("rowheader"),
    ).toHaveText("2");
    await expect(content.getByText("second line")).toBeVisible();
    await expect(content.locator("textarea, input")).toHaveCount(0);
  });

  test("shows the size and the limit of a file that is too large", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, "big.log");
    await expect(
      dialog.getByText(/^This file is 1,048,576 bytes/),
    ).toContainText("512 KiB");
    await expect(
      page.getByRole("region", { name: "File content" }),
    ).toHaveCount(0);
  });

  test("says that a binary file is not shown", async ({ page }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, "image.bin");
    await expect(dialog.getByText("Binary file, not shown")).toBeVisible();
  });

  test("says that a link points outside and never asks for its target", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, "outside-link");
    await expect(
      dialog.getByText(/points outside the agent's working folder/),
    ).toBeVisible();
    await openTreeItem(page, "broken-link");
    await expect(dialog.getByText(/missing or forms a loop/)).toBeVisible();
    const reads = await sentFields(page, "agent.files.read");
    expect(reads).toEqual([]);
  });

  test("says that a name that is not valid UTF-8 cannot be opened", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, "bad-�name.txt");
    await expect(dialog.getByText(/not valid UTF-8/)).toBeVisible();
    expect(await sentFields(page, "agent.files.read")).toEqual([]);
  });

  test("says that a secret-looking file may hold a secret", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, ".env");
    await expect(dialog.getByText("This file may hold a secret")).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Reveal" })).toBeVisible();
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);
  });

  test("says that the folder is not in a git repository", async ({ page }) => {
    const dialog = await openFileView(page, AGENT_NOT_REPOSITORY);
    await openTab(page, "Changes");
    await expect(
      dialog.getByText("This folder is not in a git repository."),
    ).toBeVisible();
    await openTab(page, "History");
    await expect(
      dialog.getByText("This folder is not in a git repository."),
    ).toBeVisible();
  });

  test("says that there is no change and no commit", async ({ page }) => {
    const dialog = await openFileView(page, AGENT_CLEAN);
    await openTab(page, "Changes");
    await expect(
      dialog.getByText("No change since the last commit."),
    ).toBeVisible();
    await openTab(page, "History");
    await expect(dialog.getByText("No commit yet.")).toBeVisible();
  });

  test("shows a diff, and says when a diff is too large or binary", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTab(page, "Changes");
    await page.getByRole("button", { name: /src\/main\.txt/ }).click();
    await expect(page.getByRole("region", { name: "Diff" })).toContainText(
      "second line changed",
    );
    await page.getByRole("button", { name: /huge\.txt/ }).click();
    await expect(dialog.getByText(/over the limit/)).toContainText("512 KiB");
    await page.getByRole("button", { name: /image\.bin/ }).click();
    await expect(dialog.getByText("Binary file, not shown")).toBeVisible();
    await page.getByRole("button", { name: /quiet\.txt/ }).click();
    await expect(
      dialog.getByText("This file has no change here."),
    ).toBeVisible();
  });

  test("shows each failure with its reason and a Refresh action", async ({
    page,
  }) => {
    const reason = "the folder does not exist";
    const dialog = await openFileView(page, "agent-ada", {
      refusals: {
        "agent.files.list": { code: "internal", message: reason },
        "agent.files.changes": {
          code: "internal",
          message: "git did not answer in time",
        },
      },
    });
    await expect(dialog.getByRole("alert")).toContainText(reason);
    await expect(
      dialog.getByRole("tabpanel").getByRole("button", { name: "Refresh" }),
    ).toBeVisible();
    await expectNoSeriousViolations(page);

    await openTab(page, "Changes");
    await expect(dialog.getByRole("alert")).toContainText(
      "git did not answer in time",
    );
    const before = (await sentFields(page, "agent.files.changes")).length;
    await dialog
      .getByRole("tabpanel")
      .getByRole("button", { name: "Refresh" })
      .click();
    await expect
      .poll(async () => (await sentFields(page, "agent.files.changes")).length)
      .toBeGreaterThan(before);
  });
});

test.describe("reveal", () => {
  test("shows a secret file once and hides it again on the next open", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, ".env");
    expect(await lastRevealFlag(page, ".env")).toBe(false);
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);

    await dialog.getByRole("button", { name: "Reveal" }).click();
    await expect(page.getByText("API_TOKEN=abc")).toBeVisible();
    expect(await lastRevealFlag(page, ".env")).toBe(true);

    await openTreeItem(page, "notes.txt");
    await expect(page.getByText("plain notes")).toBeVisible();
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);
    expect(await lastRevealFlag(page, "notes.txt")).toBe(false);

    await openTreeItem(page, ".env");
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);
    expect(await lastRevealFlag(page, ".env")).toBe(false);
  });

  test("hides a revealed file on a tab switch and on refresh", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTreeItem(page, ".env");
    await dialog.getByRole("button", { name: "Reveal" }).click();
    await expect(page.getByText("API_TOKEN=abc")).toBeVisible();

    await openTab(page, "Changes");
    await openTab(page, "Files");
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);

    await openTreeItem(page, ".env");
    await dialog.getByRole("button", { name: "Reveal" }).click();
    await expect(page.getByText("API_TOKEN=abc")).toBeVisible();
    await dialog.getByRole("button", { name: "Refresh", exact: true }).click();
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);
  });

  test("treats a rename of a secret-looking file as secret until revealed", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTab(page, "Changes");
    await page.getByRole("button", { name: /notes\.txt/ }).click();
    await expect(
      dialog.getByText("This change may hold a secret"),
    ).toBeVisible();
    await expect(page.getByText("API_TOKEN")).toHaveCount(0);
    const asked = await sentFields(page, "agent.files.diff");
    expect(asked.at(-1)).toMatchObject({
      path: "notes.txt",
      old_path: ".env",
      reveal: false,
    });

    await dialog.getByRole("button", { name: "Reveal" }).click();
    await expect(page.getByRole("region", { name: "Diff" })).toContainText(
      "API_TOKEN=abc",
    );
    expect((await sentFields(page, "agent.files.diff")).at(-1)).toMatchObject({
      reveal: true,
    });
  });
});

test.describe("hostile text", () => {
  test("shows a file as text, with control characters as symbols", async ({
    page,
  }) => {
    const requests: string[] = [];
    page.on("request", (request) => requests.push(request.url()));
    const dialog = await openFileView(page);
    await openTreeItem(page, "hostile.txt");
    const content = page.getByRole("region", { name: "File content" });
    await expect(content).toContainText(`<script>window.${SCRIPT_FLAG} = true`);
    await expect(content).toContainText("<img src=");
    await expect(content).toContainText("␛");
    await expect(content).toContainText("\\u202E");
    await expect(dialog.locator("img, script, a")).toHaveCount(0);
    expect(await page.evaluate((flag) => flag in window, SCRIPT_FLAG)).toBe(
      false,
    );
    expect(requests.some((url) => url.includes(HOSTILE_FILE_IMAGE))).toBe(
      false,
    );
  });

  test("shows a commit message as text, with control characters as symbols", async ({
    page,
  }) => {
    const requests: string[] = [];
    page.on("request", (request) => requests.push(request.url()));
    const dialog = await openFileView(page);
    await openTab(page, "History");
    await dialog
      .getByRole("list", { name: "Commits" })
      .getByRole("button")
      .first()
      .click();
    const details = page.getByRole("region", { name: "Commit details" });
    await expect(details).toContainText(`<script>window.${SCRIPT_FLAG} = true`);
    await expect(details).toContainText("<img src=");
    await expect(details).toContainText("␛");
    await expect(details).toContainText("\\u202E");
    await expect(dialog.locator("img, script, a")).toHaveCount(0);
    expect(await page.evaluate((flag) => flag in window, SCRIPT_FLAG)).toBe(
      false,
    );
    expect(requests.some((url) => url.includes(HOSTILE_COMMIT_IMAGE))).toBe(
      false,
    );
  });
});

test.describe("changes and history", () => {
  test("marks added and removed lines with a sign, numbers, and hidden text", async ({
    page,
  }) => {
    await openFileView(page);
    await openTab(page, "Changes");
    await page.getByRole("button", { name: /src\/main\.txt/ }).click();
    const diff = page.getByRole("region", { name: "Diff" });
    const added = diff
      .getByRole("row")
      .filter({ hasText: "second line changed" });
    await expect(added.locator('[aria-hidden="true"]')).toHaveText("+");
    await expect(added.locator(".sr-only")).toHaveText("added");
    const removed = diff
      .getByRole("row")
      .filter({ hasText: "second line" })
      .first();
    await expect(removed.locator(".sr-only")).toHaveText("removed");
    await expect(removed.getByRole("rowheader")).toHaveText("2");
    await expect(diff).toContainText("@@ -1,3 +1,4 @@");
    await expect(diff).toContainText("\\ No newline at end of file");
    await expect(
      page.getByRole("button", { name: /renamed notes\.txt/ }),
    ).toContainText("from .env");
    await expect(
      page.getByRole("button", { name: /deleted gone\.txt/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /untracked new\.txt/ }),
    ).toBeVisible();
    await expectNoSeriousViolations(page);
  });

  test("lists 50 commits, then 100, then all 120, and opens a commit and its diff", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await openTab(page, "History");
    const commits = dialog
      .getByRole("list", { name: "Commits" })
      .getByRole("listitem");
    await expect(commits).toHaveCount(FIRST_PAGE);
    await dialog.getByRole("button", { name: "Show more" }).click();
    await expect(commits).toHaveCount(2 * FIRST_PAGE);
    await dialog.getByRole("button", { name: "Show more" }).click();
    await expect(commits).toHaveCount(SEEDED_COMMIT_COUNT);
    await expect(dialog.getByRole("button", { name: "Show more" })).toHaveCount(
      0,
    );
    expect(
      (await sentFields(page, "agent.files.history")).map(
        (fields) => fields.skip,
      ),
    ).toEqual([0, 50, 100]);

    await commits.nth(1).getByRole("button").click();
    const details = page.getByRole("region", { name: "Commit details" });
    await expect(details).toContainText("Ada Lovelace");
    await expect(details).toContainText(/[0-9a-f]{40}/);
    await details.getByRole("button", { name: /src\/main\.txt/ }).click();
    await expect(page.getByRole("region", { name: "Diff" })).toContainText(
      "second line changed",
    );
    const asked = (await sentFields(page, "agent.files.diff")).at(-1);
    expect(asked?.commit).toMatch(/^[0-9a-f]{40}$/);
    await expectNoSeriousViolations(page);
  });
});

test.describe("reflow", () => {
  test("keeps every part inside the dialog at 200 percent zoom", async ({
    page,
  }) => {
    const dialog = await openFileView(page);
    await page.setViewportSize(ZOOMED_200_PERCENT);
    await openTreeItem(page, "hostile.txt");
    await expect(
      page.getByRole("region", { name: "File content" }),
    ).toBeVisible();
    expect(await overflowsSideways(dialog)).toBe(false);
    await openTab(page, "Changes");
    await page.getByRole("button", { name: /src\/main\.txt/ }).click();
    await expect(page.getByRole("region", { name: "Diff" })).toBeVisible();
    expect(await overflowsSideways(dialog)).toBe(false);
    await openTab(page, "History");
    await expect(dialog.getByRole("list", { name: "Commits" })).toBeVisible();
    expect(await overflowsSideways(dialog)).toBe(false);
  });
});
