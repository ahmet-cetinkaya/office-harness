<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import type { Floor as FloorModel } from "../state/floor-groups";
  import Desk from "./desk.svelte";

  interface Props {
    floor: FloorModel;
    /** The floor's project name, or null on the no-project floor. */
    projectName: string | null;
    /** True while the agent's stop request is open. */
    isStopping: (agentId: string) => boolean;
    /** Moves focus within the floor on an arrow key (D9). */
    onnavigate?: (event: KeyboardEvent) => void;
    /** Opens the agent's terminal in the scene (D5). */
    onopen?: (agentId: string) => void;
  }

  let { floor, projectName, isStopping, onnavigate, onopen }: Props = $props();

  // The no-project floor's id is the one null maps to, so two floors can
  // never share a label id.
  const headingId = $derived(`floor-${floor.projectId ?? "none"}`);
</script>

<section
  aria-labelledby={headingId}
  class="flex flex-col gap-3 rounded-md border border-neutral-200 p-4"
>
  <h3
    id={headingId}
    tabindex="-1"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    {floor.name}
    <span class="font-normal text-neutral-700">({floor.agentCount})</span>
  </h3>
  {#if floor.agentCount === 0}
    <p class="text-neutral-700">This floor has no agents.</p>
  {:else}
    {#each floor.groups as group (group.name ?? "")}
      {#if group.name !== null}
        <h4 class="text-sm font-medium text-neutral-700">{group.name}</h4>
      {/if}
      <ul
        class="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(16rem,1fr))]"
      >
        {#each group.agents as agent (agent.id)}
          <li>
            <Desk
              {agent}
              {projectName}
              isStopping={isStopping(agent.id)}
              {onnavigate}
              {onopen}
            />
          </li>
        {/each}
      </ul>
    {/each}
  {/if}
</section>
