<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  The scene view renders the floors, keyboard, announcements, and the name
  matcher. It does not restore `last_view`: the restore reads the stored
  view and applies it in the panel store, because the panel owns the project
  choice and the selected agent that the restore sets. This view neither
  reads nor writes `last_view`.
-->
<script lang="ts">
  import { tick } from "svelte";
  import { shownStatusText, isActive, type OfficeStore } from "../../agent";
  import { INITIAL_HIERARCHY, type HierarchyController } from "../../hierarchy";
  import {
    focusHeadingUnlessRunning,
    type NeedsYouController,
  } from "../../needs-you";
  import { pressedAppShortcut } from "../../../core/shortcuts";
  import { TerminalView } from "../../terminal";
  import { ALL_PROJECTS, ProjectSwitcher } from "../../panel";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import Floor from "../components/floor.svelte";
  import {
    floorGroups,
    searchMatches,
    type Floor as FloorModel,
    type ProjectChoice,
  } from "../state/floor-groups";

  interface Props {
    store: OfficeStore;
    /** The 0002 tree controller whose answer splits floors into departments (D2). */
    hierarchy: HierarchyController;
    /** The project choice the panel store holds; null is "All projects" (D7). */
    projectChoice: string | null;
    /** Applies the switcher's next choice, so both views share one choice (D7). */
    onProjectChoice: (choice: string | null) => void;
    /** The desk whose terminal is embedded, or none (the shell records it). */
    openAgentId?: string | null;
    /** A click on the desk that is already open: its rows are read again. */
    onreopen: (agentId: string) => void;
    /** The Needs you list, whose top agent the jump shortcut opens (0008 D5). */
    needsYou: NeedsYouController;
  }

  let {
    store,
    hierarchy,
    projectChoice,
    onProjectChoice,
    openAgentId = $bindable(null),
    onreopen,
    needsYou,
  }: Props = $props();

  const office = $derived(store.snapshot);
  const controller = $derived(store.controller);
  const client = $derived(store.client);
  // Same subscription pattern as the panel view: the controller's snapshot
  // is a plain value, and the effect's cleanup stops the subscription when
  // the view unmounts. Subscribe calls back at once, so the tree starts
  // at the controller's current snapshot.
  let treeSnapshot = $state(INITIAL_HIERARCHY);
  $effect(() => hierarchy.subscribe((next) => (treeSnapshot = next)));

  // A choice that names a project the office no longer has shows every
  // floor, as the panel's `effectiveChoice` does for its rows (D7).
  const choice = $derived<ProjectChoice>(
    projectChoice !== ALL_PROJECTS &&
      office.projects.some((project) => project.id === projectChoice)
      ? { kind: "one", projectId: projectChoice }
      : { kind: "all" },
  );

  let searchText = $state("");

  const floors = $derived(
    floorGroups(office.agents, office.projects, choice, treeSnapshot.tree),
  );

  /** Every desk in reading order: floor by floor, department by department (D9). */
  const deskOrder = $derived(
    floors.flatMap((floor) => floor.groups.flatMap((group) => group.agents)),
  );

  const shown = $derived.by(() => {
    if (searchText.trim() === "") return floors;
    const matchedIds = new Set(
      searchMatches(deskOrder, searchText).map((agent) => agent.id),
    );
    return keepMatching(floors, matchedIds);
  });

  /**
   * The floors whose desks survive a search. A floor that loses every desk
   * drops with them, and so does a floor without desks, so a search shows
   * only floors that still have matches (D7).
   */
  function keepMatching(
    all: FloorModel[],
    matchedIds: ReadonlySet<string>,
  ): FloorModel[] {
    return all
      .map((floor) => ({
        ...floor,
        groups: floor.groups
          .map((group) => ({
            ...group,
            agents: group.agents.filter((agent) => matchedIds.has(agent.id)),
          }))
          .filter((group) => group.agents.length > 0),
      }))
      .filter((floor) => floor.groups.length > 0);
  }

  /** The ids of the desks on screen, in reading order (D9's focus walks). */
  function shownOrder(): string[] {
    return shown
      .flatMap((floor) => floor.groups.flatMap((group) => group.agents))
      .map((agent) => agent.id);
  }

  // The desk of the first match takes focus on each search change (D7).
  // Only the text is read inside the effect: the agents and the desk order
  // are read in the async tail, so a pushed status change never steals
  // focus back. Spaces-only text is no search and moves no focus.
  $effect(() => {
    const text = searchText;
    if (text.trim() === "") return;
    void tick().then(() => {
      const [firstMatch] = searchMatches(deskOrder, text);
      if (firstMatch === undefined) return;
      deskElement(firstMatch.id)?.focus();
    });
  });

  function isStopping(agentId: string): boolean {
    return office.stoppingAgentIds.has(agentId);
  }

  /** The desk button of one agent, searched inside the scene only. */
  function deskElement(agentId: string): HTMLElement | null {
    return (
      sceneRoot?.querySelector<HTMLElement>(
        `[data-agent-id="${CSS.escape(agentId)}"]`,
      ) ?? null
    );
  }

  let sceneRoot = $state<HTMLElement | null>(null);

  // --- Keyboard (D9): roving tabindex across the desks (0005 T5, part 2).
  // Tab moves desk to desk in reading order, because each desk is a plain
  // button in DOM order. The arrows additionally move within one floor,
  // per the APG layout-grid practice (research R4), and stop at the floor
  // edge instead of wrapping.
  /** The desk ids of one floor, in reading order, for the arrow moves. */
  function deskIdsOfFloor(floor: FloorModel): string[] {
    return floor.groups.flatMap((group) =>
      group.agents.map((agent) => agent.id),
    );
  }

  function ondeskskeydown(event: KeyboardEvent): void {
    const isArrow =
      event.key === "ArrowDown" ||
      event.key === "ArrowUp" ||
      event.key === "ArrowLeft" ||
      event.key === "ArrowRight";
    if (!isArrow) return;
    const desk = event.target as HTMLElement;
    const agentId = desk.getAttribute("data-agent-id");
    if (agentId === null) return;
    // The neighbours come from the shown floors, so focus never aims at a
    // desk an active search has hidden.
    const floor = shown.find((candidate) =>
      deskIdsOfFloor(candidate).includes(agentId),
    );
    if (floor === undefined) return;
    const ids = deskIdsOfFloor(floor);
    const index = ids.indexOf(agentId);
    const step =
      event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
    const next = ids[index + step];
    if (next === undefined) return;
    event.preventDefault();
    deskElement(next)?.focus();
  }

  // --- Announcements (D9): one polite live region. A status change writes
  // "name, status"; a deletion that moves focus writes where focus went.
  let announcement = $state("");

  function announce(text: string): void {
    announcement = text;
  }

  // The zero-match search is visible text and a live-region message, so a
  // screen reader hears the empty result (D9); spaces-only is no search
  // and says nothing.
  $effect(() => {
    if (searchText.trim() !== "" && shown.length === 0) {
      announce("No desk matches the search.");
    }
  });

  // The snapshot comparison walks the previous agents by id, so a desk
  // status change is found however it arrives (event, resync, reload).
  let knownStatuses = new Map<string, string>();
  $effect(() => {
    const statuses = new Map(
      office.agents.map((agent) => [agent.id, agent.status]),
    );
    for (const [agentId, status] of statuses) {
      const before = knownStatuses.get(agentId);
      if (before === undefined || before === status) continue;
      const agent = office.agents.find((candidate) => candidate.id === agentId);
      if (agent === undefined) continue;
      announce(`${agent.name}, ${shownStatusText(agent, isStopping(agentId))}`);
    }
    knownStatuses = statuses;
  });

  // When the focused desk's agent is deleted, focus moves to the next desk
  // in reading order, to the previous when none follows, and to the
  // empty-state heading when no desk is left; each move is announced (D9).
  let focusedAgentId = $state<string | null>(null);
  /** The focused desk's seat in the reading order, kept from focusin. */
  let focusedSeat = $state(0);
  const EMPTY_HEADING_ID = "scene-empty-heading";

  // A removed focused desk fires focusout with no relatedTarget after the
  // snapshot has already dropped it, so both the id and its reading-order
  // seat are kept from focusin. The fate is decided one tick later, as the
  // hierarchy view does: a desk rebuilt by the same change takes the focus
  // back through focusin, and only a truly deleted agent stays gone.
  function ondesksfocusin(event: FocusEvent): void {
    const desk = event.target as HTMLElement;
    const agentId = desk.getAttribute("data-agent-id");
    if (agentId === null) return;
    focusedAgentId = agentId;
    // The seat is kept in the shown order, so a deletion under an active
    // search moves focus to the next rendered desk, never a hidden one.
    const shownIds = shownOrder();
    focusedSeat = shownIds.indexOf(agentId);
  }

  function ondesksfocusout(event: FocusEvent): void {
    if (sceneRoot?.contains(event.relatedTarget as Node | null)) return;
    const gone = focusedAgentId;
    const seat = focusedSeat;
    focusedAgentId = null;
    if (gone === null) return;
    void tick().then(() => {
      if (
        office.agents.some((agent) => agent.id === gone) &&
        deskElement(gone) !== null
      ) {
        return;
      }
      const survivors = shownOrder();
      const next = survivors[seat] ?? survivors[seat - 1];
      const heading = document.getElementById(EMPTY_HEADING_ID);
      // The announcement follows the focus, never leads it: a desk the
      // search has hidden takes no focus and makes no claim (D9).
      if (next !== undefined) {
        const target = deskElement(next);
        if (target === null) return;
        target.focus();
        const agent = office.agents.find((candidate) => candidate.id === next);
        announce(`Focus moved to ${agent?.name ?? next}.`);
      } else if (heading !== null) {
        heading.focus();
        announce("Focus moved to the empty state.");
      }
    });
  }

  // --- Open a desk (D5, FR-005): the agent's terminal embeds in the scene,
  // no second window; the office selection is never touched (FR-006). A
  // freshly mounted terminal focuses itself once the attach answers; an
  // already open one takes focus here, as the panel does.
  function openDesk(agentId: string): void {
    const wasOpen = openAgentId === agentId;
    openAgentId = agentId;
    if (wasOpen) {
      onreopen(agentId);
      void tick().then(() => {
        sceneRoot?.querySelector("textarea")?.focus();
      });
    }
  }

  // --- Needs you (0008 D5, FR-010): the scene has no other app shortcut, so
  // this window listener handles the jump only. The panel's selection and
  // project choice stay as they are.
  function jumpToTopDesk(event: KeyboardEvent): void {
    if (event.repeat || pressedAppShortcut(event)?.kind !== "jump") return;
    event.preventDefault();
    const agentId = needsYou.takeTop((id) =>
      office.agents.some((agent) => agent.id === id),
    );
    if (agentId === null) return;
    openDesk(agentId);
    const agent = office.agents.find((known) => known.id === agentId);
    void focusHeadingUnlessRunning(
      sceneRoot,
      agent !== undefined && isActive(agent.status),
    );
  }

  $effect(() => {
    window.addEventListener("keydown", jumpToTopDesk);
    return () => window.removeEventListener("keydown", jumpToTopDesk);
  });

  // A count change speaks through the scene's one live region.
  let lastNeedsYouAnnouncement: string | null = null;
  $effect(() =>
    needsYou.subscribe((next) => {
      if (next.announcement === lastNeedsYouAnnouncement) return;
      lastNeedsYouAnnouncement = next.announcement;
      if (next.announcement !== null) announce(next.announcement);
    }),
  );

  /** The focus shortcut hands focus back to the desk that opened it (D5). */
  function leaveTerminalToDesk(): void {
    if (openAgentId !== null) deskElement(openAgentId)?.focus();
  }
