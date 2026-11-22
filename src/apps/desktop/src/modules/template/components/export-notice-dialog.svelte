<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";

  interface Props {
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { onconfirm, oncancel }: Props = $props();
</script>

<Dialog title="Export templates" onclose={oncancel}>
  <p class="text-neutral-700">
    The file leaves out every environment value. Flags and startup prompts are
    written as they are, so remove a secret from them first. Press Escape to
    cancel.
  </p>
  <div class="flex flex-wrap gap-3">
    <Button onclick={onconfirm}>Choose a file and export</Button>
    <Button variant="secondary" onclick={oncancel}>Cancel</Button>
  </div>
</Dialog>
