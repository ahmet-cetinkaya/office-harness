<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import StatusPanel from "$shared/ui/status-panel.svelte";

  interface Props {
    onretry: () => void;
  }

  let { onretry }: Props = $props();
</script>

<StatusPanel title="The server did not accept this app">
  <p class="text-neutral-700">
    The app reads the office token from a file in the office data folder. The
    server rejected that token, so it will not share any agents with this app.
    To give the app a token that the server accepts:
  </p>
  <ol class="list-decimal space-y-2 pl-6 text-neutral-900">
    <li>
      In the office data folder, the one that holds the file server.json, delete
      the file named token.
    </li>
    <li>
      Stop the office server program. This screen then changes to the not
      running screen.
    </li>
    <li>
      Choose Start server there. The server writes a new token, and the app
      reads it.
    </li>
  </ol>
  <p class="text-neutral-700">
    If you changed the token file another way, choose Check again.
  </p>
  <div>
    <Button onclick={onretry}>Check again</Button>
  </div>
</StatusPanel>
