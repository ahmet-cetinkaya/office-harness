<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { Project } from "../../agent";
  import type { NewTeam } from "../services/hierarchy-service";
  import type { FailureText } from "../state/hierarchy-controller";

  interface Props {
    projects: Project[];
    isBusy: boolean;
    onsubmit: (team: NewTeam) => Promise<FailureText>;
    onclose: () => void;
  }

  let { projects, isBusy, onsubmit, onclose }: Props = $props();

  const NO_PROJECT = "";
  let name = $state("");
  let projectId = $state(NO_PROJECT);
  let leadName = $state("");
  let leadCommand = $state("");
  let failure = $state<FailureText>(null);

  const projectOptions = $derived([
    { value: NO_PROJECT, label: "No project" },
    ...projects.map((project) => ({ value: project.id, label: project.name })),
  ]);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const team: NewTeam = {
      name: name.trim(),
      lead: { name: leadName.trim(), command: leadCommand.trim() },
    };
    if (projectId !== NO_PROJECT) team.project_id = projectId;
    failure = await onsubmit(team);
    if (failure === null) onclose();
  }
</script>

<Dialog title="New team" {onclose}>
  <form class="flex flex-col gap-4" onsubmit={submit}>
    <TextField
      label="Team name"
      hint="Up to 64 characters, unique in the office."
      bind:value={name}
    />
    <SelectField
      label="Project"
      options={projectOptions}
      bind:value={projectId}
    />
    <TextField label="Team lead name" bind:value={leadName} />
    <TextField
      label="Team lead command"
      hint="The CLI program and its fixed arguments, for example claude."
      bind:value={leadCommand}
    />
    {#if failure !== null}
      <FailureMessage lead="The team was not created." message={failure} />
    {/if}
    <div class="flex flex-wrap gap-3">
      <Button type="submit" disabled={isBusy}>Create team</Button>
      <Button variant="secondary" onclick={onclose}>Cancel</Button>
    </div>
  </form>
</Dialog>
