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
  new node: WebKitGTK sends an accessibility event for an inserted node, and
  not for the text of an existing node that changed. -->
<p aria-live="polite" aria-atomic="true" {role} class="sr-only">
  {#key announcement}<span>{announcement ?? ""}</span>{/key}
</p>
