<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";

  interface Props {
    title: string;
    body: string;
    confirmLabel: string;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { title, body, confirmLabel, onconfirm, oncancel }: Props = $props();

  // Cancel takes the first focus, so a stray Enter does not confirm.
  let cancelButton = $state<HTMLButtonElement | null>(null);
</script>

<Dialog {title} onclose={oncancel} initialFocus={cancelButton}>
  <p class="text-neutral-700">{body} Press Escape to cancel.</p>
  <div class="flex flex-wrap gap-3">
    <Button variant="danger" onclick={onconfirm}>{confirmLabel}</Button>
    <Button variant="secondary" onclick={oncancel} bind:element={cancelButton}>
      Cancel
    </Button>
  </div>
</Dialog>
