<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import { toCoreRequestError, type CoreClient } from "../../../core/api";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";

  interface Props {
    client: CoreClient;
  }

  let { client }: Props = $props();

  // `null` while the core has not answered yet.
  let isRegistered = $state<boolean | null>(null);
  let isRegistering = $state(false);
  let failure = $state<string | null>(null);

  const statusText = $derived(
    isRegistered === null
      ? "Checking whether the server starts at login."
      : isRegistered
        ? "The server starts when you log in."
        : "The server does not start when you log in.",
  );

  async function refreshStatus(): Promise<void> {
    try {
      isRegistered = await client.loginStartStatus();
    } catch (rejection) {
      failure = toCoreRequestError(rejection).message;
    }
  }

  async function register(): Promise<void> {
    isRegistering = true;
    failure = null;
    try {
      await client.registerLoginStart();
      isRegistered = true;
    } catch (rejection) {
      failure = toCoreRequestError(rejection).message;
    } finally {
      isRegistering = false;
    }
  }

  onMount(() => void refreshStatus());
</script>

<section
  aria-labelledby="login-start-heading"
  class="mx-auto flex max-w-5xl flex-col items-start gap-3 px-6 pb-6"
>
  <h2 id="login-start-heading" class="text-xl font-semibold text-neutral-900">
    Server at login
  </h2>
  <p class="text-neutral-700">
    Agents run only while the office server runs. Registering the server keeps
    it running without this app, from your next login on.
  </p>
  <p aria-live="polite" class="text-sm text-neutral-700">{statusText}</p>
  {#if failure !== null}
    <FailureMessage lead="The server was not registered." message={failure} />
  {/if}
  <Button onclick={() => void register()} disabled={isRegistering}>
    {isRegistering ? "Registering the server" : "Start the server at login"}
  </Button>
</section>
