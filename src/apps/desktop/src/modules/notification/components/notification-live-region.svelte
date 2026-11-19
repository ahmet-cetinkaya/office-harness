<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  interface Props {
    /** The text to announce, or null for none. */
    announcement: string | null;
  }

  let { announcement }: Props = $props();
</script>

<!-- Announces each new text once; the next one replaces it. -->
<p aria-live="polite" class="sr-only">
  {announcement ?? ""}
</p>
