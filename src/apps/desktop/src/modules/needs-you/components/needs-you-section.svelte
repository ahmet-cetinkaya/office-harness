<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { EMPTY_TEXT, type NeedsYouRow } from "../models/needs-you";
  import type { DismissFailure } from "../state/needs-you-controller";

  interface Props {
    rows: NeedsYouRow[];
    /** The number of items, as text beside the heading (FR-009). */
    count: number;
    /** True while the focus shortcut holds the jump key (0008 D6). */
    isJumpUnbound: boolean;
    ondismiss: (agentId: string) => Promise<DismissFailure>;
  }

  let { rows, count, isJumpUnbound, ondismiss }: Props = $props();

  const HEADING_ID = "needs-you-heading";
  const COLLISION_TEXT =
    "The jump shortcut is off because the focus shortcut uses Ctrl+Shift+F11. Change the focus_shortcut setting (see the README) and reopen the app to turn the jump on.";

  let failure = $state<DismissFailure>(null);
  let heading = $state<HTMLHeadingElement | null>(null);

  // The dismissed row's button is gone, so focus would fall to the page.
  async function dismiss(agentId: string): Promise<void> {
    const request = ondismiss(agentId);
    heading?.focus();
    failure = await request;
  }
</script>

<section aria-labelledby={HEADING_ID} class="flex flex-col gap-2">
  <h2
    id={HEADING_ID}
    bind:this={heading}
    tabindex="-1"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    Needs you
    {#if count > 0}
      <span class="font-normal text-neutral-700">({count})</span>
    {/if}
  </h2>
  {#if isJumpUnbound}
    <p class="text-sm text-neutral-900">{COLLISION_TEXT}</p>
  {/if}
  {#if failure !== null}
    <FailureMessage lead="The item did not leave." message={failure} />
  {/if}
  {#if rows.length === 0}
    <p class="text-neutral-700">{EMPTY_TEXT}</p>
  {:else}
    <ul class="flex max-h-72 flex-col gap-2 overflow-y-auto">
      {#each rows as row (row.agentId)}
        <li>
          <div
            role="group"
            aria-label={row.label}
            class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-neutral-400 px-3 py-2 text-sm text-neutral-900"
          >
            <span class="flex flex-col">
              <span class="font-medium">{row.agentName}</span>
              {#if row.projectName !== null}
                <span class="text-neutral-700">project {row.projectName}</span>
              {/if}
              <span>{row.event}</span>
              <span class="text-neutral-700">{row.wait}</span>
            </span>
            <Button
              variant="secondary"
              onclick={() => void dismiss(row.agentId)}
            >
              <span aria-hidden="true">Dismiss</span>
              <span class="sr-only">Dismiss {row.agentName}</span>
            </Button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
