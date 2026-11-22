// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Locator, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type {
  NeedsYouItem,
  NeedsYouKind,
} from "../src/modules/needs-you/models/needs-you";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const JUMP_KEY = "Control+Shift+F11";
const JUMP_KEY_LABEL = "Ctrl+Shift+F11";
const EMPTY_TEXT = "No agent needs you";
const FAILED_DETAIL = "the command ended with exit code 2";
const SECOND_MS = 1_000;
const FIRST_JUMP_BUDGET_MS = 1_000;

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

const PROJECTS = [
  { id: "project-site", name: "Site", folder: "/work/site" },
  { id: "project-api", name: "Api", folder: "/work/api" },
];
const EMPTY_PROJECT = {
  id: "project-empty",
  name: "Empty",
  folder: "/work/empty",
};

/** Six agents; Zed never has an item, so its terminal can be open first. */
const AGENTS = [
  agent({ project_id: "project-site" }),
  agent({ id: "agent-bo", name: "Bo", project_id: "project-api" }),
  agent({ id: "agent-cy", name: "Cy", status: "waiting_for_input" }),
  agent({ id: "agent-di", name: "Di" }),
  agent({ id: "agent-ed", name: "Ed", status: "exited" }),
  agent({ id: "agent-zed", name: "Zed" }),
];

function item(
  agentId: string,
  kind: NeedsYouKind,
  waitedMs: number,
  detail: string | null = null,
): NeedsYouItem {
  const firstEventAt = Date.now() - waitedMs;
  return {
    agent_id: agentId,
    kind,
    detail,
    first_event_at: firstEventAt,
    latest_at: firstEventAt,
    origin_notification_id: null,
    report: null,
  };
}

/** Five items of mixed kinds; the server's order is Cy, Ada, Bo, Di, Ed. */
function fiveItems(): NeedsYouItem[] {
  return [
    item("agent-ed", "exited", 10 * SECOND_MS),
    item("agent-di", "auto_hold", 20 * SECOND_MS, "directives_per_agent"),
    item("agent-bo", "bell", 30 * SECOND_MS),
    item("agent-ada", "waiting_for_input", 40 * SECOND_MS, "5000"),
    item("agent-cy", "failed", 50 * SECOND_MS, FAILED_DETAIL),
  ];
}
const FIVE_NAMES = ["Cy", "Ada", "Bo", "Di", "Ed"];
/** Ed has exited: its read-only terminal cannot take focus (0001), so the
 * jump puts focus on the terminal view's heading, which names the agent
 * (0008 FR-007, Efe's decision of 2026-11-22). */
const NOT_RUNNING_NAMES = new Set(["Ed"]);

/** The opened terminal view's focusable heading (terminal-view.svelte). */
function terminalHeading(page: Page): Locator {
  return page.locator("#terminal-heading");
}

async function openPanel(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: AGENTS, projects: PROJECTS, ...office },
  });
  await page.goto("/");
  await expect(needsYouSection(page)).toBeVisible();
}

function needsYouSection(page: Page) {
  return page.getByRole("region", { name: /^Needs you/ });
}

function liveRegion(page: Page) {
  return page.locator("p.sr-only[aria-live='polite']");
}

function terminalInput(page: Page, name: string) {
  return page.getByLabel(`Terminal of ${name}`, { exact: true });
}

async function openTerminalOf(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: new RegExp(`^${name},`) }).click();
  await expect(terminalInput(page, name)).toBeFocused();
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

/** The count of every `set_window_title` invoke, oldest first. */
function titleCounts(page: Page): Promise<number[]> {
  return page.evaluate(() =>
    window.__mockCore.calls
      .filter((call) => call.command === "set_window_title")
      .map((call) => Number(call.args.count)),
  );
}

function requestsOf(page: Page, kind: string): Promise<unknown[]> {
  return page.evaluate(
    (requested) =>
      window.__mockCore.calls
        .filter(
          (call) =>
            call.command === "send_request" && call.args.kind === requested,
        )
        .map((call) => call.args.fields),
    kind,
  );
}

