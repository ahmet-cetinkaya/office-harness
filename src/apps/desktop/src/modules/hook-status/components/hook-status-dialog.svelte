<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    SWITCH_OFF_TEXT,
    SWITCH_ON_TEXT,
    HOOK_STATUS_HELP_TEXT,
  } from "../models/hook-status";
  import type { HookStatusController } from "../state/hook-status-controller";

  interface Props {
    controller: HookStatusController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  const HELP_ID = "hook-status-help";
  const ERROR_ID = "hook-status-error";

  let isOn = $state<boolean | null>(null);
  let failure = $state<string | null>(null);
  let isSaving = $state(false);
  let formElement = $state<HTMLFormElement | null>(null);

  onMount(async () => {
    try {
      isOn = await controller.readIsOn();
    } catch (rejection) {
      failure = controller.describeFailure(rejection);
    }
  });

  // The form renders after the dialog mounts, so the first focus waits.
  $effect(() => {
    formElement?.querySelector<HTMLElement>("input")?.focus();
  });

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isOn === null) return;
    isSaving = true;
    failure = await controller.save(isOn);
    isSaving = false;
    if (failure === null) onclose();
  }
</script>

<Dialog title="Status from CLI hooks" {onclose}>
  {#if isOn === null && failure === null}
    <p role="status" class="text-neutral-700">Loading the status setting.</p>
  {:else if isOn === null}
    <FailureMessage
      lead="The status setting did not load."
      message={failure ?? ""}
    />
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  {:else}
    <form bind:this={formElement} class="flex flex-col gap-4" onsubmit={save}>
      <p id={HELP_ID} class="text-sm text-neutral-700">
        {HOOK_STATUS_HELP_TEXT}
      </p>
      <label class="flex items-start gap-3 text-sm text-neutral-900">
        <input
          type="checkbox"
          role="switch"
          aria-describedby={failure === null
            ? HELP_ID
            : `${HELP_ID} ${ERROR_ID}`}
          class="mt-0.5 size-5 shrink-0 accent-blue-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
          bind:checked={isOn}
        />
        Status from CLI hooks
      </label>
      <p class="text-sm text-neutral-900">
        {isOn ? SWITCH_ON_TEXT : SWITCH_OFF_TEXT}
      </p>
      {#if failure !== null}
        <FailureMessage
          id={ERROR_ID}
          lead="Status from CLI hooks was not saved."
          message={failure}
        />
      {/if}
      <div class="flex flex-wrap gap-3">
        <Button type="submit" disabled={isSaving}>Save</Button>
        <Button variant="secondary" onclick={onclose}>Cancel</Button>
      </div>
    </form>
  {/if}
</Dialog>
