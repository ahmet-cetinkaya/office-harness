// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type {
  RunRecord,
  Schedule,
} from "../src/modules/schedule/models/schedule";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const NARROW_VIEWPORT = { width: 360, height: 740 };
const FOCUS_OUTLINE_WIDTH = "2px";
const HOUR_MS = 3_600_000;
const SCHEDULE_CAP_REFUSAL =
  "the office already has 1 schedules, the schedule cap";

function agent(id: string, name: string): Agent {
  return {
    id,
    name,
    command: "claude",
    working_folder: "/work/repo",
    project_id: null,
    machine_id: "machine",
    status: "idle",
    created_at: 1_700_000_000_000,
    last_line: null,
  } as Agent;
}

function schedule(overrides: Partial<Schedule> = {}): Schedule {
  return {
    id: "schedule-1",
    name: "nightly",
    text: "run the tests",
    target_agent_id: "agent-ana",
    repeat: { kind: "daily", time: "09:00" },
    on: true,
    next_due: Date.now() + 5 * HOUR_MS,
    last_handled_due: null,
    last_missed: null,
    off_reason: null,
    ...overrides,
  };
}

function run(overrides: Partial<RunRecord> = {}): RunRecord {
  return {
    id: 1,
    schedule_id: "schedule-1",
    due: Date.now() - HOUR_MS,
    started: Date.now() - HOUR_MS,
    via: "due",
    result: "submitted",
    reason: null,
    ...overrides,
  };
}

const AGENTS = [agent("agent-ana", "Ana"), agent("agent-bo", "Bo")];

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: AGENTS, ...office },
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Idle sleep" })).toBeVisible();
}

async function openSchedules(page: Page) {
  await page.getByRole("button", { name: "Schedules", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Schedules" });
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

const seeded: OfficeSeed = {
  schedules: [schedule()],
  scheduleRuns: {
    "schedule-1": [
      run({ result: "skipped", reason: "the target was stopped" }),
    ],
  },
};

test.describe("list", () => {
  test("an empty office says no schedules and offers New schedule", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openSchedules(page);

    await expect(dialog.getByText("No schedules.")).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "New schedule" }),
    ).toBeVisible();
  });

  test("a row shows name, target, repeat, switch as text, next run, and last result", async ({
    page,
  }) => {
    await openOffice(page, seeded);
    const dialog = await openSchedules(page);
    const row = dialog.getByRole("article", {
      name: /^nightly, on, next run /,
    });

    await expect(row).toContainText("Ana");
    await expect(row).toContainText("every day at 09:00");
    await expect(row.getByText("on", { exact: true })).toBeVisible();
    await expect(row).toContainText("Skipped, due");
    await expect(row).toHaveAccessibleDescription(
      /"run the tests" to Ana, every day at 09:00\. Last run: Skipped/,
    );
  });

  test("a schedule that the office switched off says why, in text", async ({
    page,
  }) => {
    await openOffice(page, {
      schedules: [
        schedule({ on: false, next_due: null, off_reason: "time_missed" }),
      ],
    });
    const dialog = await openSchedules(page);
    const row = dialog.getByRole("article", {
      name: /^nightly, off, next run none/,
    });

    await expect(row).toContainText("Switched off by the office");
    await expect(row).toContainText("time was missed");
  });

  test("the history lists the runs with due time, start, how, result, and reason", async ({
    page,
  }) => {
    await openOffice(page, {
      schedules: [schedule()],
      scheduleRuns: {
        "schedule-1": [
          run({ id: 2, via: "now", result: "refused", reason: "on hold" }),
          run({ id: 1 }),
        ],
      },
    });
    const dialog = await openSchedules(page);

    await dialog.getByRole("button", { name: "History of nightly" }).click();
    const history = page.getByRole("dialog", { name: "History of nightly" });

    await expect(history.getByRole("listitem")).toHaveCount(2);
    const newest = history.getByRole("listitem").first();
    await expect(newest).toContainText("Run now");
    await expect(newest).toContainText("Refused");
    await expect(newest).toContainText("on hold");
    await expect(newest).toContainText("Due");
    await expect(newest).toContainText("Started");
    await page.keyboard.press("Escape");
    await expect(history).toHaveCount(0);
  });
});

