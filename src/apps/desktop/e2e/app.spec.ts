// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { installMockCore, type MockCoreOptions } from "./support/mock-core";

async function openWith(page: Page, options: MockCoreOptions): Promise<void> {
  await installMockCore(page, options);
  await page.goto("/");
}

async function expectNoAccessibilityViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  expect(report.violations).toEqual([]);
}

test("the connecting state tells the user that the app is connecting", async ({
  page,
}) => {
  await openWith(page, { initialState: { status: "loading" } });

  await expect(page.getByRole("heading", { name: "Connecting" })).toBeVisible();
  await expect(page.getByRole("status")).toContainText(
    "Connecting to the office server",
  );
  await expectNoAccessibilityViolations(page);
});

test("a busy server is shown as busy and the app retries on its own", async ({
  page,
}) => {
  await openWith(page, { initialState: { status: "busy" } });

  await expect(
    page.getByRole("heading", { name: "The office server is busy" }),
  ).toBeVisible();
  await expect(page.getByRole("status")).toContainText(
    "tries again on its own",
  );
  await expectNoAccessibilityViolations(page);
});

test("the connected state shows the office shell", async ({ page }) => {
  await openWith(page, { initialState: { status: "connected" } });

  await expect(
    page.getByRole("heading", { name: "office-harness" }),
  ).toBeVisible();
  await expectNoAccessibilityViolations(page);
});

test.describe("not reachable", () => {
  const notReachable = { status: "not_reachable" } as const;

  test("offers to start the server and reaches the office when it starts", async ({
    page,
  }) => {
    await openWith(page, {
      initialState: notReachable,
      startServer: { outcome: "ok", stateAfterStart: { status: "connected" } },
    });
    await expect(
      page.getByRole("heading", { name: "The office server is not running" }),
    ).toBeVisible();
    await expectNoAccessibilityViolations(page);

    await page.getByRole("button", { name: "Start server" }).focus();
    await page.keyboard.press("Enter");

    await expect(
      page.getByRole("heading", { name: "office-harness" }),
    ).toBeVisible();
  });

  test("says in text when the server did not start", async ({ page }) => {
    await openWith(page, {
      initialState: notReachable,
      startServer: {
        outcome: "fail",
        code: "start_failed",
        message: "the server program could not be started",
      },
    });

    await page.getByRole("button", { name: "Start server" }).click();

    const alert = page.getByRole("alert");
    await expect(alert).toContainText("The server did not start.");
    await expect(alert).toContainText("could not be started");
    await expect(
      page.getByRole("button", { name: "Start server" }),
    ).toBeEnabled();
    await expectNoAccessibilityViolations(page);
  });
});

test.describe("not authorized", () => {
  test("lists the steps to give the app a valid token", async ({ page }) => {
    await openWith(page, { initialState: { status: "not_authorized" } });

    await expect(
      page.getByRole("heading", { name: "The server did not accept this app" }),
    ).toBeVisible();
    await expect(page.getByRole("listitem")).toHaveCount(3);
    await expectNoAccessibilityViolations(page);
  });

  test("checks again on request", async ({ page }) => {
    await openWith(page, { initialState: { status: "not_authorized" } });

    await page.getByRole("button", { name: "Check again" }).click();

    const calls = await page.evaluate(() => window.__mockCore.calls);
    expect(calls.map((call) => call.command)).toContain("retry_connection");
  });
});

test.describe("version mismatch", () => {
  const mismatch = {
    status: "version_mismatch",
    server_version: "0.0.9",
    app_version: "0.1.0",
    can_restart: true,
  } as const;

  test("names both versions", async ({ page }) => {
    await openWith(page, { initialState: mismatch });

    await expect(
      page.getByRole("heading", { name: "The server has another version" }),
    ).toBeVisible();
    await expect(page.getByText("version 0.0.9")).toBeVisible();
    await expectNoAccessibilityViolations(page);
  });

  test("asks for confirmation, and Escape cancels it", async ({ page }) => {
    await openWith(page, { initialState: mismatch });

    await page.getByRole("button", { name: "Restart server" }).click();
    await expect(page.getByRole("button", { name: "Cancel" })).toBeFocused();
    await expectNoAccessibilityViolations(page);
    await page.keyboard.press("Escape");

    await expect(
      page.getByRole("button", { name: "Restart server" }),
    ).toBeVisible();
    const calls = await page.evaluate(() => window.__mockCore.calls);
    expect(calls.map((call) => call.command)).not.toContain("send_request");
  });

  test("restarts only after the user confirms", async ({ page }) => {
    await openWith(page, { initialState: mismatch });

    await page.getByRole("button", { name: "Restart server" }).click();
    await page.getByRole("button", { name: "Restart and stop agents" }).click();

    const calls = await page.evaluate(() => window.__mockCore.calls);
    expect(
      calls.find((call) => call.command === "send_request")?.args,
    ).toMatchObject({ kind: "server.restart" });
  });

  test("offers no restart for a protocol the app does not know", async ({
    page,
  }) => {
    await openWith(page, { initialState: { ...mismatch, can_restart: false } });

    await expect(
      page.getByRole("button", { name: "Restart server" }),
    ).toHaveCount(0);
    await expect(page.getByText("does not know")).toBeVisible();
    await expectNoAccessibilityViolations(page);
  });
});

test("a state change from the core replaces the view without a reload", async ({
  page,
}) => {
  await openWith(page, { initialState: { status: "loading" } });

  await page.evaluate(() =>
    window.__mockCore.push({ status: "not_authorized" }),
  );

  await expect(
    page.getByRole("heading", { name: "The server did not accept this app" }),
  ).toBeVisible();
});

test("a page without the desktop core says so", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("alert")).toContainText(
    "No connection to the desktop core.",
  );
  await expectNoAccessibilityViolations(page);
});
