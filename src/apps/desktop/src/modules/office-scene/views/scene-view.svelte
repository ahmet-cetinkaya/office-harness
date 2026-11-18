<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  Part 1 of the scene build (0005 T3): the skeleton. A later commit in
  this phase adds on top: the terminal embed on desk open with focus (D5),
  the `view` field in `last_view` and its restore (D6), the panel's name
  matcher swapped into `searchMatches` (D7, research R2), the keyboard
  roving tabindex, the live region, and the reduced-motion scan (D9, D10),
  plus the e2e for departments from a seeded tree, in-place updates
  (SC-002, SC-003), the PB-4 recorded-call assertion, and the role, team,
  and settings seeds that test/mock-core-extensions adds to mock-core.
-->
<script lang="ts">
  import { tick } from "svelte";
  import type { OfficeStore } from "../../agent";
  import { INITIAL_HIERARCHY, type HierarchyController } from "../../hierarchy";
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
  }

  let { store, hierarchy, projectChoice, onProjectChoice }: Props = $props();

  const office = $derived(store.snapshot);
  const controller = $derived(store.controller);
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
      document
        .querySelector<HTMLElement>(
          `[data-agent-id="${CSS.escape(firstMatch.id)}"]`,
        )
        ?.focus();
    });
  });

  function isStopping(agentId: string): boolean {
    return office.stoppingAgentIds.has(agentId);
  }
</script>

<div
  role="region"
  aria-labelledby="scene-heading"
  class="mx-auto flex max-w-6xl flex-col gap-6 p-6"
>
  <h1 id="scene-heading" class="text-2xl font-semibold text-neutral-900">
    Office scene
  </h1>

  {#if office.notice !== null}
    <FailureMessage lead={office.notice.lead} message={office.notice.message} />
  {/if}

  {#if office.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the office.</p>
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
      aria-labelledby="scene-empty-heading"
      class="flex flex-col items-start gap-3"
    >
      <h2
        id="scene-empty-heading"
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
        <!-- TODO(0005 part 2, D7): swap in the panel's name matcher when it
          lands, so both views return the same matches. -->
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
      <p role="status" class="text-neutral-700">No desk matches the search.</p>
    {:else}
      <div class="flex flex-col gap-4">
        {#each shown as floor (floor.projectId ?? "none")}
          {@const floorProjectName =
            floor.projectId === null
              ? null
              : (office.projects.find(
                  (project) => project.id === floor.projectId,
                )?.name ?? null)}
          <Floor {floor} projectName={floorProjectName} {isStopping} />
        {/each}
      </div>
    {/if}
  {/if}
</div>
