// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const UNCLEAN_WARNING =
  "The server stopped uncleanly. CLI processes of the marked agents from before may still run, and resuming can start a second copy next to them.";
const UNCLEAN_MARK = "Was running when the server stopped uncleanly";
const REFUSED_REASON = "the CLI refused the session";
const FAILED_REASON = "the new session ended within its check period";
const DIALOG_NAME = "Resume interrupted agents";

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ada",
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "interrupted",
    created_at: 1_700_000_000_000,
    last_line: null,
    ...overrides,
  };
}

const ADA = agent();
const BO = agent({ id: "agent-bo", name: "Bo" });
const CY = agent({ id: "agent-cy", name: "Cy" });
const DEE = agent({ id: "agent-dee", name: "Dee", status: "working" });

async function openPanel(page: Page, office: OfficeSeed): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Hold office" })).toBeVisible();
}

async function openDialog(page: Page, count: number) {
  await page
    .getByRole("button", { name: `Resume interrupted (${count})` })
    .click();
  const dialog = page.getByRole("dialog", { name: DIALOG_NAME });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("checkbox").first()).toBeVisible();
  return dialog;
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

async function requestFields(page: Page, kind: string) {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter(
      (call) => call.command === "send_request" && call.args.kind === kind,
    )
    .map((call) => call.args.fields);
}

