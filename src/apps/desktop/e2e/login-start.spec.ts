// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { installMockCore, type MockCoreOptions } from "./support/mock-core";

async function openConnected(
  page: Page,
  loginStart?: MockCoreOptions["loginStart"],
): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    loginStart,
  });
  await page.goto("/");
}

async function expectNoAccessibilityViolations(page: Page): Promise<void> {
  const report = await new AxeBuilder({ page }).analyze();
  expect(report.violations).toEqual([]);
}

test("says that the server does not start at login yet, and registers it", async ({
  page,
}) => {
  await openConnected(page);

  await expect(
    page.getByRole("heading", { name: "Server at login" }),
  ).toBeVisible();
  await expect(page.getByText("does not start when you log in")).toBeVisible();
  await expectNoAccessibilityViolations(page);

  await page.getByRole("button", { name: "Start the server at login" }).click();

  await expect(
    page.getByText("The server starts when you log in."),
  ).toBeVisible();
  const calls = await page.evaluate(() => window.__mockCore.calls);
  expect(calls.map((call) => call.command)).toContain("register_login_start");
  await expectNoAccessibilityViolations(page);
});

test("shows an already registered server as registered", async ({ page }) => {
  await openConnected(page, { registered: true });

  await expect(
    page.getByText("The server starts when you log in."),
  ).toBeVisible();
});

test("says in text when the registration failed", async ({ page }) => {
  await openConnected(page, {
    failure: {
      code: "login_start_failed",
      message: "login item: refused",
    },
  });

  await page.getByRole("button", { name: "Start the server at login" }).focus();
  await page.keyboard.press("Enter");

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("The server was not registered.");
  await expect(alert).toContainText("login item: refused");
  await expect(page.getByText("does not start when you log in")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Start the server at login" }),
  ).toBeEnabled();
  await expectNoAccessibilityViolations(page);
});
