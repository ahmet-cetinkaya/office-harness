// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Additions, EnvironmentEntry } from "../../agent";

/** The roles an agent template may carry (0009 D2). */
export type TemplateRole = "ceo" | "lead" | "member";

/** An agent definition: an agent template, or one entry of a team template. */
export interface AgentBody {
  name_pattern: string;
  command: string;
  working_folder: string | null;
  additions: Additions;
  role: TemplateRole | null;
}

export interface MemberEntry {
  entry: AgentBody;
  count: number;
}

export interface TeamBody {
  lead: AgentBody;
  members: MemberEntry[];
}

export type TemplateKind = "agent" | "team";

export type Template =
  | {
      id: string;
      name: string;
      kind: "agent";
      body: AgentBody;
      updated_at: number;
    }
  | {
      id: string;
      name: string;
      kind: "team";
      body: TeamBody;
      updated_at: number;
    };

/** One template of an import preview, with the name it will be saved under. */
export type ImportPreviewEntry =
  | { name: string; save_name: string; kind: "agent"; body: AgentBody }
  | { name: string; save_name: string; kind: "team"; body: TeamBody };

/** The `body_version` that every body on the wire carries (data model). */
export const BODY_VERSION = 1;

/** The most agents one team template may create (FR-010). */
export const TEAM_AGENT_LIMIT = 50;

type Fields = Record<string, unknown>;

function fieldsOf(value: unknown): Fields {
  return typeof value === "object" && value !== null ? (value as Fields) : {};
}

function textOf(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function textOrNull(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function listOf(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function readEnvironmentEntry(value: unknown): EnvironmentEntry {
  const fields = fieldsOf(value);
  return { name: textOf(fields.name), value: textOrNull(fields.value) };
}

function readRole(value: unknown): TemplateRole | null {
  return value === "ceo" || value === "lead" || value === "member"
    ? value
    : null;
}

export function readAgentBody(value: unknown): AgentBody {
  const fields = fieldsOf(value);
  const additions = fieldsOf(fields.additions);
  return {
    name_pattern: textOf(fields.name_pattern),
    command: textOf(fields.command),
    working_folder: textOrNull(fields.working_folder),
    additions: {
      flags: listOf(additions.flags).map(textOf),
      env: listOf(additions.env).map(readEnvironmentEntry),
      startup_prompt: textOrNull(additions.startup_prompt),
    },
    role: readRole(fields.role),
  };
}

export function readTeamBody(value: unknown): TeamBody {
  const fields = fieldsOf(value);
  return {
    lead: readAgentBody(fields.lead),
    members: listOf(fields.members).map((member) => {
      const memberFields = fieldsOf(member);
      return {
        entry: readAgentBody(memberFields.entry),
        count: Number(memberFields.count ?? 0),
      };
    }),
  };
}

function isKind(value: unknown): value is TemplateKind {
  return value === "agent" || value === "team";
}

/** A template as the server sends it, or null for an answer of another shape. */
export function readTemplate(value: unknown): Template | null {
  const fields = fieldsOf(value);
  if (typeof fields.id !== "string" || !isKind(fields.kind)) return null;
  const common = {
    id: fields.id,
    name: textOf(fields.name),
    updated_at: Number(fields.updated_at ?? 0),
  };
  return fields.kind === "agent"
    ? { ...common, kind: "agent", body: readAgentBody(fields.body) }
    : { ...common, kind: "team", body: readTeamBody(fields.body) };
}

export function readTemplates(value: unknown): Template[] {
  return listOf(value).flatMap((entry) => readTemplate(entry) ?? []);
}

export function readImportPreview(value: unknown): ImportPreviewEntry[] {
  return listOf(value).flatMap((entry): ImportPreviewEntry[] => {
    const fields = fieldsOf(entry);
    if (!isKind(fields.kind)) return [];
    const common = {
      name: textOf(fields.name),
      save_name: textOf(fields.save_name),
    };
    return [
      fields.kind === "agent"
        ? { ...common, kind: "agent", body: readAgentBody(fields.body) }
        : { ...common, kind: "team", body: readTeamBody(fields.body) },
    ];
  });
}

/** The agents one creation from the body makes (FR-010). */
export function agentCountOf(body: AgentBody | TeamBody): number {
  if (!("members" in body)) return 1;
  return 1 + body.members.reduce((total, member) => total + member.count, 0);
}
