// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent, AgentHook } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const KEY = "hook_status";
const DIALOG_NAME = "Status from CLI hooks";
const EXACT: AgentHook = {
  source: "exact",
  reason: null,
  ask_open: false,
  compacting: false,
};

function agent(hook: AgentHook | null): Agent {
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
    hook,
  };
}

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
  });
  await page.goto("/");
}

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

async function openDetails(page: Page, rowName: string): Promise<void> {
  await page.getByRole("button", { name: rowName }).focus();
  await page.keyboard.press("Enter");
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

test("the setting is on by default, and turning it off by keyboard stores off", async ({
  page,
}) => {
  await openOffice(page);
  await pressOn(page, DIALOG_NAME);
  const dialog = page.getByRole("dialog", { name: DIALOG_NAME });
  const toggle = dialog.getByRole("switch", { name: DIALOG_NAME });
  await expect(toggle).toBeFocused();
  await expect(toggle).toBeChecked();
  await expect(dialog).toContainText(
    "On: claude sessions report their status.",
  );
  await expect(dialog).toContainText(
    "The change applies from the next session start of each agent.",
  );
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Space");
  await expect(dialog).toContainText(
    "Off: the status follows the quiet period.",
  );
  await pressOn(page, "Save");

  await expect(dialog).toHaveCount(0);
  const settings = await page.evaluate(() => window.__mockCore.settings);
  expect(settings.filter((setting) => setting.key === KEY)).toEqual([
    { key: KEY, value: "off" },
  ]);
});

test("the list says asks permission or compacting next to the status", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent(EXACT)] });
  const row = page.getByRole("button", { name: /^Ada, working/ });
  await expect(row).toHaveAccessibleName("Ada, working");

  const withHook = (hook: AgentHook) =>
    page.evaluate(
      (value) =>
        window.__mockCore.pushEvent({
          type: "hook.changed",
          agent_id: "agent-ada",
          hook: value,
        }),
      hook,
    );
  await withHook({ ...EXACT, compacting: true });
  await expect(row).toHaveAccessibleName("Ada, working, compacting");
  await expect(row).toContainText("working, compacting");

  await withHook({ ...EXACT, compacting: true, ask_open: true });
  await expect(row).toHaveAccessibleName("Ada, working, asks permission");
  await expect(row).toContainText("working, asks permission");
  await expectNoSeriousViolations(page);

  await page.evaluate(() =>
    window.__mockCore.pushEvent({
      type: "hook.changed",
      agent_id: "agent-ada",
      hook: null,
    }),
  );
  await expect(row).toHaveAccessibleName("Ada, working");
});

const SOURCES: { name: string; hook: AgentHook | null; text: string }[] = [
  { name: "exact", hook: EXACT, text: "exact" },
  {
    name: "estimated",
    hook: { ...EXACT, source: "estimated", reason: "hook status is off" },
    text: "estimated (hook status is off)",
  },
  {
    name: "late",
    hook: { ...EXACT, source: "late", reason: "no report for a stalled turn" },
    text: "late (no report for a stalled turn)",
  },
];

for (const { name, hook, text } of SOURCES) {
  test(`the details show the ${name} status source`, async ({ page }) => {
    await openOffice(page, { agents: [agent(hook)] });
    await openDetails(page, "Ada, working");
    const source = page.getByText("Status source", { exact: true });
    await expect(source).toBeVisible();
    await expect(source.locator("xpath=following-sibling::dd[1]")).toHaveText(
      text,
    );
  });
}

test("the details show no status source without a running session", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent(null)] });
  await openDetails(page, "Ada, working");
  await expect(
    page.getByRole("heading", { name: "Ada", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Status source", { exact: true })).toHaveCount(0);
});

test("Needs you says asks permission, finished its turn, and its turn ended with an error", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent(EXACT)] });
  const section = page.getByRole("region", { name: "Needs you" });
  const attention = (kind: string, detail?: string) =>
    page.evaluate(
      ([eventKind, eventDetail]) =>
        window.__mockCore.attention(
          "agent-ada",
          eventKind as "permission_asked",
          eventDetail,
        ),
      [kind, detail],
    );

  await attention("permission_asked");
  await expect(section).toContainText("asks permission");
  await attention("waiting_for_input", "turn_stopped");
  await expect(section).toContainText("finished its turn");
  await attention("waiting_for_input", "turn_failed");
  await expect(section).toContainText("its turn ended with an error");
  await attention("waiting_for_input", "5000");
  await expect(section).toContainText("quiet for 5 s");
  await expectNoSeriousViolations(page);
});
