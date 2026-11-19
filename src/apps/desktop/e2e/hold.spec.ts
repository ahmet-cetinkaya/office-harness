// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const INVALID_LIMIT_MESSAGE =
  "a limit is a whole number of at least 1 and at most 1000";

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

const AGENTS = [
  agent(),
  agent({ id: "agent-bo", name: "Bo", status: "stopped" }),
];

async function openPanel(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: AGENTS, ...office },
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

async function requestFields(page: Page, kind: string) {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter(
      (call) => call.command === "send_request" && call.args.kind === kind,
    )
    .map((call) => call.args.fields);
}

/** Presses Enter on a control that took focus by keyboard, not by click. */
async function pressByKeyboard(page: Page, name: string): Promise<void> {
  const button = page.getByRole("button", { name, exact: true });
  await button.focus();
  await page.keyboard.press("Enter");
}

const OFFICE_ON_HOLD = "Office on hold";

test.describe("office hold", () => {
  test("the office is held and released by keyboard, and says so as text", async ({
    page,
  }) => {
    await openPanel(page);
    await expect(page.getByText(OFFICE_ON_HOLD, { exact: true })).toHaveCount(
      0,
    );

    await pressByKeyboard(page, "Hold office");

    const bar = page.getByRole("region", { name: "Office hold" });
    await expect(bar.getByText(OFFICE_ON_HOLD, { exact: true })).toBeVisible();
    await expect(
      bar.getByRole("button", { name: "Release office" }),
    ).toBeVisible();
    // The office hold names no agent (contract: agent_id absent).
    expect(await requestFields(page, "hold.set")).toEqual([undefined]);
    await expectNoSeriousViolations(page);

    await pressByKeyboard(page, "Release office");

    await expect(bar.getByText(OFFICE_ON_HOLD, { exact: true })).toHaveCount(0);
    await expect(
      bar.getByRole("button", { name: "Hold office" }),
    ).toBeVisible();
    expect(await requestFields(page, "hold.release")).toEqual([undefined]);
  });

  test("an office that was on hold before the app opened shows the hold", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: {
        agents: AGENTS,
        holds: { office: { set_by: "user", since: 1_700_000_000_000 } },
      },
    });
    await page.goto("/");

    await expect(page.getByText(OFFICE_ON_HOLD, { exact: true })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Release office" }),
    ).toBeVisible();
  });

  test("the hold change is announced in the live region", async ({ page }) => {
    await openPanel(page);
    const region = page.locator("p.sr-only[aria-live='polite']");

    await pressByKeyboard(page, "Hold office");
    await expect(region).toHaveText(OFFICE_ON_HOLD);

    await pressByKeyboard(page, "Release office");
    await expect(region).toHaveText("Office released from hold");
  });

  test("one key press, Ctrl+Shift+H, holds the office from the panel (SC-005)", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("heading", { name: "office-harness" }).click();

    const bar = page.getByRole("region", { name: "Office hold" });
    await page.keyboard.press("Control+Shift+H");

    await expect(bar.getByText(OFFICE_ON_HOLD, { exact: true })).toBeVisible();
    expect(await requestFields(page, "hold.set")).toHaveLength(1);

    await page.keyboard.press("Control+Shift+H");

    await expect(bar.getByText(OFFICE_ON_HOLD, { exact: true })).toHaveCount(0);
  });

  test("Ctrl+Shift+H holds the office while focus is in a terminal (D9 = b)", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Ada, working" }).click();
    const input = page.getByLabel("Terminal of Ada", { exact: true });
    await expect(input).toBeFocused();

    await page.keyboard.press("Control+Shift+H");

    const bar = page.getByRole("region", { name: "Office hold" });
    await expect(bar.getByText(OFFICE_ON_HOLD, { exact: true })).toBeVisible();
    expect(await requestFields(page, "hold.set")).toHaveLength(1);
    // A later key reaching the terminal shows any earlier byte has arrived:
    // the hold key went to the app only, so no Ctrl+H byte reached the CLI.
    await expect(input).toBeFocused();
    await page.keyboard.type("x");
    const typed = async () =>
      (await requestFields(page, "terminal.input"))
        .map((fields) =>
          Buffer.from(
            String((fields as { data?: string }).data),
            "base64",
          ).toString(),
        )
        .join("");
    await expect.poll(typed).toContain("x");
    expect(await typed()).toBe("x");
  });

  test("the server's already-held message reaches the user", async ({
    page,
  }) => {
    await openPanel(page, {
      refusals: {
        "hold.set": {
          code: "conflict",
          message: "the office is already on hold",
        },
      },
    });

    await pressByKeyboard(page, "Hold office");

    const alert = page.getByRole("alert");
    await expect(alert).toContainText("the office is already on hold");
    await expectNoSeriousViolations(page);
  });

  test("the office record opens beside the hold control with the office entries (A11)", async ({
    page,
  }) => {
    await openPanel(page);
    await pressByKeyboard(page, "Hold office");
    await pressByKeyboard(page, "Release office");

    const bar = page.getByRole("region", { name: "Office hold" });
    await expect(
      bar.getByRole("button", { name: "Hold office" }),
    ).toBeVisible();
    await bar.getByRole("button", { name: "Office record" }).click();

    const dialog = page.getByRole("dialog", { name: "Office record" });
    const entries = dialog.getByRole("listitem");
    await expect(entries).toHaveCount(2);
    // Newest first.
    await expect(entries.first()).toContainText("Released from hold by user");
    await expect(entries.last()).toContainText("Put on hold by user");
    await expectNoSeriousViolations(page);

    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(
      bar.getByRole("button", { name: "Office record" }),
    ).toBeFocused();
  });
});

