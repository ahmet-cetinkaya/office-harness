// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  CoreRequestError,
  type CoreClient,
  type ExportOutcome,
  type ServerEvent,
} from "../../../core/api";
import {
  createTemplateController,
  EXPORT_CANCELLED_TEXT,
  EXPORTED_TEXT,
} from "./template-controller";

const BODY = {
  body_version: 1,
  name_pattern: "a-{n}",
  command: "claude",
  working_folder: null,
  additions: {
    flags: [],
    env: [{ name: "KEY", value: null }],
    startup_prompt: null,
  },
  role: null,
  own_worktree: false,
};
const TEMPLATE = {
  id: "t1",
  name: "Reviewer",
  kind: "agent",
  body: BODY,
  updated_at: 1,
};
const PREVIEW = [
  { name: "Reviewer", save_name: "Reviewer (2)", kind: "agent", body: BODY },
];

interface Script {
  answers?: Record<string, unknown>;
  failures?: Record<string, CoreRequestError>;
  importText?: string | null;
  exportOutcome?: ExportOutcome;
  exportFailure?: CoreRequestError;
}

function fakeClient(script: Script = {}) {
  const requests: string[] = [];
  const written: string[] = [];
  let onEvent: (event: ServerEvent) => void = () => {};
  const client: CoreClient = {
    watchConnection: async () => {},
    watchEvents: async (callback) => {
      onEvent = callback;
    },
    watchTerminalFrames: async () => {},
    sendRequest: async (kind) => {
      requests.push(kind);
      const failure = script.failures?.[kind];
      if (failure !== undefined) throw failure;
      return script.answers?.[kind] ?? {};
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
    exportTemplatesFile: async (contents) => {
      if (script.exportFailure !== undefined) throw script.exportFailure;
      written.push(contents);
      return script.exportOutcome ?? "saved";
    },
    importTemplatesFile: async () => script.importText ?? null,
  };
  return {
    client,
    requests,
    written,
    push: (event: ServerEvent) => onEvent(event),
  };
}

function observe(client: CoreClient) {
  const controller = createTemplateController(client);
  let last = controller.snapshot;
  controller.subscribe((next) => (last = next));
  void controller.watch();
  return { controller, state: () => last };
}

describe("template controller", () => {
  test("load reads template.list", async () => {
    const { client } = fakeClient({
      answers: { "template.list": { templates: [TEMPLATE] } },
    });
    const { controller, state } = observe(client);

    await controller.load();

    expect(state().phase).toBe("ready");
    expect(state().templates.map((template) => template.name)).toEqual([
      "Reviewer",
    ]);
  });

  test("a failed load shows the reason", async () => {
    const { client } = fakeClient({
      failures: {
        "template.list": new CoreRequestError("internal", "storage failed"),
      },
    });
    const { controller, state } = observe(client);

    await controller.load();

    expect(state().phase).toBe("failed");
    expect(state().failure).toBe("storage failed");
  });

  test("a template.changed event replaces the list", async () => {
    const { client, push } = fakeClient();
    const { state } = observe(client);

    push({ type: "template.changed", templates: [TEMPLATE] });

    expect(state().templates).toHaveLength(1);
  });

  test("a save returns the server's refusal text", async () => {
    const { client } = fakeClient({
      failures: {
        "template.save": new CoreRequestError(
          "conflict",
          'template name "A" is already used',
        ),
      },
    });
    const { controller } = observe(client);

    const failure = await controller.save({
      name: "A",
      kind: "agent",
      body: readBody(),
    });

    expect(failure).toBe('template name "A" is already used');
  });

  test("an import shows the preview and saves nothing until confirm", async () => {
    const { client, requests } = fakeClient({
      importText: "file text",
      answers: { "template.import.preview": { templates: PREVIEW } },
    });
    const { controller, state } = observe(client);

    await controller.beginImport();

    expect(state().importPreview?.entries[0].save_name).toBe("Reviewer (2)");
    expect(requests).toEqual(["template.import.preview"]);
  });

  test("a cancel after the preview saves nothing", async () => {
    const { client, requests } = fakeClient({
      importText: "file text",
      answers: { "template.import.preview": { templates: PREVIEW } },
    });
    const { controller, state } = observe(client);
    await controller.beginImport();

    controller.cancelImport();

    expect(state().importPreview).toBeNull();
    expect(requests).not.toContain("template.import.confirm");
  });

  test("a confirm saves the previewed text and closes the preview", async () => {
    const { client, requests } = fakeClient({
      importText: "file text",
      answers: {
        "template.import.preview": { templates: PREVIEW },
        "template.import.confirm": { templates: [TEMPLATE] },
      },
    });
    const { controller, state } = observe(client);
    await controller.beginImport();

    await controller.confirmImport();

    expect(requests).toContain("template.import.confirm");
    expect(state().importPreview).toBeNull();
    expect(state().templates).toHaveLength(1);
  });

  test("a refused file shows its reason and no preview", async () => {
    const { client } = fakeClient({
      importText: "bad",
      failures: {
        "template.import.preview": new CoreRequestError(
          "invalid",
          "the file version 2 is not supported; this build reads version 1",
        ),
      },
    });
    const { controller, state } = observe(client);

    await controller.beginImport();

    expect(state().importPreview).toBeNull();
    expect(state().importFailure).toContain("version 2");
  });

  test("a closed open dialog does nothing", async () => {
    const { client, requests } = fakeClient({ importText: null });
    const { controller, state } = observe(client);

    await controller.beginImport();

    expect(requests).toEqual([]);
    expect(state().importFailure).toBeNull();
    expect(state().isBusy).toBe(false);
  });

  test("an unreadable file shows the core's reason", async () => {
    const { client } = fakeClient();
    client.importTemplatesFile = async () => {
      throw new CoreRequestError(
        "template_file_failed",
        "the file was not read: it is larger than 256 KiB",
      );
    };
    const { controller, state } = observe(client);

    await controller.beginImport();

    expect(state().importFailure).toContain("256 KiB");
  });

  test("an export writes the server's text to the picked file", async () => {
    const { client, written } = fakeClient({
      answers: { "template.export": { text: "exported text" } },
    });
    const { controller, state } = observe(client);

    await controller.exportTemplates();

    expect(written).toEqual(["exported text"]);
    expect(state().exportResult).toBe(EXPORTED_TEXT);
  });

  test("a closed save dialog says that no file was written", async () => {
    const { client } = fakeClient({ exportOutcome: "cancelled" });
    const { controller, state } = observe(client);

    await controller.exportTemplates();

    expect(state().exportResult).toBe(EXPORT_CANCELLED_TEXT);
  });

  test("a failed write shows the reason", async () => {
    const { client } = fakeClient({
      exportFailure: new CoreRequestError(
        "template_file_failed",
        "the file was not written: disk full",
      ),
    });
    const { controller, state } = observe(client);

    await controller.exportTemplates();

    expect(state().exportFailure).toBe("the file was not written: disk full");
    expect(state().exportResult).toBeNull();
  });
});

function readBody() {
  return {
    name_pattern: "a",
    command: "claude",
    working_folder: null,
    additions: { flags: [], env: [], startup_prompt: null },
    role: null,
    own_worktree: false,
  };
}
