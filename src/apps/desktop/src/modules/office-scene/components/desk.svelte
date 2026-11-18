<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { shownStatusText, type Agent } from "../../agent";

  interface Props {
    agent: Agent;
    /** The agent's project's name, or null on the no-project floor. */
    projectName: string | null;
    /** True while the agent's stop request is open. */
    isStopping: boolean;
    /** Moves focus within the floor on an arrow key (D9). */
    onnavigate?: (event: KeyboardEvent) => void;
  }

  let { agent, projectName, isStopping, onnavigate }: Props = $props();

  // The name is set by hand so that it reads as one sentence (D4). It
  // carries everything the desk shows, as in the panel row, so axe accepts
  // the label.
  const entryName = $derived(
    [
      agent.name,
      ...(projectName !== null ? [`project ${projectName}`] : []),
      shownStatusText(agent, isStopping),
      ...(agent.last_line !== null ? [agent.last_line] : []),
    ].join(", "),
  );
</script>

<button
  type="button"
  data-agent-id={agent.id}
  aria-label={entryName}
  class="flex w-full flex-col items-start gap-1 rounded-md border border-neutral-400 bg-white px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  onkeydown={onnavigate}
>
  <span class="w-full truncate font-medium">{agent.name}</span>
  {#if projectName !== null}
    <span class="text-neutral-700">project {projectName}</span>
  {/if}
  <span class="text-neutral-700">{shownStatusText(agent, isStopping)}</span>
  {#if agent.last_line !== null}
    <span class="w-full truncate text-neutral-700">{agent.last_line}</span>
  {/if}
</button>
