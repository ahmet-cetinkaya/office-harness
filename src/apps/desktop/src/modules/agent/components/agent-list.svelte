<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { cn } from "$shared/utils";
  import { agentEntryName, shownStatusText, type Agent } from "../models/agent";

  interface Props {
    agents: Agent[];
    selectedAgentId: string | null;
    stoppingAgentIds: ReadonlySet<string>;
    onselect: (agentId: string) => void;
  }

  let { agents, selectedAgentId, stoppingAgentIds, onselect }: Props = $props();

  const base =
    "flex min-h-11 w-full flex-col rounded-md border px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";
</script>

<section aria-labelledby="agents-heading" class="flex flex-col gap-2">
  <h2
    id="agents-heading"
    tabindex="-1"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    Agents
  </h2>
  <ul class="flex flex-col gap-2">
    {#each agents as agent (agent.id)}
      {@const isSelected = agent.id === selectedAgentId}
      {@const isStopping = stoppingAgentIds.has(agent.id)}
      <li>
        <!-- The name is set by hand so that it reads "name, status" in one go. -->
        <button
          type="button"
          aria-label={agentEntryName(agent, isStopping)}
          aria-current={isSelected ? "true" : undefined}
          class={cn(
            base,
            isSelected
              ? "border-2 border-blue-700 bg-blue-50"
              : "border-neutral-400 bg-white",
          )}
          onclick={() => onselect(agent.id)}
        >
          <span class="font-medium">{agent.name}</span>
          <span class="text-neutral-700"
            >{shownStatusText(agent, isStopping)}</span
          >
        </button>
      </li>
    {/each}
  </ul>
</section>
