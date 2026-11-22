// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient, ExportOutcome } from "../../../core/api";
import {
  BODY_VERSION,
  readAgentBody,
  readImportPreview,
  readTemplate,
  readTemplates,
  type AgentBody,
  type ImportPreviewEntry,
  type Template,
  type TemplateKind,
  type TeamBody,
} from "../models/template";

/** The fields of `template.save` (contract 0009). */
export interface TemplateToSave {
  /** Absent creates a template; present replaces that template. */
  id?: string;
  name: string;
  kind: TemplateKind;
  body: AgentBody | TeamBody;
}

/** Where an agent of a role goes: its parent and its team (contract 0009). */
export interface DirectPlace {
  parent_agent_id: string;
  team_id: string;
}

export interface AgentCreation {
  templateId: string;
  place: DirectPlace | null;
  projectId: string | null;
}

export interface TeamCreation {
  templateId: string;
  teamName: string;
  parentAgentId: string;
  projectId: string | null;
}

function answerFields(answer: unknown): Record<string, unknown> {
  return typeof answer === "object" && answer !== null
    ? (answer as Record<string, unknown>)
    : {};
}

/** The typed face of the `template.*` requests and the two file commands. */
export function createTemplateService(client: CoreClient) {
  const send = (kind: string, fields?: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    list: async (): Promise<Template[]> =>
      readTemplates(answerFields(await send("template.list")).templates),

    /** Saves a template. The body carries the version that the server expects. */
    save: async (template: TemplateToSave): Promise<Template | null> => {
      const answer = await send("template.save", {
        ...(template.id === undefined ? {} : { template_id: template.id }),
        name: template.name,
        kind: template.kind,
        body: { body_version: BODY_VERSION, ...template.body },
      });
      return readTemplate(answerFields(answer).template);
    },

    delete: async (templateId: string): Promise<void> => {
      await send("template.delete", { template_id: templateId });
    },

    /** An unsaved agent body from one agent, for the editor to save. */
    fromAgent: async (agentId: string): Promise<AgentBody> =>
      readAgentBody(
        answerFields(await send("template.from_agent", { agent_id: agentId }))
          .body,
      ),

    createAgent: async (creation: AgentCreation): Promise<void> => {
      await send("template.create_agent", {
        template_id: creation.templateId,
        ...(creation.place === null ? {} : { place: creation.place }),
        ...(creation.projectId === null
          ? {}
          : { project_id: creation.projectId }),
      });
    },

    createTeam: async (creation: TeamCreation): Promise<void> => {
      await send("template.create_team", {
        template_id: creation.templateId,
        team_name: creation.teamName,
        parent_agent_id: creation.parentAgentId,
        ...(creation.projectId === null
          ? {}
          : { project_id: creation.projectId }),
      });
    },

    /** The file text of every template, with every environment value null. */
    exportText: async (): Promise<string> => {
      const { text } = answerFields(await send("template.export"));
      return typeof text === "string" ? text : "";
    },

    importPreview: async (text: string): Promise<ImportPreviewEntry[]> =>
      readImportPreview(
        answerFields(await send("template.import.preview", { text })).templates,
      ),

    importConfirm: async (text: string): Promise<Template[]> =>
      readTemplates(
        answerFields(await send("template.import.confirm", { text })).templates,
      ),

    writeExportFile: (text: string): Promise<ExportOutcome> =>
      client.exportTemplatesFile(text),

    readImportFile: (): Promise<string | null> => client.importTemplatesFile(),
  };
}

export type TemplateService = ReturnType<typeof createTemplateService>;
