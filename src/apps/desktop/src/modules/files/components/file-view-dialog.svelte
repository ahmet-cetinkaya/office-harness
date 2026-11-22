<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import type { CoreClient } from "../../../core/api";
  import { formatLoadedAt, visibleLine } from "../models/file-view";
  import { createFileViewService } from "../services/file-view-service";
  import {
    createFileViewState,
    type FileViewTab,
  } from "../state/file-view.svelte";
  import ChangeList from "./change-list.svelte";
  import FileContent from "./file-content.svelte";
  import FileTree from "./file-tree.svelte";
  import HistoryList from "./history-list.svelte";

  interface Props {
    client: CoreClient;
    agentId: string;
    agentName: string;
    onclose: () => void;
  }

  const TABS: { id: FileViewTab; label: string }[] = [
    { id: "files", label: "Files" },
    { id: "changes", label: "Changes" },
    { id: "history", label: "History" },
  ];

  let { client, agentId, agentName, onclose }: Props = $props();

  const uid = $props.id();
  // The dialog mounts for one open of one agent, so both are first values.
  // svelte-ignore state_referenced_locally
  const fileView = createFileViewState(createFileViewService(client), agentId);
  const view = fileView.view;
  const readAt = $derived(view[view.tab].readAt);
  let tabList = $state<HTMLElement | null>(null);

  onMount(() => fileView.showTab("files"));

  function moveTab(event: KeyboardEvent, index: number): void {
    const last = TABS.length - 1;
    const targets: Record<string, number> = {
      ArrowRight: index === last ? 0 : index + 1,
      ArrowLeft: index === 0 ? last : index - 1,
      Home: 0,
      End: last,
    };
    const target = targets[event.key];
    if (target === undefined) return;
    event.preventDefault();
    fileView.showTab(TABS[target].id);
    tabList?.querySelectorAll<HTMLElement>('[role="tab"]')[target]?.focus();
  }
</script>

<Dialog title={`Files of ${agentName}`} {onclose} size="wide">
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <div
      class="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm text-neutral-900"
    >
      {#if view.root !== null}
        <p class="break-all">
          <span class="font-medium">Folder</span>
          {visibleLine(view.root)}
        </p>
      {/if}
      {#if readAt !== null}
        <p>Loaded at {formatLoadedAt(readAt)}</p>
      {/if}
      <Button variant="secondary" onclick={fileView.refresh}>Refresh</Button>
    </div>

    <div
      bind:this={tabList}
      role="tablist"
      aria-label="File view parts"
      class="flex gap-1 border-b border-neutral-400"
    >
      {#each TABS as tab, index (tab.id)}
        <button
          type="button"
          role="tab"
          id={`${uid}-tab-${tab.id}`}
          aria-selected={view.tab === tab.id}
          aria-controls={`${uid}-panel`}
          tabindex={view.tab === tab.id ? 0 : -1}
          class="min-h-11 rounded-t-md px-4 text-sm font-medium text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-700 aria-selected:border-b-2 aria-selected:border-blue-700 aria-selected:text-blue-800"
          onclick={() => fileView.showTab(tab.id)}
          onkeydown={(event) => moveTab(event, index)}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div
      role="tabpanel"
      id={`${uid}-panel`}
      aria-labelledby={`${uid}-tab-${view.tab}`}
      class="flex min-h-0 flex-1 flex-col"
    >
      {#if view.tab === "files"}
        <div
          class="grid min-h-0 flex-1 grid-rows-[minmax(8rem,40%)_minmax(0,1fr)] gap-4 md:grid-cols-[minmax(14rem,22rem)_minmax(0,1fr)] md:grid-rows-1"
        >
          <FileTree {fileView} />
          <FileContent {fileView} />
        </div>
      {:else if view.tab === "changes"}
        <ChangeList {fileView} />
      {:else}
        <HistoryList {fileView} />
      {/if}
    </div>
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  </div>
</Dialog>