test.describe("agent hold", () => {
  test("an agent is held and released by keyboard; row and details say on hold", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Ada, working" }).click();
    const details = page.getByRole("region", { name: "Ada" });

    await pressByKeyboard(page, "Hold agent");

    // The row's accessible name carries the hold (FR-011).
    await expect(
      page.getByRole("button", { name: "Ada, working, on hold" }),
    ).toBeVisible();
    await expect(details.getByText("on hold", { exact: true })).toBeVisible();
    expect(await requestFields(page, "hold.set")).toEqual([
      { agent_id: "agent-ada" },
    ]);
    // Holding an agent leaves the office alone.
    await expect(page.getByText(OFFICE_ON_HOLD, { exact: true })).toHaveCount(
      0,
    );
    await expectNoSeriousViolations(page);

    await pressByKeyboard(page, "Release agent");

    await expect(
      page.getByRole("button", { name: "Ada, working", exact: true }),
    ).toBeVisible();
    await expect(details.getByText("on hold", { exact: true })).toHaveCount(0);
    expect(await requestFields(page, "hold.release")).toEqual([
      { agent_id: "agent-ada" },
    ]);
  });

  test("an agent's already-held message reaches the user", async ({ page }) => {
    await openPanel(page, {
      refusals: {
        "hold.set": {
          code: "conflict",
          message: "the agent is already on hold",
        },
      },
    });
    await page.getByRole("button", { name: "Ada, working" }).click();

    await pressByKeyboard(page, "Hold agent");

    await expect(page.getByRole("alert")).toContainText(
      "the agent is already on hold",
    );
  });

  test("an automatic hold shows its reason, and the notification names the agent and the limit (FR-008)", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Ada, working" }).click();

    await page.evaluate(() =>
      window.__mockCore.autoHold("agent-ada", "directives sent by one agent"),
    );

    const details = page.getByRole("region", { name: "Ada" });
    await expect(
      page.getByRole("button", { name: "Ada, working, on hold" }),
    ).toBeVisible();
    await expect(details).toContainText(
      'Held automatically: the limit "directives sent by one agent"',
    );
    const expected =
      'Ada was put on hold after 3 refusals by the limit "directives sent by one agent"';
    await expect(page.locator("p.sr-only[aria-live='polite']")).toContainText(
      expected,
    );

    // The agent's terminal is open, yet the notification stays unread.
    await page.getByRole("button", { name: "Notifications, 1 unread" }).click();
    const dialog = page.getByRole("dialog", { name: "Notifications" });
    await expect(
      dialog.getByRole("button", { name: `Ada, ${expected.slice(4)}` }),
    ).toBeVisible();
    await expectNoSeriousViolations(page);
  });
});

