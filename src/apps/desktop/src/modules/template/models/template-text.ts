// SPDX-License-Identifier: AGPL-3.0-or-later

import { environmentLineText, visibleText } from "../../agent";
import { roleText } from "../../hierarchy";
import {
  agentCountOf,
  type AgentBody,
  type Template,
  type TeamBody,
} from "./template";

const NONE_TEXT = "none";
const PROJECT_FOLDER_TEXT = "the project folder";

/** One label and its value, as the preview and the create dialog list them. */
export interface TextRow {
  label: string;
  value: string;
}

/** One agent definition's rows, or one titled group of them. */
export interface TextGroup {
  title: string;
  rows: TextRow[];
}

function listText(lines: string[], emptyText: string): string {
  return lines.length === 0 ? emptyText : lines.join("\n");
}

/**
 * The rows of one agent definition in the reading order of the spec: name
 * pattern, role, command, folder, flags, environment variable names, startup
 * prompt. Every text goes through `visibleText`, so a control character from an
 * imported file is shown as a symbol and never applied (FR-008).
 */
export function agentRows(body: AgentBody): TextRow[] {
  return [
    { label: "Name pattern", value: visibleText(body.name_pattern) },
    { label: "Role", value: roleText(body.role) },
    { label: "Command", value: visibleText(body.command) },
    {
      label: "Working folder",
      value:
        body.working_folder === null
          ? PROJECT_FOLDER_TEXT
          : visibleText(body.working_folder),
    },
    {
      label: "Flags",
      value: listText(body.additions.flags.map(visibleText), NONE_TEXT),
    },
    {
      label: "Environment variables",
      value: listText(body.additions.env.map(environmentLineText), NONE_TEXT),
    },
    {
      label: "Startup prompt",
      value:
        body.additions.startup_prompt === null
          ? NONE_TEXT
          : visibleText(body.additions.startup_prompt),
    },
  ];
}

/** The groups of a body: one for an agent, the lead and each member entry for a team. */
export function bodyGroups(body: AgentBody | TeamBody): TextGroup[] {
  if (!("members" in body)) return [{ title: "Agent", rows: agentRows(body) }];
  return [
    { title: "Lead", rows: agentRows(body.lead) },
    ...body.members.map((member, index) => ({
      title: `Members, entry ${index + 1}, ${member.count} ${member.count === 1 ? "agent" : "agents"}`,
      rows: agentRows(member.entry),
    })),
  ];
}

/** "agent" or "team", as a word for a list or a select. */
export function kindText(kind: Template["kind"]): string {
  return kind === "agent" ? "Agent template" : "Team template";
}

/** One line under a template's name: its command or its size. */
export function templateSummaryText(template: Template): string {
  if (template.kind === "agent") {
    return `${visibleText(template.body.command)}, ${roleText(template.body.role)}`;
  }
  return `${agentCountOf(template.body)} agents: a lead and ${template.body.members.length === 1 ? "one member entry" : `${template.body.members.length} member entries`}`;
}
