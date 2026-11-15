// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent, Session } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ada",
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "stopped",
    created_at: 1_700_000_000_000,
    ...overrides,
  };
}

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-old",
    agent_id: "agent-ada",
    cli_session_id: "cli-1",
    continued_from: null,
    additions_snapshot: { flags: [], env: [], startup_prompt: null },
    started_at: 1_700_000_000_000,
    ended_at: 1_700_000_600_000,
    exit_result: "0",
    can_continue: true,
    ...overrides,
  };
}

async function openOffice(page: Page, office: OfficeSeed = {}): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
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

async function pressOn(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
}

async function requestKinds(page: Page): Promise<string[]> {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter((call) => call.command === "send_request")
    .map((call) => String(call.args.kind));
}

test("an empty office says so and offers to create the first agent", async ({
  page,
}) => {
  await openOffice(page);

  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create the first agent" }),
  ).toBeVisible();
  await expectNoSeriousViolations(page);
});

test("an agent is created and started from the keyboard alone", async ({
  page,
}) => {
  await openOffice(page);

  await pressOn(page, "Create the first agent");
  await expect(page.getByRole("dialog", { name: "New agent" })).toBeVisible();
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.keyboard.type("Ada");
  await page.keyboard.press("Tab");
  await page.keyboard.type("claude");
  await page.keyboard.press("Tab");
  await page.keyboard.type("/work/ada");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: "Create agent" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");

  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Ada, stopped" }),
  ).toBeVisible();
  await pressOn(page, "Start agent");

  await expect(
    page.getByRole("button", { name: "Ada, starting" }),
  ).toBeVisible();
  expect(await requestKinds(page)).toContain("agent.start");
});

test("a form says in text why the server refused it", async ({ page }) => {
  await openOffice(page);

  await pressOn(page, "Create the first agent");
  await page.getByRole("button", { name: "Create agent" }).click();

  await expect(page.getByRole("alert")).toContainText("name is required");
  await expect(page.getByRole("dialog", { name: "New agent" })).toBeVisible();
});

test("a status event changes the text without a reload", async ({ page }) => {
  await openOffice(page, { agents: [agent({ status: "starting" })] });
  await expect(
    page.getByRole("button", { name: "Ada, starting" }),
  ).toBeVisible();

  await page.evaluate(() =>
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: "agent-ada",
      status: "waiting_for_input",
    }),
  );

  await expect(
    page.getByRole("button", { name: "Ada, waiting for input" }),
  ).toBeVisible();
});

test("a running agent's additions edit applies to the next session, and another client's save refreshes the editor", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent({ status: "working" })] });

  await page.getByRole("button", { name: "Ada, working" }).click();
  await expect(page.getByText("Applies to the next session.")).toBeVisible();
  await page.getByLabel("Flags", { exact: true }).fill("--model\nopus");
  await page.getByRole("button", { name: "Save additions" }).click();
  await expect(page.getByText("Additions saved.")).toBeVisible();
  expect(await requestKinds(page)).toContain("agent.additions.set");

  await page.evaluate(() =>
    window.__mockCore.pushEvent({
      type: "agent.additions.changed",
      agent_id: "agent-ada",
      additions: {
        flags: ["--from-another-client"],
        env: [{ name: "MODE", value: "fast" }],
        startup_prompt: "Be brief.",
        updated_at: 2,
        applies_next_session: true,
      },
    }),
  );

  await expect(page.getByLabel("Flags", { exact: true })).toHaveValue(
    "--from-another-client",
  );
  await expect(
    page.getByLabel("Environment variables", { exact: true }),
  ).toHaveValue("MODE=fast");
  await expect(page.getByLabel("Startup prompt", { exact: true })).toHaveValue(
    "Be brief.",
  );
});

