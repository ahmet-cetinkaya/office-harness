<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import type { Phase } from "../state/file-view.svelte";

  interface Props {
    phase: Phase;
    failure: string | null;
    loadingText: string;
    failureLead: string;
    emptyText: string;
    onrefresh: () => void;
  }

  let {
    phase,
    failure,
    loadingText,
    failureLead,
    emptyText,
    onrefresh,
  }: Props = $props();
</script>

{#if phase === "loading"}
  <p role="status" class="text-neutral-700">{loadingText}</p>
{:else if phase === "failed"}
  <div class="flex flex-col items-start gap-3">
    <FailureMessage lead={failureLead} message={failure ?? ""} />
    <Button variant="secondary" onclick={onrefresh}>Refresh</Button>
  </div>
{:else if phase === "empty"}
  <p role="status" class="text-neutral-700">{emptyText}</p>
{/if}
