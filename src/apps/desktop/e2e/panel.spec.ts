// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import type { HierarchyTree } from "../src/modules/hierarchy/models/hierarchy";
import type { NotificationRow } from "../src/modules/notification/models/notification";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const DESKTOP = { width: 1280, height: 720 };
const PHONE = { width: 360, height: 800 };

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

/** One agent per project choice: Site, Api, and no project. */
const OFFICE_AGENTS = [
  agent({ project_id: "project-site" }),
  agent({
    id: "agent-bo",
    name: "Bo",
    project_id: "project-api",
    status: "stopped",
  }),
  agent({ id: "agent-cy", name: "Cy", status: "waiting_for_input" }),
];

/** The 0002 tree that gives every seeded agent a role, and two of them a team. */
const TEAM_TREE: HierarchyTree = {
  ceo: { id: "agent-ada", role: "ceo" },
  teams: [
    {
      id: "team-backend",
      name: "Backend",
      project_id: null,
      lead: { id: "agent-bo", role: "lead" },
      members: [{ id: "agent-cy", role: "member" }],
    },
  ],
  unassigned: [],
};

async function openPanel(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: OFFICE_AGENTS, projects: PROJECTS, ...office },
  });
  await page.goto("/");
}

async function expectNoSeriousViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  const serious = report.violations.filter((violation) =>
    SERIOUS_IMPACTS.includes(violation.impact ?? ""),
  );
  expect(serious).toEqual([]);
}

/** Axe at the plan's two widths: desktop first, then phone. */
async function expectNoSeriousViolationsAtBothWidths(
  page: Page,
): Promise<void> {
  await expectNoSeriousViolations(page);
  await page.setViewportSize(PHONE);
  await expectNoSeriousViolations(page);
  await page.setViewportSize(DESKTOP);
}

async function chooseProject(page: Page, label: string): Promise<void> {
  await page.getByLabel("Project", { exact: true }).selectOption({ label });
}

/** The focus ring that the shared classes give every control. */
async function expectFocusVisible(row: ReturnType<Page["getByRole"]>) {
  const outline = await row.evaluate((element) => {
    const style = getComputedStyle(element);
    return { width: style.outlineWidth, style: style.outlineStyle };
  });
  expect(outline.style).not.toBe("none");
  expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(2);
}

test("All projects lists the agents of every project and the agent without one", async ({
  page,
}) => {
  await openPanel(page);

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Bo, project Api, stopped" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toBeVisible();
  await expect(list.getByRole("listitem")).toHaveCount(3);

  await expectNoSeriousViolationsAtBothWidths(page);
});

test("one project shows only its own agents, and All projects brings the rest back", async ({
  page,
}) => {
  await openPanel(page);

  await chooseProject(page, "Site");
  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", { name: "Bo, project Api, stopped" }),
  ).toHaveCount(0);
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toHaveCount(0);

  await chooseProject(page, "All projects");
  await expect(list.getByRole("listitem")).toHaveCount(3);
});

