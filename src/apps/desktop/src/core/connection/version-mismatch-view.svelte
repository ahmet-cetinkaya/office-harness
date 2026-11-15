<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import StatusPanel from "$shared/ui/status-panel.svelte";
  import type { ActionProgress } from "./connection-controller";

  interface Props {
    serverVersion: string;
    appVersion: string;
    canRestart: boolean;
    action: ActionProgress;
    onrestart: () => void;
  }

  let { serverVersion, appVersion, canRestart, action, onrestart }: Props =
    $props();

  let isConfirming = $state(false);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  const isRestarting = $derived(action.phase === "running");

  // Moving focus into the confirmation keeps a keyboard user on the choice.
  $effect(() => {
    if (isConfirming) cancelButton?.focus();
  });

  function confirmRestart(): void {
    isConfirming = false;
    onrestart();
  }

  function cancelOnEscape(event: KeyboardEvent): void {
    if (event.key === "Escape") isConfirming = false;
  }
</script>

<StatusPanel title="The server has another version">
  <p class="text-neutral-700">
    The app is version {appVersion} and the running server is version {serverVersion}.
    They cannot work together until the server runs the same version.
  </p>
  {#if action.phase === "failed"}
    <FailureMessage
      lead="The server did not restart."
      message={action.message}
    />
  {/if}
  {#if !canRestart}
    <p class="text-neutral-700">
      This server speaks a protocol that the app does not know, so the app
      cannot ask it to restart. Stop the server program yourself, then reopen
      the app.
    </p>
  {:else if isConfirming}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      role="group"
      aria-labelledby="restart-warning"
      class="flex flex-col gap-3 rounded-md border border-neutral-400 p-4"
      onkeydown={cancelOnEscape}
    >
      <p id="restart-warning" class="text-neutral-900">
        Restarting the server stops every running agent and ends its session.
        The server stops, then you start it again. Press Escape to cancel.
      </p>
      <div class="flex flex-wrap gap-3">
        <Button onclick={confirmRestart}>Restart and stop agents</Button>
        <Button
          variant="secondary"
          onclick={() => (isConfirming = false)}
          bind:element={cancelButton}
        >
          Cancel
        </Button>
      </div>
    </div>
  {:else}
    <div>
      <Button onclick={() => (isConfirming = true)} disabled={isRestarting}>
        {isRestarting ? "Stopping the server" : "Restart server"}
      </Button>
    </div>
  {/if}
  {#if isRestarting}
    <p role="status" class="text-sm text-neutral-700">
      Stopping the server. The app then shows its Start action.
    </p>
  {/if}
</StatusPanel>
