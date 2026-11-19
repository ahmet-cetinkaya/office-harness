<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  interface Props {
    /** The text of the newest notification, or null for none. */
    announcement: string | null;
  }

  let { announcement }: Props = $props();
</script>

<!-- Announces each new notification once; the next one replaces the text. -->
<p aria-live="polite" class="sr-only">
  {announcement === null ? "" : `Notification: ${announcement}`}
</p>