test("an empty project says so and offers to create an agent", async ({
  page,
}) => {
  await openPanel(page, { agents: [agent()] });

  await chooseProject(page, "Api");

  await expect(
    page.getByRole("heading", { name: "This project has no agents" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create an agent in this project" }),
  ).toBeVisible();
  // The hierarchy stays reachable while the chosen project has no agents.
  await expect(page.getByRole("region", { name: "Hierarchy" })).toBeVisible();
  await expectNoSeriousViolationsAtBothWidths(page);
});

test("the empty office keeps its offer at both widths", async ({ page }) => {
  await openPanel(page, { agents: [], projects: [] });

  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
  // The hierarchy stays reachable: the first CEO is built from it (US1).
  await expect(page.getByRole("region", { name: "Hierarchy" })).toBeVisible();
  await expectNoSeriousViolationsAtBothWidths(page);
});

test("the agent form opens with the empty project's project chosen", async ({
  page,
}) => {
  await openPanel(page, { agents: [agent()] });

  await chooseProject(page, "Api");
  await page
    .getByRole("button", { name: "Create an agent in this project" })
    .click();

  const dialog = page.getByRole("dialog", { name: "New agent" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Project", { exact: true })).toHaveValue(
    "project-api",
  );
});

test("the header's New agent forgets the empty project's preselection (LOW-3)", async ({
  page,
}) => {
  await openPanel(page, { agents: [agent()] });

  await chooseProject(page, "Api");
  await page
    .getByRole("button", { name: "Create an agent in this project" })
    .click();
  const dialog = page.getByRole("dialog", { name: "New agent" });
  await expect(dialog.getByLabel("Project", { exact: true })).toHaveValue(
    "project-api",
  );
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);

  await page.getByRole("button", { name: "New agent", exact: true }).click();
  await expect(
    page
      .getByRole("dialog", { name: "New agent" })
      .getByLabel("Project", { exact: true }),
  ).toHaveValue("");
});

test("a row names the agent, its project, its status, and its role and team", async ({
  page,
}) => {
  await openPanel(page, { hierarchy: TEAM_TREE });

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working, CEO" }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", {
      name: "Bo, project Api, stopped, team lead, Backend",
    }),
  ).toBeVisible();
  await expect(
    list.getByRole("button", {
      name: "Cy, waiting for input, member, Backend",
    }),
  ).toBeVisible();

  // The same names hold when the panel lays out in one column.
  await page.setViewportSize(PHONE);
  await expect(
    list.getByRole("button", {
      name: "Bo, project Api, stopped, team lead, Backend",
    }),
  ).toBeVisible();
  await page.setViewportSize(DESKTOP);
});

test("a row without a role and team leaves them out of the name", async ({
  page,
}) => {
  await openPanel(page);

  const list = page.getByRole("region", { name: "Agents" }).getByRole("list");
  await expect(
    list.getByRole("button", { name: "Ada, project Site, working" }),
  ).toHaveAttribute("aria-label", "Ada, project Site, working");
  await expect(
    list.getByRole("button", { name: "Cy, waiting for input" }),
  ).toHaveAttribute("aria-label", "Cy, waiting for input");
});

test("the last output line shows on the row", async ({ page }) => {
  await openPanel(page, {
    agents: [agent({ last_line: "Building the project" })],
  });

  await expect(page.getByText("Building the project")).toBeVisible();
});

test("a status and a line change update the row in place and keep the selection and focus", async ({
  page,
}) => {
  await openPanel(page);

  const row = page.getByRole("button", {
    name: "Ada, project Site, working",
  });
  await row.click();
  await expect(row).toHaveAttribute("aria-current", "true");
  // The opened terminal takes focus; the user returns to the list.
  await row.focus();

  await page.evaluate(() => {
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-ada",
      status: "waiting_for_input",
    });
    window.__mockCore.pushEvent({
      type: "agent.line",
      agent_id: "agent-ada",
      line: "Ready for input.",
    });
  });

  const updated = page.getByRole("button", {
    name: "Ada, project Site, waiting for input",
  });
  await expect(updated).toBeVisible();
  await expect(updated).toHaveAttribute("aria-current", "true");
  await expect(updated).toBeFocused();
  await expect(page.getByText("Ready for input.")).toBeVisible();
});

test("keyboard focus is visible on a row at both widths", async ({ page }) => {
  await openPanel(page);

  const row = page.getByRole("button", {
    name: "Ada, project Site, working",
  });
  await row.focus();
  await expectFocusVisible(row);

  await page.setViewportSize(PHONE);
  await expectFocusVisible(row);
  await page.setViewportSize(DESKTOP);
});

/** One agent beyond the table's nine direct shortcuts (A1). */
function agentAt(position: number): Agent {
  return agent({
    id: `agent-${position}`,
    name: `Agent ${position}`,
    status: "working",
  });
}

/** Twelve running agents, so every terminal takes focus: Ada and nine more. */
const TWELVE_AGENTS = [
  OFFICE_AGENTS[0],
  agent({ id: "agent-bo", name: "Bo", project_id: "project-api" }),
  agent({ id: "agent-cy", name: "Cy" }),
  ...[4, 5, 6, 7, 8, 9, 10, 11, 12].map(agentAt),
];

