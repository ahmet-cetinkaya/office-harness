// SPDX-License-Identifier: AGPL-3.0-or-later

import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore, type OfficeSeed } from "./support/mock-core";

const AGENT_ID = "11111111-2222-4333-8444-555555555555";
const KIND_OUTPUT = 2;
const KIND_DROPPED = 3;
const SCREEN = ".xterm-accessibility-tree";
const RENDERED_ROWS = ".xterm-rows";

function runningAgent(): Agent {
  return {
    id: AGENT_ID,
    name: "Ada",
    command: "claude",
    working_folder: "/work/ada",
    project_id: null,
    machine_id: "machine",
    status: "working",
    created_at: 1_700_000_000_000,
  };
}

async function openTerminal(
  page: Page,
  office: OfficeSeed = {},
): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: [runningAgent()],
      snapshots: { [AGENT_ID]: "saved output" },
      ...office,
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Ada, working" }).click();
}

async function terminalRequests(page: Page) {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter((call) => call.command === "send_request")
    .map((call) => ({
      kind: String(call.args.kind),
      fields: (call.args.fields ?? {}) as Record<string, unknown>,
    }))
    .filter((request) => request.kind.startsWith("terminal."));
}

async function sentInputs(page: Page): Promise<string[]> {
  const inputs = (await terminalRequests(page)).filter(
    (request) => request.kind === "terminal.input",
  );
  return inputs.map((request) =>
    Buffer.from(String(request.fields.data), "base64").toString(),
  );
}

test("attaching shows the saved output, then live output", async ({ page }) => {
  await openTerminal(page);

  await expect(page.locator(SCREEN)).toContainText("saved output");
  await expect(page.locator(RENDERED_ROWS)).toContainText("saved output");
  const attach = (await terminalRequests(page)).find(
    (request) => request.kind === "terminal.attach",
  );
  expect(attach?.fields.agent_id).toBe(AGENT_ID);
  expect(attach?.fields.cols).toBeGreaterThan(0);

  await page.evaluate(
    ([kind, id]) =>
      window.__mockCore.pushTerminalFrame(
        kind as number,
        id as string,
        "\r\nlive line",
      ),
    [KIND_OUTPUT, AGENT_ID],
  );

  await expect(page.locator(SCREEN)).toContainText("live line");
});

test("typing goes out as terminal.input", async ({ page }) => {
  await openTerminal(page);
  await expect(
    page.getByLabel("Terminal of Ada", { exact: true }),
  ).toBeFocused();

  await page.keyboard.type("hi");
  await page.keyboard.press("Enter");

  // One request is in flight at a time, so fast typing may share a request.
  await expect.poll(async () => (await sentInputs(page)).join("")).toBe("hi\r");
});

test("the focus shortcut leaves the terminal and is the only key held back", async ({
  page,
}) => {
  await openTerminal(page);
  const input = page.getByLabel("Terminal of Ada", { exact: true });
  await expect(input).toBeFocused();
  await expect(page.getByText("Ctrl+Shift+F12")).toBeVisible();

  await page.keyboard.press("Control+Shift+F12");

  await expect(input).not.toBeFocused();
  await expect(
    page.getByRole("heading", { name: "Terminal", exact: true }),
  ).toBeFocused();
  const sent = (await terminalRequests(page)).filter(
    (request) => request.kind === "terminal.input",
  );
  expect(sent).toEqual([]);
});

test("Shift+Tab goes to the agent and keeps focus in the terminal", async ({
  page,
}) => {
  await openTerminal(page);
  const input = page.getByLabel("Terminal of Ada", { exact: true });
  await expect(input).toBeFocused();

  await page.keyboard.press("Shift+Tab");

  await expect
    .poll(async () => (await sentInputs(page)).join(""))
    .toBe("\u001b[Z");
  await expect(input).toBeFocused();
});

test("queries in live output get no reply from the view", async ({ page }) => {
  await openTerminal(page);
  await expect(page.locator(RENDERED_ROWS)).toContainText("saved output");

  await page.evaluate(
    ([kind, id]) =>
      window.__mockCore.pushTerminalFrame(
        kind as number,
        id as string,
        "\r\nquery\u001b[c\u001b[6n",
      ),
    [KIND_OUTPUT, AGENT_ID],
  );
  await expect(page.locator(RENDERED_ROWS)).toContainText("query");
  await page.keyboard.type("x");

  // Requests keep their order, so a reply to a query would come before "x".
  await expect.poll(() => sentInputs(page)).toEqual(["x"]);
});

test("a stopped agent shows its last output and takes no input", async ({
  page,
}) => {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: [{ ...runningAgent(), status: "stopped" }],
      snapshots: { [AGENT_ID]: "saved output" },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Ada, stopped" }).click();

  await expect(page.locator(RENDERED_ROWS)).toContainText("saved output");
  await page.getByLabel("Terminal of Ada", { exact: true }).press("x");
  expect(await sentInputs(page)).toEqual([]);
});

test("Tab moves past the terminal of a stopped agent", async ({ page }) => {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: [{ ...runningAgent(), status: "stopped" }],
      snapshots: { [AGENT_ID]: "saved output" },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Ada, stopped" }).click();
  const input = page.getByLabel("Terminal of Ada", { exact: true });
  await input.focus();

  await page.keyboard.press("Tab");

  await expect(input).not.toBeFocused();
});

test("a configured shortcut replaces the default", async ({ page }) => {
  await openTerminal(page, { focusShortcut: "Ctrl+Shift+K" });
  const input = page.getByLabel("Terminal of Ada", { exact: true });
  await expect(input).toBeFocused();
  await expect(page.getByText("Ctrl+Shift+K")).toBeVisible();

  await page.keyboard.press("Control+Shift+K");

  await expect(input).not.toBeFocused();
});

test("dropped output is announced in text", async ({ page }) => {
  await openTerminal(page);
  await expect(page.locator(SCREEN)).toContainText("saved output");

  await page.evaluate(
    ([kind, id]) =>
      window.__mockCore.pushTerminalFrame(kind as number, id as string, ""),
    [KIND_DROPPED, AGENT_ID],
  );

  await expect(page.getByText("Some output was dropped.")).toBeVisible();
});

test("a renamed agent updates the terminal label", async ({ page }) => {
  await openTerminal(page);
  await expect(
    page.getByLabel("Terminal of Ada", { exact: true }),
  ).toBeFocused();

  await page.evaluate(
    (agent) => window.__mockCore.pushEvent({ type: "agent.changed", agent }),
    { ...runningAgent(), name: "Grace" },
  );

  await expect(
    page.getByLabel("Terminal of Grace", { exact: true }),
  ).toHaveCount(1);
});

test("a deleted agent closes the terminal with a message", async ({ page }) => {
  await openTerminal(page);
  await expect(page.locator(SCREEN)).toContainText("saved output");

  await page.evaluate(
    (id) =>
      window.__mockCore.pushEvent({ type: "agent.deleted", agent_id: id }),
    AGENT_ID,
  );

  await expect(page.getByText("The agent was deleted.")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Terminal", exact: true }),
  ).toHaveCount(0);
  await expect
    .poll(async () =>
      (await terminalRequests(page)).some(
        (request) => request.kind === "terminal.detach",
      ),
    )
    .toBe(true);
});
