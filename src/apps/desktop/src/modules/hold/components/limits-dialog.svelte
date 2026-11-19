<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import {
    firstLimitProblem,
    LIMIT_MAX,
    LIMIT_MIN,
    LIMIT_SETTINGS,
  } from "../models/limits";
  import type { HoldController } from "../state/hold-controller";

  interface Props {
    controller: HoldController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  const ERROR_ID = "limits-error";

  let values = $state<Record<string, string> | null>(null);
  let failure = $state<string | null>(null);
  /** The limit whose value the client refused, so its field points at the error. */
  let invalidKey = $state<string | null>(null);
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

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (values === null) return;
    const refused = firstLimitProblem(values);
    invalidKey = refused?.limit.key ?? null;
    if (refused !== null) {
      failure = `${refused.limit.label}: ${refused.problem}`;
      return;
    }
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
          errorId={invalidKey === limit.key ? ERROR_ID : undefined}
          bind:value={values[limit.key]}
        />
      {/each}
      {#if failure !== null}
        <FailureMessage
          id={ERROR_ID}
          lead="The limits were not saved."
          message={failure}
        />
      {/if}
      <div class="flex flex-wrap gap-3">
        <Button type="submit" disabled={isSaving}>Save limits</Button>
        <Button variant="secondary" onclick={onclose}>Cancel</Button>
      </div>
    </form>
  {/if}
</Dialog>
