<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import StatusPanel from "$shared/ui/status-panel.svelte";
</script>

<StatusPanel title="The office server is busy">
  <p role="status" class="text-neutral-700">
    The office server is handling other connections. The app tries again on its
    own.
  </p>
</StatusPanel>
