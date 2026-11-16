<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import StatusPanel from "$shared/ui/status-panel.svelte";
  import type { ActionProgress } from "./connection-controller";

  interface Props {
    action: ActionProgress;
    onstart: () => void;
  }

  let { action, onstart }: Props = $props();
  const isStarting = $derived(action.phase === "running");
</script>

<StatusPanel title="The office server is not running">
  <p class="text-neutral-700">
    The app cannot reach the office server. Agents keep running only while the
    server runs. Start it here, and it keeps running after you close the app.
  </p>
  {#if action.phase === "failed"}
    <FailureMessage lead="The server did not start." message={action.message} />
  {/if}
  <div>
    <Button onclick={onstart} disabled={isStarting}>
      {isStarting ? "Starting the server" : "Start server"}
    </Button>
  </div>
  {#if isStarting}
    <p role="status" class="text-sm text-neutral-700">
      Starting the server. This takes a few seconds.
    </p>
  {/if}
</StatusPanel>
