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
  import { createPanelStore, PanelView } from "./modules/panel";
  import { SceneView } from "./modules/office-scene";
  import {
    createHierarchyController,
    HierarchyView,
  } from "./modules/hierarchy";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import StatusPanel from "$shared/ui/status-panel.svelte";
  import Button from "$shared/ui/button.svelte";

  const client = createTauriCoreClient();
  const connection = createConnectionStore(client);
  const panel = createPanelStore(client);
  const hierarchy = createHierarchyController(client);
  const snapshot = $derived(connection.snapshot);
  const isConnected = $derived(snapshot.state.status === "connected");

  // The shell owns the open view (0005 D6); the last-view write and its
  // restore on start land with the scene's part 2 (T4).
  let openView: "panel" | "scene" = $state("panel");

  onMount(() => {
    void connection.start();
    void panel.office.controller.watch();
    void hierarchy.watch();
  });

  // The office is loaded again on each new connection, because events that
  // arrived while the app was disconnected are lost.
  $effect(() => {
    if (isConnected) {
      void panel.office.controller.load();
      void hierarchy.load();
    }
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
    <!-- The shell owns the view switch, so both views offer the other in
      one step (0005 D6). The last-view write lands with the scene's T4. -->
    <div class="mx-auto flex max-w-6xl px-6 pt-6">
      {#if openView === "panel"}
        <Button variant="secondary" onclick={() => (openView = "scene")}
          >To the scene</Button
        >
      {:else}
        <Button variant="secondary" onclick={() => (openView = "panel")}
          >To the panel</Button
        >
      {/if}
    </div>
    {#if openView === "panel"}
      <PanelView store={panel} {hierarchy}>
        {#snippet side()}
          <HierarchyView
            controller={hierarchy}
            agents={panel.office.snapshot.agents}
            projects={panel.office.snapshot.projects}
            selectedAgentId={panel.office.snapshot.selectedAgentId}
            onselect={panel.office.controller.select}
          />
        {/snippet}
      </PanelView>
    {:else}
      <SceneView
        store={panel.office}
        {hierarchy}
        projectChoice={panel.projectChoice}
        onProjectChoice={panel.setProjectChoice}
      />
    {/if}
    <LoginStartPanel {client} />
  {/if}
</main>
