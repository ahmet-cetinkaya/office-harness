// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  formatEnvironment,
  formatFlags,
  parseEnvironment,
  parseFlags,
  parseStartupPrompt,
} from "../../agent";
import type {
  AgentBody,
  TeamBody,
  Template,
  TemplateKind,
  TemplateRole,
} from "./template";

/** An agent definition as the editor holds it: every field as text. */
export interface AgentDraft {
  namePattern: string;
  command: string;
  workingFolder: string;
  flagsText: string;
  environmentText: string;
  promptText: string;
  /** Kept as saved; only a direct creation uses it (A4). */
  role: TemplateRole | null;
}

export interface MemberDraft {
  entry: AgentDraft;
  /** Text, so that a half-typed number does not vanish. */
  count: string;
}

export interface TeamDraft {
  lead: AgentDraft;
  members: MemberDraft[];
}

/** The defaults of the spec's A2 for a lead and a member entry. */
const LEAD_PATTERN = "{team}-lead";
const MEMBER_PATTERN = "{team}-member-{n}";
const DEFAULT_COUNT = "1";
const WHOLE_NUMBER = /^\d+$/;

export type DraftResult<Body> =
  { ok: true; body: Body } | { ok: false; message: string };

export function emptyAgentDraft(namePattern = ""): AgentDraft {
  return {
    namePattern,
    command: "",
    workingFolder: "",
    flagsText: "",
    environmentText: "",
    promptText: "",
    role: null,
  };
}

export function emptyTeamDraft(): TeamDraft {
  return {
    lead: emptyAgentDraft(LEAD_PATTERN),
    members: [newMemberDraft()],
  };
}

export function newMemberDraft(): MemberDraft {
  return { entry: emptyAgentDraft(MEMBER_PATTERN), count: DEFAULT_COUNT };
}

export function draftOfAgentBody(body: AgentBody): AgentDraft {
  return {
    namePattern: body.name_pattern,
    command: body.command,
    workingFolder: body.working_folder ?? "",
    flagsText: formatFlags(body.additions.flags),
    environmentText: formatEnvironment(body.additions.env),
    promptText: body.additions.startup_prompt ?? "",
    role: body.role,
  };
}

export function draftOfTeamBody(body: TeamBody): TeamDraft {
  return {
    lead: draftOfAgentBody(body.lead),
    members: body.members.map((member) => ({
      entry: draftOfAgentBody(member.entry),
      count: String(member.count),
    })),
  };
}

/** A field that is empty after trimming has no value, as the create form reads it. */
function folderOf(text: string): string | null {
  return text.trim() === "" ? null : text.trim();
}

/**
 * The body of one draft. Only the text that cannot be read at all is refused
 * here (an environment line without a name); the save rules are the server's,
 * so the editor cannot drift from them (D2).
 */
export function agentBodyOfDraft(
  draft: AgentDraft,
  where: string,
): DraftResult<AgentBody> {
  const environment = parseEnvironment(draft.environmentText);
  if (!environment.ok) {
    return { ok: false, message: `${where}: ${environment.message}` };
  }
  return {
    ok: true,
    body: {
      name_pattern: draft.namePattern.trim(),
      command: draft.command.trim(),
      working_folder: folderOf(draft.workingFolder),
      additions: {
        flags: parseFlags(draft.flagsText),
        env: environment.entries,
        startup_prompt: parseStartupPrompt(draft.promptText),
      },
      role: draft.role,
    },
  };
}

export function teamBodyOfDraft(draft: TeamDraft): DraftResult<TeamBody> {
  const lead = agentBodyOfDraft(draft.lead, "lead");
  if (!lead.ok) return lead;
  const members: TeamBody["members"] = [];
  for (const [index, member] of draft.members.entries()) {
    const where = `members[${index}]`;
    const entry = agentBodyOfDraft(member.entry, `${where}.entry`);
    if (!entry.ok) return entry;
    const count = member.count.trim();
    if (!WHOLE_NUMBER.test(count)) {
      return {
        ok: false,
        message: `${where}: the count is not a whole number`,
      };
    }
    members.push({ entry: entry.body, count: Number(count) });
  }
  return { ok: true, body: { lead: lead.body, members } };
}

export const KIND_OPTIONS: { value: TemplateKind; label: string }[] = [
  { value: "agent", label: "Agent template" },
  { value: "team", label: "Team template" },
];

export const ROLE_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "No role" },
  { value: "ceo", label: "CEO" },
  { value: "lead", label: "Team lead" },
  { value: "member", label: "Member" },
];

/** What the editor opens from: a template to edit, or an agent to save as one. */
export type FormStart =
  { template: Template } | { name: string; agent: AgentDraft } | null;

export interface FormValues {
  /** The template being edited, or null for a new one. */
  existing: Template | null;
  name: string;
  kind: TemplateKind;
  agent: AgentDraft;
  team: TeamDraft;
}

export function formValuesOf(start: FormStart): FormValues {
  if (start === null) {
    return {
      existing: null,
      name: "",
      kind: "agent",
      agent: emptyAgentDraft(),
      team: emptyTeamDraft(),
    };
  }
  if ("agent" in start) {
    return {
      existing: null,
      name: start.name,
      kind: "agent",
      agent: start.agent,
      team: emptyTeamDraft(),
    };
  }
  const { template } = start;
  return {
    existing: template,
    name: template.name,
    kind: template.kind,
    agent:
      template.kind === "agent"
        ? draftOfAgentBody(template.body)
        : emptyAgentDraft(),
    team:
      template.kind === "team"
        ? draftOfTeamBody(template.body)
        : emptyTeamDraft(),
  };
}
