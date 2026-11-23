<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import {
    IDLE_MINUTES_MAX,
    IDLE_MINUTES_MIN,
    IDLE_SLEEP_HELP_TEXT,
    type IdleSleepForm,
  } from "../models/sleep";
  import type { SleepController } from "../state/sleep-controller";

  interface Props {
    controller: SleepController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  const ERROR_ID = "idle-sleep-error";

  let form = $state<IdleSleepForm | null>(null);
  let failure = $state<string | null>(null);
  let isSaving = $state(false);
  let formElement = $state<HTMLFormElement | null>(null);

  onMount(async () => {
    try {
      form = await controller.readForm();
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
    if (form === null) return;
    isSaving = true;
    failure = await controller.saveForm(form);
    isSaving = false;
    if (failure === null) onclose();
  }
</script>

<Dialog title="Idle sleep" {onclose}>
  {#if form === null && failure === null}
    <p role="status" class="text-neutral-700">
      Loading the idle sleep setting.
    </p>
  {:else if form === null}
    <FailureMessage
      lead="The idle sleep setting did not load."
      message={failure ?? ""}
    />
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  {:else}
    <form bind:this={formElement} class="flex flex-col gap-4" onsubmit={save}>
      <p class="text-sm text-neutral-700">{IDLE_SLEEP_HELP_TEXT}</p>
      <label class="flex items-start gap-3 text-sm text-neutral-900">
        <input
          type="checkbox"
          role="switch"
          class="mt-0.5 size-5 shrink-0 accent-blue-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
          bind:checked={form.isOn}
        />
        Put idle agents to sleep
      </label>
      {#if form.isOn}
        <TextField
          label="Idle time in minutes"
          hint={`A whole number of minutes from ${IDLE_MINUTES_MIN} to ${IDLE_MINUTES_MAX}.`}
          errorId={failure === null ? undefined : ERROR_ID}
          bind:value={form.minutes}
        />
      {/if}
      {#if failure !== null}
        <FailureMessage
          id={ERROR_ID}
          lead="Idle sleep was not saved."
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
