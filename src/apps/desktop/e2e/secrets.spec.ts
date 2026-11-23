// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import {
  installMockCore,
  type OfficeSeed,
  type SecretsSeed,
} from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const MARKER = "unique-marker-value-0026";
const NEW_MARKER = "second-marker-value-0026";

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

async function openSecrets(
  page: Page,
  secrets: SecretsSeed = {},
  office: OfficeSeed = {},
) {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: {
      agents: [agent(), agent({ id: "agent-bo", name: "Bo" })],
      secrets,
      ...office,
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Secrets", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Secrets" });
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

async function requestFields(page: Page, kind: string) {
  const calls = await page.evaluate(() => window.__mockCore.calls);
  return calls
    .filter(
      (call) => call.command === "send_request" && call.args.kind === kind,
    )
    .map((call) => call.args.fields);
}

test.describe("the Secrets dialog", () => {
  test("an empty store says so, offers one action, and has no serious violation", async ({
    page,
  }) => {
    const dialog = await openSecrets(page);

    await expect(dialog.getByRole("status")).toHaveText("No secret saved yet.");
    await expect(
      dialog.getByText("No secret is saved. Add one below"),
    ).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Save secret" }),
    ).toBeEnabled();
    await expectNoSeriousViolations(page);
  });

  test("the person adds a secret with the keyboard, the value is dots, and the field is empty after", async ({
    page,
  }) => {
    const dialog = await openSecrets(page);
    const value = dialog.getByLabel("Value", { exact: true });
    await expect(value).toHaveAttribute("type", "password");

    await dialog.getByLabel("Name", { exact: true }).fill("OPENAI_API_KEY");
    await value.fill(MARKER);
    await value.press("Enter");

    await expect(
      dialog.getByRole("heading", { name: /OPENAI_API_KEY/ }),
    ).toBeVisible();
    await expect(dialog.getByText("version 1")).toBeVisible();
    await expect(dialog.getByRole("status")).toHaveText(
      "Stored with: OS keyring",
    );
    await expect(value).toHaveValue("");
    expect(await requestFields(page, "secret.set")).toEqual([
      { name: "OPENAI_API_KEY", value: MARKER },
    ]);
    // The value is on no screen of the app after the save.
    expect(await page.content()).not.toContain(MARKER);
  });

  test("a short value and a bad name are refused on the page, and nothing is sent", async ({
    page,
  }) => {
    const dialog = await openSecrets(page);

    await dialog.getByLabel("Name", { exact: true }).fill("api key");
    await dialog.getByLabel("Value", { exact: true }).fill("short");
    await dialog.getByRole("button", { name: "Save secret" }).click();

    await expect(dialog.getByRole("alert")).toContainText("The name is");
    await expect(dialog.getByLabel("Value", { exact: true })).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    expect(await requestFields(page, "secret.set")).toEqual([]);
    expect(await page.content()).not.toContain("short");
  });

  test("a grant, a replace, and a revoke show in words", async ({ page }) => {
    const dialog = await openSecrets(page, {
      store: { state: "ready", custody: "os_keyring" },
      secrets: [{ name: "API_KEY", version: 1 }],
    });

    await dialog.getByLabel("Grant API_KEY to").selectOption({ label: "Ada" });
    await dialog.getByRole("button", { name: "Grant API_KEY" }).click();
    await expect(dialog.getByText("not read yet")).toBeVisible();

    await dialog
      .getByRole("button", { name: "Replace value of API_KEY" })
      .click();
    const field = dialog.getByLabel("New value of API_KEY");
    await expect(field).toBeVisible();
    await field.fill(NEW_MARKER);
    await field.press("Enter");

    await expect(dialog.getByText("version 2")).toBeVisible();
    expect(await requestFields(page, "secret.set")).toEqual([
      { name: "API_KEY", value: NEW_MARKER },
    ]);
    await dialog
      .getByRole("button", { name: "Revoke Ada from API_KEY" })
      .click();
    await expect(dialog.getByText("No agent holds a grant.")).toBeVisible();
    expect(await page.content()).not.toContain(NEW_MARKER);
  });

  test("an agent that read an older version shows as behind, in text", async ({
    page,
  }) => {
    const dialog = await openSecrets(page, {
      store: { state: "ready", custody: "key_file" },
      secrets: [
        {
          name: "API_KEY",
          version: 3,
          grants: [
            { agent_id: "agent-ada", last_read_version: 2, last_read_at: 9 },
            { agent_id: "agent-bo", last_read_version: 3, last_read_at: 9 },
          ],
        },
      ],
    });

    await expect(dialog.getByRole("status")).toHaveText(
      "Stored with: key file",
    );
    await expect(
      dialog.getByText("read version 2, behind (current 3)"),
    ).toBeVisible();
    await expect(dialog.getByText("read version 3, current")).toBeVisible();
    await expectNoSeriousViolations(page);
  });

  test("a delete asks first and then removes the secret", async ({ page }) => {
    const dialog = await openSecrets(page, {
      store: { state: "ready", custody: "os_keyring" },
      secrets: [{ name: "API_KEY", version: 1 }],
    });

    await dialog.getByRole("button", { name: "Delete API_KEY…" }).click();
    await expect(
      dialog.getByText("Delete API_KEY and its 0 grants?"),
    ).toBeVisible();
    expect(await requestFields(page, "secret.delete")).toEqual([]);
    await dialog
      .getByRole("button", { name: "Delete API_KEY", exact: true })
      .click();

    await expect(dialog.getByText("No secret is saved.")).toBeVisible();
    expect(await requestFields(page, "secret.delete")).toEqual([
      { name: "API_KEY" },
    ]);
  });

  test("a locked store says why, refuses saves, and resets after a confirmation", async ({
    page,
  }) => {
    const dialog = await openSecrets(page, {
      store: {
        state: "locked",
        custody: "os_keyring",
        locked_reason: "the keyring is locked",
      },
      secrets: [
        { name: "A_KEY", version: 1 },
        { name: "B_KEY", version: 1 },
      ],
    });

    await expect(dialog.getByRole("status")).toHaveText(
      "Locked: the keyring is locked (OS keyring)",
    );
    await expect(
      dialog.getByRole("button", { name: "Save secret" }),
    ).toBeDisabled();
    await dialog.getByRole("button", { name: "Reset…" }).click();
    await expect(
      dialog.getByText("This removes all 2 saved secrets and their key."),
    ).toBeVisible();
    await dialog.getByRole("button", { name: "Reset the store" }).click();

    await expect(dialog.getByRole("status")).toHaveText("No secret saved yet.");
    expect(await requestFields(page, "secret.reset")).toEqual([
      { confirm_count: 2 },
    ]);
    await expectNoSeriousViolations(page);
  });

  test("Escape closes the dialog and focus returns to the button that opened it", async ({
    page,
  }) => {
    const dialog = await openSecrets(page);

    await page.keyboard.press("Escape");

    await expect(dialog).toBeHidden();
    await expect(
      page.getByRole("button", { name: "Secrets", exact: true }),
    ).toBeFocused();
  });

  test("save, grant, replace, revoke, and delete work with the keyboard alone (SC-009)", async ({
    page,
  }) => {
    const started = Date.now();
    await installMockCore(page, {
      initialState: { status: "connected" },
      office: { agents: [agent(), agent({ id: "agent-bo", name: "Bo" })] },
    });
    await page.goto("/");
    const dialog = await openByKeyboard(page, "Secrets");

    await dialog.getByLabel("Name", { exact: true }).focus();
    await page.keyboard.type("API_KEY");
    await page.keyboard.press("Tab");
    await page.keyboard.type(MARKER);
    await page.keyboard.press("Enter");
    await expect(dialog.getByText("version 1")).toBeVisible();
    await expect(dialog.getByLabel("Value", { exact: true })).toHaveValue("");

    await pressByKeyboard(page, dialog, "Grant API_KEY");
    await expect(dialog.getByText("not read yet")).toBeVisible();
    await pressByKeyboard(page, dialog, "Replace value of API_KEY");
    await page.keyboard.type(NEW_MARKER);
    await page.keyboard.press("Enter");
    await expect(dialog.getByText("version 2")).toBeVisible();
    await pressByKeyboard(page, dialog, "Revoke Ada from API_KEY");
    await expect(dialog.getByText("No agent holds a grant.")).toBeVisible();
    await pressByKeyboard(page, dialog, "Delete API_KEY…");
    // Focus moved to the question's button: Enter confirms.
    await expect(
      dialog.getByRole("button", { name: "Delete API_KEY", exact: true }),
    ).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(dialog.getByText("No secret is saved.")).toBeVisible();
    expect(Date.now() - started).toBeLessThan(120_000);
    expect(await page.content()).not.toContain(MARKER);
  });
});

/** Presses Enter on a button that took focus without a click. */
async function pressByKeyboard(
  page: Page,
  scope: ReturnType<Page["getByRole"]>,
  name: string,
  exact = false,
): Promise<void> {
  await scope.getByRole("button", { name, exact }).focus();
  await page.keyboard.press("Enter");
}

async function openByKeyboard(page: Page, name: string) {
  await page.getByRole("button", { name, exact: true }).focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name });
  await expect(dialog).toBeVisible();
  return dialog;
}
