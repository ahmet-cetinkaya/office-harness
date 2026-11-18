<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";

  interface ConfirmChoice {
    label: string;
    body: string;
  }

  interface Props {
    title: string;
    body: string;
    confirmLabel: string;
    /** The mutually exclusive answers when the question is a choice
     * (FR-007 stop). Null keeps the single confirm button. */
    choices?: ConfirmChoice[] | null;
    /** Whether the second choice is selected; only read with choices. */
    selectedChoice?: boolean | null;
    onchoose?: ((withDescendants: boolean) => void) | null;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let {
    title,
    body,
    confirmLabel,
    choices = null,
    selectedChoice = null,
    onchoose = null,
    onconfirm,
    oncancel,
  }: Props = $props();

  // Cancel takes the first focus, so a stray Enter does not confirm.
  let cancelButton = $state<HTMLButtonElement | null>(null);
</script>

<Dialog {title} onclose={oncancel} initialFocus={cancelButton}>
  <p class="text-neutral-700">{body} Press Escape to cancel.</p>
  {#if choices !== null && onchoose !== null}
    <div class="flex flex-col gap-2">
      {#each choices as choice, index (choice.label)}
        <label class="flex items-start gap-2 text-sm text-neutral-900">
          <input
            type="radio"
            name="stop-choice"
            class="mt-1"
            checked={selectedChoice === (index === 1)}
            onchange={() => onchoose(index === 1)}
          />
          <span>
            <strong>{choice.label}</strong>
            <span class="block text-neutral-700">{choice.body}</span>
          </span>
        </label>
      {/each}
    </div>
  {/if}
  <div class="flex flex-wrap gap-3">
    <Button variant="danger" onclick={onconfirm}>{confirmLabel}</Button>
    <Button variant="secondary" onclick={oncancel} bind:element={cancelButton}>
      Cancel
    </Button>
  </div>
</Dialog>