test.describe("resume interrupted", () => {
  test("the button shows the count of interrupted agents and is absent at zero", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA, BO, DEE] });
    await expect(
      page.getByRole("button", { name: "Resume interrupted (2)" }),
    ).toBeVisible();
  });

  test("no interrupted agent, no button", async ({ page }) => {
    await openPanel(page, { agents: [DEE] });
    await expect(
      page.getByRole("button", { name: /Resume interrupted/ }),
    ).toHaveCount(0);
  });

  test("the list shows each agent with its plan text and checks them all", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [ADA, BO, CY],
      holds: { agents: [{ agent_id: "agent-cy", set_by: "user", since: 1 }] },
      resume: { "agent-bo": { canContinue: false, reason: "no session id" } },
    });
    const dialog = await openDialog(page, 3);

    const boxes = dialog.getByRole("checkbox");
    await expect(boxes).toHaveCount(3);
    await expect(boxes.nth(0)).toBeChecked();
    await expect(boxes.nth(1)).toBeChecked();
    // A held agent starts unchecked (FR-002).
    await expect(boxes.nth(2)).not.toBeChecked();
    await expect(
      dialog.getByText(/^Continues the session started /),
    ).toHaveCount(2);
    await expect(
      dialog.getByText("Starts a new session: no session id"),
    ).toBeVisible();
    await expect(dialog.getByText("On hold", { exact: true })).toBeVisible();
    // Nothing started before Confirm.
    expect(await requestFields(page, "resume.start")).toEqual([]);
    await expectNoSeriousViolations(page);
  });

  test("unchecking one and confirming sends the others, with live results", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [ADA, BO, CY],
      resume: {
        "agent-bo": { outcome: "refuses" },
        "agent-cy": { outcome: "fails" },
      },
      resumeCheckMs: 400,
    });
    const dialog = await openDialog(page, 3);
    await dialog.getByRole("checkbox", { name: /Ada/ }).uncheck();

    await dialog.getByRole("button", { name: "Confirm" }).click();

    expect(await requestFields(page, "resume.start")).toEqual([
      { agent_ids: ["agent-bo", "agent-cy"] },
    ]);
    const results = dialog.getByRole("status", { name: "Resume results" });
    await expect(
      results.getByText("Checking: continuing a session"),
    ).toHaveCount(2);
    await expect(results).toContainText(`Bo: New session: ${REFUSED_REASON}`);
    await expect(results).toContainText(`Cy: On hold: ${FAILED_REASON}`);
    await expect(results).not.toContainText("Ada");
    await expectNoSeriousViolations(page);

    await dialog.getByRole("button", { name: "Close" }).click();
    await expect(dialog).toHaveCount(0);
    // Ada was left alone.
    await expect(
      page.getByRole("button", { name: "Ada, interrupted" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /^Bo, working/ }),
    ).toBeVisible();
  });

  test("the reason of a new session shows in the agent's action record", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [BO],
      resume: { "agent-bo": { outcome: "refuses" } },
    });
    const dialog = await openDialog(page, 1);
    await dialog.getByRole("button", { name: "Confirm" }).click();
    await expect(dialog.getByRole("status")).toContainText("New session");
    await page.keyboard.press("Escape");

    await page.getByRole("button", { name: /^Bo, working/ }).click();
    const record = page.getByRole("region", { name: "Action record" });
    await expect(record).toContainText(`Reason: ${REFUSED_REASON}`);
  });

  test("an unclean stop marks the agent, warns, and holds Confirm until acknowledged", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [ADA, BO],
      resume: { "agent-ada": { afterUncleanStop: true } },
    });
    const dialog = await openDialog(page, 2);
    const confirm = dialog.getByRole("button", { name: "Confirm" });
    const acknowledgement = dialog.getByRole("checkbox", {
      name: "I understand that a second copy may start",
    });

    await expect(dialog.getByText(UNCLEAN_MARK)).toHaveCount(1);
    await expect(dialog.getByText(UNCLEAN_WARNING)).toBeVisible();
    await expect(confirm).toHaveAccessibleDescription(
      new RegExp(UNCLEAN_WARNING),
    );
    await expect(confirm).toBeDisabled();
    await expectNoSeriousViolations(page);

    await acknowledgement.check();
    await expect(confirm).toBeEnabled();
    await acknowledgement.uncheck();
    await expect(confirm).toBeDisabled();

    // Without the marked agent checked, no acknowledgement is needed.
    await dialog.getByRole("checkbox", { name: /Ada/ }).uncheck();
    await expect(confirm).toBeEnabled();
  });

  test("no mark, no warning", async ({ page }) => {
    await openPanel(page, { agents: [ADA, BO] });
    const dialog = await openDialog(page, 2);

    await expect(dialog.getByText(/stopped uncleanly/)).toHaveCount(0);
    await expect(dialog.getByRole("checkbox")).toHaveCount(2);
    await expect(dialog.getByRole("button", { name: "Confirm" })).toBeEnabled();
  });

  test("Confirm is unavailable with nothing checked and says so", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA] });
    const dialog = await openDialog(page, 1);

    await dialog.getByRole("checkbox", { name: /Ada/ }).uncheck();

    await expect(dialog.getByText("Check at least one agent")).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Confirm" }),
    ).toBeDisabled();
  });

  test("the keyboard alone opens, checks, confirms, and closes", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA, BO] });
    // Bo resumes, so the count in the button's name falls while it is open.
    const opener = page.getByRole("button", { name: /^Resume interrupted/ });
    await opener.focus();
    await page.keyboard.press("Enter");

    const dialog = page.getByRole("dialog", { name: DIALOG_NAME });
    const boxes = dialog.getByRole("checkbox");
    await expect(boxes.first()).toBeFocused();
    await page.keyboard.press("Space");
    await expect(boxes.first()).not.toBeChecked();
    await page.keyboard.press("Tab");
    await expect(boxes.nth(1)).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(dialog.getByRole("button", { name: "Confirm" })).toBeFocused();
    await page.keyboard.press("Enter");

    const results = dialog.getByRole("status", { name: "Resume results" });
    await expect(results).toContainText("Bo: Continued");
    expect(await requestFields(page, "resume.start")).toEqual([
      { agent_ids: ["agent-bo"] },
    ]);

    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(opener).toBeFocused();
  });

  test("Escape closes the dialog while the list still loads or runs", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA, BO], resumeCheckMs: 300 });
    const opener = page.getByRole("button", { name: "Resume interrupted (2)" });
    await opener.click();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(opener).toBeFocused();
  });
});
