<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import type { Project } from "../models/agent";
  import AgentDetails from "../components/agent-details.svelte";
  import AgentFormDialog from "../components/agent-form-dialog.svelte";
  import AgentList from "../components/agent-list.svelte";
  import ConfirmDialog from "../components/confirm-dialog.svelte";
  import ProjectFormDialog from "../components/project-form-dialog.svelte";
  import ProjectList from "../components/project-list.svelte";
  import type { OfficeStore } from "../state/office.store.svelte";

  interface Props {
    store: OfficeStore;
  }

  let { store }: Props = $props();

  const office = $derived(store.snapshot);
  const controller = $derived(store.controller);
  const client = $derived(store.client);
  const selectedAgent = $derived(
    office.agents.find((agent) => agent.id === office.selectedAgentId) ?? null,
  );
  const selectedProjectName = $derived(
    office.projects.find((project) => project.id === selectedAgent?.project_id)
      ?.name ?? null,
  );

  let isCreatingAgent = $state(false);
  let isCreatingProject = $state(false);
  let projectToDelete = $state<Project | null>(null);

  function deleteProject(): void {
    if (projectToDelete === null) return;
    const { id } = projectToDelete;
    projectToDelete = null;
    void controller.deleteProject(id);
  }
</script>

<div class="mx-auto flex max-w-5xl flex-col gap-6 p-6">
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
    {#if office.agents.length === 0}
      <section
        aria-labelledby="empty-heading"
        class="flex flex-col items-start gap-3"
      >
        <h2 id="empty-heading" class="text-xl font-semibold text-neutral-900">
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
    {:else}
      <div class="grid gap-6 md:grid-cols-[16rem_1fr]">
        <AgentList
          agents={office.agents}
          selectedAgentId={office.selectedAgentId}
          onselect={controller.select}
        />
        {#if selectedAgent !== null && office.details !== null}
          {@const agentId = selectedAgent.id}
          {#key agentId}
            <AgentDetails
              {client}
              agent={selectedAgent}
              details={office.details}
              statusDetail={office.statusDetails[agentId] ?? null}
              projectName={selectedProjectName}
              isBusy={office.isBusy}
              onstart={() => void controller.startAgent(agentId)}
              onstop={() => void controller.stopAgent(agentId)}
              onnewtask={() => void controller.startNewTask(agentId)}
              oncontinue={(sessionId) =>
                void controller.continueSession(agentId, sessionId)}
              ondelete={() => void controller.deleteAgent(agentId)}
              onsaveadditions={(additions) =>
                controller.saveAdditions(agentId, additions)}
              onretry={controller.retryDetails}
            />
          {/key}
        {:else}
          <p class="text-neutral-700">
            Choose an agent to see its details, sessions, and action record.
          </p>
        {/if}
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
