<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { formatTimestamp } from "$shared/utils";
  import {
    actionText,
    actorText,
    resultText,
    type ActionEntry,
  } from "../../audit";
  import type { HoldController } from "../state/hold-controller";

  interface Props {
    controller: HoldController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  let entries = $state<ActionEntry[] | null>(null);
  let failure = $state<string | null>(null);

  onMount(async () => {
    try {
      entries = await controller.officeActions();
    } catch (rejection) {
      failure = controller.describeFailure(rejection);
    }
  });
</script>

<Dialog title="Office record" {onclose}>
  {#if failure !== null}
    <FailureMessage lead="The office record did not load." message={failure} />
  {:else if entries === null}
    <p role="status" class="text-neutral-700">Loading the office record.</p>
  {:else if entries.length === 0}
    <p class="text-neutral-700">No office hold or release has been recorded.</p>
  {:else}
    <ol class="flex max-h-80 flex-col gap-1 overflow-y-auto text-sm">
      {#each entries as entry (entry.id)}
        <li>
          <time datetime={new Date(entry.at).toISOString()}>
            {formatTimestamp(entry.at)}
          </time>
          {actionText(entry.action)} by {actorText(
            entry.actor,
            entry.actor_label,
          )}. {resultText(entry.result)}
        </li>
      {/each}
    </ol>
  {/if}
  <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
</Dialog>