async function readAgentIds(page: Page): Promise<string[]> {
  const fields = (await requestsOf(page, "notification.read_agent")) as {
    agent_id: string;
  }[];
  return fields.map((field) => field.agent_id);
}

async function sentTerminalInputs(page: Page): Promise<string[]> {
  return (await requestsOf(page, "terminal.input")).map((fields) =>
    Buffer.from(
      String((fields as { data?: string }).data),
      "base64",
    ).toString(),
  );
}

test.describe("the Needs you section", () => {
  test("shows name, project, event, and wait time as text, in the server's order", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: fiveItems() });
    const section = needsYouSection(page);

    const rows = section.getByRole("group");
    await expect(rows).toHaveCount(5);
    const names = await rows.evaluateAll((elements) =>
      elements.map((element) => element.getAttribute("aria-label")),
    );
    expect(names[0]).toMatch(
      new RegExp(`^Cy, failed: ${FAILED_DETAIL}, waiting \\d+ s$`),
    );
    expect(names[1]).toMatch(
      /^Ada, project Site, quiet for 5 s, waiting \d+ s$/,
    );
    expect(names[2]).toMatch(/^Bo, project Api, bell, waiting \d+ s$/);
    expect(names[3]).toMatch(
      /^Di, put on hold by the office, limit: directives_per_agent, waiting \d+ s$/,
    );
    expect(names[4]).toMatch(/^Ed, exited, waiting \d+ s$/);
    await expect(
      section.getByRole("heading", { name: "Needs you (5)" }),
    ).toBeVisible();
    await expectNoSeriousViolations(page);
  });

  test("says that no agent needs the user when the list is empty", async ({
    page,
  }) => {
    await openPanel(page);

    await expect(
      needsYouSection(page).getByText(EMPTY_TEXT, { exact: true }),
    ).toBeVisible();
    await expectNoSeriousViolations(page);
    expect(await titleCounts(page)).toEqual([]);
  });

  test("scrolls when more items than fit exist, and the title shows the full count", async ({
    page,
  }) => {
    const manyAgents = Array.from({ length: 12 }, (_, index) =>
      agent({ id: `agent-${index}`, name: `Agent ${index}` }),
    );
    const manyItems = manyAgents.map((known, index) =>
      item(known.id, "bell", (index + 1) * SECOND_MS),
    );
    await openPanel(page, { agents: manyAgents, needsYou: manyItems });

    const list = needsYouSection(page).getByRole("list");
    const isScrollable = await list.evaluate(
      (element) => element.scrollHeight > element.clientHeight,
    );
    expect(isScrollable).toBe(true);
    await expect.poll(() => titleCounts(page)).toEqual([12]);
  });

  test("a dismiss by keyboard removes the item and leaves the agent's status", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: fiveItems() });
    const cyRow = page
      .getByRole("region", { name: "Agents" })
      .getByRole("button", { name: /^Cy,/ });
    const statusBefore = await cyRow.getAttribute("aria-label");

    await page.getByRole("button", { name: "Dismiss Cy" }).focus();
    await page.keyboard.press("Enter");

    await expect(needsYouSection(page).getByRole("group")).toHaveCount(4);
    await expect(
      needsYouSection(page).getByRole("group", { name: /^Cy,/ }),
    ).toHaveCount(0);
    expect(await requestsOf(page, "needs_you.dismiss")).toEqual([
      { agent_id: "agent-cy" },
    ]);
    expect(await cyRow.getAttribute("aria-label")).toBe(statusBefore);
    await expect(
      needsYouSection(page).getByRole("heading", { name: /^Needs you/ }),
    ).toBeFocused();
  });

  test("the dismiss button shows a focus ring", async ({ page }) => {
    await openPanel(page, { needsYou: fiveItems() });
    const dismiss = page.getByRole("button", { name: "Dismiss Cy" });

    await dismiss.focus();

    const outline = await dismiss.evaluate((element) => {
      const style = getComputedStyle(element);
      return { width: style.outlineWidth, style: style.outlineStyle };
    });
    expect(outline.style).not.toBe("none");
    expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(2);
  });
});

