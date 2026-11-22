// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ExportOutcome,
  type ServerEvent,
} from "../../../core/api";
import {
  readTemplates,
  type AgentBody,
  type ImportPreviewEntry,
  type Template,
} from "../models/template";
import {
  createTemplateService,
  type AgentCreation,
  type TeamCreation,
  type TemplateToSave,
} from "../services/template-service";

/** The import the user is looking at: the file text and what it holds. */
export interface ImportPreview {
  text: string;
  entries: ImportPreviewEntry[];
}

export interface TemplateSnapshot {
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  templates: Template[];
  /** The preview that waits for confirm or cancel, or null. */
  importPreview: ImportPreview | null;
  /** Why the last file could not be previewed or imported, or null. */
  importFailure: string | null;
  /** What the last export did, as text for a status line, or null. */
  exportResult: string | null;
  /** The reason the last export failed, or null. */
  exportFailure: string | null;
  /** True while an import or export request is open. */
  isBusy: boolean;
}

export const INITIAL_TEMPLATES: TemplateSnapshot = {
  phase: "loading",
  failure: null,
  templates: [],
  importPreview: null,
  importFailure: null,
  exportResult: null,
  exportFailure: null,
  isBusy: false,
};

const EVENT_TEMPLATE_CHANGED = "template.changed";
const EVENT_RESYNC = "client.resync";
const REQUEST_FAILED = "The request failed.";

export const EXPORTED_TEXT = "The templates were exported.";
export const EXPORT_CANCELLED_TEXT =
  "The export was cancelled. No file was written.";

/** A request's refusal text, or null when the request went through. */
export type FailureText = string | null;

/**
 * Owns the template list, the import in progress, and the export. It reads
 * `template.list` on load and replaces its copy on each `template.changed`.
 */
export function createTemplateController(client: CoreClient) {
  const service = createTemplateService(client);
  let snapshot: TemplateSnapshot = INITIAL_TEMPLATES;
  const listeners = new Set<(snapshot: TemplateSnapshot) => void>();
  // An event is newer than any read that is still out, so it ends that read's
  // right to publish.
  let newestRead = 0;

  function publish(changes: Partial<TemplateSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function submit(work: () => Promise<unknown>): Promise<FailureText> {
    try {
      await work();
      return null;
    } catch (rejection) {
      return describe(rejection);
    }
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const templates = await service.list();
      if (readId !== newestRead) return;
      publish({ phase: "ready", failure: null, templates });
    } catch (rejection) {
      if (readId !== newestRead) return;
      publish({ phase: "failed", failure: describe(rejection) });
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_RESYNC) {
      void load();
      return;
    }
    if (event.type !== EVENT_TEMPLATE_CHANGED) return;
    newestRead += 1;
    publish({
      phase: "ready",
      failure: null,
      templates: readTemplates(event.templates),
    });
  }

  /** Reads the file text and previews it. Nothing is saved (FR-008). */
  async function beginImport(): Promise<void> {
    publish({ isBusy: true, importFailure: null, exportResult: null });
    try {
      const text = await service.readImportFile();
      if (text === null) {
        publish({ isBusy: false });
        return;
      }
      const entries = await service.importPreview(text);
      publish({ isBusy: false, importPreview: { text, entries } });
    } catch (rejection) {
      publish({ isBusy: false, importFailure: describe(rejection) });
    }
  }

  /** Saves the previewed templates. A refusal stays in the preview dialog. */
  async function confirmImport(): Promise<void> {
    const { importPreview } = snapshot;
    if (importPreview === null || snapshot.isBusy) return;
    publish({ isBusy: true, importFailure: null });
    try {
      const templates = await service.importConfirm(importPreview.text);
      publish({ isBusy: false, importPreview: null, templates });
    } catch (rejection) {
      publish({ isBusy: false, importFailure: describe(rejection) });
    }
  }

  function cancelImport(): void {
    publish({ importPreview: null, importFailure: null });
  }

  function exportResultText(outcome: ExportOutcome): string {
    return outcome === "saved" ? EXPORTED_TEXT : EXPORT_CANCELLED_TEXT;
  }

  /** Asks the server for the file text, then writes it to the picked file. */
  async function exportTemplates(): Promise<void> {
    if (snapshot.isBusy) return;
    publish({ isBusy: true, exportResult: null, exportFailure: null });
    try {
      const outcome = await service.writeExportFile(await service.exportText());
      publish({ isBusy: false, exportResult: exportResultText(outcome) });
    } catch (rejection) {
      publish({ isBusy: false, exportFailure: describe(rejection) });
    }
  }

  return {
    /** The snapshot of the last change. */
    get snapshot(): TemplateSnapshot {
      return snapshot;
    },

    /**
     * Calls back on each change, starting with the current snapshot. Returns
     * the stop function, as Svelte effects expect.
     */
    subscribe(listener: (snapshot: TemplateSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch (rejection) {
        publish({ phase: "failed", failure: describe(rejection) });
      }
    },

    /** Loads the list. Call it each time the connection becomes ready. */
    load,

    save: (template: TemplateToSave): Promise<FailureText> =>
      submit(() => service.save(template)),

    delete: (templateId: string): Promise<FailureText> =>
      submit(() => service.delete(templateId)),

    /** The unsaved body of one agent, or the server's reason it cannot be read. */
    fromAgent: (agentId: string): Promise<AgentBody> =>
      service.fromAgent(agentId),

    describeFailure: describe,

    createAgent: (creation: AgentCreation): Promise<FailureText> =>
      submit(() => service.createAgent(creation)),

    createTeam: (creation: TeamCreation): Promise<FailureText> =>
      submit(() => service.createTeam(creation)),

    beginImport,
    confirmImport,
    cancelImport,
    exportTemplates,
  };
}

export type TemplateController = ReturnType<typeof createTemplateController>;
