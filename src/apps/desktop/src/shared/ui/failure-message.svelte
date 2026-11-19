<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  interface Props {
    lead: string;
    message: string;
    /** For a field's aria-describedby, when the message is about that field. */
    id?: string;
  }

  let { lead, message, id }: Props = $props();
</script>

<!-- The lead words carry the meaning, so the red tint is not the only cue. -->
<p
  {id}
  role="alert"
  class="rounded-md border border-red-700 bg-red-50 p-3 text-sm text-red-900"
>
  <strong>{lead}</strong>
  {message}
</p>
