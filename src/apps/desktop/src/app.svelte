<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import { createTauriCoreClient } from "./core/api";
  import {
    createConnectionStore,
    BusyView,
    LoadingView,
    NotReachableView,
    VersionMismatchView,
  } from "./core/connection";
  import { LoginStartPanel, NotAuthorizedView } from "./modules/access";
  import { createOfficeStore, OfficeView } from "./modules/agent";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import StatusPanel from "$shared/ui/status-panel.svelte";

  const client = createTauriCoreClient();
  const connection = createConnectionStore(client);
  const office = createOfficeStore(client);
  const snapshot = $derived(connection.snapshot);
  const isConnected = $derived(snapshot.state.status === "connected");

  onMount(() => {
    void connection.start();
    void office.controller.watch();
  });

  // The office is loaded again on each new connection, because events that
  // arrived while the app was disconnected are lost.
  $effect(() => {
    if (isConnected) void office.controller.load();
  });
</script>

<main>
  {#if snapshot.coreError !== null}
    <StatusPanel title="The app core did not answer">
      <FailureMessage
        lead="No connection to the desktop core."
        message="Open office-harness from its own window. A browser tab has no desktop core."
      />
    </StatusPanel>
  {:else if snapshot.state.status === "loading"}
    <LoadingView />
  {:else if snapshot.state.status === "busy"}
    <BusyView />
  {:else if snapshot.state.status === "not_reachable"}
    <NotReachableView
      action={snapshot.action}
      onstart={connection.startServer}
    />
  {:else if snapshot.state.status === "not_authorized"}
    <NotAuthorizedView onretry={connection.retry} />
  {:else if snapshot.state.status === "version_mismatch"}
    <VersionMismatchView
      serverVersion={snapshot.state.server_version}
      appVersion={snapshot.state.app_version}
      canRestart={snapshot.state.can_restart}
      action={snapshot.action}
      onrestart={connection.restartServer}
    />
  {:else}
    <OfficeView store={office} />
    <LoginStartPanel {client} />
  {/if}
</main>
