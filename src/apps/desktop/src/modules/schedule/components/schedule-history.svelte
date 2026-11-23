<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    dueText,
    NO_HISTORY_TEXT,
    resultText,
    viaText,
    type Schedule,
  } from "../models/schedule";
  import type { HistorySnapshot } from "../state/schedules-controller";

  interface Props {
    schedule: Schedule;
    history: HistorySnapshot | null;
    onclose: () => void;
  }

  let { schedule, history, onclose }: Props = $props();
</script>

<Dialog title={`History of ${schedule.name}`} size="wide" {onclose}>
  <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
    <p class="text-sm text-neutral-700">The last 20 runs, newest first.</p>
    {#if history === null || history.isLoading}
      <p role="status" class="text-neutral-700">Loading the history.</p>
    {:else if history.failure !== null}
      <FailureMessage
        lead="The history did not load."
        message={history.failure}
      />
    {:else if history.runs.length === 0}
      <p class="text-sm text-neutral-700">{NO_HISTORY_TEXT}</p>
    {:else}
      <ul class="flex flex-col gap-2">
        {#each history.runs as run (run.id)}
          <li class="rounded-md border border-neutral-400 p-3">
            <dl
              class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm text-neutral-900"
            >
              <dt class="font-medium">Due</dt>
              <dd>{dueText(run.due)}</dd>
              <dt class="font-medium">Started</dt>
              <dd>{dueText(run.started)}</dd>
              <dt class="font-medium">By</dt>
              <dd>{viaText(run.via)}</dd>
              <dt class="font-medium">Result</dt>
              <dd>{resultText(run.result)}</dd>
              {#if run.reason !== null}
                <dt class="font-medium">Reason</dt>
                <dd class="break-words">{run.reason}</dd>
              {/if}
            </dl>
          </li>
        {/each}
      </ul>
    {/if}
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  </div>
</Dialog>
