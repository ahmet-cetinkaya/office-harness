// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const AGENT_ID = "11111111-2222-4333-8444-555555555555";
const ASLEEP_TEXT = "Asleep. Press Enter or Wake to continue the last session.";
const INVALID_MESSAGE =
  "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440";
const NO_LONGER_ASLEEP = "This agent is no longer asleep";
const LAST_OUTPUT = "last output before sleep";
const RENDERED_ROWS = ".xterm-rows";
const REFUSED_REASON = "the CLI cannot continue this session";
const FAILED_REASON = "the new session ended within its check period";

function ada(overrides: Partial<Agent> = {}): Agent {
  return {
    id: AGENT_ID,
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "asleep",
    created_at: 1_700_000_000_000,
    last_line: null,
    ...overrides,
  };
}

async function openPanel(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: [ada()],
      snapshots: { [AGENT_ID]: LAST_OUTPUT },
      ...office,
    },
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Idle sleep" })).toBeVisible();
}

async function openAsleepTerminal(
  page: Page,
  office: OfficeSeed = {},
): Promise<void> {
  await openPanel(page, office);
  await page.getByRole("button", { name: "Ada, asleep" }).click();
  // Below the fold, xterm pauses its rendering.
  await page.locator(".xterm").scrollIntoViewIfNeeded();
  await expect(page.locator(RENDERED_ROWS)).toContainText(LAST_OUTPUT);
}

async function openDialog(page: Page) {
  await page.getByRole("button", { name: "Idle sleep" }).click();
  const dialog = page.getByRole("dialog", { name: "Idle sleep" });
  await expect(dialog.getByRole("switch")).toBeVisible();
  return dialog;
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

async function requests(page: Page, kind: string) {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter(
      (call) => call.command === "send_request" && call.args.kind === kind,
    )
    .map((call) => call.args.fields);
}

async function idleSleepWrites(page: Page): Promise<string[]> {
  const saved = await page.evaluate(() => window.__mockCore.settings);
  return saved
    .filter((setting) => setting.key === "idle_sleep_minutes")
    .map((setting) => String(setting.value));
}

function wakeResult(page: Page) {
  return page.getByRole("status", { name: "Wake result" });
}

test.describe("idle sleep setting", () => {
  test("it is off by default and turning it on stores 30", async ({ page }) => {
    await openPanel(page);
    const dialog = await openDialog(page);

    await expect(dialog.getByRole("switch")).not.toBeChecked();
    await expect(dialog.getByLabel("Idle time in minutes")).toHaveCount(0);
    await expect(dialog).toContainText(
      "a background task or a development server",
    );

    await dialog.getByRole("switch").check();
    const minutes = dialog.getByLabel("Idle time in minutes");
    await expect(minutes).toHaveValue("30");
    await dialog.getByRole("button", { name: "Save" }).click();

    await expect(dialog).toHaveCount(0);
    expect(await idleSleepWrites(page)).toEqual(["30"]);
    const again = await openDialog(page);
    await expect(again.getByRole("switch")).toBeChecked();
    await expect(again.getByLabel("Idle time in minutes")).toHaveValue("30");
  });

  test("4 and 1441 are refused and the previous value stays", async ({
    page,
  }) => {
    await openPanel(page);
    let dialog = await openDialog(page);
    await dialog.getByRole("switch").check();
    await dialog.getByLabel("Idle time in minutes").fill("45");
    await dialog.getByRole("button", { name: "Save" }).click();
    await expect(dialog).toHaveCount(0);

    dialog = await openDialog(page);
    const minutes = dialog.getByLabel("Idle time in minutes");
    for (const text of ["4", "1441", "abc"]) {
      await minutes.fill(text);
      await dialog.getByRole("button", { name: "Save" }).click();
      await expect(dialog.getByRole("alert")).toContainText(INVALID_MESSAGE);
      await expect(minutes).toHaveAttribute("aria-invalid", "true");
    }
    await expectNoSeriousViolations(page);
    await page.keyboard.press("Escape");

    expect(await idleSleepWrites(page)).toEqual(["45"]);
    const again = await openDialog(page);
    await expect(again.getByLabel("Idle time in minutes")).toHaveValue("45");
  });

  test("the server's refusal is shown as sent", async ({ page }) => {
    await openPanel(page, {
      refusals: {
        "settings.set": { code: "internal", message: "the disk is full" },
      },
    });
    const dialog = await openDialog(page);
    await dialog.getByRole("switch").check();
    await dialog.getByRole("button", { name: "Save" }).click();

    await expect(dialog.getByRole("alert")).toContainText("the disk is full");
    await expect(dialog).toBeVisible();
  });

  test("keyboard only: focus is visible, Space turns it on, Escape closes", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Idle sleep" }).focus();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "Idle sleep" });
    const toggle = dialog.getByRole("switch");
    await expect(toggle).toBeFocused();
    await expect(toggle).toHaveCSS("outline-style", "solid");

    await page.keyboard.press("Space");
    await expect(toggle).toBeChecked();
    await page.keyboard.press("Tab");
    await expect(dialog.getByLabel("Idle time in minutes")).toBeFocused();
    await page.keyboard.press("Escape");

    await expect(dialog).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Idle sleep" }),
    ).toBeFocused();
    expect(await idleSleepWrites(page)).toEqual([]);
  });
});

