// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const SWEEP_TEXT = "Ending processes left from the last run";
const DIALOG_NAME = "Resume interrupted agents";
const UNCLEAN_WARNING =
  "The server stopped uncleanly. CLI processes of the marked agents from before may still run, and resuming can start a second copy next to them.";
const COMMAND_DISPLAY_LIMIT = 1000;

function agent(id: string, name: string): Agent {
  return {
    id,
    name,
    command: "claude",
    working_folder: `/work/${name.toLowerCase()}`,
    project_id: null,
    machine_id: "machine",
    status: "interrupted",
    created_at: 1_700_000_000_000,
    last_line: null,
  };
}

const ADA = agent("agent-ada", "Ada");
const BO = agent("agent-bo", "Bo");
const CY = agent("agent-cy", "Cy");
const DEE = agent("agent-dee", "Dee");

async function openPanel(page: Page, office: OfficeSeed): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Hold office" })).toBeVisible();
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

test.describe("orphan sweep (0017)", () => {
  test("the panel says so while the sweep runs, and the text is gone after done", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA], sweep: "running" });
    const status = page.getByRole("status").filter({ hasText: SWEEP_TEXT });

    await expect(status).toBeVisible();
    await expectNoSeriousViolations(page);

    await page.evaluate(() => window.__mockCore.setSweep("done"));

    await expect(page.getByText(SWEEP_TEXT)).toHaveCount(0);
    // The end is spoken, never shown: the region is screen-reader only.
    await expect(
      page.getByRole("status").filter({ hasText: "is finished" }),
    ).toHaveClass(/sr-only/);
  });

  test("a panel that opens after the sweep shows nothing", async ({ page }) => {
    await openPanel(page, { agents: [ADA] });
    await expect(page.getByText(SWEEP_TEXT)).toHaveCount(0);
    await expect(
      page.getByRole("status").filter({ hasText: "is finished" }),
    ).toHaveCount(0);
  });

  test("the dialog shows the text in place of the list, with no Confirm, then loads the plan on done", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA, BO], sweep: "running" });
    await page.getByRole("button", { name: "Resume interrupted (2)" }).click();
    const dialog = page.getByRole("dialog", { name: DIALOG_NAME });

    await expect(dialog.getByText(SWEEP_TEXT)).toBeVisible();
    await expect(dialog.getByRole("checkbox")).toHaveCount(0);
    await expect(dialog.getByRole("button", { name: "Confirm" })).toHaveCount(
      0,
    );
    await expect(dialog.getByRole("button", { name: "Close" })).toBeVisible();

    await page.evaluate(() => window.__mockCore.setSweep("done"));

    await expect(dialog.getByRole("checkbox")).toHaveCount(2);
    await expect(dialog.getByText(SWEEP_TEXT)).toHaveCount(0);
    await expect(dialog.getByRole("button", { name: "Confirm" })).toBeEnabled();
  });

  test("the three findings mark the entries, and only may_run gates Confirm", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [ADA, BO, CY, DEE],
      resume: {
        "agent-ada": { uncleanStop: { ended: 1 } },
        "agent-bo": { uncleanStop: { ended: 3 } },
        "agent-cy": { uncleanStop: null },
        "agent-dee": { uncleanStop: { may_run: "could not end: denied" } },
      },
    });
    await page.getByRole("button", { name: "Resume interrupted (4)" }).click();
    const dialog = page.getByRole("dialog", { name: DIALOG_NAME });
    const confirm = dialog.getByRole("button", { name: "Confirm" });
    const acknowledgement = dialog.getByRole("checkbox", {
      name: "I understand that a second copy may start",
    });
    // Four agents and the acknowledgement.
    await expect(dialog.getByRole("checkbox")).toHaveCount(5);

    await expect(
      dialog.getByText("Ended 1 process left from the last run"),
    ).toHaveCount(1);
    await expect(
      dialog.getByText("Ended 3 processes left from the last run"),
    ).toHaveCount(1);
    await expect(
      dialog.getByText(
        "Was running when the server stopped uncleanly: could not end: denied",
      ),
    ).toHaveCount(1);
    await expect(dialog.getByText(UNCLEAN_WARNING)).toHaveCount(1);
    await expect(confirm).toBeDisabled();
    await expectNoSeriousViolations(page);

    // Without the may_run agent, the ended and null findings do not gate.
    await dialog.getByRole("checkbox", { name: /Dee/ }).uncheck();
    await expect(confirm).toBeEnabled();
    await dialog.getByRole("checkbox", { name: /Dee/ }).check();
    await expect(confirm).toBeDisabled();
    await acknowledgement.check();
    await expect(confirm).toBeEnabled();
  });

  test("ended and none findings alone show no warning and do not gate Confirm", async ({
    page,
  }) => {
    await openPanel(page, {
      agents: [ADA, BO],
      resume: {
        "agent-ada": { uncleanStop: { ended: 2 } },
        "agent-bo": { uncleanStop: null },
      },
    });
    await page.getByRole("button", { name: "Resume interrupted (2)" }).click();
    const dialog = page.getByRole("dialog", { name: DIALOG_NAME });

    await expect(
      dialog.getByText("Ended 2 processes left from the last run"),
    ).toBeVisible();
    await expect(dialog.getByText(/stopped uncleanly/)).toHaveCount(0);
    await expect(
      dialog.getByRole("checkbox", { name: /second copy/ }),
    ).toHaveCount(0);
    await expect(dialog.getByRole("button", { name: "Confirm" })).toBeEnabled();
  });

  test("the action record shows the sweep entries as text, with the control character escaped and the long command cut", async ({
    page,
  }) => {
    const longCommand = `${"x".repeat(COMMAND_DISPLAY_LIMIT)}…abababab`;
    await openPanel(page, { agents: [ADA] });
    await page.evaluate(
      ({ longCommand: long }) => {
        const core = window.__mockCore;
        core.recordAction(
          "agent-ada",
          "end_leftover",
          "office",
          "identity 12: ended by force: pid 4242, session s-1, run\u001b[2J <b>bold</b>",
        );
        core.recordAction(
          "agent-ada",
          "end_leftover",
          "office",
          `identity 13: ended: pid 4243, session s-1, ${long}`,
        );
        core.recordAction(
          "agent-ada",
          "helper_limit",
          "office",
          "the limit of 200 helpers per session",
        );
      },
      { longCommand },
    );

    await page.getByRole("button", { name: /^Ada, interrupted/ }).click();
    const record = page.getByRole("region", { name: "Action record" });

    await expect(record).toContainText(
      "Checked a leftover process by the office",
    );
    await expect(record).toContainText(
      "Detail: identity 12: ended by force: pid 4242, session s-1, run\\u{1b}[2J <b>bold</b>",
    );
    await expect(record.locator("b")).toHaveCount(0);
    await expect(record).toContainText(`${"x".repeat(60)}`);
    await expect(record).toContainText("…abababab");
    await expect(record).toContainText("Stopped storing helpers by the office");
    await expect(record).toContainText(
      "Succeeded. Detail: the limit of 200 helpers per session",
    );
    await expectNoSeriousViolations(page);
  });

  test("each agent with a kept leftover shows in the list and in Needs you, with the reason", async ({
    page,
  }) => {
    await openPanel(page, { agents: [ADA, BO] });
    await page.evaluate(() => {
      const core = window.__mockCore;
      core.attention("agent-ada", "leftover_kept", "Operation not permitted");
      core.attention("agent-bo", "leftover_kept", "Access is denied");
    });

    const needsYou = page.getByRole("region", { name: /Needs you/ });
    await expect(needsYou).toContainText(
      "has a leftover process that could not be ended: Operation not permitted",
    );
    await expect(needsYou).toContainText(
      "has a leftover process that could not be ended: Access is denied",
    );
    await expectNoSeriousViolations(page);
  });
});
