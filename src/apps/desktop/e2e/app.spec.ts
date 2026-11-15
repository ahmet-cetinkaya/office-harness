// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("the shell renders and has no accessibility violations", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "office-harness" }),
  ).toBeVisible();

  const report = await new AxeBuilder({ page }).analyze();
  expect(report.violations).toEqual([]);
});
