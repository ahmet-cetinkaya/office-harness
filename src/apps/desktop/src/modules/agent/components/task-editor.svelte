<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { tick } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { AgentTask } from "../models/agent";
  import { visibleText } from "../models/task-text";
  import type { FailureText } from "../state/office-controller";

  interface Props {
    task: AgentTask | null;
    isBusy: boolean;
    /** Saves the text; blank text clears the task. Answers a refusal, or null. */
    onsave: (text: string) => Promise<FailureText>;
  }

  let { task, isBusy, onsave }: Props = $props();

  let isEditing = $state(false);
  let draft = $state("");
  let failure = $state<FailureText>(null);
  let editor = $state<HTMLElement | null>(null);
  let editButton = $state<HTMLButtonElement | null>(null);

  // No maxlength on the field: it counts UTF-16 units, the server counts
  // Unicode scalar values, so the server's refusal is the one rule.
  async function startEditing(): Promise<void> {
    draft = task?.text ?? "";
    failure = null;
    isEditing = true;
    await tick();
    editor?.querySelector("textarea")?.focus();
  }

  async function stopEditing(): Promise<void> {
    isEditing = false;
    await tick();
    editButton?.focus();
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    failure = await onsave(draft);
    if (failure === null) await stopEditing();
  }

  async function clear(): Promise<void> {
    failure = await onsave("");
  }
</script>

<section aria-labelledby="task-heading" class="flex flex-col gap-2">
  <h3 id="task-heading" class="text-lg font-semibold text-neutral-900">Task</h3>
  {#if isEditing}
    <form bind:this={editor} class="flex flex-col gap-3" onsubmit={save}>
      <TextField
        label="Task text"
        hint="Shown in the agent's row. Nothing is typed into the terminal. Empty text clears the task."
        isMultiline
        bind:value={draft}
      />
      <div class="flex flex-wrap gap-3">
        <Button type="submit" disabled={isBusy}>Save task</Button>
        <Button variant="secondary" onclick={stopEditing}>Cancel</Button>
      </div>
    </form>
  {:else}
    {#if task === null}
      <p class="text-sm text-neutral-700">No task text.</p>
    {:else}
      <p class="text-sm break-words whitespace-pre-wrap text-neutral-900">
        {visibleText(task.text)}
      </p>
    {/if}
    <div class="flex flex-wrap gap-3">
      <Button
        variant="secondary"
        bind:element={editButton}
        onclick={startEditing}
      >
        {task === null ? "Set task" : "Edit task"}
      </Button>
      {#if task !== null}
        <Button variant="secondary" disabled={isBusy} onclick={clear}>
          Clear task
        </Button>
      {/if}
    </div>
  {/if}
  {#if failure !== null}
    <FailureMessage lead="The task was not saved." message={failure} />
  {/if}
</section>
