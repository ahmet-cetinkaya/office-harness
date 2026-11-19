<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { shownStatusText, type Agent } from "../../agent";
  import { AGENT_ON_HOLD_TEXT } from "../../hold";
  import { hierarchyText, type AgentHierarchy } from "../models/panel";

  interface Props {
    agent: Agent;
    projectName: string | null;
    hierarchy: AgentHierarchy;
    /** True while the stop request of this agent is open. */
    isStopping: boolean;
    isSelected: boolean;
    /** True while the agent is on hold; the row says so as text. */
    isOnHold: boolean;
    onopen: (agentId: string) => void;
  }

  let {
    agent,
    projectName,
    hierarchy,
    isStopping,
    isSelected,
    isOnHold,
    onopen,
  }: Props = $props();

  const hierarchyLine = $derived(hierarchyText(hierarchy));

  // The name is set by hand so that it reads as one sentence in the row's
  // reading order (D11). The last line is visible text, so the name carries
  // it too; axe fails a label that leaves visible text out.
  const entryName = $derived(
    [
      agent.name,
      ...(projectName !== null ? [`project ${projectName}`] : []),
      shownStatusText(agent, isStopping),
      ...(isOnHold ? [AGENT_ON_HOLD_TEXT] : []),
      ...(hierarchyLine === "" ? [] : [hierarchyLine]),
      ...(agent.last_line !== null ? [agent.last_line] : []),
    ].join(", "),
  );

  const base =
    "flex min-h-11 w-full flex-col items-start rounded-md border px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";
</script>

<li>
  <button
    type="button"
    aria-label={entryName}
    aria-current={isSelected ? "true" : undefined}
    class={base}
    onclick={() => onopen(agent.id)}
  >
    <span class="flex w-full flex-wrap items-center gap-x-2">
      <span class="font-medium">{agent.name}</span>
      {#if projectName !== null}
        <span class="text-neutral-700">project {projectName}</span>
      {/if}
      <span class="text-neutral-700">{shownStatusText(agent, isStopping)}</span>
      {#if isOnHold}
        <span class="font-medium text-neutral-900">{AGENT_ON_HOLD_TEXT}</span>
      {/if}
      {#if hierarchyLine !== ""}
        <span class="text-neutral-700">{hierarchyLine}</span>
      {/if}
    </span>
    {#if agent.last_line !== null}
      <span class="line-clamp-1 text-neutral-700">{agent.last_line}</span>
    {/if}
  </button>
</li>
