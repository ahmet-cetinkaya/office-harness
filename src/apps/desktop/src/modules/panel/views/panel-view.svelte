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
  import type { PanelStore } from "../state/panel-store.svelte";
  import AgentRow from "../components/agent-row.svelte";
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

  const choice = $derived(
    effectiveChoice(office.projects, store.projectChoice),
  );
  const shown = $derived(visibleAgents(office.agents, choice));
  const emptyProject = $derived(isEmptyProject(office.projects, shown, choice));

  const AGENTS_HEADING_ID = "panel-agents-heading";
  const EMPTY_HEADING_ID = "panel-empty-heading";

  let isCreatingAgent = $state(false);
  /** The project the agent form opens with, from an empty-project offer. */
  let createAgentInProject = $state<string | null>(null);
  let isCreatingProject = $state(false);
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
   * A row click opens the agent's terminal in the detail area (FR-004). A
   * freshly mounted terminal focuses itself once the attach answers; an
   * already open one takes focus here, because nothing remounts for it.
   */
  async function openAgent(agentId: string): Promise<void> {
    const wasSelected = office.selectedAgentId === agentId;
    controller.select(agentId);
    await tick();
    if (wasSelected) detailArea?.querySelector("textarea")?.focus();
  }
</script>

<div class="mx-auto flex max-w-6xl flex-col gap-6 p-6">
  <header class="flex flex-wrap items-center justify-between gap-3">
    <h1 class="text-2xl font-semibold text-neutral-900">office-harness</h1>
    {#if office.phase === "ready"}
      <div class="flex flex-wrap gap-3">
        <Button variant="secondary" onclick={() => (isCreatingProject = true)}>
          New project
        </Button>
        <Button onclick={() => (isCreatingAgent = true)}>New agent</Button>
      </div>
    {/if}
  </header>

  {#if office.notice !== null}
    <FailureMessage lead={office.notice.lead} message={office.notice.message} />
  {/if}

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
        <Button onclick={() => (isCreatingAgent = true)}>
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
        <Button
          onclick={() => {
            createAgentInProject = choice;
            isCreatingAgent = true;
          }}
        >
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
                  onstart={() => void controller.startAgent(agentId)}
                  onstop={() => void controller.stopAgent(agentId)}
                  onnewtask={() => void controller.startNewTask(agentId)}
                  oncontinue={(sessionId) =>
                    void controller.continueSession(agentId, sessionId)}
                  ondelete={() => void deleteAgent(agentId)}
                  onsaveadditions={(additions) =>
                    controller.saveAdditions(agentId, additions)}
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

{#if isCreatingAgent}
  <AgentFormDialog
    projects={office.projects}
    isBusy={office.isBusy}
    initialProjectId={createAgentInProject}
    onsubmit={controller.createAgent}
    onclose={() => (isCreatingAgent = false)}
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
