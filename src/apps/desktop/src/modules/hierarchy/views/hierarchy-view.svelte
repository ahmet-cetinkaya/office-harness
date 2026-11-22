<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    WorktreeDeleteDialog,
    deleteResultText,
    type Agent,
    type NewAgent,
    type Project,
    type WorktreeChoice,
    type WorktreeStatus,
  } from "../../agent";
  import HierarchyAgentDialog from "../components/hierarchy-agent-dialog.svelte";
  import HierarchyConfirmDialog from "../components/hierarchy-confirm-dialog.svelte";
  import TeamFormDialog from "../components/team-form-dialog.svelte";
  import TreeRow from "../components/tree-row.svelte";
  import {
    branchOfAgent,
    buildHierarchy,
    descendantIdsOf,
  } from "../models/hierarchy";
  import type { HierarchyNode } from "../models/hierarchy";
  import { allowedRolesAt, placeOf, type RoleKind } from "../models/placement";
  import {
    ROLE_WORDS,
    countAgentsText,
    deleteQuestionText,
    roleKindText,
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
  // `createLeadId` is the row it acts under, both read when it opens. A
  // choice that stopped being valid since the button rendered still sends:
  // the server refuses with its own reason (FR-003).
  let createRole = $state<RoleKind | null>(null);
  /** The lead a member dialog acts under, read when the dialog opens. */
  let createLeadId = $state<string | null>(null);
  let isCreatingTeam = $state(false);
  /** A create request is in flight; the dialog disables its submit so a
   * double Enter or double click cannot send two creates. */
  let isBusy = $state(false);

  function openCreate(role: RoleKind): void {
    if (role === "lead") {
      // A team lead comes with its team in one request (FR-002), so the
      // lead's dialog is the team dialog.
      openCreateTeam();
      return;
    }
    createLeadId = selectedAgentId;
    createRole = role;
  }

  /** The request one role at one place makes. A place that stopped allowing
   * the role still sends: the server refuses with its own reason, so the
   * text cannot drift from the real rules (FR-003). */
  function submitCreateRole(agent: NewAgent): Promise<FailureText> {
    const role = createRole;
    if (role === null) return Promise.resolve(null);
    if (role === "ceo") return controller.createCeo(agent);
    return controller.createMember(agent, createLeadId ?? "");
  }

  function openCreateTeam(): void {
    isCreatingTeam = true;
  }

  // The delete and stop refusal describes the agent as the tree held it when
  // the request was sent, so it is kept with the branch the agent sat in.
  // It shows while the agent still sits there: another client's unrelated
  // change must not wipe the message, and a move or removal must.
  let refusal = $state<{
    agentId: string;
    branch: string | null;
    text: string;
  } | null>(null);
  const shownRefusal = $derived.by(() => {
    if (refusal === null) return null;
    return branchOfAgent(snapshot.tree, refusal.agentId) === refusal.branch
      ? refusal.text
      : null;
  });

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
    isBusy = true;
    try {
      const answered = await work();
      if (answered === null) closeCreate();
      return answered;
    } finally {
      isBusy = false;
    }
  }

  // The delete and stop flows. The question is asked with the descendant
  // count the server holds now, and the answer carries that count for the
  // server to re-check (FR-006, FR-007). A stop with descendants is a
  // two-choice question: stop only this agent, or stop the counted subtree.
  type PendingAction = { kind: "delete" } | { kind: "stop" };
  let pending = $state<PendingAction | null>(null);
  let pendingAgent = $state<{ id: string; name: string } | null>(null);
  let pendingDescendants = $state(0);
  let pendingCountFailure = $state<string | null>(null);
  /** The worktrees among the agents a delete removes; empty asks no
   * worktree question (0010, FR-005). */
  let pendingWorktrees = $state<WorktreeStatus[]>([]);
  /** What the last delete left in the repositories, as text. */
  let deleteResult = $state<string | null>(null);
  /** The stop answer the user picked; delete has no choice to make. */
  let pendingStopWithDescendants = $state(false);

  const pendingQuestion = $derived.by(() => {
    if (pending === null || pendingAgent === null) return null;
    return pending.kind === "delete"
      ? deleteQuestionText(pendingAgent.name, pendingDescendants)
      : stopQuestionText(pendingAgent.name, pendingDescendants);
  });

  const stopChoices = $derived.by(() => {
    if (pending?.kind !== "stop" || pendingAgent === null) return null;
    const below = countAgentsText(pendingDescendants);
    return [
      {
        label: `Stop only ${pendingAgent.name}`,
        body: `${below} below ${pendingAgent.name} keep running.`,
      },
      {
        label: `Stop ${pendingAgent.name} and ${below}`,
        body: `The agents below stop in the same action.`,
      },
    ];
  });

  /** The worktrees among the agents a delete removes, or null after the
   * refusal is shown when they could not be read. */
  async function readWorktrees(
    agentId: string,
  ): Promise<WorktreeStatus[] | null> {
    const ids = [agentId, ...descendantIdsOf(snapshot.tree, agentId)];
    const hasWorktree = agents.some(
      (known) => ids.includes(known.id) && known.worktree,
    );
    if (!hasWorktree) return [];
    try {
      return await controller.worktreeStatus(ids);
    } catch (rejection) {
      refusal = {
        agentId,
        branch: branchOfAgent(snapshot.tree, agentId) ?? null,
        text: controller.describeCountFailure(rejection),
      };
      return null;
    }
  }

  async function ask(
    kind: PendingAction["kind"],
    node: Pick<HierarchyNode, "agentId" | "title">,
  ): Promise<void> {
    if (node.agentId === null) return;
    pendingStopWithDescendants = false;
    pendingWorktrees = [];
    if (kind === "delete") {
      const statuses = await readWorktrees(node.agentId);
      if (statuses === null) return;
      pendingWorktrees = statuses;
    }
    try {
      pendingDescendants = await controller.countDescendants(node.agentId);
      pendingCountFailure = null;
    } catch (rejection) {
      // The count is part of the question; hiding its failure would read a
      // wrong subtree size as fact (FR-006, FR-007).
      pendingDescendants = 0;
      pendingCountFailure = controller.describeCountFailure(rejection);
    }
    pending = { kind };
    pendingAgent = { id: node.agentId, name: node.title };
  }

  /** The label of the stop answer the user picked, for the confirm button. */
  const pendingConfirmLabel = $derived.by(() => {
    if (pending === null) return null;
    if (pending.kind === "delete") return "Delete agent";
    if (pendingDescendants === 0) return "Stop agent";
    if (pendingStopWithDescendants) {
      return `Stop ${pendingAgent?.name} and ${countAgentsText(pendingDescendants)}`;
    }
    return `Stop only ${pendingAgent?.name}`;
  });

  function chooseStop(withDescendants: boolean): void {
    pendingStopWithDescendants = withDescendants;
  }

  async function confirmPending(choice?: WorktreeChoice): Promise<void> {
    if (pending === null || pendingAgent === null) return;
    const { kind } = pending;
    const { id, name } = pendingAgent;
    const includeDescendants =
      pendingDescendants > 0 &&
      (kind === "delete" || pendingStopWithDescendants);
    pending = null;
    pendingAgent = null;
    deleteResult = null;
    let answered: FailureText = null;
    let isChanged = false;
    if (kind === "delete") {
      const outcome = await controller.deleteSubtree(
        id,
        includeDescendants,
        pendingDescendants,
        choice,
      );
      if (outcome.kind === "deleted") {
        deleteResult = deleteResultText(outcome.result);
      } else {
        answered = outcome.message;
        isChanged = outcome.kind === "changed";
      }
    } else {
      answered = await controller.stopSubtree(id, includeDescendants);
    }
    // A refusal is kept with the branch the agent sat in when it was
    // refused; a request that went through drops the last one.
    refusal =
      answered === null
        ? null
        : {
            agentId: id,
            branch: branchOfAgent(snapshot.tree, id) ?? null,
            text: answered,
          };
    // A worktree changed after the question, so the question is asked again.
    if (isChanged) await ask("delete", { agentId: id, title: name });
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

  {#if deleteResult !== null}
    <p role="status" class="text-sm text-neutral-900">{deleteResult}</p>
  {/if}

  {#if shownRefusal !== null}
    <FailureMessage
      lead="The hierarchy did not change."
      message={shownRefusal}
    />
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
    {isBusy}
    onsubmit={(agent) => submitCreate(() => submitCreateRole(agent))}
    onworktreeinfo={controller.worktreeInfo}
    onclose={closeCreate}
  />
{/if}
{#if isCreatingTeam}
  <TeamFormDialog
    {projects}
    {isBusy}
    onsubmit={(team) => submitCreate(() => controller.createTeam(team))}
    onclose={closeCreate}
  />
{/if}
{#if pending?.kind === "delete" && pendingAgent !== null && pendingQuestion !== null && pendingWorktrees.length > 0}
  <WorktreeDeleteDialog
    title={pendingQuestion.title}
    body={pendingCountFailure === null
      ? pendingQuestion.body
      : `${pendingQuestion.body} The descendant count could not be read: ${pendingCountFailure}`}
    statuses={pendingWorktrees}
    onanswer={(choice) => void confirmPending(choice)}
    oncancel={cancelPending}
  />
{:else if pending !== null && pendingAgent !== null && pendingQuestion !== null}
  <HierarchyConfirmDialog
    title={pendingQuestion.title}
    body={pendingCountFailure === null
      ? pendingQuestion.body
      : `${pendingQuestion.body} The descendant count could not be read: ${pendingCountFailure}`}
    choices={pending.kind === "stop" && pendingDescendants > 0
      ? stopChoices
      : null}
    selectedChoice={pending.kind === "stop" && pendingDescendants > 0
      ? pendingStopWithDescendants
      : null}
    onchoose={pending.kind === "stop" && pendingDescendants > 0
      ? chooseStop
      : null}
    confirmLabel={pendingConfirmLabel ?? "Confirm"}
    onconfirm={() => void confirmPending()}
    oncancel={cancelPending}
  />
{/if}
