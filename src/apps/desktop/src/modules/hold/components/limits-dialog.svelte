<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import {
    LIMIT_MAX,
    LIMIT_MIN,
    LIMIT_SETTINGS,
    limitProblem,
  } from "../models/limits";
  import type { HoldController } from "../state/hold-controller";

  interface Props {
    controller: HoldController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  let values = $state<Record<string, string> | null>(null);
  let failure = $state<string | null>(null);
  let isSaving = $state(false);
  let form = $state<HTMLFormElement | null>(null);

  onMount(async () => {
    try {
      values = await controller.readLimits();
    } catch (rejection) {
      failure = controller.describeFailure(rejection);
    }
  });

  // The fields render after the dialog mounts, so the first focus waits.
  $effect(() => {
    form?.querySelector<HTMLElement>("input")?.focus();
  });

  /** The first limit that the client refuses, as "label: reason", or null. */
  function firstProblem(current: Record<string, string>): string | null {
    for (const limit of LIMIT_SETTINGS) {
      const problem = limitProblem(current[limit.key]);
      if (problem !== null) return `${limit.label}: ${problem}`;
    }
    return null;
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (values === null) return;
    failure = firstProblem(values);
    if (failure !== null) return;
    isSaving = true;
    failure = await controller.saveLimits(values);
    isSaving = false;
    if (failure === null) onclose();
  }
</script>

<Dialog title="Limits" {onclose}>
  {#if values === null && failure === null}
    <p role="status" class="text-neutral-700">Loading the limits.</p>
  {:else if values === null}
    <FailureMessage lead="The limits did not load." message={failure ?? ""} />
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  {:else}
    <form bind:this={form} class="flex flex-col gap-4" onsubmit={save}>
      <p class="text-sm text-neutral-700">
        Each limit counts the actions of agents within 60 seconds.
      </p>
      {#each LIMIT_SETTINGS as limit (limit.key)}
        <TextField
          label={limit.label}
          hint={`Default ${limit.defaultValue}. A whole number from ${LIMIT_MIN} to ${LIMIT_MAX}.`}
          bind:value={values[limit.key]}
        />
      {/each}
      {#if failure !== null}
        <FailureMessage lead="The limits were not saved." message={failure} />
      {/if}
      <div class="flex flex-wrap gap-3">
        <Button type="submit" disabled={isSaving}>Save limits</Button>
        <Button variant="secondary" onclick={onclose}>Cancel</Button>
      </div>
    </form>
  {/if}
</Dialog>
