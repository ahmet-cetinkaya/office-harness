<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    ACKNOWLEDGEMENT_LABEL,
    ACKNOWLEDGEMENT_NEEDED_TEXT,
    confirmBlock,
    hasUncleanStopMark,
    marksOf,
    NOTHING_CHECKED_TEXT,
    planText,
    resultText,
    UNCLEAN_STOP_WARNING,
    WAITING_FOR_RESULT_TEXT,
    type AgentIdentity,
  } from "../models/resume";
  import {
    INITIAL_RESUME,
    type ResumeController,
  } from "../state/resume-controller";

  interface Props {
    controller: ResumeController;
    identify: (agentId: string) => AgentIdentity;
    onclose: () => void;
  }

  let { controller, identify, onclose }: Props = $props();

  const uid = $props.id();
  const warningId = `${uid}-warning`;
  let snapshot = $state(INITIAL_RESUME);
  let checkboxes = $state<(HTMLInputElement | null)[]>([]);
  let closeButton = $state<HTMLButtonElement | null>(null);

  const block = $derived(
    confirmBlock(
      snapshot.entries,
      snapshot.checkedIds,
      snapshot.isAcknowledged,
    ),
  );
  const hasWarning = $derived(hasUncleanStopMark(snapshot.entries));
  // The first check box takes focus once the list is in; before and after
  // the list, Close does.
  const initialFocus = $derived(checkboxes[0] ?? closeButton);

  $effect(() => controller.subscribe((next) => (snapshot = next)));
  onMount(() => void controller.open());
</script>

<Dialog title="Resume interrupted agents" {onclose} {initialFocus}>
  {#if snapshot.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the resume plan.</p>
  {:else if snapshot.phase === "failed"}
    <FailureMessage
      lead="The resume plan did not load."
      message={snapshot.failure ?? ""}
    />
  {:else if snapshot.phase === "choosing"}
    <ul class="flex max-h-80 flex-col gap-3 overflow-y-auto">
      {#each snapshot.entries as entry, index (entry.agent_id)}
        {@const agent = identify(entry.agent_id)}
        <li>
          <label class="flex items-start gap-3 text-sm text-neutral-900">
            <input
              type="checkbox"
              class="mt-0.5 size-5 shrink-0 accent-blue-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
              checked={snapshot.checkedIds.includes(entry.agent_id)}
              onchange={() => controller.toggle(entry.agent_id)}
              bind:this={checkboxes[index]}
            />
            <span class="flex flex-col">
              <span><strong>{agent.name}</strong>, {agent.role}</span>
              <span class="text-neutral-700">{planText(entry)}</span>
              {#each marksOf(entry) as mark (mark)}
                <span class="font-medium text-neutral-900">{mark}</span>
              {/each}
            </span>
          </label>
        </li>
      {/each}
    </ul>
    {#if hasWarning}
      <p
        id={warningId}
        class="rounded-md border border-neutral-400 p-3 text-sm text-neutral-900"
      >
        <strong>Warning.</strong>
        {UNCLEAN_STOP_WARNING}
      </p>
      <label class="flex items-start gap-3 text-sm text-neutral-900">
        <input
          type="checkbox"
          class="mt-0.5 size-5 shrink-0 accent-blue-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
          checked={snapshot.isAcknowledged}
          onchange={(event) =>
            controller.acknowledge(event.currentTarget.checked)}
        />
        {ACKNOWLEDGEMENT_LABEL}
      </label>
    {/if}
    {#if snapshot.failure !== null}
      <FailureMessage
        lead="The resume did not start."
        message={snapshot.failure}
      />
    {/if}
    {#if block === "nothing_checked"}
      <p class="text-sm text-neutral-700">{NOTHING_CHECKED_TEXT}</p>
    {:else if block === "needs_acknowledgement"}
      <p class="text-sm text-neutral-700">{ACKNOWLEDGEMENT_NEEDED_TEXT}</p>
    {/if}
  {:else}
    <div role="status" aria-label="Resume results">
      <ul
        class="flex max-h-80 flex-col gap-2 overflow-y-auto text-sm text-neutral-900"
      >
        {#each snapshot.sentIds as agentId (agentId)}
          {@const result = snapshot.results[agentId]}
          <li>
            <strong>{identify(agentId).name}</strong>:
            {result === undefined
              ? WAITING_FOR_RESULT_TEXT
              : resultText(result)}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
  <div class="flex flex-wrap gap-3">
    {#if snapshot.phase === "choosing"}
      <Button
        disabled={block !== null || snapshot.isStarting}
        onclick={() => void controller.confirm()}
        describedby={hasWarning ? warningId : undefined}
      >
        Confirm
      </Button>
    {/if}
    <Button variant="secondary" onclick={onclose} bind:element={closeButton}>
      Close
    </Button>
  </div>
</Dialog>