test.describe("keyboard", () => {
  test("make, switch off, edit, switch on, Run now, cancel and confirm a delete", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openSchedules(page);

    await dialog.getByRole("button", { name: "New schedule" }).focus();
    await page.keyboard.press("Enter");
    const form = page.getByRole("dialog", { name: "New schedule" });
    await form.getByLabel("Name", { exact: true }).fill("tidy");
    await form.getByLabel("Text sent to the target").fill("tidy the folder");
    await form
      .getByLabel("Target", { exact: true })
      .selectOption({ label: "Bo" });
    await form.getByLabel("Time", { exact: true }).fill("07:45");
    await form.getByRole("button", { name: "Save schedule" }).focus();
    await page.keyboard.press("Enter");
    await expect(form).toHaveCount(0);

    const row = dialog.getByRole("article", { name: /^tidy, on/ });
    await expect(row).toContainText("every day at 07:45");
    const switchButton = row.getByRole("button", { name: "Switch off tidy" });
    await switchButton.focus();
    await expect(switchButton).toHaveCSS("outline-width", FOCUS_OUTLINE_WIDTH);
    await page.keyboard.press("Enter");
    await expect(
      dialog.getByRole("article", { name: /^tidy, off/ }),
    ).toBeVisible();

    await dialog.getByRole("button", { name: "Edit tidy" }).focus();
    await page.keyboard.press("Enter");
    const edit = page.getByRole("dialog", { name: "Edit tidy" });
    await expect(edit.getByLabel("Name", { exact: true })).toHaveValue("tidy");
    await edit.getByLabel("Name", { exact: true }).fill("tidy up");
    await edit.getByRole("button", { name: "Save schedule" }).focus();
    await page.keyboard.press("Enter");
    await expect(edit).toHaveCount(0);

    await dialog.getByRole("button", { name: "Switch on tidy up" }).focus();
    await page.keyboard.press("Enter");
    await expect(
      dialog.getByRole("article", { name: /^tidy up, on/ }),
    ).toBeVisible();

    await dialog.getByRole("button", { name: "Run now tidy up" }).focus();
    await page.keyboard.press("Enter");
    await expect
      .poll(async () => (await sentFields(page, "schedule.run_now")).length)
      .toBe(1);

    const remove = dialog.getByRole("button", { name: "Delete tidy up" });
    await remove.focus();
    await page.keyboard.press("Enter");
    const confirm = page.getByRole("dialog", { name: "Delete tidy up?" });
    await expect(confirm).toContainText("Its history goes");
    await expect(confirm).toContainText("action record");
    await expect(confirm).toContainText("already waiting");
    await expect(confirm.getByRole("button", { name: "Cancel" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(confirm).toHaveCount(0);
    expect(await sentFields(page, "schedule.delete")).toEqual([]);
    await expect(
      dialog.getByRole("article", { name: /^tidy up/ }),
    ).toBeVisible();

    await remove.focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Shift+Tab");
    await expect(
      confirm.getByRole("button", { name: "Delete schedule" }),
    ).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(dialog.getByText("No schedules.")).toBeVisible();
  });

  test("Escape closes the form and changes nothing", async ({ page }) => {
    await openOffice(page, seeded);
    const dialog = await openSchedules(page);

    await dialog.getByRole("button", { name: "Edit nightly" }).click();
    const form = page.getByRole("dialog", { name: "Edit nightly" });
    await form.getByLabel("Name", { exact: true }).fill("changed");
    await page.keyboard.press("Escape");

    await expect(form).toHaveCount(0);
    await expect(
      dialog.getByRole("article", { name: /^nightly/ }),
    ).toBeVisible();
    expect(await sentFields(page, "schedule.save")).toEqual([]);
  });
});

test.describe("save refusals", () => {
  test("each failing field shows its rule, from the server", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openSchedules(page);
    await dialog.getByRole("button", { name: "New schedule" }).click();
    const form = page.getByRole("dialog", { name: "New schedule" });

    await form.getByLabel("Repeat", { exact: true }).selectOption("once");
    await form.getByLabel("Name", { exact: true }).fill("past");
    await form.getByLabel("Text sent to the target").fill("go");
    await form.getByLabel("Date", { exact: true }).fill("2001-01-01");
    await form.getByLabel("Time", { exact: true }).fill("09:00");
    await form.getByRole("button", { name: "Save schedule" }).click();

    const alert = form.getByRole("alert");
    await expect(alert).toContainText(
      "Repeat a once repeat must name a future local time",
    );
    expect(await sentFields(page, "schedule.save")).toHaveLength(1);
  });

  test("a blank name and a bad time are caught before the server, with their rules", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openSchedules(page);
    await dialog.getByRole("button", { name: "New schedule" }).click();
    const form = page.getByRole("dialog", { name: "New schedule" });

    await form.getByLabel("Text sent to the target").fill("go");
    await form.getByLabel("Time", { exact: true }).fill("25:99");
    await form.getByRole("button", { name: "Save schedule" }).click();

    const alert = form.getByRole("alert");
    await expect(alert).toContainText("Name must not be blank");
    await expect(alert).toContainText(
      "a time must be hours and minutes, HH:MM",
    );
    await expect(form.getByLabel("Name", { exact: true })).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    expect(await sentFields(page, "schedule.save")).toEqual([]);
  });

  test("a weekly repeat has labelled day checkboxes and needs one", async ({
    page,
  }) => {
    await openOffice(page);
    const dialog = await openSchedules(page);
    await dialog.getByRole("button", { name: "New schedule" }).click();
    const form = page.getByRole("dialog", { name: "New schedule" });

    await form.getByLabel("Repeat", { exact: true }).selectOption("weekly");
    await form.getByLabel("Name", { exact: true }).fill("weekly");
    await form.getByLabel("Text sent to the target").fill("go");
    await form.getByRole("button", { name: "Save schedule" }).click();
    await expect(form.getByRole("alert")).toContainText(
      "a weekly repeat needs at least one day",
    );

    await form.getByRole("checkbox", { name: "Monday" }).check();
    await form.getByRole("checkbox", { name: "Friday" }).check();
    await form.getByRole("button", { name: "Save schedule" }).click();
    await expect(form).toHaveCount(0);
    await expect(
      dialog.getByRole("article", { name: /^weekly/ }),
    ).toContainText("every Monday, Friday at 09:00");
  });

  test("the schedule cap refusal shows its text", async ({ page }) => {
    await openOffice(page, {
      schedules: [schedule()],
      limits: { schedule_cap: "1" },
    });
    const dialog = await openSchedules(page);
    await dialog.getByRole("button", { name: "New schedule" }).click();
    const form = page.getByRole("dialog", { name: "New schedule" });

    await form.getByLabel("Name", { exact: true }).fill("one more");
    await form.getByLabel("Text sent to the target").fill("go");
    await form.getByRole("button", { name: "Save schedule" }).click();

    await expect(form.getByRole("alert")).toContainText(SCHEDULE_CAP_REFUSAL);
  });

  test("a switch on that the server refuses shows the refusal and keeps the row off", async ({
    page,
  }) => {
    await openOffice(page, {
      schedules: [
        schedule({ on: false, next_due: null, off_reason: "target_deleted" }),
      ],
    });
    const dialog = await openSchedules(page);

    await dialog.getByRole("button", { name: "Switch on nightly" }).click();

    await expect(dialog.getByRole("alert")).toContainText(
      "a switch on is refused",
    );
    await expect(
      dialog.getByRole("article", { name: /^nightly, off/ }),
    ).toBeVisible();
  });
});

