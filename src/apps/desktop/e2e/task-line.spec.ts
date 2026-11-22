// SPDX-License-Identifier: AGPL-3.0-or-later

// 0011 task line: the row line, the details editor, and safe display. Every
// action runs by keyboard, and axe scans each state.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const TASK_TOO_LONG_MESSAGE = "a task text has at most 8,192 characters";
const TOO_LONG_LENGTH = 8193;
const TASK_CHANGED_NOTICE =
  "The task changed while you were editing. Saving replaces the new text with yours.";
const TASK_AGE_TICK_MS = 60_000;
const SLOW_SAVE_MS = 1_000;

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
    last_line: "Reading the tests",
    task: null,
    ...overrides,
  };
}

async function openPanel(
  page: Page,
  agents: Agent[] = [agent()],
  setTaskDelayMs?: number,
) {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents, setTaskDelayMs },
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: /^Ada/ })).toBeVisible();
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

async function pressByKeyboard(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

async function openDetails(page: Page): Promise<void> {
  await page.getByRole("button", { name: /^Ada/ }).first().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Task" })).toBeVisible();
}

function pushTask(page: Page, text: string, setAt = Date.now()) {
  return page.evaluate(
    ({ text, setAt }) =>
      window.__mockCore.pushEvent({
        type: "agent.task",
        agent_id: "agent-ada",
        task: { text, set_at: setAt, source: "directive" },
      }),
    { text, setAt },
  );
}

/** The row of Ada in the list, found by its visible text, not its name. */
function row(page: Page) {
  return page.getByRole("button", { name: /^Ada/ }).first();
}

test.describe("task line", () => {
  test("a pushed task shows in the row and in its accessible name", async ({
    page,
  }) => {
    await openPanel(page);
    await pushTask(page, "Fix the login form");

    await expect(row(page)).toContainText(
      "Task given 0 min ago: Fix the login form",
    );
    await expect(row(page)).toHaveAccessibleName(
      /Task given 0 min ago: Fix the login form/,
    );
    await expectNoSeriousViolations(page);
  });

  test("the task event keeps the last line and the status", async ({
    page,
  }) => {
    await openPanel(page);
    await pushTask(page, "Fix the login form");

    await expect(row(page)).toContainText("Reading the tests");
    await expect(row(page)).toContainText("working");
    await expect(row(page)).toContainText("Task given");
  });

  test("an escape code shows as visible symbols on one line", async ({
    page,
  }) => {
    await openPanel(page);
    await pushTask(page, "\u001b[31mred\nsecond line");

    await expect(row(page)).toContainText("Task given 0 min ago: ␛[31mred");
    await expect(row(page)).not.toContainText("second line");
    await expect(row(page)).not.toContainText("\u001b");
  });

  test("a long task stops at 80 characters with an ellipsis", async ({
    page,
  }) => {
    await openPanel(page);
    await pushTask(page, "x".repeat(81));

    await expect(row(page)).toContainText(`${"x".repeat(80)}…`);
  });

  test("a task is set, edited, and cleared by keyboard", async ({ page }) => {
    await openPanel(page);
    await openDetails(page);
    await expect(page.getByText("No task text.")).toBeVisible();

    await pressByKeyboard(page, "Set task");
    await expect(page.getByLabel("Task text")).toBeFocused();
    await page.keyboard.type("Write the report");
    await expectNoSeriousViolations(page);
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");

    await expect(page.getByRole("button", { name: "Edit task" })).toBeFocused();
    await expect(page.getByText("Write the report").first()).toBeVisible();
    await expect(row(page)).toContainText(
      "Task given 0 min ago: Write the report",
    );
    await expect(page.getByText("Changed the task text")).toHaveCount(1);

    await pressByKeyboard(page, "Edit task");
    await page.getByLabel("Task text").fill("Write the summary");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");
    await expect(row(page)).toContainText("Write the summary");

    await pressByKeyboard(page, "Clear task");
    await expect(page.getByText("No task text.")).toBeVisible();
    await expect(page.getByRole("button", { name: "Set task" })).toBeFocused();
    await expect(row(page)).not.toContainText("Task given");
    await expect(row(page)).toContainText("Reading the tests");
    await expectNoSeriousViolations(page);
  });

  test("Cancel leaves the task as it was", async ({ page }) => {
    await openPanel(page, [agent({ task: task("Keep me") })]);
    await openDetails(page);

    await pressByKeyboard(page, "Edit task");
    await page.getByLabel("Task text").fill("Dropped");
    await pressByKeyboard(page, "Cancel");

    await expect(page.getByRole("button", { name: "Edit task" })).toBeFocused();
    await expect(row(page)).toContainText("Keep me");
  });

  test("a task that arrives while the editor is open shows a notice and keeps the draft", async ({
    page,
  }) => {
    await openPanel(page, [agent({ task: task("Old task") })]);
    await openDetails(page);

    await pressByKeyboard(page, "Edit task");
    await page.getByLabel("Task text").fill("My draft");
    await expect(page.getByText(TASK_CHANGED_NOTICE)).toHaveCount(0);

    await pushTask(page, "Directive text");

    await expect(page.getByText(TASK_CHANGED_NOTICE)).toBeVisible();
    await expect(page.getByLabel("Task text")).toHaveValue("My draft");
    await expectNoSeriousViolations(page);
  });

  test("saving the user's own text shows no changed notice while the answer is slow", async ({
    page,
  }) => {
    await openPanel(page, [agent({ task: task("Old task") })], SLOW_SAVE_MS);
    await openDetails(page);

    await pressByKeyboard(page, "Edit task");
    await page.getByLabel("Task text").fill("My own text");
    await pressByKeyboard(page, "Save task");

    // The task event arrives within milliseconds; the save answers only after
    // SLOW_SAVE_MS, so the editor is open with the new text in the state.
    await page.waitForTimeout(SLOW_SAVE_MS / 2);
    await expect(page.getByLabel("Task text")).toBeVisible();
    // A one-shot count: toHaveCount(0) would retry until the save answers.
    expect(await page.getByText(TASK_CHANGED_NOTICE).count()).toBe(0);

    await expect(page.getByLabel("Task text")).toBeHidden();
    await expect(page.getByText(TASK_CHANGED_NOTICE)).toHaveCount(0);
  });

  test("8,193 characters show the refusal and keep the editor open", async ({
    page,
  }) => {
    await openPanel(page);
    await openDetails(page);

    await pressByKeyboard(page, "Set task");
    await page.getByLabel("Task text").fill("a".repeat(TOO_LONG_LENGTH));
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");

    await expect(page.getByText(TASK_TOO_LONG_MESSAGE)).toBeVisible();
    await expect(page.getByLabel("Task text")).toBeVisible();
    await expectNoSeriousViolations(page);
  });

  test("the details show the full text with control codes as symbols", async ({
    page,
  }) => {
    await openPanel(page, [agent({ task: task("a\u001b[31mb‮c\nline two") })]);
    await openDetails(page);

    const section = page.getByRole("region", { name: "Task" });
    await expect(section).toContainText("a␛[31mb\\u202Ec");
    await expect(section).toContainText("line two");
  });

  test("a new task clears the line", async ({ page }) => {
    await openPanel(page, [agent({ task: task("Old task") })]);
    await openDetails(page);
    await expect(row(page)).toContainText("Old task");

    await pressByKeyboard(page, "Start new task");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Start new task" })
      .press("Enter");

    await expect(row(page)).not.toContainText("Task given");
  });

  test("only a row with a task runs the one-minute age clock", async ({
    page,
  }) => {
    await page.addInitScript((tickMs) => {
      const live = new Set<number>();
      const setTimer = window.setInterval.bind(window);
      const clearTimer = window.clearInterval.bind(window);
      window.setInterval = ((handler: TimerHandler, delay?: number) => {
        const id = setTimer(handler, delay);
        if (delay === tickMs) live.add(id);
        return id;
      }) as typeof window.setInterval;
      window.clearInterval = ((id?: number) => {
        live.delete(id as number);
        clearTimer(id);
      }) as typeof window.clearInterval;
      Reflect.set(window, "liveAgeClocks", () => live.size);
    }, TASK_AGE_TICK_MS);
    const ageClocks = () =>
      page.evaluate(() => Reflect.get(window, "liveAgeClocks")());
    await openPanel(page, [
      agent(),
      agent({ id: "agent-bo", name: "Bo", task: task("Has a task") }),
    ]);
    await expect(page.getByRole("button", { name: /^Bo/ })).toBeVisible();
    expect(await ageClocks()).toBe(1);

    await pushTask(page, "Ada gets one");
    await expect(row(page)).toContainText("Task given");
    expect(await ageClocks()).toBe(2);
  });

  test("the scene desks show no task text", async ({ page }) => {
    await openPanel(page, [agent({ task: task("Secret plan") })]);
    await page.getByRole("button", { name: "To the scene" }).click();

    const scene = page.getByRole("region", { name: "Office scene" });
    await expect(scene.getByText("Ada").first()).toBeVisible();
    await expect(scene).not.toContainText("Secret plan");
    await expect(scene).not.toContainText("Task given");
  });
});

function task(text: string): Agent["task"] {
  return { text, set_at: Date.now(), source: "user" };
}
