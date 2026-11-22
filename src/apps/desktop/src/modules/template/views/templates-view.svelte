<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { ConfirmDialog } from "../../agent";
  import ExportNoticeDialog from "../components/export-notice-dialog.svelte";
  import ImportPreviewDialog from "../components/import-preview-dialog.svelte";
  import TemplateEditorDialog from "../components/template-editor-dialog.svelte";
  import type { Template } from "../models/template";
  import type { FormStart } from "../models/template-form";
  import { kindText, templateSummaryText } from "../models/template-text";
  import {
    INITIAL_TEMPLATES,
    type FailureText,
    type TemplateController,
  } from "../state/template-controller";

  interface Props {
    controller: TemplateController;
    /** Leaves the view for the panel. */
    onback: () => void;
  }

  let { controller, onback }: Props = $props();

  // The controller's snapshot is a plain value, so the view subscribes and
  // keeps the last seen one in runes state; the effect's cleanup stops the
  // subscription when the view unmounts.
  let snapshot = $state(INITIAL_TEMPLATES);
  $effect(() => controller.subscribe((next) => (snapshot = next)));

  const HEADING_ID = "templates-heading";

  /** The editor is closed (undefined), new (null), or on a template. */
  let editorStart = $state<FormStart | undefined>(undefined);
  let templateToDelete = $state<Template | null>(null);
  let deleteFailure = $state<FailureText>(null);
  let isExportNoticeOpen = $state(false);

  function openEditor(template: Template | null): void {
    editorStart = template === null ? null : { template };
  }

  async function deleteTemplate(): Promise<void> {
    if (templateToDelete === null) return;
    const { id } = templateToDelete;
    templateToDelete = null;
    deleteFailure = await controller.delete(id);
  }

  function exportTemplates(): void {
    isExportNoticeOpen = false;
    void controller.exportTemplates();
  }
</script>

<section aria-labelledby={HEADING_ID} class="flex flex-col gap-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h2
      id={HEADING_ID}
      tabindex="-1"
      class="text-xl font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
    >
      Templates
    </h2>
    <Button variant="secondary" onclick={onback}>Back to the panel</Button>
  </div>
  <p class="text-neutral-700">
    A template is a copy of an agent or team definition. Editing or deleting one
    changes no agent that was created from it.
  </p>

  <div class="flex flex-wrap gap-3">
    <Button onclick={() => openEditor(null)}>New template</Button>
    <Button
      variant="secondary"
      disabled={snapshot.isBusy}
      onclick={() => void controller.beginImport()}
    >
      Import templates
    </Button>
    <Button
      variant="secondary"
      disabled={snapshot.isBusy || snapshot.templates.length === 0}
      onclick={() => (isExportNoticeOpen = true)}
    >
      Export templates
    </Button>
  </div>

  {#if snapshot.exportResult !== null}
    <p role="status" class="text-sm text-neutral-900">
      {snapshot.exportResult}
    </p>
  {/if}
  {#if snapshot.exportFailure !== null}
    <FailureMessage
      lead="The templates were not exported."
      message={snapshot.exportFailure}
    />
  {/if}
  {#if snapshot.importFailure !== null && snapshot.importPreview === null}
    <FailureMessage
      lead="The file was not imported."
      message={snapshot.importFailure}
    />
  {/if}
  {#if deleteFailure !== null}
    <FailureMessage
      lead="The template was not deleted."
      message={deleteFailure}
    />
  {/if}

  {#if snapshot.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the templates.</p>
  {:else if snapshot.phase === "failed"}
    <FailureMessage
      lead="The templates did not load."
      message={snapshot.failure ?? ""}
    />
    <div>
      <Button variant="secondary" onclick={() => void controller.load()}>
        Try again
      </Button>
    </div>
  {:else if snapshot.templates.length === 0}
    <p class="text-neutral-700">
      No template exists yet. Choose New template to build one, or Import
      templates to read a file.
    </p>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each snapshot.templates as template (template.id)}
        <li
          class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-neutral-400 p-3"
        >
          <div class="flex flex-col gap-1">
            <h3 class="text-base font-semibold text-neutral-900">
              {template.name}
            </h3>
            <p class="text-sm text-neutral-700">
              {kindText(template.kind)}: {templateSummaryText(template)}
            </p>
          </div>
          <div class="flex flex-wrap gap-3">
            <Button variant="secondary" onclick={() => openEditor(template)}>
              Edit {template.name}
            </Button>
            <Button
              variant="secondary"
              onclick={() => (templateToDelete = template)}
            >
              Delete {template.name}
            </Button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if editorStart !== undefined}
  <TemplateEditorDialog
    start={editorStart}
    isBusy={snapshot.isBusy}
    onsave={controller.save}
    onclose={() => (editorStart = undefined)}
  />
{/if}
{#if templateToDelete !== null}
  <ConfirmDialog
    text={{
      title: `Delete template ${templateToDelete.name}?`,
      body: "Agents and teams that were created from it stay as they are.",
      confirmLabel: "Delete template",
      isDestructive: true,
    }}
    onconfirm={() => void deleteTemplate()}
    oncancel={() => (templateToDelete = null)}
  />
{/if}
{#if isExportNoticeOpen}
  <ExportNoticeDialog
    onconfirm={exportTemplates}
    oncancel={() => (isExportNoticeOpen = false)}
  />
{/if}
{#if snapshot.importPreview !== null}
  <ImportPreviewDialog
    entries={snapshot.importPreview.entries}
    failure={snapshot.importFailure}
    isBusy={snapshot.isBusy}
    onconfirm={() => void controller.confirmImport()}
    oncancel={controller.cancelImport}
  />
{/if}
