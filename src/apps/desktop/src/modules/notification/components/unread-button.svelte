<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";

  interface Props {
    /** The number of unread notifications, as text (FR-007). */
    count: number;
    onopen: () => void;
  }

  let { count, onopen }: Props = $props();
</script>

<Button variant="secondary" onclick={onopen}>
  Notifications, {count} unread
</Button>
