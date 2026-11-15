<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { NewProject } from "../models/agent";
  import type { FailureText } from "../state/office-controller";

  interface Props {
    isBusy: boolean;
    onsubmit: (project: NewProject) => Promise<FailureText>;
    onclose: () => void;
  }

  let { isBusy, onsubmit, onclose }: Props = $props();

  let name = $state("");
  let folder = $state("");
  let failure = $state<FailureText>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    failure = await onsubmit({ name: name.trim(), folder: folder.trim() });
    if (failure === null) onclose();
  }
</script>

<Dialog title="New project" {onclose}>
  <form class="flex flex-col gap-4" onsubmit={submit}>
    <TextField label="Name" bind:value={name} />
    <TextField
      label="Folder"
      hint="An absolute path to a folder that exists."
      bind:value={folder}
    />
    {#if failure !== null}
      <FailureMessage lead="The project was not created." message={failure} />
    {/if}
    <div class="flex flex-wrap gap-3">
      <Button type="submit" disabled={isBusy}>Create project</Button>
      <Button variant="secondary" onclick={onclose}>Cancel</Button>
    </div>
  </form>
</Dialog>
