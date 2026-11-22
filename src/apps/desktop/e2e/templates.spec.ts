// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";
import type { Agent } from "../src/modules/agent/models/agent";
import {
  installMockCore,
  type OfficeSeed,
  type RecordedCall,
  type TemplateFilesOptions,
  type TemplateSeed,
} from "./support/mock-core";

const SERIOUS_IMPACTS = ["serious", "critical"];
const WORK_FOLDER = "/work/office";
const ESCAPE_CODE = String.fromCharCode(0x1b);
/** The success criterion SC-001: a 15-agent tree in under 3 minutes. */
const SC_001_LIMIT_MS = 3 * 60 * 1000;
const SC_001_CREATE_ACTIONS = 3;

function agent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "agent-ada",
    name: "Ada",
    command: "claude",
    working_folder: WORK_FOLDER,
    project_id: null,
    machine_id: "machine",
    status: "stopped",
    created_at: 1_700_000_000_000,
    last_line: null,
    ...overrides,
  };
}

function agentBody(overrides: Record<string, unknown> = {}) {
  return {
    name_pattern: "reviewer-{n}",
    command: "claude",
    working_folder: WORK_FOLDER,
    additions: { flags: [], env: [], startup_prompt: null },
    role: null,
    ...overrides,
  };
}

const CEO_TEMPLATE: TemplateSeed = {
  id: "template-ceo",
  name: "CEO",
  kind: "agent",
  body: { body_version: 1, ...agentBody({ name_pattern: "ceo", role: "ceo" }) },
};

const SQUAD_TEMPLATE: TemplateSeed = {
  id: "template-squad",
  name: "Squad",
  kind: "team",
  body: {
    body_version: 1,
    lead: agentBody({ name_pattern: "{team}-lead" }),
    members: [
      { entry: agentBody({ name_pattern: "{team}-member-{n}" }), count: 6 },
    ],
  },
};

const KEYED_TEMPLATE: TemplateSeed = {
  id: "template-keyed",
  name: "Keyed",
  kind: "agent",
  body: {
    body_version: 1,
    ...agentBody({
      name_pattern: "keyed-{n}",
      additions: {
        flags: ["--verbose"],
        env: [{ name: "API_KEY", value: null }],
        startup_prompt: "Read the plan.",
      },
    }),
  },
};

async function openOffice(
  page: Page,
  office: OfficeSeed = {},
  templateFiles?: TemplateFilesOptions,
): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office,
    templateFiles,
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

/** Presses a control by keyboard only: focus it, then Enter. */
async function activate(page: Page, control: Locator): Promise<void> {
  await control.focus();
  await page.keyboard.press("Enter");
}

async function activateButton(page: Page, name: string | RegExp) {
  await activate(page, page.getByRole("button", { name, exact: true }));
}

async function openTemplates(page: Page): Promise<void> {
  await activateButton(page, "Templates");
  await expect(
    page.getByRole("heading", { name: "Templates", level: 2 }),
  ).toBeFocused();
}

async function recordedCalls(page: Page): Promise<RecordedCall[]> {
  return page.evaluate(() => window.__mockCore.calls);
}

async function requestKinds(page: Page): Promise<string[]> {
  return (await recordedCalls(page))
    .filter((call) => call.command === "send_request")
    .map((call) => String(call.args.kind));
}

/** Fills a field and presses Enter in it, which submits its form. */
async function fillAndSubmit(page: Page, field: Locator, text: string) {
  await field.fill(text);
  await page.keyboard.press("Enter");
}

const IMPORT_FILE = JSON.stringify({
  format: "office-harness-templates",
  version: 1,
  templates: [
    {
      name: "Reviewer",
      kind: "agent",
      body: {
        body_version: 1,
        ...agentBody({
          additions: {
            flags: ["--verbose"],
            env: [{ name: "API_KEY", value: "secret-value" }],
            startup_prompt: `run ${ESCAPE_CODE}[31mred`,
          },
        }),
      },
    },
  ],
});