/** The twelve names in list order, so a shortcut's position finds its row. */
const TWELVE_NAMES = [
  "Ada",
  "Bo",
  "Cy",
  ...[4, 5, 6, 7, 8, 9, 10, 11, 12].map((position) => `Agent ${position}`),
];

/** The terminal's input area, the element that holds focus in a terminal. */
function terminalInput(page: Page, name: string) {
  return page.getByLabel(`Terminal of ${name}`, { exact: true });
}

async function sentTerminalInputs(page: Page): Promise<string[]> {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  const sent: string[] = [];
  for (const call of calls) {
    if (call.command !== "send_request") continue;
    const fields = call.args.fields as { data?: unknown } | undefined;
    if (call.args.kind === "terminal.input") {
      sent.push(Buffer.from(String(fields?.data ?? ""), "base64").toString());
    }
  }
  return sent;
}

test.describe("agent shortcuts", () => {
  test("shortcut 1 opens the first agent's terminal", async ({ page }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Cy, waiting for input" }).click();
    const input = terminalInput(page, "Cy");
    await expect(input).toBeFocused();

    await page.keyboard.press("Control+Shift+F1");

    await expect(terminalInput(page, "Ada")).toBeFocused();
  });

  for (const position of [2, 3, 4, 5, 6, 7, 8, 9]) {
    test(`shortcut ${position} opens the agent at position ${position}`, async ({
      page,
    }) => {
      await openPanel(page, { agents: TWELVE_AGENTS });
      await page
        .getByRole("button", { name: "Ada, project Site, working" })
        .click();
      await expect(terminalInput(page, "Ada")).toBeFocused();

      await page.keyboard.press(`Control+Shift+F${position}`);

      await expect(
        terminalInput(page, TWELVE_NAMES[position - 1]),
      ).toBeFocused();
    });
  }

  test("agent 10 is reached through the switcher, by no key", async ({
    page,
  }) => {
    await openPanel(page, { agents: TWELVE_AGENTS });
    await page
      .getByRole("button", { name: "Ada, project Site, working" })
      .click();
    const ada = terminalInput(page, "Ada");
    await expect(ada).toBeFocused();

    // No combination moves straight to agent 10; F10 opens the switcher,
    // and its filter finds the agent behind the table's reach (A1).
    for (const [index, name] of TWELVE_NAMES.slice(0, 9).entries()) {
      await page.keyboard.press(`Control+Shift+F${index + 1}`);
      await expect(terminalInput(page, name)).toBeFocused();
    }
    await page.keyboard.press("Control+Shift+F10");
    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    await expect(dialog).toBeVisible();
    await dialog
      .getByRole("textbox", { name: "Filter by name" })
      .fill("Agent 10");
    await dialog.getByRole("button", { name: "Agent 10, working" }).click();
    await expect(terminalInput(page, "Agent 10")).toBeFocused();
  });

  test("a key that is no table combination reaches the CLI", async ({
    page,
  }) => {
    await openPanel(page);
    await page.getByRole("button", { name: "Cy, waiting for input" }).click();
    const input = terminalInput(page, "Cy");
    await expect(input).toBeFocused();

    await page.keyboard.type("hi");
    // F11 is outside the table, so its sequence goes to the agent too.
    await page.keyboard.press("Control+Shift+F11");
    await page.keyboard.press("Enter");

    // One request is in flight at a time, so fast typing may share one.
    await expect
      .poll(async () => (await sentTerminalInputs(page)).join(""))
      .toBe(`hi\u001b[23;6~\r`);
    await expect(input).toBeFocused();
  });
});