test.describe("events", () => {
  test("a schedule.changed event updates the row and a schedule.removed event drops it", async ({
    page,
  }) => {
    await openOffice(page, seeded);
    const dialog = await openSchedules(page);

    await page.evaluate(
      (changed) => {
        window.__mockCore.pushEvent({
          type: "schedule.changed",
          schedule: changed,
        });
      },
      schedule({ on: false, next_due: null }),
    );
    await expect(
      dialog.getByRole("article", { name: /^nightly, off/ }),
    ).toHaveCount(1);
    await expect(dialog.getByRole("article")).toHaveCount(1);

    await page.evaluate(() => {
      window.__mockCore.pushEvent({
        type: "schedule.removed",
        id: "schedule-1",
      });
    });
    await expect(dialog.getByText("No schedules.")).toBeVisible();
  });
});

test.describe("notifications", () => {
  test("a schedule row without an agent is plain text and opens the schedules view", async ({
    page,
  }) => {
    await openOffice(page, {
      ...seeded,
      notifications: [
        {
          id: "n-missed",
          agent_id: null,
          agent_name: "2 schedules missed runs",
          event: "schedule",
          detail: null,
          created_at: Date.now(),
          read_at: null,
        },
      ],
    });
    await page
      .getByRole("button", { name: /Unread|Notifications/ })
      .first()
      .click();
    const list = page.getByRole("dialog", { name: "Notifications" });
    const item = list.getByRole("button", { name: "2 schedules missed runs" });
    await expect(item).toBeVisible();
    await item.click();

    await expect(page.getByRole("dialog", { name: "Schedules" })).toBeVisible();
  });

  test("a missed-runs row that arrives again with its id is one row", async ({
    page,
  }) => {
    await openOffice(page);
    const push = (text: string) =>
      page.evaluate((agentName) => {
        window.__mockCore.pushEvent({
          type: "notification.created",
          notification: {
            id: "n-missed",
            agent_id: null,
            agent_name: agentName,
            event: "schedule",
            detail: null,
            created_at: Date.now(),
            read_at: null,
          },
        });
      }, text);
    await push("1 schedule missed runs");
    await push("3 schedules missed runs");

    await page
      .getByRole("button", { name: /Unread|Notifications/ })
      .first()
      .click();
    const list = page.getByRole("dialog", { name: "Notifications" });
    await expect(list.getByRole("listitem")).toHaveCount(1);
    await expect(list).toContainText("3 schedules missed runs");
  });
});