test("an empty templates view says so and offers save and import", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent()] });

  await openTemplates(page);

  await expect(page.getByText("No template exists yet.")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "New template", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Import templates", exact: true }),
  ).toBeVisible();
  await expectNoSeriousViolations(page);
});

test("saves an agent as a template from its details", async ({ page }) => {
  await openOffice(page, {
    agents: [agent()],
    additions: {
      "agent-ada": {
        flags: ["--verbose"],
        env: [{ name: "API_KEY", value: null }],
        startup_prompt: null,
        updated_at: 1,
        applies_next_session: false,
      },
    },
  });
  await activate(page, page.getByRole("button", { name: /^Ada,/ }));
  await activateButton(page, "Save as template");

  const dialog = page.getByRole("dialog", { name: "Save Ada as a template" });
  await expect(dialog.getByLabel("Template name")).toHaveValue("Ada");
  await expect(dialog.getByLabel("Agent: name pattern")).toHaveValue("Ada-{n}");
  await expect(dialog.getByLabel("Agent: command")).toHaveValue("claude");
  await expect(dialog.getByLabel("Agent: environment variables")).toHaveValue(
    "API_KEY",
  );
  await expectNoSeriousViolations(page);
  await fillAndSubmit(page, dialog.getByLabel("Template name"), "Reviewer");

  await expect(dialog).toBeHidden();
  await openTemplates(page);
  await expect(
    page.getByRole("heading", { name: "Reviewer", level: 3 }),
  ).toBeVisible();
  expect(await requestKinds(page)).toContain("template.save");
});

async function buildAgentTemplate(page: Page, name: string) {
  await activateButton(page, "New template");
  const dialog = page.getByRole("dialog", { name: "New template" });
  await dialog.getByLabel("Template name").fill(name);
  await dialog.getByLabel("Agent: name pattern").fill("ceo");
  await dialog.getByLabel("Agent: command").fill("claude");
  await dialog.getByLabel("Agent: working folder").fill(WORK_FOLDER);
  await dialog.getByLabel("Agent: role").selectOption("ceo");
  await activate(page, dialog.getByRole("button", { name: "Save template" }));
  await expect(dialog).toBeHidden();
}

async function buildTeamTemplate(page: Page, name: string) {
  await activateButton(page, "New template");
  const dialog = page.getByRole("dialog", { name: "New template" });
  await dialog.getByLabel("Template name").fill(name);
  await dialog.getByLabel("Kind").selectOption("team");
  await dialog.getByLabel("Lead: command").fill("claude");
  await dialog.getByLabel("Lead: working folder").fill(WORK_FOLDER);
  await dialog.getByLabel("Member entry 1: command").fill("claude");
  await dialog.getByLabel("Member entry 1: working folder").fill(WORK_FOLDER);
  await dialog.getByLabel("Member entry 1: count").fill("6");
  await activate(page, dialog.getByRole("button", { name: "Save template" }));
  await expect(dialog).toBeHidden();
}

test("builds a team template with a lead and a member entry", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent()] });
  await openTemplates(page);

  await activateButton(page, "New template");
  const dialog = page.getByRole("dialog", { name: "New template" });
  await dialog.getByLabel("Kind").selectOption("team");
  await expect(dialog.getByLabel("Lead: name pattern")).toHaveValue(
    "{team}-lead",
  );
  await expect(dialog.getByLabel("Member entry 1: name pattern")).toHaveValue(
    "{team}-member-{n}",
  );
  await expectNoSeriousViolations(page);
  await activate(page, dialog.getByRole("button", { name: "Cancel" }));
  await expect(dialog).toBeHidden();

  await buildTeamTemplate(page, "Squad");

  await expect(
    page.getByRole("heading", { name: "Squad", level: 3 }),
  ).toBeVisible();
  await expect(
    page.getByText("7 agents: a lead and one member entry"),
  ).toBeVisible();
});