test.describe("the agent switcher", () => {
  test("the switcher shortcut opens the dialog with focus in the filter", async ({
    page,
  }) => {
    await openPanel(page, { agents: TWELVE_AGENTS });
    await page
      .getByRole("button", { name: "Ada, project Site, working" })
      .click();
    await expect(terminalInput(page, "Ada")).toBeFocused();

    await page.keyboard.press("Control+Shift+F10");

    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    await expect(dialog).toBeVisible();
    await expect(
      dialog.getByRole("textbox", { name: "Filter by name" }),
    ).toBeFocused();
    await expectNoSeriousViolations(page);
  });

  test("the filter narrows the list and a choice opens that terminal", async ({
    page,
  }) => {
    await openPanel(page, { agents: TWELVE_AGENTS });

    await page.keyboard.press("Control+Shift+F10");
    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    await dialog
      .getByRole("textbox", { name: "Filter by name" })
      .fill("Agent 1");

    // "Agent 1" matches Agent 10, 11, and 12, and none of Ada, Bo, Cy.
    await expect(dialog.getByRole("listitem")).toHaveCount(3);
    await dialog.getByRole("button", { name: "Agent 11, working" }).click();

    await expect(dialog).toHaveCount(0);
    await expect(terminalInput(page, "Agent 11")).toBeFocused();
  });

  test("Tab stays inside the dialog and Escape returns focus", async ({
    page,
  }) => {
    await openPanel(page, { agents: TWELVE_AGENTS });
    const row = page.getByRole("button", {
      name: "Ada, project Site, working",
    });
    await row.click();
    await expect(terminalInput(page, "Ada")).toBeFocused();

    await page.keyboard.press("Control+Shift+F10");
    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    await expect(dialog).toBeVisible();
    const filter = dialog.getByRole("textbox", { name: "Filter by name" });
    await expect(filter).toBeFocused();

    // Tab cycles inside: the dialog is modal, so the page behind is inert.
    await page.keyboard.press("Tab");
    await expect(
      dialog.getByRole("button", { name: "Ada, project Site, working" }),
    ).toBeFocused();

    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(terminalInput(page, "Ada")).toBeFocused();
  });

  test("an empty match says so and Enter opens nothing", async ({ page }) => {
    await openPanel(page);
    await page.keyboard.press("Control+Shift+F10");
    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    await expect(dialog).toBeVisible();
    await dialog
      .getByRole("textbox", { name: "Filter by name" })
      .fill("nobody");

    await expect(dialog.getByText("No agent matches.")).toBeVisible();

    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("group", { name: "Live terminal" }),
    ).toHaveCount(0);
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });

  test("an agent whose stop request is open shows Stopping", async ({
    page,
  }) => {
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: { agents: OFFICE_AGENTS, projects: PROJECTS },
      stopDelayMs: 1_500,
    });
    await page.goto("/");
    await page
      .getByRole("button", { name: "Ada, project Site, working" })
      .click();
    await page.getByRole("button", { name: "Stop agent" }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Stop agent" })
      .click();

    await page.keyboard.press("Control+Shift+F10");
    const dialog = page.getByRole("dialog", { name: "Find an agent" });
    // The option reads as the row does while the stop is open, not as the
    // status the server still holds (LOW-2).
    await expect(
      dialog.getByRole("button", { name: "Ada, project Site, Stopping" }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("region", { name: "Agents" })
        .getByRole("button", { name: "Ada, project Site, Stopping" }),
    ).toBeVisible();
  });
});

test("a shortcut reaches a live terminal with output within one second (SC-002)", async ({
  page,
}) => {
  // A terminal frame carries the agent id as 16 bytes, so the seed id is a
  // UUID, as the terminal tests use (mock-core's frame contract).
  const adaId = "11111111-2222-4333-8444-555555555555";
  await openPanel(page, {
    agents: [
      agent({ id: adaId }),
      agent({
        id: "agent-bo",
        name: "Bo",
        project_id: "project-api",
        status: "stopped",
      }),
      agent({ id: "agent-cy", name: "Cy", status: "waiting_for_input" }),
    ],
    snapshots: { [adaId]: "saved output" },
  });
  await page.getByRole("button", { name: "Cy, waiting for input" }).click();
  await expect(terminalInput(page, "Cy")).toBeFocused();

  const started = Date.now();
  await page.keyboard.press("Control+Shift+F1");
  await expect(terminalInput(page, "Ada")).toBeFocused();
  await expect(page.locator(".xterm-rows")).toContainText("saved output", {
    timeout: 1_000,
  });
  // The clock runs while the expectations above hold it, so the whole
  // shortcut-to-output wait is the time at the last met expectation.
  expect(Date.now() - started).toBeLessThan(1_000);
});