test.describe("live region", () => {
  test("selecting an agent does not read an old notification over a newer hold", async ({
    page,
  }) => {
    await openPanel(page);
    const region = page.locator("p.sr-only[aria-live='polite']");

    await page.evaluate(() =>
      window.__mockCore.autoHold("agent-ada", "directives sent by one agent"),
    );
    await expect(region).toContainText("Notification: Ada was put on hold");

    await pressByKeyboard(page, "Hold office");
    await expect(region).toHaveText(OFFICE_ON_HOLD);

    // Selecting an agent marks its notifications read, which publishes the
    // notification snapshot again with its old announcement.
    await page.getByRole("button", { name: /^Ada, working/ }).click();
    await expect(page.getByRole("region", { name: "Ada" })).toBeVisible();
    await expect(region).toHaveText(OFFICE_ON_HOLD);
  });
});

test.describe("limits", () => {
  async function openLimits(page: Page) {
    await page.getByRole("button", { name: "Limits" }).click();
    const dialog = page.getByRole("dialog", { name: "Limits" });
    await expect(
      dialog.getByLabel("Directives sent by one agent"),
    ).toBeVisible();
    return dialog;
  }

  test("the dialog shows the four limits with their defaults and saves new values", async ({
    page,
  }) => {
    await openPanel(page);
    const dialog = await openLimits(page);

    await expect(dialog.getByLabel("Directives sent by one agent")).toHaveValue(
      "10",
    );
    await expect(
      dialog.getByLabel("Directives sent to one target agent"),
    ).toHaveValue("3");
    await expect(dialog.getByLabel("Agents created by one agent")).toHaveValue(
      "5",
    );
    await expect(
      dialog.getByLabel("Starts, stops, and new tasks by one agent"),
    ).toHaveValue("10");
    // Focus lands on the first field.
    await expect(
      dialog.getByLabel("Directives sent by one agent"),
    ).toBeFocused();
    await expectNoSeriousViolations(page);

    await dialog.getByLabel("Directives sent to one target agent").fill("7");
    await dialog.getByRole("button", { name: "Save limits" }).click();

    await expect(dialog).toHaveCount(0);
    const saved = await page.evaluate(() => window.__mockCore.settings);
    expect(saved).toContainEqual({
      key: "limit_directives_per_target",
      value: "7",
    });
    // The new value shows when the dialog opens again.
    const again = await openLimits(page);
    await expect(
      again.getByLabel("Directives sent to one target agent"),
    ).toHaveValue("7");
  });

  test("a value that is not a limit is refused with the message, and nothing is saved", async ({
    page,
  }) => {
    await openPanel(page);
    const dialog = await openLimits(page);

    for (const text of ["0", "-3", "abc", "1001"]) {
      const field = dialog.getByLabel("Agents created by one agent");
      await field.fill(text);
      await dialog.getByRole("button", { name: "Save limits" }).click();
      await expect(dialog.getByRole("alert")).toContainText(
        INVALID_LIMIT_MESSAGE,
      );
      await expect(field).toHaveAttribute("aria-invalid", "true");
      await expect(field).toHaveAttribute("aria-describedby", /limits-error/);
      await expect(
        dialog.getByLabel("Directives sent by one agent"),
      ).not.toHaveAttribute("aria-invalid");
    }
    await expect(dialog).toBeVisible();
    // The shell's own `last_view` writes are not limits.
    const saved = await page.evaluate(() =>
      window.__mockCore.settings.filter((setting) =>
        setting.key.startsWith("limit_"),
      ),
    );
    expect(saved).toEqual([]);
    await expectNoSeriousViolations(page);
  });

  test("the server's refusal of a limit is shown as sent", async ({ page }) => {
    await openPanel(page, {
      refusals: {
        "settings.set": { code: "invalid", message: INVALID_LIMIT_MESSAGE },
      },
    });
    const dialog = await openLimits(page);

    await dialog.getByRole("button", { name: "Save limits" }).click();

    await expect(dialog.getByRole("alert")).toContainText(
      INVALID_LIMIT_MESSAGE,
    );
    await expect(dialog).toBeVisible();
  });

  test("Escape closes the dialog and focus returns to the Limits control", async ({
    page,
  }) => {
    await openPanel(page);
    const dialog = await openLimits(page);

    await page.keyboard.press("Escape");

    await expect(dialog).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Limits" })).toBeFocused();
  });
});
