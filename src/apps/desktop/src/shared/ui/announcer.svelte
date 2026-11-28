<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  interface Props {
    /** The text to announce, or null for none. */
    announcement: string | null;
    /** `status` for a region that speaks only after the user acts. */
    role?: "status";
  }

  let { announcement, role }: Props = $props();
</script>

<!-- A screen-reader-only polite region that stays mounted. Each new text is a
  new node, the form that a screen reader is most likely to report. WebKitGTK
  2.54 sent no event for this or for two other markups (see 0007 QA report). -->
<p aria-live="polite" aria-atomic="true" {role} class="sr-only">
  {#key announcement}<span>{announcement ?? ""}</span>{/key}
</p>
