// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const JUMP_KEY = "Control+Shift+F11";
const SHORT_TEXT = "The migration is done and the tests pass.";
const LONG_TEXT = `${"The nightly run found a slow query in the report page. ".repeat(6)}End of report.`;
const SHOW_WHOLE_NAME = "Show the whole report";
const SHOW_LESS_NAME = "Show less";

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

async function openPanel(page: Page): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: [agent(), agent({ id: "agent-bo", name: "Bo" })] },
  });
  await page.goto("/");
  await expect(needsYouSection(page)).toBeVisible();
}

function needsYouSection(page: Page) {
  return page.getByRole("region", { name: /^Needs you/ });
}

function sendReport(
  page: Page,
  agentId: string,
  kind: string,
  text: string,
): Promise<void> {
  return page.evaluate(
    ([id, reportKind, reportText]) =>
      window.__mockCore.report(id, reportKind, reportText),
    [agentId, kind, text],
  );
}

test.describe("a report in the Needs you section", () => {
  test("a report row shows the event, the kind, and the text, and names the report as its description", async ({
    page,
  }) => {
    await openPanel(page);

    await sendReport(page, "agent-ada", "done", SHORT_TEXT);

    const row = needsYouSection(page).getByRole("group", { name: /^Ada,/ });
    await expect(row).toHaveAccessibleName(
      /^Ada, sent a report: done, waiting \d+ s$/,
    );
    await expect(row).toContainText("sent a report: done");
    await expect(row).toContainText(`done: ${SHORT_TEXT}`);
    await expect(row).toHaveAccessibleDescription(`done: ${SHORT_TEXT}`);
    await expect(
      row.getByRole("button", { name: SHOW_WHOLE_NAME }),
    ).toHaveCount(0);
  });

  test("the report text is shown as text, never as markup", async ({
    page,
  }) => {
    await openPanel(page);

    await sendReport(page, "agent-ada", "info", "<b>bold</b>");

    const row = needsYouSection(page).getByRole("group", { name: /^Ada,/ });
    await expect(row).toContainText("info: <b>bold</b>");
    await expect(row.locator("b")).toHaveCount(0);
  });

  test("a long report is clamped, and the toggle works from the keyboard", async ({
    page,
  }) => {
    await openPanel(page);
    await sendReport(page, "agent-ada", "question", LONG_TEXT);
    const row = needsYouSection(page).getByRole("group", { name: /^Ada,/ });
    const toggle = row.getByRole("button", { name: SHOW_WHOLE_NAME });
    const text = row.getByText(`question: ${LONG_TEXT}`);

    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    const clampedHeight = (await text.boundingBox())?.height ?? 0;
    const box = await toggle.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);

    await toggle.focus();
    await page.keyboard.press("Enter");

    const lessToggle = row.getByRole("button", { name: SHOW_LESS_NAME });
    await expect(lessToggle).toHaveAttribute("aria-expanded", "true");
    await expect(lessToggle).toBeFocused();
    expect((await text.boundingBox())?.height ?? 0).toBeGreaterThan(
      clampedHeight,
    );

    await page.keyboard.press("Space");

    await expect(
      row.getByRole("button", { name: SHOW_WHOLE_NAME }),
    ).toHaveAttribute("aria-expanded", "false");
  });

  test("the toggle shows a focus ring and does not open the terminal", async ({
    page,
  }) => {
    await openPanel(page);
    await sendReport(page, "agent-ada", "blocked", LONG_TEXT);
    const toggle = needsYouSection(page).getByRole("button", {
      name: SHOW_WHOLE_NAME,
    });

    await toggle.focus();
    const outline = await toggle.evaluate((element) => {
      const style = getComputedStyle(element);
      return { width: style.outlineWidth, style: style.outlineStyle };
    });
    await toggle.click();

    expect(outline.style).not.toBe("none");
    expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(2);
    await expect(page.getByLabel("Terminal of Ada")).toHaveCount(0);
  });

  test("the jump shortcut opens the terminal of the report's agent and the row leaves", async ({
    page,
  }) => {
    await openPanel(page);
    await sendReport(page, "agent-ada", "done", SHORT_TEXT);
    await expect(needsYouSection(page).getByRole("group")).toHaveCount(1);

    await page.keyboard.press(JUMP_KEY);

    await expect(
      page.getByLabel("Terminal of Ada", { exact: true }),
    ).toBeFocused();
    await expect(needsYouSection(page).getByRole("group")).toHaveCount(0);
  });

  test("a report notification names the kind", async ({ page }) => {
    await openPanel(page);

    await sendReport(page, "agent-bo", "blocked", SHORT_TEXT);

    await expect(
      page.getByText("Bo sent a report: blocked").first(),
    ).toBeAttached();
  });

  test("the section has no serious accessibility violation with short and long reports", async ({
    page,
  }) => {
    await openPanel(page);
    await sendReport(page, "agent-ada", "done", SHORT_TEXT);
    await sendReport(page, "agent-bo", "question", LONG_TEXT);
    await expect(needsYouSection(page).getByRole("group")).toHaveCount(2);

    const report = await new AxeBuilder({ page })
      .include('section[aria-labelledby="needs-you-heading"]')
      .analyze();

    expect(
      report.violations.filter((violation) =>
        SERIOUS_IMPACTS.includes(violation.impact ?? ""),
      ),
    ).toEqual([]);
  });
});
