<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import type { AgentIdentity } from "../models/resume";
  import type { ResumeController } from "../state/resume-controller";
  import ResumeDialog from "./resume-dialog.svelte";

  interface Props {
    controller: ResumeController;
    /** How many agents of the office are interrupted. */
    count: number;
    identify: (agentId: string) => AgentIdentity;
  }

  let { controller, count, identify }: Props = $props();

  let isOpen = $state(false);
</script>

<!-- The dialog stays mounted when the count falls to zero, since resuming
  itself empties the count while the results are on screen. -->
{#if count > 0}
  <Button variant="secondary" onclick={() => (isOpen = true)}>
    Resume interrupted ({count})
  </Button>
{/if}
{#if isOpen}
  <ResumeDialog {controller} {identify} onclose={() => (isOpen = false)} />
{/if}