test.describe("an asleep agent", () => {
  test("shows asleep in the list and the scene", async ({ page }) => {
    await openPanel(page);
    await expect(
      page.getByRole("button", { name: "Ada, asleep" }),
    ).toBeVisible();

    await page.getByRole("button", { name: "To the scene" }).click();
    await expect(
      page.getByRole("region", { name: "Office scene" }).getByRole("button", {
        name: "Ada, asleep",
      }),
    ).toBeVisible();
  });

  test("its terminal view shows the last output, the asleep text, and Wake", async ({
    page,
  }) => {
    await openAsleepTerminal(page);

    const section = page.getByRole("region", { name: "Terminal" });
    await expect(section.getByText(ASLEEP_TEXT)).toBeVisible();
    await expect(section.getByRole("button", { name: "Wake" })).toBeVisible();
    await expectNoSeriousViolations(page);
  });

  test("Enter in the focused terminal view wakes the agent", async ({
    page,
  }) => {
    await openAsleepTerminal(page, { wakeCheckMs: 300 });
    await page.getByLabel("Terminal of Ada", { exact: true }).focus();

    await page.keyboard.press("Enter");

    await expect(wakeResult(page)).toHaveText(/Waking|Checking|Continued/);
    await expect(wakeResult(page)).toHaveText("Continued");
    await expect(
      page.getByRole("button", { name: "Ada, working" }),
    ).toBeVisible();
    expect(await requests(page, "wake.start")).toEqual([
      { agent_id: AGENT_ID },
    ]);
    expect(await requests(page, "terminal.input")).toEqual([]);
  });

  test("other keys and a paste reach no CLI and do not wake", async ({
    page,
  }) => {
    await openAsleepTerminal(page);
    const input = page.getByLabel("Terminal of Ada", { exact: true });
    await input.focus();

    await page.keyboard.press("x");
    await page.keyboard.press("Space");
    await page.keyboard.press("Control+c");
    await input.evaluate((element) => {
      const transfer = new DataTransfer();
      transfer.setData("text/plain", "pasted\n");
      element.dispatchEvent(
        new ClipboardEvent("paste", {
          clipboardData: transfer,
          bubbles: true,
          cancelable: true,
        }),
      );
    });

    expect(await requests(page, "terminal.input")).toEqual([]);
    expect(await requests(page, "wake.start")).toEqual([]);
    await expect(
      page.getByRole("button", { name: "Ada, asleep" }),
    ).toBeVisible();
  });

  test("Wake in the details wakes the agent", async ({ page }) => {
    await openAsleepTerminal(page);

    await page
      .getByRole("heading", { name: "Ada", level: 2 })
      .locator("..")
      .getByRole("button", { name: "Wake" })
      .click();

    await expect(wakeResult(page)).toHaveText("Continued");
    expect(await requests(page, "wake.start")).toEqual([
      { agent_id: AGENT_ID },
    ]);
    await expect(page.getByText(/Woke by user/)).toBeVisible();
  });

  test("Wake in the scene desk terminal wakes the agent", async ({ page }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "To the scene" }).click();
    await page.getByRole("button", { name: "Ada, asleep" }).click();
    const desk = page.getByRole("region", { name: "Desk terminal" });
    await expect(desk.getByText(ASLEEP_TEXT)).toBeVisible();

    await desk.getByRole("button", { name: "Wake" }).click();

    await expect(wakeResult(page)).toHaveText("Continued");
  });
});

test.describe("wake results", () => {
  test("a wake shows Waking, then Checking, then Continued", async ({
    page,
  }) => {
    await openAsleepTerminal(page, { wakeCheckMs: 600 });
    const panelWake = page
      .getByRole("region", { name: "Terminal" })
      .getByRole("button", { name: "Wake" });

    await panelWake.click();

    await expect(wakeResult(page)).toHaveText("Checking");
    await expect(wakeResult(page)).toHaveText("Continued");
  });

  test("a CLI that cannot continue starts a new session with the reason", async ({
    page,
  }) => {
    await openAsleepTerminal(page, {
      wake: { [AGENT_ID]: { canContinue: false } },
    });
    await page
      .getByRole("region", { name: "Terminal" })
      .getByRole("button", { name: "Wake" })
      .click();

    await expect(wakeResult(page)).toHaveText(`New session: ${REFUSED_REASON}`);
    await expect(
      page.getByText(`Reason: new session: ${REFUSED_REASON}`),
    ).toBeVisible();
  });

  test("a failed wake puts the agent on hold with the reason", async ({
    page,
  }) => {
    await openAsleepTerminal(page, {
      wake: { [AGENT_ID]: { outcome: "fails" } },
    });
    await page
      .getByRole("region", { name: "Terminal" })
      .getByRole("button", { name: "Wake" })
      .click();

    await expect(wakeResult(page)).toHaveText(`On hold: ${FAILED_REASON}`);
  });

  test("a wake of an agent that is no longer asleep says so", async ({
    page,
  }) => {
    await openAsleepTerminal(page, {
      refusals: {
        "wake.start": {
          code: "conflict",
          message: "the agent is no longer asleep",
        },
      },
    });
    await page
      .getByRole("region", { name: "Terminal" })
      .getByRole("button", { name: "Wake" })
      .click();

    await expect(wakeResult(page)).toHaveText(NO_LONGER_ASLEEP);
  });

  test("a new sleep clears the last result", async ({ page }) => {
    await openAsleepTerminal(page);
    await page
      .getByRole("region", { name: "Terminal" })
      .getByRole("button", { name: "Wake" })
      .click();
    await expect(wakeResult(page)).toHaveText("Continued");

    await page.evaluate(
      (agentId) =>
        window.__mockCore.pushEvent({
          type: "agent.status",
          agent_id: agentId,
          status: "asleep",
        }),
      AGENT_ID,
    );

    await expect(wakeResult(page)).toHaveText("");
  });
});
