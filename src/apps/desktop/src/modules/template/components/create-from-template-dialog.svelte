<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { Agent, Project } from "../../agent";
  import type { HierarchyTree } from "../../hierarchy";
  import {
    directPlaceOf,
    needsPlace,
    teamChoices,
  } from "../models/create-place";
  import type { Template } from "../models/template";
  import { bodyGroups, kindText } from "../models/template-text";
  import type {
    FailureText,
    TemplateController,
  } from "../state/template-controller";
  import BodyGroups from "./body-groups.svelte";

  interface Props {
    controller: TemplateController;
    templates: Template[];
    tree: HierarchyTree;
    agents: Agent[];
    projects: Project[];
    onclose: () => void;
  }

  let { controller, templates, tree, agents, projects, onclose }: Props =
    $props();

  const NO_PROJECT = "";
  // svelte-ignore state_referenced_locally
  let templateId = $state(templates[0]?.id ?? "");
  let teamId = $state("");
  let teamName = $state("");
  let projectId = $state(NO_PROJECT);
  let failure = $state<FailureText>(null);
  /** True while the create request is open, so a double Enter sends one. */
  let isBusy = $state(false);

  const template = $derived(templates.find((known) => known.id === templateId));
  const role = $derived(template?.kind === "agent" ? template.body.role : null);
  const teams = $derived(teamChoices(tree));
  const ceoName = $derived(
    agents.find((agent) => agent.id === tree.ceo?.id)?.name ?? null,
  );
  const isPlaced = $derived(needsPlace(role));
  // A team goes under the CEO, and a lead or member needs a team to go into.
  const canCreate = $derived(
    template?.kind === "team"
      ? tree.ceo !== null
      : !isPlaced || teams.length > 0,
  );
  const templateOptions = $derived(
    templates.map((known) => ({
      value: known.id,
      label: `${known.name} (${kindText(known.kind)})`,
    })),
  );
  const teamOptions = $derived(
    teams.map((team) => ({ value: team.teamId, label: team.name })),
  );
  const projectOptions = $derived([
    { value: NO_PROJECT, label: "No project" },
    ...projects.map((project) => ({ value: project.id, label: project.name })),
  ]);

  // The team select starts on the first team and follows a list that changed.
  $effect(() => {
    if (!teams.some((team) => team.teamId === teamId)) {
      teamId = teams[0]?.teamId ?? "";
    }
  });

  function createRequest(chosen: Template): Promise<FailureText> {
    const project = projectId === NO_PROJECT ? null : projectId;
    if (chosen.kind === "team") {
      return controller.createTeam({
        templateId: chosen.id,
        teamName: teamName.trim(),
        parentAgentId: tree.ceo?.id ?? "",
        projectId: project,
      });
    }
    return controller.createAgent({
      templateId: chosen.id,
      place: directPlaceOf(chosen.body.role, tree, teamId),
      projectId: isPlaced ? null : project,
    });
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (template === undefined || isBusy) return;
    isBusy = true;
    failure = await createRequest(template);
    isBusy = false;
    if (failure === null) onclose();
  }
</script>

<Dialog title="Create from template" {onclose}>
  {#if templates.length === 0}
    <p class="text-neutral-700">
      No template exists yet. Save an agent as a template, or build one in the
      templates view.
    </p>
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  {:else if template !== undefined}
    <form class="flex flex-col gap-4" onsubmit={submit}>
      <SelectField
        label="Template"
        options={templateOptions}
        bind:value={templateId}
      />
      {#if template.kind === "team"}
        <TextField
          label="Team name"
          hint="Up to 64 characters, unique in the office."
          bind:value={teamName}
        />
        <p class="text-sm text-neutral-900">
          <strong>Parent of the lead:</strong>
          {ceoName ?? "none, the office has no CEO yet"}
        </p>
      {:else if isPlaced && teams.length === 0}
        <p class="text-sm text-neutral-900">
          <strong>This template has a role that needs a team.</strong>
          The office has no team yet.
        </p>
      {:else if isPlaced}
        <SelectField label="Team" options={teamOptions} bind:value={teamId} />
      {/if}
      {#if !isPlaced}
        <SelectField
          label="Project"
          options={projectOptions}
          bind:value={projectId}
        />
      {/if}
      <h3 class="text-base font-semibold text-neutral-900">
        What the template holds
      </h3>
      <BodyGroups groups={bodyGroups(template.body)} level="h4" />
      {#if failure !== null}
        <FailureMessage lead="Nothing was created." message={failure} />
      {/if}
      <div class="flex flex-wrap gap-3">
        <Button type="submit" disabled={isBusy || !canCreate}>
          {template.kind === "team" ? "Create team" : "Create agent"}
        </Button>
        <Button variant="secondary" onclick={onclose}>Cancel</Button>
      </div>
    </form>
  {/if}
</Dialog>