</script>

<div
  bind:this={sceneRoot}
  role="region"
  aria-labelledby="scene-heading"
  class="mx-auto flex max-w-6xl flex-col gap-6 p-6"
>
  <h1 id="scene-heading" class="text-2xl font-semibold text-neutral-900">
    Office scene
  </h1>

  <!-- One polite region for the whole scene; the newest message wins, which
    coalesces a burst of changes into the last one (D9, research R4). -->
  <p role="status" class="sr-only">{announcement}</p>

  {#if office.notice !== null}
    <FailureMessage lead={office.notice.lead} message={office.notice.message} />
  {/if}

  {#if office.phase === "loading"}
    <p class="text-neutral-700">Loading the office.</p>
  {:else if office.phase === "failed"}
    <FailureMessage
      lead="The office did not load."
      message={office.failure ?? ""}
    />
    <div>
      <Button variant="secondary" onclick={() => void controller.load()}>
        Try again
      </Button>
    </div>
  {:else if office.agents.length === 0}
    <section
      aria-labelledby={EMPTY_HEADING_ID}
      class="flex flex-col items-start gap-3"
    >
      <h2
        id={EMPTY_HEADING_ID}
        tabindex="-1"
        class="text-xl font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
      >
        The office is empty
      </h2>
      <p class="text-neutral-700">
        No agent exists yet. Each agent gets a desk here once one exists, and
        the create action stays in the panel.
      </p>
    </section>
  {:else}
    <div class="flex flex-wrap items-end gap-3">
      <ProjectSwitcher
        projects={office.projects}
        value={choice.kind === "one" ? choice.projectId : ALL_PROJECTS}
        onchange={onProjectChoice}
      />
      <div class="flex flex-col gap-1">
        <label for="scene-search" class="text-sm font-medium text-neutral-900">
          Search agents
        </label>
        <input
          id="scene-search"
          bind:value={searchText}
          type="search"
          autocomplete="off"
          class="min-h-11 w-64 rounded-md border border-neutral-500 bg-white px-3 py-2 text-sm text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
        />
      </div>
    </div>

    {#if shown.length === 0}
      <p class="text-neutral-700">No desk matches the search.</p>
    {:else}
      <!-- The zero-match case routes through the one live region below,
        so a screen reader hears it too (D9). -->
      <div
        class="flex flex-col gap-4"
        onfocusin={ondesksfocusin}
        onfocusout={ondesksfocusout}
      >
        {#each shown as floor (floor.projectId ?? "none")}
          {@const floorProjectName =
            floor.projectId === null
              ? null
              : (office.projects.find(
                  (project) => project.id === floor.projectId,
                )?.name ?? null)}
          <Floor
            {floor}
            projectName={floorProjectName}
            {isStopping}
            onnavigate={ondeskskeydown}
            onopen={openDesk}
          />
        {/each}
      </div>
    {/if}

    {#if openAgentId !== null}
      {@const openAgent = office.agents.find(
        (agent) => agent.id === openAgentId,
      )}
      {#if openAgent !== undefined}
        <section aria-label="Desk terminal">
          {#key openAgent.id}
            <TerminalView
              {client}
              agentId={openAgent.id}
              agentName={openAgent.name}
              isRunning={isActive(openAgent.status)}
              onFocusLeave={leaveTerminalToDesk}
            />
          {/key}
        </section>
      {/if}
    {/if}
  {/if}
</div>
