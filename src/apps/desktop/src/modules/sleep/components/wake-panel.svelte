<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import {
    ASLEEP_TEXT,
    isWakeBusy,
    wakeText,
    type WakeEntry,
  } from "../models/sleep";
  import {
    INITIAL_SLEEP,
    type SleepController,
  } from "../state/sleep-controller";

  interface Props {
    controller: SleepController;
    agentId: string;
    /** True while the agent is asleep; the text and Wake show only then. */
    isAsleep: boolean;
  }

  let { controller, agentId, isAsleep }: Props = $props();

  let snapshot = $state(INITIAL_SLEEP);
  const entry = $derived<WakeEntry | undefined>(snapshot.wakes[agentId]);

  $effect(() => controller.subscribe((next) => (snapshot = next)));
</script>

<div class="flex flex-col gap-2">
  {#if isAsleep}
    <p class="text-sm text-neutral-900">{ASLEEP_TEXT}</p>
    <div>
      <Button
        disabled={isWakeBusy(entry)}
        onclick={() => void controller.wake(agentId)}>Wake</Button
      >
    </div>
  {/if}
  <!-- Always in the page, so that a screen reader hears the first result. -->
  <p role="status" aria-label="Wake result" class="text-sm text-neutral-900">
    {entry === undefined ? "" : wakeText(entry)}
  </p>
</div>