/** One seeded notification row, as `notification.list` answers with it. */
function notification(
  overrides: Partial<NotificationRow> = {},
): NotificationRow {
  return {
    id: "notification-1",
    agent_id: "agent-bo",
    agent_name: "Bo",
    event: "exited",
    created_at: 1_700_000_000_000,
    read_at: null,
    ...overrides,
  };
}

async function requestKinds(page: Page): Promise<string[]> {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter((call) => call.command === "send_request")
    .map((call) => String(call.args.kind));
}

test.describe("notifications", () => {
  test("the unread count shows as text, and the dialog lists the rows (FR-007)", async ({
    page,
  }) => {
    await openPanel(page, {
      notifications: [
        notification({ id: "n1", event: "exited" }),
        notification({
          id: "n2",
          agent_id: "agent-cy",
          agent_name: "Cy",
          event: "waiting_for_input",
          created_at: 1_700_000_100_000,
        }),
        notification({
          id: "n3",
          event: "bell",
          read_at: 1_700_000_200_000,
        }),
      ],
    });

    await expect(
      page.getByRole("button", { name: "Notifications, 2 unread" }),
    ).toBeVisible();

    await page.getByRole("button", { name: "Notifications, 2 unread" }).click();
    const dialog = page.getByRole("dialog", { name: "Notifications" });
    await expect(dialog).toBeVisible();
    // Unread first, then newest first (D4); the read row is there too.
    await expect(
      dialog.getByRole("button", { name: "Cy, waits for your input" }),
    ).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Bo, exited" }),
    ).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Bo, rang the bell" }),
    ).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Bo, exited, agent deleted" }),
    ).toHaveCount(0);
    await expectNoSeriousViolationsAtBothWidths(page);
  });

  test("a live region announces each new notification once (FR-006)", async ({
    page,
  }) => {
    await openPanel(page);

    await page.evaluate(() => {
      window.__mockCore.notify("Bo", "agent-bo", "bell");
    });
    await expect(
      page.getByRole("button", { name: "Notifications, 1 unread" }),
    ).toBeVisible();
    // The office's own live region, not the login panel's (both are polite).
    const region = page.locator("p.sr-only[aria-live='polite']");
    await expect(region).toContainText("Bo rang the bell");

    await page.evaluate(() => {
      window.__mockCore.notify("Cy", "agent-cy", "waiting_for_input");
    });
    await expect(region).toContainText("Cy waits for your input");
  });

  test("a click opens the agent's terminal and clears its unread row (FR-009)", async ({
    page,
  }) => {
    await openPanel(page, {
      notifications: [notification({ id: "n1", event: "bell" })],
    });

    await page.getByRole("button", { name: "Notifications, 1 unread" }).click();
    const dialog = page.getByRole("dialog", { name: "Notifications" });
    await dialog.getByRole("button", { name: "Bo, rang the bell" }).click();

    await expect(dialog).toHaveCount(0);
    // Bo is stopped, so the terminal shows his last session's output.
    await expect(
      page.getByRole("group", { name: "Live terminal of Bo" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Notifications, 0 unread" }),
    ).toBeVisible();
    expect(await requestKinds(page)).toContain("notification.read");
  });

  test("a row of a deleted agent says so and opens nothing (edge case)", async ({
    page,
  }) => {
    await openPanel(page, {
      notifications: [
        notification({
          id: "n1",
          agent_id: "agent-gone",
          agent_name: "Hal",
          event: "failed",
        }),
      ],
    });

    await page.getByRole("button", { name: "Notifications, 1 unread" }).click();
    const dialog = page.getByRole("dialog", { name: "Notifications" });
    const row = dialog.getByRole("button", {
      name: "Hal, failed, agent deleted",
    });
    await expect(row).toBeVisible();
    await expect(row).toBeDisabled();
    await row.click({ force: true });

    await expect(
      page.getByRole("group", { name: "Live terminal" }),
    ).toHaveCount(0);
    await expect(dialog).toBeVisible();
    // The count still says 1: a disabled row is not read by a click.
    await expect(
      page.getByRole("button", { name: "Notifications, 1 unread" }),
    ).toBeVisible();
  });

  test("opening a terminal by any route reads that agent's rows (FR-009)", async ({
    page,
  }) => {
    await openPanel(page, {
      notifications: [
        notification({ id: "n1", event: "exited" }),
        notification({
          id: "n2",
          agent_id: "agent-cy",
          agent_name: "Cy",
          event: "bell",
        }),
      ],
    });

    await page
      .getByRole("button", { name: "Bo, project Api, stopped" })
      .click();
    await expect(
      page.getByRole("group", { name: "Live terminal of Bo" }),
    ).toBeVisible();

    await expect
      .poll(async () => {
        const calls = await page.evaluate(() => window.__mockCore.calls);
        return calls.some(
          (call) =>
            call.command === "send_request" &&
            call.args.kind === "notification.read_agent" &&
            (call.args.fields as { agent_id?: string })?.agent_id ===
              "agent-bo",
        );
      })
      .toBe(true);
    await expect(
      page.getByRole("button", { name: "Notifications, 1 unread" }),
    ).toBeVisible();
  });

  test("the last view restores the project, the agent, and its terminal (FR-008)", async ({
    page,
  }) => {
    await openPanel(page, {
      lastView: JSON.stringify({
        project: "project-api",
        agent: "agent-bo",
        terminal_open: true,
      }),
    });

    await expect(
      page.getByRole("group", { name: "Live terminal of Bo" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Bo, project Api, stopped" }),
    ).toHaveAttribute("aria-current", "true");
    const project = page.getByLabel("Project", { exact: true });
    await expect(project).toHaveValue("project-api");
  });

  test("a last view whose agent is gone says so and opens nothing (US3)", async ({
    page,
  }) => {
    await openPanel(page, {
      lastView: JSON.stringify({
        project: "project-site",
        agent: "agent-gone",
        terminal_open: true,
      }),
    });

    await expect(
      page
        .getByRole("alert")
        .getByText("The last view's agent no longer exists."),
    ).toBeVisible();
    await expect(
      page.getByRole("group", { name: "Live terminal" }),
    ).toHaveCount(0);
    // The project choice survives (edge case): Site's own agent still shows.
    await expect(
      page.getByRole("button", { name: "Ada, project Site, working" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Bo, project Api, stopped" }),
    ).toHaveCount(0);
  });

  test("the app writes the last view back as the user moves (D6)", async ({
    page,
  }) => {
    await openPanel(page);

    await page.getByRole("button", { name: "Cy, waiting for input" }).click();
    await expect(terminalInput(page, "Cy")).toBeFocused();

    // The debounce is one second (D6).
    await page.waitForTimeout(1_200);
    await expect
      .poll(async () => {
        const calls = await page.evaluate(() => window.__mockCore.calls);
        const write = [...calls]
          .reverse()
          .find(
            (call) =>
              call.command === "send_request" &&
              call.args.kind === "settings.set" &&
              (call.args.fields as { key?: string })?.key === "last_view",
          );
        return (write?.args.fields as { value?: string })?.value ?? "";
      })
      .toBe(
        JSON.stringify({
          project: null,
          agent: "agent-cy",
          terminal_open: true,
          view: "panel",
        }),
      );
  });

  test("the notification dialog is axe clean at both widths", async ({
    page,
  }) => {
    await openPanel(page, {
      notifications: [notification({ id: "n1" })],
    });

    await page.getByRole("button", { name: "Notifications, 1 unread" }).click();
    await expect(
      page.getByRole("dialog", { name: "Notifications" }),
    ).toBeVisible();
    await expectNoSeriousViolationsAtBothWidths(page);
  });
});
