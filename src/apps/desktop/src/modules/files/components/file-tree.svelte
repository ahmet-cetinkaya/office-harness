<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import {
    childPath,
    formatByteCount,
    isFolderEntry,
    ROOT_PATH,
    visibleLine,
    type FileEntry,
  } from "../models/file-view";
  import type { FileViewState } from "../state/file-view.svelte";

  interface Props {
    fileView: FileViewState;
  }

  interface Row {
    path: string;
    depth: number;
    entry: FileEntry;
    isFolder: boolean;
    isExpanded: boolean;
  }

  let { fileView }: Props = $props();

  const files = $derived(fileView.view.files);
  const rows = $derived(rowsOf(ROOT_PATH, 1));
  const rootMore = $derived(files.folders[ROOT_PATH]?.more ?? 0);
  let tree = $state<HTMLElement | null>(null);
  let focusedPath = $state<string | null>(null);
  const tabStopPath = $derived(
    rows.some((row) => row.path === focusedPath)
      ? focusedPath
      : (rows[0]?.path ?? null),
  );

  function rowsOf(folder: string, depth: number): Row[] {
    return (files.folders[folder]?.entries ?? []).flatMap((entry) => {
      const path = childPath(folder, entry.name);
      const isFolder = isFolderEntry(entry);
      const isExpanded = isFolder && files.expanded[path] === true;
      const row = { path, depth, entry, isFolder, isExpanded };
      return isExpanded ? [row, ...rowsOf(path, depth + 1)] : [row];
    });
  }

  function focusRow(index: number): void {
    const row = rows[index];
    if (row === undefined) return;
    focusedPath = row.path;
    tree?.querySelectorAll<HTMLElement>('[role="treeitem"]')[index]?.focus();
  }

  function indexOfParent(index: number): number {
    for (let candidate = index - 1; candidate >= 0; candidate -= 1) {
      if (rows[candidate].depth < rows[index].depth) return candidate;
    }
    return -1;
  }

  function moveRight(index: number): void {
    const row = rows[index];
    if (!row.isFolder) return;
    if (!row.isExpanded) fileView.setExpanded(row.path, true);
    else if (rows[index + 1]?.depth > row.depth) focusRow(index + 1);
  }

  function moveLeft(index: number): void {
    const row = rows[index];
    if (row.isExpanded) fileView.setExpanded(row.path, false);
    else focusRow(indexOfParent(index));
  }

  function open(index: number): void {
    focusedPath = rows[index].path;
    fileView.openEntry(rows[index].path, rows[index].entry);
  }

  function onkeydown(event: KeyboardEvent, index: number): void {
    const handlers: Record<string, () => void> = {
      ArrowDown: () => focusRow(Math.min(index + 1, rows.length - 1)),
      ArrowUp: () => focusRow(Math.max(index - 1, 0)),
      Home: () => focusRow(0),
      End: () => focusRow(rows.length - 1),
      ArrowRight: () => moveRight(index),
      ArrowLeft: () => moveLeft(index),
      Enter: () => open(index),
      " ": () => open(index),
    };
    const handler = handlers[event.key];
    if (handler === undefined) return;
    event.preventDefault();
    handler();
  }
</script>

{#if rows.length > 0}
  <div class="flex min-h-0 flex-col gap-2">
    <div
      bind:this={tree}
      role="tree"
      aria-label="Files"
      class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-400 bg-white py-1"
    >
      {#each rows as row, index (row.path)}
        <div
          role="treeitem"
          tabindex={row.path === tabStopPath ? 0 : -1}
          aria-level={row.depth}
          aria-selected={files.selectedPath === row.path}
          aria-expanded={row.isFolder ? row.isExpanded : undefined}
          data-path={row.path}
          style:padding-left={`${row.depth * 0.75}rem`}
          class="flex min-h-11 cursor-pointer items-center gap-2 pr-3 text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-700 aria-selected:bg-blue-50"
          onclick={() => open(index)}
          onkeydown={(event) => onkeydown(event, index)}
        >
          <span aria-hidden="true" class="w-3 shrink-0 text-neutral-700">
            {row.isFolder ? (row.isExpanded ? "▾" : "▸") : ""}
          </span>
          <span class="min-w-0 break-all">{visibleLine(row.entry.name)}</span>
          {#if row.entry.kind === "file"}
            <span class="sr-only">, file</span>
            {#if row.entry.size !== null}
              <span class="shrink-0 text-xs text-neutral-700">
                {formatByteCount(row.entry.size)}
              </span>
            {/if}
          {:else if row.entry.kind === "folder"}
            <span class="sr-only">, folder</span>
          {:else}
            <span class="shrink-0 text-xs text-neutral-700">
              {row.entry.kind}
            </span>
          {/if}
        </div>
      {/each}
    </div>
    {#if rootMore > 0}
      <p class="text-sm text-neutral-700">{rootMore} more not shown</p>
    {/if}
  </div>
{/if}