test("a team template over the limit is refused with its reason", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent()] });
  await openTemplates(page);
  await activateButton(page, "New template");
  const dialog = page.getByRole("dialog", { name: "New template" });
  await dialog.getByLabel("Template name").fill("Huge");
  await dialog.getByLabel("Kind").selectOption("team");
  await dialog.getByLabel("Lead: command").fill("claude");
  await dialog.getByLabel("Member entry 1: command").fill("claude");
  await dialog.getByLabel("Member entry 1: count").fill("50");

  await activate(page, dialog.getByRole("button", { name: "Save template" }));

  await expect(dialog.getByRole("alert")).toContainText(
    "the team would create 51 agents; the limit is 50",
  );
  await expectNoSeriousViolations(page);
});

async function createFromTemplate(
  page: Page,
  templateLabel: string,
  teamName: string | null,
) {
  await activateButton(page, "Create from template");
  const dialog = page.getByRole("dialog", { name: "Create from template" });
  await dialog.getByLabel("Template").selectOption({ label: templateLabel });
  if (teamName !== null) {
    await dialog.getByLabel("Team name").fill(teamName);
  }
  await activate(
    page,
    dialog.getByRole("button", {
      name: teamName === null ? "Create agent" : "Create team",
    }),
  );
  await expect(dialog).toBeHidden();
}

test("creates a CEO and then two teams of one lead and six members from the templates", async ({
  page,
}) => {
  await openOffice(page, { templates: [CEO_TEMPLATE, SQUAD_TEMPLATE] });

  await createFromTemplate(page, "CEO (Agent template)", null);
  await createFromTemplate(page, "Squad (Team template)", "api");
  await createFromTemplate(page, "Squad (Team template)", "web");

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(tree.getByRole("treeitem", { name: /^ceo, CEO/ })).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: /^api-lead, team lead/ }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: /^api-member-6, member/ }),
  ).toBeVisible();
  await expect(
    tree.getByRole("treeitem", { name: /^web-member-6, member/ }),
  ).toBeVisible();
  const agentRows = await tree
    .getByRole("treeitem", { name: /, (CEO|team lead|member)/ })
    .count();
  expect(agentRows).toBe(15);
  await expectNoSeriousViolations(page);
});

test("a taken team name creates nothing and every reason shows as text", async ({
  page,
}) => {
  await openOffice(page, {
    templates: [CEO_TEMPLATE, SQUAD_TEMPLATE],
    refusals: {
      "template.create_team": {
        code: "invalid",
        message:
          'the folder "/work/missing" does not exist\nteam name "api" is already used',
      },
    },
  });
  await createFromTemplate(page, "CEO (Agent template)", null);

  await activateButton(page, "Create from template");
  const dialog = page.getByRole("dialog", { name: "Create from template" });
  await dialog
    .getByLabel("Template")
    .selectOption({ label: "Squad (Team template)" });
  await dialog.getByLabel("Team name").fill("api");
  await activate(page, dialog.getByRole("button", { name: "Create team" }));

  const alert = dialog.getByRole("alert");
  await expect(alert).toContainText(
    'the folder "/work/missing" does not exist',
  );
  await expect(alert).toContainText('team name "api" is already used');
  await expectNoSeriousViolations(page);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  expect(await page.getByRole("treeitem", { name: /^api-lead/ }).count()).toBe(
    0,
  );
});

test("the create dialog shows a variable without a value as no value", async ({
  page,
}) => {
  await openOffice(page, { templates: [KEYED_TEMPLATE] });

  await activateButton(page, "Create from template");

  const dialog = page.getByRole("dialog", { name: "Create from template" });
  await expect(dialog.getByText("API_KEY: no value")).toBeVisible();
  await expectNoSeriousViolations(page);
});

test("the import preview shows variable names, the values line and an escape symbol, and a cancel saves nothing", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent()] }, { importText: IMPORT_FILE });
  await openTemplates(page);

  await activateButton(page, "Import templates");

  const dialog = page.getByRole("dialog", { name: "Import preview" });
  await expect(dialog.getByText("API_KEY: no value")).toBeVisible();
  await expect(
    dialog.getByText("Values in the file are not imported", { exact: false }),
  ).toBeVisible();
  await expect(dialog.getByText("run ␛[31mred")).toBeVisible();
  expect(await dialog.getByText("secret-value").count()).toBe(0);
  await expectNoSeriousViolations(page);

  await page.keyboard.press("Escape");

  await expect(dialog).toBeHidden();
  await expect(page.getByText("No template exists yet.")).toBeVisible();
  expect(await requestKinds(page)).not.toContain("template.import.confirm");
});

