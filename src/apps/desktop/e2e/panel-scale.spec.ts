// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { Agent } from "../src/modules/agent/models/agent";
import { installMockCore } from "./support/mock-core";

// 0003/SC-001 and SC-007 name 30 agents in 3 projects: 3 projects of 10.
const PROJECT_COUNT = 3;
const AGENTS_PER_PROJECT = 10;
const AGENT_COUNT = PROJECT_COUNT * AGENTS_PER_PROJECT;
const WAITING_AGENT_INDEX = 17;
/** SC-007: the person finds the waiting agent and types within 10 seconds. */
const FIND_AND_TYPE_LIMIT_MS = 10_000;

const PROJECTS = Array.from({ length: PROJECT_COUNT }, (_, floor) => ({
  id: `project-${floor}`,
  name: `Floor ${floor}`,
  folder: `/work/floor-${floor}`,
}));

function agentName(index: number): string {
  return `Agent ${String(index).padStart(2, "0")}`;
}

const AGENTS: Agent[] = Array.from({ length: AGENT_COUNT }, (_, index) => ({
  id: `agent-${index}`,
  name: agentName(index),
  command: "claude",
  working_folder: `/work/${index}`,
  project_id: PROJECTS[Math.floor(index / AGENTS_PER_PROJECT)].id,
  machine_id: "machine",
  status: "working",
  created_at: 1_700_000_000_000 + index,
  last_line: null,
}));

function rowName(index: number, status: string): string {
  const project = PROJECTS[Math.floor(index / AGENTS_PER_PROJECT)].name;
  return `${agentName(index)}, project ${project}, ${status}`;
}

async function openPanelWith30Agents(page: Page): Promise<void> {
  await installMockCore(page, {
    initialState: { status: "connected" },
    office: { agents: AGENTS, projects: PROJECTS },
  });
  await page.goto("/");
}

test("the panel lists 30 agents of 3 projects, each with its project and status", async ({
  page,
}) => {
  await openPanelWith30Agents(page);

  for (let index = 0; index < AGENT_COUNT; index += 1) {
    await expect(
      page.getByRole("button", { name: rowName(index, "working") }),
    ).toBeVisible();
  }
  const report = await new AxeBuilder({ page }).analyze();
  expect(
    report.violations.filter((violation) =>
      ["serious", "critical"].includes(violation.impact ?? ""),
    ),
  ).toEqual([]);
});

test("a burst of status changes updates the rows in place and keeps selection and focus (SC-001)", async ({
  page,
}) => {
  await openPanelWith30Agents(page);
  const selected = page.getByRole("button", { name: rowName(5, "working") });
  await selected.click();
  await expect(selected).toHaveAttribute("aria-current", "true");
  await selected.focus();

  await page.evaluate((count) => {
    for (let index = 0; index < count; index += 1) {
      window.__mockCore.pushEvent({
        type: "agent.status",
        agent_id: `agent-${index}`,
        status: index % 2 === 0 ? "waiting_for_input" : "stopped",
      });
    }
  }, AGENT_COUNT);

  for (let index = 0; index < AGENT_COUNT; index += 1) {
    const status = index % 2 === 0 ? "waiting for input" : "stopped";
    await expect(
      page.getByRole("button", { name: rowName(index, status) }),
    ).toBeVisible();
  }
  const updated = page.getByRole("button", {
    name: rowName(5, "stopped"),
  });
  await expect(updated).toHaveAttribute("aria-current", "true");
  await expect(updated).toBeFocused();
});

test("the person finds the waiting agent among 30 and types within 10 seconds (SC-007)", async ({
  page,
}) => {
  await openPanelWith30Agents(page);
  const started = Date.now();

  await page.evaluate((agentId) => {
    window.__mockCore.pushEvent({
      type: "agent.status",
      agent_id: agentId,
      status: "waiting_for_input",
    });
  }, `agent-${WAITING_AGENT_INDEX}`);
  const waiting = page.getByRole("button", {
    name: rowName(WAITING_AGENT_INDEX, "waiting for input"),
  });
  await waiting.focus();
  await page.keyboard.press("Enter");

  const input = page.getByLabel(
    `Terminal of ${agentName(WAITING_AGENT_INDEX)}`,
    {
      exact: true,
    },
  );
  await expect(input).toBeFocused();
  await page.keyboard.type("y");

  expect(Date.now() - started).toBeLessThan(FIND_AND_TYPE_LIMIT_MS);
});
