<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import TreeRow from "./tree-row.svelte";
  import type { HierarchyNode } from "../models/hierarchy";

  interface Props {
    node: HierarchyNode;
    selectedAgentId: string | null;
    /** The row that holds focus, or null before the first focus. */
    activeId: string | null;
    /** The id that takes focus when nothing else does (roving tabindex). */
    firstId: string | null;
    onactivate: (node: HierarchyNode) => void;
    /** Keeps the active row in step when focus arrives from outside. */
    onfocus: (node: HierarchyNode) => void;
    /** The tree's key model, run on the row that has the event. */
    onkeydown: (event: KeyboardEvent) => void;
  }

  let {
    node,
    selectedAgentId,
    activeId,
    firstId,
    onactivate,
    onfocus,
    onkeydown,
  }: Props = $props();

  const base =
    "flex min-h-11 w-full items-center gap-2 rounded-md border border-neutral-400 bg-white px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";
  const isBranch = $derived(node.children.length > 0);
</script>

<!--
  One row of the hierarchy tree, recursive so that each expanded parent holds
  its children in a role="group". The accessible name carries title, role, and
  status as text; aria-level and aria-expanded carry depth and state, never
  color or indentation alone. Click and Enter activate the row; no control
  sits inside the treeitem. The tree container owns the key handling, which
  the key events reach by bubbling.
-->
<div
  role="treeitem"
  data-node-id={node.id}
  aria-label={node.name}
  aria-level={node.level}
  aria-expanded={isBranch ? node.expanded : undefined}
  aria-selected={node.agentId !== null
    ? node.agentId === selectedAgentId
    : undefined}
  tabindex={node.id === (activeId ?? firstId) ? 0 : -1}
  class={base}
  onclick={() => onactivate(node)}
  onfocus={() => onfocus(node)}
  {onkeydown}
>
  {#if isBranch}
    <span aria-hidden="true" class="w-3 shrink-0 text-neutral-700">
      {node.expanded ? "▾" : "▸"}
    </span>
  {/if}
  <div class="flex flex-col">
    <span class="font-medium">{node.title}</span>
    {#if node.meta !== ""}
      <span class="text-neutral-700">{node.meta}</span>
    {/if}
  </div>
</div>
{#if isBranch && node.expanded}
  <div role="group" class="ml-4 flex flex-col gap-1">
    {#each node.children as child (child.id)}
      <TreeRow
        node={child}
        {selectedAgentId}
        {activeId}
        {firstId}
        {onactivate}
        {onfocus}
        {onkeydown}
      />
    {/each}
  </div>
{/if}