test("the import preview opens with focus on its explanation, and Enter right after opening saves nothing", async ({
  page,
}) => {
  await openOffice(page, { agents: [agent()] }, { importText: IMPORT_FILE });
  await openTemplates(page);
  await activateButton(page, "Import templates");

  const dialog = page.getByRole("dialog", { name: "Import preview" });
  const explanation = dialog.getByText("Nothing is saved yet.", {
    exact: false,
  });
  await expect(explanation).toBeFocused();
  await expect(
    dialog.getByRole("button", { name: /^Save (this|these)/ }),
  ).not.toBeFocused();
  await expect(
    dialog.getByRole("list", { name: "Templates in the file" }),
  ).toHaveAttribute("tabindex", "0");

  await page.keyboard.press("Enter");

  await expect(dialog).toBeVisible();
  expect(await requestKinds(page)).not.toContain("template.import.confirm");
});

test("confirming the import preview saves the templates", async ({ page }) => {
  await openOffice(
    page,
    { agents: [agent()], templates: [{ ...CEO_TEMPLATE, name: "Reviewer" }] },
    { importText: IMPORT_FILE },
  );
  await openTemplates(page);
  await activateButton(page, "Import templates");

  const dialog = page.getByRole("dialog", { name: "Import preview" });
  await expect(
    dialog.getByRole("heading", { name: "Reviewer (2)", level: 3 }),
  ).toBeVisible();
  await activateButton(page, "Save this template");

  await expect(dialog).toBeHidden();
  await expect(
    page.getByRole("heading", { name: "Reviewer (2)", level: 3 }),
  ).toBeVisible();
  const calls = await recordedCalls(page);
  const confirm = calls.find(
    (call) => call.args.kind === "template.import.confirm",
  );
  expect(confirm).toBeDefined();
});

test("an invalid import shows its reason and saves nothing", async ({
  page,
}) => {
  const versionTwo = JSON.stringify({
    format: "office-harness-templates",
    version: 2,
    templates: [],
  });
  await openOffice(page, { agents: [agent()] }, { importText: versionTwo });
  await openTemplates(page);

  await activateButton(page, "Import templates");

  await expect(page.getByRole("alert")).toContainText(
    "the file version 2 is not supported; this build reads version 1",
  );
  expect(await page.getByRole("dialog").count()).toBe(0);
  await expectNoSeriousViolations(page);
});

test("a file that the core refuses shows the core's reason", async ({
  page,
}) => {
  await openOffice(
    page,
    { agents: [agent()] },
    {
      importFailure: {
        code: "template_file_failed",
        message: "the file was not read: it is larger than 256 KiB",
      },
    },
  );
  await openTemplates(page);

  await activateButton(page, "Import templates");

  await expect(page.getByRole("alert")).toContainText(
    "the file was not read: it is larger than 256 KiB",
  );
});