test.describe("the jump shortcut", () => {
  test("five presses from a focused terminal open the five terminals in order and empty the list", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: fiveItems() });
    await openTerminalOf(page, "Zed");

    for (const [index, name] of FIVE_NAMES.entries()) {
      await page.keyboard.press(JUMP_KEY);
      const timeout = index === 0 ? FIRST_JUMP_BUDGET_MS : undefined;
      if (NOT_RUNNING_NAMES.has(name)) {
        await expect(terminalInput(page, name)).toBeAttached({ timeout });
        await expect(terminalHeading(page)).toBeFocused({ timeout });
        await expect(terminalHeading(page)).toHaveAccessibleName(
          `Terminal, ${name}`,
        );
        continue;
      }
      // SC-001: the top agent's terminal has focus within one second.
      await expect(terminalInput(page, name)).toBeFocused({ timeout });
    }

    await expect(
      needsYouSection(page).getByText(EMPTY_TEXT, { exact: true }),
    ).toBeVisible();
    // The key never reaches the CLI.
    expect(await sentTerminalInputs(page)).toEqual([]);
  });

  test("with an empty list the view stays and the live region says so", async ({
    page,
  }) => {
    await openPanel(page);
    await openTerminalOf(page, "Zed");

    await page.keyboard.press(JUMP_KEY);

    await expect(liveRegion(page)).toHaveText(EMPTY_TEXT);
    await expect(terminalInput(page, "Zed")).toBeFocused();
    expect(await readAgentIds(page)).toEqual(["agent-zed"]);
  });

  test("opens an agent of a project that the choice hides, as the switcher does", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: [item("agent-ada", "bell", SECOND_MS)] });
    await page
      .getByLabel("Project", { exact: true })
      .selectOption({ label: "Api" });
    await expect(
      page
        .getByRole("region", { name: "Agents" })
        .getByRole("button", { name: /^Ada,/ }),
    ).toHaveCount(0);

    await page.keyboard.press(JUMP_KEY);

    await expect(terminalInput(page, "Ada")).toBeFocused();
  });

  test("an agent deleted at the press is skipped for the next item", async ({
    page,
  }) => {
    await openPanel(page, {
      needsYou: [
        item("agent-ada", "failed", 20 * SECOND_MS),
        item("agent-bo", "bell", 10 * SECOND_MS),
      ],
    });
    await page.evaluate(() =>
      window.__mockCore.pushEvent({
        type: "agent.deleted",
        agent_id: "agent-ada",
      }),
    );

    await page.keyboard.press(JUMP_KEY);

    await expect(terminalInput(page, "Bo")).toBeFocused();
  });

  test("two fast presses before the server answers open the first and then the second item", async ({
    page,
  }) => {
    await openPanel(page, {
      readAgentDelayMs: 1_500,
      needsYou: [
        item("agent-cy", "failed", 20 * SECOND_MS, FAILED_DETAIL),
        item("agent-ada", "bell", 10 * SECOND_MS),
      ],
    });

    await page.keyboard.press(JUMP_KEY);
    await page.keyboard.press(JUMP_KEY);

    await expect(terminalInput(page, "Ada")).toBeFocused();
    expect(await readAgentIds(page)).toEqual(["agent-cy", "agent-ada"]);
  });

  test("a stored focus shortcut equal to the jump key keeps the key and unbinds the jump", async ({
    page,
  }) => {
    await openPanel(page, {
      focusShortcut: JUMP_KEY_LABEL,
      needsYou: [item("agent-ada", "bell", SECOND_MS)],
    });
    await expect(
      needsYouSection(page).getByText(/The jump shortcut is off because/),
    ).toContainText(
      "focus shortcut uses Ctrl+Shift+F11. Change the focus_shortcut setting (see the README)",
    );
    await expectNoSeriousViolations(page);

    // In the panel the key opens no agent.
    await page.keyboard.press(JUMP_KEY);
    await expect(terminalInput(page, "Ada")).toHaveCount(0);

    // In a focused terminal it moves focus out, as the focus shortcut does.
    await openTerminalOf(page, "Zed");
    await page.keyboard.press(JUMP_KEY);
    await expect(
      page.getByRole("heading", { name: "Terminal, Zed" }),
    ).toBeFocused();
    expect(await readAgentIds(page)).toEqual(["agent-zed"]);
    expect(await sentTerminalInputs(page)).toEqual([]);
  });

  test("the collision line is absent with the default focus shortcut", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: [item("agent-ada", "bell", SECOND_MS)] });
    // The shared load has settled once a terminal shows its focus key.
    await openTerminalOf(page, "Zed");
    await expect(page.getByText("Ctrl+Shift+F12").first()).toBeVisible();

    await expect(
      needsYouSection(page).getByText(/The jump shortcut is off/),
    ).toHaveCount(0);
  });

  test("in the scene it opens the top desk and leaves the panel's choices alone", async ({
    page,
  }) => {
    await openPanel(page, { needsYou: fiveItems() });
    const projectBefore = await page
      .getByLabel("Project", { exact: true })
      .inputValue();

    await page.getByRole("button", { name: "To the scene" }).click();
    await page.keyboard.press(JUMP_KEY);

    await expect(terminalInput(page, "Cy")).toBeFocused();
    await page.getByRole("button", { name: "To the panel" }).click();
    await expect(
      page.getByText(
        "Choose an agent to open its terminal, details, and action record.",
      ),
    ).toBeVisible();
    expect(await page.getByLabel("Project", { exact: true }).inputValue()).toBe(
      projectBefore,
    );
  });
});

