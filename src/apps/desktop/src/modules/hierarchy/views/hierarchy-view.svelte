<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import type { Agent, Project } from "../../agent";
  import TreeRow from "../components/tree-row.svelte";
  import { buildHierarchy } from "../models/hierarchy";
  import type { HierarchyNode } from "../models/hierarchy";
  import { INITIAL_HIERARCHY } from "../state/hierarchy-controller";
  import type { HierarchyController } from "../state/hierarchy-controller";

  interface Props {
    controller: HierarchyController;
    agents: Agent[];
    projects: Project[];
    /** The agent the details pane shows, if any. */
    selectedAgentId: string | null;
    onselect: (agentId: string) => void;
    /** Shown while the office has no tree content at all. */
    empty?: Snippet;
  }

  let {
    controller,
    agents,
    projects,
    selectedAgentId,
    onselect,
    empty,
  }: Props = $props();

  // The controller's snapshot is a plain value, so the view subscribes and
  // keeps the last seen one in runes state; the effect's cleanup stops the
  // subscription when the view unmounts. Subscribe calls back with the
  // current snapshot at once, so `snapshot` starts at the controller's.
  let snapshot = $state(INITIAL_HIERARCHY);
  $effect(() => controller.subscribe((next) => (snapshot = next)));

  const collapsedIds = new SvelteSet<string>();
  let activeId = $state<string | null>(null);

  const roots = $derived.by(() =>
    buildHierarchy(snapshot.tree, agents, projects, collapsedIds),
  );
  const firstId = $derived(roots.length > 0 ? roots[0].id : null);

  function collapse(id: string): void {
    collapsedIds.add(id);
  }

  function expand(id: string): void {
    collapsedIds.delete(id);
  }

  /** One visible row in tree order, parents first. */
  function visibleRows(nodes: HierarchyNode[]): HierarchyNode[] {
    const rows: HierarchyNode[] = [];
    for (const node of nodes) {
      rows.push(node);
      if (node.expanded && node.children.length > 0) {
        rows.push(...visibleRows(node.children));
      }
    }
    return rows;
  }

  const rows = $derived(visibleRows(roots));
  const rowById = $derived(new SvelteMap(rows.map((row) => [row.id, row])));
  const parentOf = $derived.by(() => {
    const parents = new SvelteMap<string, string>();
    for (const node of rows) {
      for (const child of node.children) parents.set(child.id, node.id);
    }
    return parents;
  });

  /** Moves the active row and its focus; the tab order needs a render first. */
  async function focusRow(id: string): Promise<void> {
    activeId = id;
    await tick();
    document.querySelector<HTMLElement>(`[data-node-id="${id}"]`)?.focus();
  }

  function move(delta: number): void {
    if (rows.length === 0) return;
    const index = rows.findIndex((row) => row.id === activeId);
    const next =
      index === -1
        ? delta > 0
          ? 0
          : rows.length - 1
        : Math.min(rows.length - 1, Math.max(0, index + delta));
    void focusRow(rows[next].id);
  }

  function moveWithinBranch(last: boolean): void {
    const branch = focusedRow();
    if (branch === undefined || !branch.expanded) return;
    const children = visibleRows(branch.children);
    if (children.length === 0) return;
    const target = last ? children[children.length - 1] : children[0];
    void focusRow(target.id);
  }

  function focusedRow(): HierarchyNode | undefined {
    return rowById.get(activeId ?? "");
  }

  function onkeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "ArrowRight":
        event.preventDefault();
        {
          const row = focusedRow();
          if (row === undefined) return;
          if (row.children.length === 0) return;
          if (row.expanded) moveWithinBranch(false);
          else expand(row.id);
        }
        break;
      case "ArrowLeft":
        event.preventDefault();
        {
          const row = focusedRow();
          if (row === undefined) return;
          if (row.expanded && row.children.length > 0) collapse(row.id);
          else {
            const parent = parentOf.get(row.id);
            if (parent !== undefined) void focusRow(parent);
          }
        }
        break;
      case "Home":
        event.preventDefault();
        if (rows.length > 0) void focusRow(rows[0].id);
        break;
      case "End":
        event.preventDefault();
        if (rows.length > 0) void focusRow(rows[rows.length - 1].id);
        break;
      case "Enter":
        event.preventDefault();
        {
          const row = focusedRow();
          if (row === undefined) return;
          if (row.agentId !== null) onselect(row.agentId);
          else if (row.children.length > 0) {
            if (row.expanded) collapse(row.id);
            else expand(row.id);
          }
        }
        break;
      case "Escape":
        // The collapse is the row's own cancel; dialogs close themselves.
        event.preventDefault();
        {
          const row = focusedRow();
          if (row?.expanded && row.children.length > 0) collapse(row.id);
        }
        break;
    }
  }

  function onactivate(node: HierarchyNode): void {
    activeId = node.id;
    if (node.agentId !== null) {
      onselect(node.agentId);
      return;
    }
    if (node.children.length > 0) {
      if (node.expanded) collapse(node.id);
      else expand(node.id);
    }
  }

  /** Focus that arrives from outside (Tab, a programmatic focus). */
  function onfocus(node: HierarchyNode): void {
    activeId = node.id;
  }

  // When the row that held focus leaves the tree (a delete elsewhere, a
  // reload), focus returns to its branch so keyboard use never drops to the
  // page body.
  $effect(() => {
    if (activeId !== null && !rowById.has(activeId)) {
      activeId = firstId;
      void tick().then(() => {
        if (firstId !== null) {
          document
            .querySelector<HTMLElement>(`[data-node-id="${firstId}"]`)
            ?.focus();
        }
      });
    }
  });
</script>

<section aria-labelledby="hierarchy-heading" class="flex flex-col gap-3">
  <h2
    id="hierarchy-heading"
    tabindex="-1"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    Hierarchy
  </h2>

  {#if snapshot.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the hierarchy.</p>
  {:else if snapshot.phase === "failed"}
    <FailureMessage
      lead="The hierarchy did not load."
      message={snapshot.failure ?? ""}
    />
    <div>
      <Button variant="secondary" onclick={() => void controller.load()}>
        Try again
      </Button>
    </div>
  {:else if rows.length === 0}
    {#if empty}
      {@render empty()}
    {:else}
      <p class="text-neutral-700">
        The office has no CEO and no team. Create them to build the hierarchy.
      </p>
    {/if}
  {:else}
    <!--
      Each row is the key handler for its own key events, and the rows carry
      the one tab stop (roving tabindex), as the tree pattern asks.
    -->
    <div
      role="tree"
      aria-labelledby="hierarchy-heading"
      data-testid="hierarchy-tree"
      class="flex flex-col gap-1"
    >
      {#each roots as root (root.id)}
        <TreeRow
          node={root}
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
</section>
