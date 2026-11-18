<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { NewAgent, Project } from "../models/agent";
  import type { FailureText } from "../state/office-controller";

  interface Props {
    projects: Project[];
    isBusy: boolean;
    /** The project the form starts with, or null for "No project". */
    initialProjectId?: string | null;
    onsubmit: (agent: NewAgent) => Promise<FailureText>;
    onclose: () => void;
  }

  let {
    projects,
    isBusy,
    initialProjectId = null,
    onsubmit,
    onclose,
  }: Props = $props();

  const NO_PROJECT = "";
  let name = $state("");
  let command = $state("");
  let workingFolder = $state("");
  // The dialog mounts for one open, so the preselected project is only the
  // first value.
  // svelte-ignore state_referenced_locally
  let projectId = $state(initialProjectId ?? NO_PROJECT);
  let failure = $state<FailureText>(null);

  const projectOptions = $derived([
    { value: NO_PROJECT, label: "No project" },
    ...projects.map((project) => ({ value: project.id, label: project.name })),
  ]);

  function readAgent(): NewAgent {
    const agent: NewAgent = { name: name.trim(), command: command.trim() };
    if (workingFolder.trim() !== "")
      agent.working_folder = workingFolder.trim();
    if (projectId !== NO_PROJECT) agent.project_id = projectId;
    return agent;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    failure = await onsubmit(readAgent());
    if (failure === null) onclose();
  }
</script>

<Dialog title="New agent" {onclose}>
  <form class="flex flex-col gap-4" onsubmit={submit}>
    <TextField label="Name" bind:value={name} />
    <TextField
      label="Command"
      hint="The CLI program and its fixed arguments, for example claude."
      bind:value={command}
    />
    <TextField
      label="Working folder"
      hint="An absolute path. Leave it empty to use the project folder."
      bind:value={workingFolder}
    />
    <SelectField
      label="Project"
      options={projectOptions}
      bind:value={projectId}
    />
    {#if failure !== null}
      <FailureMessage lead="The agent was not created." message={failure} />
    {/if}
    <div class="flex flex-wrap gap-3">
      <Button type="submit" disabled={isBusy}>Create agent</Button>
      <Button variant="secondary" onclick={onclose}>Cancel</Button>
    </div>
  </form>
</Dialog>
