<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { formatTimestamp } from "$shared/utils";
  import {
    actionText,
    actorText,
    resultText,
    type ActionEntry,
  } from "../models/action-entry";

  interface Props {
    entries: ActionEntry[];
  }

  let { entries }: Props = $props();
</script>

<section aria-labelledby="action-record-heading" class="flex flex-col gap-2">
  <h3 id="action-record-heading" class="text-lg font-semibold text-neutral-900">
    Action record
  </h3>
  {#if entries.length === 0}
    <p class="text-neutral-700">No action has been recorded for this agent.</p>
  {:else}
    <ol class="flex flex-col gap-1 text-sm text-neutral-900">
      {#each entries as entry (entry.id)}
        <li>
          <time datetime={new Date(entry.at).toISOString()}>
            {formatTimestamp(entry.at)}
          </time>
          {actionText(entry.action)} by {actorText(entry.actor)}. {resultText(
            entry.result,
          )}
        </li>
      {/each}
    </ol>
  {/if}
</section>