test.describe("with an empty project chosen", () => {
  async function chooseEmptyProject(page: Page): Promise<void> {
    await openPanel(page, {
      projects: [...PROJECTS, EMPTY_PROJECT],
      needsYou: [item("agent-ada", "bell", SECOND_MS)],
    });
    await page
      .getByLabel("Project", { exact: true })
      .selectOption({ label: "Empty" });
    await expect(
      page.getByRole("heading", { name: "This project has no agents" }),
    ).toBeVisible();
  }

  test("Needs you still covers every project", async ({ page }) => {
    await chooseEmptyProject(page);

    await expect(
      needsYouSection(page).getByRole("heading", { name: "Needs you (1)" }),
    ).toBeVisible();
    await needsYouSection(page)
      .getByRole("button", { name: "Dismiss Ada" })
      .click();
    await expect(
      needsYouSection(page).getByText(EMPTY_TEXT, { exact: true }),
    ).toBeVisible();
  });

  test("the jump shows all projects first, so the opened terminal renders", async ({
    page,
  }) => {
    await chooseEmptyProject(page);

    await page.keyboard.press(JUMP_KEY);

    await expect(terminalInput(page, "Ada")).toBeFocused();
    await expect(page.getByLabel("Project", { exact: true })).toHaveValue("");
    expect(await readAgentIds(page)).toEqual(["agent-ada"]);
  });
});

