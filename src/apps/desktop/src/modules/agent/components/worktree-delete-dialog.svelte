<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import {
    KEEP_WORKTREES,
    changeStateText,
    removeChoice,
    worktreesWithChanges,
    type WorktreeChoice,
    type WorktreeStatus,
  } from "../models/worktree";

  interface Props {
    title: string;
    /** What the delete does to the agents, including a subtree count. */
    body: string;
    statuses: WorktreeStatus[];
    onanswer: (choice: WorktreeChoice) => void;
    oncancel: () => void;
  }

  let { title, body, statuses, onanswer, oncancel }: Props = $props();

  const withChanges = $derived(worktreesWithChanges(statuses));
  let isConfirmingRemoval = $state(false);
  // Keep is the default answer and takes the first focus; the second
  // question starts on Cancel, which keeps everything.
  let keepButton = $state<HTMLButtonElement | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);

  function remove(): void {
    if (withChanges.length === 0) onanswer(removeChoice(statuses));
    else isConfirmingRemoval = true;
  }
</script>

{#if isConfirmingRemoval}
  <Dialog
    title="Remove worktrees with changes?"
    onclose={oncancel}
    initialFocus={cancelButton}
  >
    <div role="alert" class="flex flex-col gap-2 text-neutral-700">
      <p>Removing deletes these changes. They cannot be brought back.</p>
      <ul class="list-disc pl-5">
        {#each withChanges as status (status.agent_id)}
          <li>
            <strong>{status.name}</strong>: {changeStateText(status)}
          </li>
        {/each}
      </ul>
    </div>
    <div class="flex flex-wrap gap-3">
      <Button variant="danger" onclick={() => onanswer(removeChoice(statuses))}>
        Remove worktrees and changes
      </Button>
      <Button
        variant="secondary"
        onclick={oncancel}
        bind:element={cancelButton}
      >
        Cancel and keep everything
      </Button>
    </div>
  </Dialog>
{:else}
  <Dialog {title} onclose={oncancel} initialFocus={keepButton}>
    <p class="text-neutral-700">{body} Press Escape to cancel.</p>
    <ul class="list-disc pl-5 text-sm text-neutral-900">
      {#each statuses as status (status.agent_id)}
        <li>
          <strong>{status.name}</strong>, branch {status.branch}:
          {changeStateText(status)}
        </li>
      {/each}
    </ul>
    <p class="text-sm text-neutral-700">
      Files that git ignores are removed with the worktree.
    </p>
    <div class="flex flex-wrap gap-3">
      <Button
        onclick={() => onanswer(KEEP_WORKTREES)}
        bind:element={keepButton}
      >
        Keep worktrees
      </Button>
      <Button variant="danger" onclick={remove}>Remove worktrees</Button>
      <Button variant="secondary" onclick={oncancel}>Cancel</Button>
    </div>
  </Dialog>
{/if}