test.describe("limits", () => {
  test("the schedule cap shows its default and refuses 0 and 201", async ({
    page,
  }) => {
    await openOffice(page);
    await page.getByRole("button", { name: "Limits" }).click();
    const dialog = page.getByRole("dialog", { name: "Limits" });
    const cap = dialog.getByLabel("Schedule cap");
    await expect(cap).toHaveValue("50");

    for (const text of ["0", "201"]) {
      await cap.fill(text);
      await dialog.getByRole("button", { name: "Save limits" }).click();
      await expect(dialog.getByRole("alert")).toContainText(
        "schedule_cap is a whole number from 1 to 200",
      );
      await expect(cap).toHaveAttribute("aria-invalid", "true");
    }
    await cap.fill("200");
    await dialog.getByRole("button", { name: "Save limits" }).click();
    await expect(dialog).toHaveCount(0);
    const saved = await page.evaluate(() => window.__mockCore.settings);
    expect(saved).toContainEqual({ key: "schedule_cap", value: "200" });
  });
});

test.describe("reflow and contrast", () => {
  test("there is no horizontal scroll at 360 px, in the list and in the form", async ({
    page,
  }) => {
    await page.setViewportSize(NARROW_VIEWPORT);
    await openOffice(page, seeded);
    const dialog = await openSchedules(page);
    const overflow = () =>
      page.evaluate(() => {
        const open = [...document.querySelectorAll("dialog[open]")].pop();
        return {
          page:
            document.documentElement.scrollWidth -
            document.documentElement.clientWidth,
          dialog: (open?.scrollWidth ?? 0) - (open?.clientWidth ?? 0),
        };
      });

    expect((await overflow()).page).toBeLessThanOrEqual(0);
    expect((await overflow()).dialog).toBeLessThanOrEqual(0);
    await dialog.getByRole("button", { name: "Edit nightly" }).click();
    await page.getByLabel("Repeat", { exact: true }).selectOption("weekly");
    expect((await overflow()).page).toBeLessThanOrEqual(0);
    expect((await overflow()).dialog).toBeLessThanOrEqual(0);
  });

  test("an axe scan finds no serious or critical violation in the list, the form, and the history", async ({
    page,
  }) => {
    await openOffice(page, seeded);
    const dialog = await openSchedules(page);
    await expectNoSeriousViolations(page);

    await dialog.getByRole("button", { name: "Edit nightly" }).click();
    await expectNoSeriousViolations(page);
    await page.keyboard.press("Escape");

    await dialog.getByRole("button", { name: "History of nightly" }).click();
    await expectNoSeriousViolations(page);
  });
});
