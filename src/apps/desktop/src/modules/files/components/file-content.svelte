<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    BINARY_FILE_TEXT,
    EMPTY_FILE_TEXT,
    EMPTY_FOLDER_TEXT,
    ROOT_PATH,
    SECRET_FILE_TEXT,
    tooLargeText,
    visibleLine,
  } from "../models/file-view";
  import type { FileViewState } from "../state/file-view.svelte";
  import PartMessage from "./part-message.svelte";

  interface Props {
    fileView: FileViewState;
  }

  let { fileView }: Props = $props();

  const files = $derived(fileView.view.files);
  const viewed = $derived(files.viewed);
  const folder = $derived(files.folders[files.selectedPath ?? ROOT_PATH]);
</script>

<div class="flex min-h-0 min-w-0 flex-col gap-3">
  {#if viewed !== null}
    <h3 class="font-mono text-sm break-all text-neutral-900">
      {visibleLine(viewed.path)}
    </h3>
    {#if viewed.phase === "loading"}
      <p role="status" class="text-neutral-700">Loading the file.</p>
    {:else if viewed.phase === "failed"}
      <div class="flex flex-col items-start gap-3">
        <FailureMessage
          lead="The file did not open."
          message={viewed.message ?? ""}
        />
        <Button variant="secondary" onclick={fileView.refresh}>Refresh</Button>
      </div>
    {:else if viewed.phase === "notice"}
      <p class="text-neutral-900">{viewed.message}</p>
    {:else if viewed.read?.state === "secret"}
      <p class="text-neutral-900">{SECRET_FILE_TEXT}</p>
      <div><Button onclick={fileView.revealFile}>Reveal</Button></div>
    {:else if viewed.read?.state === "too_large"}
      <p class="text-neutral-900">
        {tooLargeText("This file", viewed.read.size)}
      </p>
    {:else if viewed.read?.state === "binary"}
      <p class="text-neutral-900">{BINARY_FILE_TEXT}</p>
    {:else if viewed.lines.length === 0}
      <p role="status" class="text-neutral-700">{EMPTY_FILE_TEXT}</p>
    {:else}
      <!-- Focusable, so the keyboard can scroll it. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        role="region"
        aria-label="File content"
        tabindex="0"
        class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-400 bg-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
      >
        <table class="border-collapse font-mono text-sm text-neutral-900">
          <tbody>
            {#each viewed.lines as line, index (index)}
              <tr>
                <th
                  scope="row"
                  class="px-3 text-right align-top font-normal text-neutral-700 select-none"
                >
                  {index + 1}
                </th>
                <td class="pr-3 whitespace-pre">{line}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {:else if folder !== undefined}
    <PartMessage
      phase={folder.phase}
      failure={folder.failure}
      loadingText="Loading the folder."
      failureLead="The folder did not load."
      emptyText={EMPTY_FOLDER_TEXT}
      onrefresh={fileView.refresh}
    />
    {#if folder.phase === "ready"}
      <p class="text-neutral-700">Choose a file to read it.</p>
      {#if folder.more > 0 && files.selectedPath !== null}
        <p class="text-neutral-700">{folder.more} more not shown</p>
      {/if}
    {/if}
  {/if}
</div>
