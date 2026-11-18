<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Dialog from "$shared/ui/dialog.svelte";
  import { FIELD_CLASS } from "$shared/ui/field-class";
  import { shownStatusText, type Agent, type Project } from "../../agent";
  import { projectNameOf } from "../models/panel";

  interface Props {
    /** Every agent of the office, of every project (FR-004, A1). */
    agents: Agent[];
    projects: Project[];
    onchoose: (agentId: string) => void;
    onclose: () => void;
  }

  let { agents, projects, onchoose, onclose }: Props = $props();

  const id = $props.id();
  const OPTION_CLASS =
    "min-h-11 w-full rounded-md px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";

  let query = $state("");
  let filter = $state<HTMLInputElement | null>(null);
  let options = $state<HTMLElement | null>(null);

  const matches = $derived(
    agents.filter((agent) =>
      agent.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  // The name reads as one sentence, as the panel's rows do (D11).
  function optionName(agent: Agent): string {
    const project = projectNameOf(agent, projects);
    return [
      agent.name,
      ...(project !== null ? [`project ${project}`] : []),
      shownStatusText(agent, false),
    ].join(", ");
  }

  function chooseFirstMatch(): void {
    const first = matches[0];
    if (first !== undefined) onchoose(first.id);
  }

  function handleFilterKeys(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      chooseFirstMatch();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      options?.querySelector("button")?.focus();
    }
  }

  function focusOption(step: number, from: HTMLElement): void {
    const buttons = [
      ...(options?.querySelectorAll<HTMLElement>("button") ?? []),
    ];
    buttons[buttons.indexOf(from) + step]?.focus();
  }

  function handleOptionKeys(event: KeyboardEvent): void {
    const step =
      event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    focusOption(step, event.currentTarget as HTMLElement);
  }
</script>

<Dialog title="Find an agent" {onclose} initialFocus={filter}>
  <div class="flex flex-col gap-3">
    <label for={id} class="text-sm font-medium text-neutral-900">
      Filter by name
    </label>
    <input
      {id}
      bind:this={filter}
      bind:value={query}
      type="text"
      autocomplete="off"
      spellcheck="false"
      class={FIELD_CLASS}
      onkeydown={handleFilterKeys}
    />
    {#if matches.length === 0}
      <p class="text-sm text-neutral-700">No agent matches.</p>
    {:else}
      <ul
        bind:this={options}
        class="flex max-h-80 flex-col gap-1 overflow-y-auto"
      >
        {#each matches as agent (agent.id)}
          <li>
            <button
              type="button"
              class={OPTION_CLASS}
              onclick={() => onchoose(agent.id)}
              onkeydown={handleOptionKeys}
            >
              {optionName(agent)}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="text-sm text-neutral-700">
      Enter opens the first match. Escape closes the switcher.
    </p>
  </div>
</Dialog>
