<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    AgentDetails,
    AgentFormDialog,
    ConfirmDialog,
    ProjectFormDialog,
    ProjectList,
    type Project,
  } from "../../agent";
  import { INITIAL_HIERARCHY, type HierarchyController } from "../../hierarchy";
  import {
    automaticHoldText,
    holdOfAgent,
    INITIAL_HOLD,
    OfficeHoldBar,
    type FailureText,
    type HoldSnapshot,
  } from "../../hold";
  import {
    INITIAL_NOTIFICATIONS,
    NotificationDialog,
    NotificationLiveRegion,
    rowText,
    UnreadButton,
    unreadCount,
    type NotificationSnapshot,
  } from "../../notification";
  import { pressedAppShortcut } from "../../../core/shortcuts";
  import type { PanelStore } from "../state/panel-store.svelte";
  import AgentRow from "../components/agent-row.svelte";
  import AgentSwitcherDialog from "../components/agent-switcher-dialog.svelte";
  import ProjectSwitcher from "../components/project-switcher.svelte";
  import {
    agentHierarchy,
    effectiveChoice,
    isEmptyProject,
    projectNameOf,
    visibleAgents,
  } from "../models/panel";

  interface Props {
    store: PanelStore;
    /** The 0002 tree that gives each row its role and team (D10). */
    hierarchy: HierarchyController;
    /** Extra content of the left column, above the agent list. */
    side?: Snippet;
  }

  let { store, hierarchy: treeController, side }: Props = $props();

  const office = $derived(store.office.snapshot);
  const controller = $derived(store.office.controller);
  const client = $derived(store.office.client);
  // The controller's snapshot is a plain value, so the view subscribes and
  // keeps the last seen one in runes state; the effect's cleanup stops the
  // subscription when the view unmounts. Subscribe calls back with the
  // current snapshot at once, so `treeSnapshot` starts at the controller's.
  let treeSnapshot = $state(INITIAL_HIERARCHY);
  $effect(() => treeController.subscribe((next) => (treeSnapshot = next)));
  // The notification snapshot is a plain value too, so it subscribes the
  // same way (see the tree comment above).
  let notificationSnapshot = $state<NotificationSnapshot>(
    INITIAL_NOTIFICATIONS,
  );
  $effect(() =>
    store.notifications.subscribe((next) => (notificationSnapshot = next)),
  );

  // The hold snapshot is a plain value too, so it subscribes the same way.
  let holdSnapshot = $state<HoldSnapshot>(INITIAL_HOLD);
  $effect(() => store.hold.subscribe((next) => (holdSnapshot = next)));
  const holds = $derived(holdSnapshot.state);
  let officeHoldFailure = $state<FailureText>(null);
  /** The last agent hold refusal and the agent it came from, so that it
   * does not show on the details of another agent. */
  let agentHoldFailure = $state<{ agentId: string; text: string } | null>(null);

  // One live region speaks for both sources; the newest change replaces the
  // text. Each source publishes a new snapshot for reasons that are not a new
  // announcement (a read, a pending flag), and its old announcement stays in
  // it, so a source speaks only when its own announcement changed.
  let liveAnnouncement = $state<string | null>(null);
  let lastHoldAnnouncement: string | null = null;
  let lastNotificationAnnouncement: string | null = null;
  $effect(() => {
    const next = holdSnapshot.announcement;
    if (next === lastHoldAnnouncement) return;
    lastHoldAnnouncement = next;
    if (next !== null) liveAnnouncement = next;
  });
  $effect(() => {
    const next = notificationSnapshot.announcement;
    if (next === lastNotificationAnnouncement) return;
    lastNotificationAnnouncement = next;
    if (next !== null) liveAnnouncement = `Notification: ${next}`;
  });

  const choice = $derived(
    effectiveChoice(office.projects, store.projectChoice),
  );
  const shown = $derived(visibleAgents(office.agents, choice));
  const emptyProject = $derived(isEmptyProject(office.projects, shown, choice));
  const notifications = $derived(notificationSnapshot);
  const unread = $derived(unreadCount(notifications.rows));

  const AGENTS_HEADING_ID = "panel-agents-heading";
  const EMPTY_HEADING_ID = "panel-empty-heading";

  let isCreatingAgent = $state(false);
  /** The project the agent form opens with, from an empty-project offer. */
  let createAgentInProject = $state<string | null>(null);
  let isCreatingProject = $state(false);
  let isSwitchingAgent = $state(false);
  let isShowingNotifications = $state(false);
  let projectToDelete = $state<Project | null>(null);
  let detailArea = $state<HTMLElement | null>(null);

  // The deleted agent's controls are gone, so focus would fall to the page.
  async function deleteAgent(agentId: string): Promise<void> {
    await controller.deleteAgent(agentId);
    await tick();
    if (office.agents.some((agent) => agent.id === agentId)) return;
    const headingId =
      office.agents.length > 0 ? AGENTS_HEADING_ID : EMPTY_HEADING_ID;
    document.getElementById(headingId)?.focus();
  }

  function deleteProject(): void {
    if (projectToDelete === null) return;
    const { id } = projectToDelete;
    projectToDelete = null;
    void controller.deleteProject(id);
  }

  /**
   * A row click or a shortcut opens the agent's terminal in the detail area
   * (FR-004). A freshly mounted terminal focuses itself once the attach
   * answers; an already open one takes focus here, because nothing remounts
   * for it. The store's select reads the agent's notifications (FR-009).
   */
  function openAgent(agentId: string): void {
    const wasSelected = office.selectedAgentId === agentId;
    controller.select(agentId);
    if (wasSelected) {
      void tick().then(() => {
        detailArea?.querySelector("textarea")?.focus();
      });
    }
  }

  /**
   * A notification click opens the agent's terminal and reads that one row
   * (FR-009). A deleted agent's row opens nothing, but the click still reads
   * it: the kept name tells the user what the row was about (edge case).
   */
  function openNotification(notificationId: string): void {
    const row = notifications.rows.find((known) => known.id === notificationId);
    isShowingNotifications = false;
    if (row === undefined) return;
    void store.notifications.read(notificationId);
    if (office.agents.some((agent) => agent.id === row.agent_id)) {
      openAgent(row.agent_id);
    }
  }

  async function toggleOfficeHold(): Promise<void> {
    const failure =
      holds.office === null
        ? await store.hold.holdOffice()
        : await store.hold.releaseOffice();
    // Undefined: the controller ignored the press, the request in flight
    // still owns the failure text.
    if (failure !== undefined) officeHoldFailure = failure;
  }

  async function showAgentHoldResult(
    agentId: string,
    request: Promise<FailureText>,
  ): Promise<void> {
    const text = await request;
    agentHoldFailure = text === null ? null : { agentId, text };
  }

  /**
   * The one app-level key handler (D8). The terminal's custom key handler
   * lets exactly the table's combinations out of xterm, and every focus in
   * the app dispatches its keydowns here. A dialog that is not the switcher
   * holds the user's attention, so its keys stay its own; a held key
   * repeats, so only the first press of a burst acts.
   */
  function handleAppKeys(event: KeyboardEvent): void {
    if (event.repeat) return;
    if (isSwitchingAgent) return;
    if (document.querySelector("dialog[open]") !== null) return;
    const pressed = pressedAppShortcut(event);
    if (pressed === null) return;
    if (pressed.kind === "office_hold") {
      event.preventDefault();
      void toggleOfficeHold();
      return;
    }
    if (pressed.kind === "switcher") {
      isSwitchingAgent = true;
      return;
    }
    if (pressed.position === null) return;
    const agent = shown[pressed.position - 1];
    if (agent !== undefined) openAgent(agent.id);
  }

  $effect(() => {
    window.addEventListener("keydown", handleAppKeys);
    return () => window.removeEventListener("keydown", handleAppKeys);
  });

  function chooseFromSwitcher(agentId: string): void {
    isSwitchingAgent = false;
    openAgent(agentId);
  }

  /**
   * The form opens for one project only: the empty-project offer sets its
   * project, and the header's action clears it again (LOW-3).
   */
  function offerAgentForm(projectId: string | null): void {
    createAgentInProject = projectId;
    isCreatingAgent = true;
  }

  function closeAgentForm(): void {
    isCreatingAgent = false;
    createAgentInProject = null;
  }
