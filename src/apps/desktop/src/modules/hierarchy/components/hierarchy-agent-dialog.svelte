<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { NewAgent } from "../../agent";
  import type { FailureText } from "../state/hierarchy-controller";

  interface Props {
    /** The row the dialog is about, as the title shows it. */
    subject: string;
    isBusy: boolean;
    onsubmit: (agent: NewAgent) => Promise<FailureText>;
    onclose: () => void;
  }

  let { subject, isBusy, onsubmit, onclose }: Props = $props();

  let name = $state("");
  let command = $state("");
  let workingFolder = $state("");
  let failure = $state<FailureText>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const agent: NewAgent = { name: name.trim(), command: command.trim() };
    if (workingFolder.trim() !== "")
      agent.working_folder = workingFolder.trim();
    failure = await onsubmit(agent);
    if (failure === null) onclose();
  }
</script>

<Dialog title={subject} {onclose}>
  <form class="flex flex-col gap-4" onsubmit={submit}>
    <TextField label="Name" bind:value={name} />
    <TextField
      label="Command"
      hint="The CLI program and its fixed arguments, for example claude."
      bind:value={command}
    />
    <TextField
      label="Working folder"
      hint="An absolute path. Leave it empty to use the team's project folder."
      bind:value={workingFolder}
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