test("the export notice shows before anything is written", async ({ page }) => {
  await openOffice(page, {
    agents: [agent()],
    templates: [KEYED_TEMPLATE],
  });
  await openTemplates(page);

  await activateButton(page, "Export templates");

  const dialog = page.getByRole("dialog", { name: "Export templates" });
  await expect(
    dialog.getByText("Flags and startup prompts are written as they are", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    dialog.getByText("leaves out every environment value", { exact: false }),
  ).toBeVisible();
  await expectNoSeriousViolations(page);
  const before = (await recordedCalls(page)).filter(
    (call) => call.command === "export_templates_file",
  );
  expect(before).toHaveLength(0);

  await activateButton(page, "Choose a file and export");

  await expect(page.getByText("The templates were exported.")).toBeVisible();
  const written = (await recordedCalls(page)).filter(
    (call) => call.command === "export_templates_file",
  );
  expect(written).toHaveLength(1);
  const contents = String(written[0].args.contents);
  expect(contents).toContain('"value":null');
});

test("a file that cannot be written shows the reason", async ({ page }) => {
  await openOffice(
    page,
    { agents: [agent()], templates: [KEYED_TEMPLATE] },
    {
      exportFailure: {
        code: "template_file_failed",
        message: "the file was not written: permission denied",
      },
    },
  );
  await openTemplates(page);
  await activateButton(page, "Export templates");

  await activateButton(page, "Choose a file and export");

  await expect(page.getByRole("alert")).toContainText(
    "the file was not written: permission denied",
  );
});

test("a template is edited and deleted by keyboard and the agents stay", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent()],
    templates: [KEYED_TEMPLATE],
  });
  await openTemplates(page);

  await activateButton(page, "Edit Keyed");
  const editor = page.getByRole("dialog", { name: "Edit Keyed" });
  await expect(editor.getByLabel("Agent: environment variables")).toHaveValue(
    "API_KEY",
  );
  await fillAndSubmit(page, editor.getByLabel("Template name"), "Keyed two");
  await expect(editor).toBeHidden();
  await activateButton(page, "Delete Keyed two");
  const confirm = page.getByRole("dialog", {
    name: "Delete template Keyed two?",
  });
  await expectNoSeriousViolations(page);
  await activateButton(page, "Delete template");

  await expect(confirm).toBeHidden();
  await expect(page.getByText("No template exists yet.")).toBeVisible();
  await activateButton(page, "Back to the panel");
  await expect(page.getByRole("button", { name: /^Ada,/ })).toBeVisible();
});

test("SC-001: a 15-agent tree from two templates in three create actions", async ({
  page,
}, testInfo) => {
  await openOffice(page, { projects: [] });
  const startedAt = Date.now();

  await openTemplates(page);
  await buildAgentTemplate(page, "CEO");
  await buildTeamTemplate(page, "Squad");
  await activateButton(page, "Back to the panel");
  await createFromTemplate(page, "CEO (Agent template)", null);
  await createFromTemplate(page, "Squad (Team template)", "api");
  await createFromTemplate(page, "Squad (Team template)", "web");

  const tree = page.getByRole("tree", { name: "Hierarchy" });
  await expect(
    tree.getByRole("treeitem", { name: /^web-member-6, member/ }),
  ).toBeVisible();
  const elapsedMs = Date.now() - startedAt;
  const agentRows = await tree
    .getByRole("treeitem", { name: /, (CEO|team lead|member)/ })
    .count();
  const createActions = (await requestKinds(page)).filter((kind) =>
    kind.startsWith("template.create_"),
  ).length;
  const evidence = {
    flow: "SC-001 scripted run on the mock core, keyboard only",
    agents: agentRows,
    createActions,
    elapsedMs,
    limitMs: SC_001_LIMIT_MS,
  };
  const evidencePath = testInfo.outputPath("sc-001-timing.json");
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`SC-001 evidence: ${evidencePath} ${JSON.stringify(evidence)}`);

  expect(agentRows).toBe(15);
  expect(createActions).toBeLessThanOrEqual(SC_001_CREATE_ACTIONS);
  expect(elapsedMs).toBeLessThan(SC_001_LIMIT_MS);
});

test("an agent's details show a variable without a value as no value", async ({
  page,
}) => {
  await openOffice(page, {
    agents: [agent()],
    additions: {
      "agent-ada": {
        flags: [],
        env: [
          { name: "API_KEY", value: null },
          { name: "MODE", value: "fast" },
        ],
        startup_prompt: null,
        updated_at: 1,
        applies_next_session: false,
      },
    },
  });

  await page.getByRole("button", { name: "Ada, stopped" }).click();

  const saved = page.getByRole("region", {
    name: "Saved environment variables",
  });
  await expect(saved.getByText("API_KEY: no value")).toBeVisible();
  await expect(saved.getByText("MODE=fast")).toBeVisible();
  await expectNoSeriousViolations(page);
});
