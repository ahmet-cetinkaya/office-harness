<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Announcer from "$shared/ui/announcer.svelte";

  interface Props {
    /** The text to announce, or null for none. */
    announcement: string | null;
  }

  let { announcement }: Props = $props();
</script>

<Announcer {announcement} />
