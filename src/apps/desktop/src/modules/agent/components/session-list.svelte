<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import { formatTimestamp } from "$shared/utils";
  import type { Session } from "../models/agent";

  interface Props {
    sessions: Session[];
    isBusy: boolean;
    oncontinue: (sessionId: string) => void;
  }

  let { sessions, isBusy, oncontinue }: Props = $props();

  const NO_REASON = "The CLI cannot continue this session.";

  function endText(session: Session): string {
    if (session.ended_at === null) return "Running";
    const result = session.exit_result ?? "unknown";
    return `Ended ${formatTimestamp(session.ended_at)}, result ${result}`;
  }
</script>

<section aria-labelledby="sessions-heading" class="flex flex-col gap-2">
  <h3 id="sessions-heading" class="text-lg font-semibold text-neutral-900">
    Sessions
  </h3>
  {#if sessions.length === 0}
    <p class="text-neutral-700">This agent has not run yet.</p>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each sessions as session (session.id)}
        <li class="flex flex-wrap items-center gap-3 text-sm text-neutral-900">
          <span>
            Started {formatTimestamp(session.started_at)}. {endText(session)}.
          </span>
          {#if session.can_continue}
            <Button
              variant="secondary"
              disabled={isBusy}
              onclick={() => oncontinue(session.id)}
            >
              Continue session from {formatTimestamp(session.started_at)}
            </Button>
          {:else}
            <span class="text-neutral-700">
              Cannot continue: {session.reason ?? NO_REASON}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
