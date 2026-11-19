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
  import { createLastViewWriter } from "./modules/office-scene/state/last-view";
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

  // The shell owns the open view (0005 D6) and is the one writer of
  // `last_view` (the `view` field, MODIFIED 0003/FR-008), at most once per
  // second and on close. The shell restores the stored view once at the
  // office's first ready, whichever view is open, and only then does
  // recording begin, so the restore's read cannot be clobbered by an eager
  // write.
  const openView = $derived(panel.openView);
  const lastView = createLastViewWriter(client);
  $effect(() => {
    if (panel.office.snapshot.phase === "ready") void panel.restoreLastView();
  });
  // The write rides the connection and the panel's restore, not the mount: a
  // pre-connection shell (not reachable, unauthorized, version mismatch)
  // sends no request at all, as the office and hierarchy loads hold to below.
  // The scene's open desk is the terminal that is open in the scene; the
  // panel's is its selected agent's embedded terminal (D6). The scene
  // reports its desk through the bindable `openAgentId` prop, and the
  // shell derives the panel side per view so the value never outlives the
  // view that owns it.
  let openAgentId = $state<string | null>(null);
  $effect(() => {
    if (!isConnected || !panel.lastView.isRestored) return;
    const terminalOpen =
      openView === "scene"
        ? openAgentId !== null
        : panel.office.snapshot.selectedAgentId !== null &&
          panel.office.snapshot.details !== null;
    lastView.write({
      project: panel.projectChoice,
      // The scene never changes the selection, so the agent is the panel's
      // in both views (0003 FR-008: the value restores the panel's place).
      agent: panel.office.snapshot.selectedAgentId,
      terminal_open: terminalOpen,
      view: openView,
    });
  });
  // The app-close moment flushes a held write (D6). The visibilitychange
  // path covers webview suspension; `pagehide` covers actual teardown.
  function onVisibility(): void {
    if (document.visibilityState === "hidden") lastView.close();
  }
  function onPageHide(): void {
    lastView.close();
  }
  $effect(() => {
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("pagehide", onPageHide);
    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("pagehide", onPageHide);
    };
  });

  onMount(() => {
    void connection.start();
    void panel.office.controller.watch();
    void panel.notifications.watch();
    void hierarchy.watch();
    void osNotificationTap();
  });

  // The office is loaded again on each new connection, because events that
  // arrived while the app was disconnected are lost.
  $effect(() => {
    if (isConnected) {
      void panel.office.controller.load();
      void panel.notifications.load();
      void hierarchy.load();
    }
  });

  /** Opens the tapped notification's agent, when it still exists (D7). */
  async function osNotificationTap(): Promise<void> {
    await panel.notifications.onTap((agentId) => {
      if (panel.office.snapshot.agents.some((agent) => agent.id === agentId)) {
        panel.office.controller.select(agentId);
      }
    });
  }
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
      one step (0005 D6). -->
    <div class="mx-auto flex max-w-6xl px-6 pt-6">
      {#if openView === "panel"}
        <Button variant="secondary" onclick={() => panel.switchView("scene")}
          >To the scene</Button
        >
      {:else}
        <Button variant="secondary" onclick={() => panel.switchView("panel")}
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
        bind:openAgentId
      />
    {/if}
    <LoginStartPanel {client} />
  {/if}
</main>
