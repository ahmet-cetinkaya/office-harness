<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import {
    changeKindText,
    NO_CHANGES_TEXT,
    NOT_REPOSITORY_TEXT,
    visibleLine,
  } from "../models/file-view";
  import type { FileViewState } from "../state/file-view.svelte";
  import DiffView from "./diff-view.svelte";
  import PartMessage from "./part-message.svelte";

  interface Props {
    fileView: FileViewState;
  }

  let { fileView }: Props = $props();

  const changes = $derived(fileView.view.changes);
</script>

{#if changes.phase === "ready"}
  <div
    class="grid min-h-0 flex-1 grid-rows-[minmax(8rem,40%)_minmax(0,1fr)] gap-4 md:grid-cols-[minmax(14rem,22rem)_minmax(0,1fr)] md:grid-rows-1"
  >
    <div class="flex min-h-0 flex-col gap-2">
      <ul
        aria-label="Changed files"
        class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-400 bg-white"
      >
        {#each changes.files as file (file.path)}
          <li>
            <button
              type="button"
              aria-current={changes.selectedPath === file.path}
              class="flex min-h-11 w-full flex-col items-start justify-center px-3 py-1 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-700 aria-[current=true]:bg-blue-50"
              onclick={() => fileView.openChange(file)}
            >
              <span class="break-all">
                <span class="font-medium">{changeKindText(file.kind)}</span>
                {visibleLine(file.path)}
              </span>
              {#if file.old_path !== null}
                <span class="text-xs break-all text-neutral-700">
                  from {visibleLine(file.old_path)}
                </span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
      {#if changes.more > 0}
        <p class="text-sm text-neutral-700">{changes.more} more not shown</p>
      {/if}
    </div>
    <DiffView
      viewed={changes.viewedDiff}
      idleText="Choose a changed file to see its diff."
      onreveal={fileView.revealDiff}
    />
  </div>
{:else}
  <PartMessage
    phase={changes.phase}
    failure={changes.failure}
    loadingText="Loading the changes."
    failureLead="The changes did not load."
    emptyText={changes.repository ? NO_CHANGES_TEXT : NOT_REPOSITORY_TEXT}
    onrefresh={fileView.refresh}
  />
{/if}
