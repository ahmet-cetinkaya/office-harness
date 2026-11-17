<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import type { Agent, NewAgent, Project } from "../../agent";
  import HierarchyAgentDialog from "../components/hierarchy-agent-dialog.svelte";
  import HierarchyConfirmDialog from "../components/hierarchy-confirm-dialog.svelte";
  import TeamFormDialog from "../components/team-form-dialog.svelte";
  import TreeRow from "../components/tree-row.svelte";
  import { buildHierarchy } from "../models/hierarchy";
  import type { HierarchyNode } from "../models/hierarchy";
  import {
    allowedRolesAt,
    placeOf,
    type CreatePlace,
    type RoleKind,
  } from "../models/placement";
  import {
    ROLE_WORDS,
    deleteQuestionText,
    roleKindText,
    roleRefusalText,
    stopQuestionText,
  } from "../models/confirmations";
  import { INITIAL_HIERARCHY } from "../state/hierarchy-controller";
  import type {
    FailureText,
    HierarchyController,
  } from "../state/hierarchy-controller";

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

  // The create flows. `createRole` names the dialog the button opens and
  // `createPlace` is the place it acts at, both read when it opens. A role
  // the place stopped allowing since the button rendered refuses with the
  // allowed roles, as the server would (FR-003).
  let createRole = $state<RoleKind | null>(null);
  let createPlace = $state<CreatePlace>({ kind: "office" });
  /** The lead a member dialog acts under, read when the dialog opens. */
  let createLeadId = $state<string | null>(null);
  let isCreatingTeam = $state(false);
  let refusal = $state<string | null>(null);

  function openCreate(role: RoleKind): void {
    if (role === "lead") {
      // A team lead comes with its team in one request (FR-002), so the
      // lead's dialog is the team dialog.
      openCreateTeam();
      return;
    }
    createPlace = placeOf(snapshot.tree, selectedAgentId);
    createLeadId = selectedAgentId;
    createRole = role;
    refusal = null;
  }

  /** The request one role at one place makes. */
  function submitCreateRole(agent: NewAgent): Promise<FailureText> {
    const role = createRole;
    const place = createPlace;
    if (role === null) return Promise.resolve(null);
    if (!allowedRolesAt(place, snapshot.tree).includes(role)) {
      const parentRole = place.kind === "under-ceo" ? "ceo" : "member";
      return Promise.resolve(roleRefusalText(role, parentRole));
    }
    if (role === "ceo") return controller.createCeo(agent);
    return controller.createMember(agent, createLeadId ?? "");
  }

  function openCreateTeam(): void {
    isCreatingTeam = true;
    refusal = null;
  }

  function closeCreate(): void {
    createRole = null;
    isCreatingTeam = false;
  }

  /** The tree row of the agent the user last activated, when it is still
   * in the tree. */
  function selectedRow(): HierarchyNode | null {
    const row = rowById.get(selectedAgentId ?? "");
    return row?.agentId !== undefined && row.agentId !== null ? row : null;
  }

  /** The agent row the row actions act on, as reactive state for the view. */
  const actionRow = $derived(selectedRow());

  async function submitCreate(
    work: () => Promise<FailureText>,
  ): Promise<FailureText> {
    const answered = await work();
    if (answered === null) closeCreate();
    return answered;
  }

  // The delete and stop flows. The question is asked with the descendant
  // count the server holds now, and the answer carries that count for the
  // server to re-check (FR-006, FR-007).
  type PendingAction = { kind: "delete" } | { kind: "stop" };
  let pending = $state<PendingAction | null>(null);
  let pendingAgent = $state<{ id: string; name: string } | null>(null);
  let pendingDescendants = $state(0);

  const pendingQuestion = $derived.by(() => {
    if (pending === null || pendingAgent === null) return null;
    return pending.kind === "delete"
      ? deleteQuestionText(pendingAgent.name, pendingDescendants)
      : stopQuestionText(pendingAgent.name, pendingDescendants);
  });

  async function ask(
    kind: PendingAction["kind"],
    node: HierarchyNode,
  ): Promise<void> {
    if (node.agentId === null) return;
    refusal = null;
    try {
      pendingDescendants = await controller.countDescendants(node.agentId);
    } catch {
      pendingDescendants = 0;
    }
    pending = { kind };
    pendingAgent = { id: node.agentId, name: node.title };
  }

  async function confirmPending(): Promise<void> {
    if (pending === null || pendingAgent === null) return;
    const { kind } = pending;
    const { id } = pendingAgent;
    const includeDescendants = pendingDescendants > 0;
    pending = null;
    pendingAgent = null;
    const answered =
      kind === "delete"
        ? await controller.deleteSubtree(
            id,
            includeDescendants,
            pendingDescendants,
          )
        : await controller.stopSubtree(id, includeDescendants);
    if (answered !== null) refusal = answered;
  }

  function cancelPending(): void {
    pending = null;
    pendingAgent = null;
  }

  // Whether keyboard focus sits inside the tree. A row that is removed while
  // focused drops focus to the page body, and Chromium reports that as a
  // focusout on the row, so the clear waits until the row's fate is settled:
  // a rebuilt row takes its focus back, and only a deleted row keeps the
  // flag for the refocus effect below.
  let treeHasFocus = $state(false);
  let treeContainer = $state<HTMLDivElement | null>(null);

  function ontreefocusin(): void {
    treeHasFocus = true;
  }

  function ontreefocusout(event: FocusEvent): void {
    if (treeContainer?.contains(event.relatedTarget as Node | null)) return;
    void tick().then(() => {
      const row = event.target;
      if (row instanceof Element && !treeContainer?.contains(row)) {
        refocusRemovedRow(row);
        return;
      }
      treeHasFocus = false;
    });
  }

  /** The rendered row for a node id, searched inside the tree only. */
  function rowElement(id: string): HTMLElement | null {
    return (
      treeContainer?.querySelector<HTMLElement>(
        `[data-node-id="${CSS.escape(id)}"]`,
      ) ?? null
    );
  }

  /**
   * The row that fired focusout has left the DOM. If its id was rebuilt
   * elsewhere in the tree, say an agent moved to another team, its new
   * element takes the focus back, and its focusin keeps the flag. A move
   * under a collapsed branch has no element to focus, so focus truly left
   * the tree. Only a row whose id is gone from the tree keeps the flag, for
   * the refocus effect below.
   */
  function refocusRemovedRow(removedRow: Element): void {
    const id = removedRow.getAttribute("data-node-id");
    if (id === null || !rowById.has(id)) return;
    const replacement = rowElement(id);
    if (replacement === null) {
      treeHasFocus = false;
      return;
    }
    replacement.focus();
  }

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
    rowElement(id)?.focus();
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
      case "Escape": {
        // The collapse is the row's own cancel; without it, Escape belongs
        // to an outer handler, such as a dialog.
        const row = focusedRow();
        if (row?.expanded && row.children.length > 0) {
          event.preventDefault();
          collapse(row.id);
        }
        break;
      }
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
  // reload), the roving tab stop moves back to the first row. Focus follows
  // only when it sat inside the tree; if the user had moved on, say to the
  // terminal, their focus stays there.
  $effect(() => {
    if (activeId !== null && !rowById.has(activeId)) {
      activeId = firstId;
      if (!treeHasFocus) return;
      void tick().then(() => {
        if (firstId !== null) rowElement(firstId)?.focus();
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

  {#if refusal !== null}
    <FailureMessage lead="The hierarchy did not change." message={refusal} />
  {/if}

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
    <div class="flex flex-wrap gap-3">
      {#each allowedRolesAt({ kind: "office" }, snapshot.tree) as role (role)}
        <Button onclick={() => openCreate(role)}>
          Create {roleKindText(role)}
        </Button>
      {/each}
    </div>
  {:else}
    <div class="flex flex-wrap gap-3">
      {#each allowedRolesAt(placeOf(snapshot.tree, selectedAgentId), snapshot.tree) as role (role)}
        <Button
          variant={role === "ceo" ? "primary" : "secondary"}
          onclick={() => openCreate(role)}
        >
          Create {roleKindText(role)}
        </Button>
      {/each}
    </div>
    <!--
      Each row is the key handler for its own key events, and the rows carry
      the one tab stop (roving tabindex), as the tree pattern asks.
    -->
    <div
      role="tree"
      aria-labelledby="hierarchy-heading"
      data-testid="hierarchy-tree"
      class="flex flex-col gap-1"
      bind:this={treeContainer}
      onfocusin={ontreefocusin}
      onfocusout={ontreefocusout}
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
    {#if actionRow !== null}
      <div
        aria-labelledby="row-actions-heading"
        class="flex flex-wrap gap-3"
        role="group"
      >
        <h3 id="row-actions-heading" class="sr-only">
          Actions for {actionRow.title}
        </h3>
        <Button variant="secondary" onclick={() => void ask("stop", actionRow)}>
          Stop {actionRow.title}
        </Button>
        <Button
          variant="secondary"
          onclick={() => void ask("delete", actionRow)}
        >
          Delete {actionRow.title}
        </Button>
      </div>
    {/if}
  {/if}
</section>

{#if createRole !== null}
  <HierarchyAgentDialog
    subject={`New ${ROLE_WORDS[createRole]}`}
    isBusy={false}
    onsubmit={(agent) => submitCreate(() => submitCreateRole(agent))}
    onclose={closeCreate}
  />
{/if}
{#if isCreatingTeam}
  <TeamFormDialog
    {projects}
    isBusy={false}
    onsubmit={(team) => submitCreate(() => controller.createTeam(team))}
    onclose={closeCreate}
  />
{/if}
{#if pending !== null && pendingAgent !== null && pendingQuestion !== null}
  <HierarchyConfirmDialog
    title={pendingQuestion.title}
    body={pendingQuestion.body}
    confirmLabel={pending.kind === "delete" ? "Delete agent" : "Stop agent"}
    onconfirm={() => void confirmPending()}
    oncancel={cancelPending}
  />
{/if}