test.describe("the window title count", () => {
  test("follows the dismissals in the panel", async ({ page }) => {
    await openPanel(page, {
      needsYou: [
        item("agent-cy", "failed", 30 * SECOND_MS, FAILED_DETAIL),
        item("agent-ada", "bell", 20 * SECOND_MS),
        item("agent-bo", "bell", 10 * SECOND_MS),
      ],
    });
    await expect.poll(() => titleCounts(page)).toEqual([3]);

    await page.getByRole("button", { name: "Dismiss Cy" }).click();
    await expect.poll(() => titleCounts(page)).toEqual([3, 2]);

    await page.getByRole("button", { name: "Dismiss Ada" }).click();
    await page.getByRole("button", { name: "Dismiss Bo" }).click();
    await expect.poll(() => titleCounts(page)).toEqual([3, 2, 1, 0]);
  });

  test("follows the jumps in the scene", async ({ page }) => {
    await openPanel(page, {
      needsYou: [
        item("agent-cy", "failed", 30 * SECOND_MS, FAILED_DETAIL),
        item("agent-ada", "bell", 20 * SECOND_MS),
        item("agent-bo", "bell", 10 * SECOND_MS),
      ],
    });
    await expect.poll(() => titleCounts(page)).toEqual([3]);
    await page.getByRole("button", { name: "To the scene" }).click();

    await page.keyboard.press(JUMP_KEY);
    await expect(terminalInput(page, "Cy")).toBeFocused();
    await page.keyboard.press(JUMP_KEY);
    await expect(terminalInput(page, "Ada")).toBeFocused();
    await page.keyboard.press(JUMP_KEY);
    await expect(terminalInput(page, "Bo")).toBeFocused();

    await expect.poll(() => titleCounts(page)).toEqual([3, 2, 1, 0]);
  });
});

test.describe("announcements", () => {
  test("a count change is announced once and a new item is not announced twice", async ({
    page,
  }) => {
    await openPanel(page);
    await page.evaluate(() => {
      const spoken: string[] = [];
      Object.assign(window, { spoken });
      const region = document.querySelector("p.sr-only[aria-live='polite']");
      new MutationObserver(() =>
        spoken.push(region?.textContent ?? ""),
      ).observe(region as Node, {
        childList: true,
        characterData: true,
        subtree: true,
      });
    });

    await page.evaluate(() =>
      window.__mockCore.attention("agent-cy", "waiting_for_input", "5000"),
    );
    await expect(liveRegion(page)).toHaveText("Needs you: 1 agent");
    // A later event of the same agent changes the item, not the count.
    await page.evaluate(() =>
      window.__mockCore.attention("agent-cy", "failed", "exit"),
    );
    await expect(liveRegion(page)).toContainText("Notification: Cy failed");

    const spoken = await page.evaluate(
      () => (window as unknown as { spoken: string[] }).spoken,
    );
    expect(spoken.filter((text) => text === "Needs you: 1 agent")).toHaveLength(
      1,
    );
    expect(spoken.some((text) => text.includes("needs you"))).toBe(false);
  });

  test("a view switch does not speak the standing count again", async ({
    page,
  }) => {
    await openPanel(page, {
      needsYou: [
        item("agent-ada", "bell", SECOND_MS),
        item("agent-bo", "bell", SECOND_MS),
      ],
    });
    await needsYouSection(page)
      .getByRole("button", { name: "Dismiss Ada" })
      .click();
    await expect(liveRegion(page)).toHaveText("Needs you: 1 agent");

    await page.getByRole("button", { name: "To the scene" }).click();
    await expect(
      page.getByRole("heading", { name: "Office scene" }),
    ).toBeVisible();
    await expect(page.locator("p.sr-only[role='status']")).not.toContainText(
      "Needs you",
    );
    await page.getByRole("button", { name: "To the panel" }).click();

    await expect(needsYouSection(page)).toBeVisible();
    await expect(liveRegion(page)).not.toContainText("Needs you");
  });

  test("an event for the on-screen agent in the focused window leaves no item, title, or count announcement", async ({
    page,
  }) => {
    await openPanel(page);
    await openTerminalOf(page, "Cy");

    await page.evaluate(() =>
      window.__mockCore.attention("agent-cy", "waiting_for_input", "5000"),
    );

    // The desktop reads the notification on arrival, which clears the item.
    await expect
      .poll(() => requestsOf(page, "notification.read"))
      .toHaveLength(1);
    await expect(
      needsYouSection(page).getByText(EMPTY_TEXT, { exact: true }),
    ).toBeVisible();
    await expect(liveRegion(page)).not.toContainText("Needs you");
    expect(await titleCounts(page)).toEqual([]);
  });
});