test("a malformed environment line is named and not sent", async ({ page }) => {
  await openOffice(page, { agents: [agent()] });

  await page.getByRole("button", { name: "Ada, stopped" }).click();
  await page
    .getByLabel("Environment variables", { exact: true })
    .fill("BROKEN");
  await page.getByRole("button", { name: "Save additions" }).click();

  await expect(page.getByRole("alert")).toContainText(
    '"BROKEN" needs the form NAME=value.',
  );
  expect(await requestKinds(page)).not.toContain("agent.additions.set");
});

test("continue is offered only for a session that can continue", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent()],
    sessions: {
      "agent-ada": [
        session({ id: "session-new", started_at: 1_700_001_000_000 }),
        session({
          id: "session-old",
          can_continue: false,
          reason: "the CLI cannot continue this kind of session",
        }),
      ],
    },
  });

  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await expect(
    page.getByRole("button", { name: /^Continue session from/ }),
  ).toHaveCount(1);
  await expect(
    page.getByText("Cannot continue: the CLI cannot continue this kind of"),
  ).toBeVisible();
});

test("a new task on a running agent asks first, and Escape cancels", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent({ status: "working" })] });
  await page.getByRole("button", { name: "Ada, working" }).click();

  await pressOn(page, "Start new task");
  const dialog = page.getByRole("dialog", { name: "Start a new task on Ada?" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Cancel" })).toBeFocused();
  await expectNoSeriousViolations(page);
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  expect(await requestKinds(page)).not.toContain("agent.new_task");

  await pressOn(page, "Start new task");
  await page.getByRole("button", { name: "Start new task" }).last().click();

  await expect.poll(() => requestKinds(page)).toContain("agent.new_task");
});

test("deleting an agent asks first and then removes it", async ({ page }) => {
  await openOffice(page, { agents: [agent()] });
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await pressOn(page, "Delete agent");
  await page.getByRole("button", { name: "Delete agent" }).last().click();

  await expect(
    page.getByRole("heading", { name: "The office is empty" }),
  ).toBeVisible();
});

test("after deleting an agent focus moves to the agents heading", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent(), agent({ id: "agent-bob", name: "Bob" })],
  });
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await pressOn(page, "Delete agent");
  await page.getByRole("button", { name: "Delete agent" }).last().click();

  await expect(
    page.getByRole("heading", { name: "Agents", exact: true }),
  ).toBeFocused();
});

test("the action record shows what was done to the agent", async ({ page }) => {
  await openOffice(page, { agents: [agent()] });
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await pressOn(page, "Start agent");

  await expect(
    page.getByRole("region", { name: "Action record" }),
  ).toContainText("Started by user. Succeeded");
});

test("a project is created from its dialog", async ({ page }) => {
  await openOffice(page, { agents: [agent()] });

  await pressOn(page, "New project");
  await page.getByLabel("Name", { exact: true }).fill("Site");
  await page.getByLabel("Folder", { exact: true }).fill("/work/site");
  await page.getByRole("button", { name: "Create project" }).click();

  await expect(page.getByText("/work/site")).toBeVisible();
  await expectNoSeriousViolations(page);
});

test.describe("accessibility", () => {
  test("the agent list with its details has no serious violation", async ({
    page,
  }) => {
    await openOffice(page, {
      agents: [agent({ status: "working" }), agent({ id: "b", name: "Bo" })],
      projects: [{ id: "p", name: "Site", folder: "/work/site" }],
      sessions: { "agent-ada": [session()] },
    });
    await expectNoSeriousViolations(page);

    await page.getByRole("button", { name: "Ada, working" }).click();
    await expect(
      page.getByRole("heading", { name: "Office additions" }),
    ).toBeVisible();

    await expectNoSeriousViolations(page);
  });

  test("the new agent dialog has no serious violation", async ({ page }) => {
    await openOffice(page, { agents: [agent()] });

    await pressOn(page, "New agent");
    await expect(page.getByRole("dialog", { name: "New agent" })).toBeVisible();

    await expectNoSeriousViolations(page);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "New agent" })).toBeFocused();
  });
});