</script>

<div class="mx-auto flex max-w-6xl flex-col gap-6 p-6">
  <header class="flex flex-wrap items-center justify-between gap-3">
    <h1 class="text-2xl font-semibold text-neutral-900">office-harness</h1>
    {#if office.phase === "ready"}
      <div class="flex flex-wrap gap-3">
        <UnreadButton
          count={unread}
          onopen={() => (isShowingNotifications = true)}
        />
        <Button variant="secondary" onclick={() => (isCreatingProject = true)}>
          New project
        </Button>
        <Button onclick={() => offerAgentForm(null)}>New agent</Button>
      </div>
      <div class="basis-full">
        <OfficeHoldBar
          controller={store.hold}
          isOfficeHeld={holds.office !== null}
          failure={officeHoldFailure}
          isPending={holdSnapshot.isOfficeRequestPending}
          ontoggle={() => void toggleOfficeHold()}
        />
      </div>
    {/if}
  </header>

  {#if office.notice !== null}
    <FailureMessage lead={office.notice.lead} message={office.notice.message} />
  {/if}

  <NotificationLiveRegion announcement={liveAnnouncement} />

  {#if office.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the office.</p>
  {:else if office.phase === "failed"}
    <FailureMessage
      lead="The office did not load."
      message={office.failure ?? ""}
    />
    <div>
      <Button variant="secondary" onclick={() => void controller.load()}>
        Try again
      </Button>
    </div>
  {:else}
    <ProjectSwitcher
      projects={office.projects}
      value={choice}
      onchange={store.setProjectChoice}
    />

    <!-- The hierarchy stays reachable in both empty states: US1 builds the
      first CEO from it, and an empty project must not hide it. With agents
      present it keeps its place at the top of the left column below. -->
    {#if (office.agents.length === 0 || emptyProject) && side}
      {@render side()}
    {/if}

    {#if office.agents.length === 0}
      <section
        aria-labelledby={EMPTY_HEADING_ID}
        class="flex flex-col items-start gap-3"
      >
        <h2
          id={EMPTY_HEADING_ID}
          tabindex="-1"
          class="text-xl font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
        >
          The office is empty
        </h2>
        <p class="text-neutral-700">
          No agent exists yet. An agent runs one CLI, such as claude or codex,
          in a folder that you choose.
        </p>
        <Button onclick={() => offerAgentForm(null)}>
          Create the first agent
        </Button>
      </section>
    {:else if emptyProject}
      <section
        aria-labelledby="panel-empty-project-heading"
        class="flex flex-col items-start gap-3"
      >
        <h2
          id="panel-empty-project-heading"
          tabindex="-1"
          class="text-xl font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
        >
          This project has no agents
        </h2>
        <p class="text-neutral-700">
          The project exists, but no agent runs in it yet.
        </p>
        <Button onclick={() => offerAgentForm(choice)}>
          Create an agent in this project
        </Button>
      </section>
    {:else}
      <div class="grid gap-6 md:grid-cols-[20rem_1fr]">
        <section
          aria-labelledby={AGENTS_HEADING_ID}
          class="flex flex-col gap-2"
        >
          {#if side}
            {@render side()}
          {/if}
          <h2
            id={AGENTS_HEADING_ID}
            tabindex="-1"
            class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
          >
            Agents
          </h2>
          <ul class="flex flex-col gap-2">
            {#each shown as agent (agent.id)}
              {@const isStopping = office.stoppingAgentIds.has(agent.id)}
              <AgentRow
                {agent}
                projectName={projectNameOf(agent, office.projects)}
                hierarchy={agentHierarchy(treeSnapshot.tree, agent.id)}
                {isStopping}
                isSelected={agent.id === office.selectedAgentId}
                isOnHold={holdOfAgent(holds, agent.id) !== null}
                onopen={openAgent}
              />
            {/each}
          </ul>
        </section>
        <div bind:this={detailArea}>
          {#if office.selectedAgentId !== null && office.details !== null}
            {@const selected = office.agents.find(
              (agent) => agent.id === office.selectedAgentId,
            )}
            {#if selected !== undefined}
              {@const agentId = selected.id}
              {#key agentId}
                <AgentDetails
                  {client}
                  agent={selected}
                  details={office.details}
                  statusDetail={office.statusDetails[agentId] ?? null}
                  projectName={projectNameOf(selected, office.projects)}
                  isBusy={office.isBusy}
                  isStopping={office.stoppingAgentIds.has(agentId)}
                  isOnHold={holdOfAgent(holds, agentId) !== null}
                  holdNote={automaticHoldText(holdOfAgent(holds, agentId))}
                  holdFailure={agentHoldFailure?.agentId === agentId
                    ? agentHoldFailure.text
                    : null}
                  onhold={() =>
                    void showAgentHoldResult(
                      agentId,
                      store.hold.holdAgent(agentId),
                    )}
                  onrelease={() =>
                    void showAgentHoldResult(
                      agentId,
                      store.hold.releaseAgent(agentId),
                    )}
                  onstart={() => void controller.startAgent(agentId)}
                  onstop={() => void controller.stopAgent(agentId)}
                  onnewtask={() => void controller.startNewTask(agentId)}
                  oncontinue={(sessionId) =>
                    void controller.continueSession(agentId, sessionId)}
                  ondelete={() => void deleteAgent(agentId)}
                  onsaveadditions={(additions) =>
                    controller.saveAdditions(agentId, additions)}
                  onsavetask={(text) => controller.saveTask(agentId, text)}
                  onretry={controller.retryDetails}
                />
              {/key}
            {/if}
          {:else}
            <p class="text-neutral-700">
              Choose an agent to open its terminal, details, and action record.
            </p>
          {/if}
        </div>
      </div>
    {/if}
    <ProjectList
      projects={office.projects}
      isBusy={office.isBusy}
      ondelete={(project) => (projectToDelete = project)}
    />
  {/if}
</div>

{#if isShowingNotifications}
  <NotificationDialog
    rows={notifications.rows.map((row) => ({
      id: row.id,
      text: rowText(
        row,
        office.agents.some((agent) => agent.id === row.agent_id),
      ),
    }))}
    onopen={openNotification}
    onclose={() => (isShowingNotifications = false)}
  />
{/if}
{#if isCreatingAgent}
  <AgentFormDialog
    projects={office.projects}
    isBusy={office.isBusy}
    initialProjectId={createAgentInProject}
    onsubmit={controller.createAgent}
    onclose={closeAgentForm}
  />
{/if}
{#if isSwitchingAgent}
  <AgentSwitcherDialog
    agents={office.agents}
    projects={office.projects}
    stoppingAgentIds={office.stoppingAgentIds}
    onchoose={chooseFromSwitcher}
    onclose={() => (isSwitchingAgent = false)}
  />
{/if}
{#if isCreatingProject}
  <ProjectFormDialog
    isBusy={office.isBusy}
    onsubmit={controller.createProject}
    onclose={() => (isCreatingProject = false)}
  />
{/if}
{#if projectToDelete !== null}
  <ConfirmDialog
    text={{
      title: `Delete project ${projectToDelete.name}?`,
      body: "The server refuses this while an agent still uses the project, and names those agents.",
      confirmLabel: "Delete project",
      isDestructive: true,
    }}
    onconfirm={deleteProject}
    oncancel={() => (projectToDelete = null)}
  />
{/if}
