<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { sweepStatusText } from "../models/sweep";
  import {
    INITIAL_SWEEP,
    type SweepController,
  } from "../state/sweep-controller";

  interface Props {
    controller: SweepController;
  }

  let { controller }: Props = $props();

  let snapshot = $state(INITIAL_SWEEP);
  const text = $derived(sweepStatusText(snapshot.phase));

  $effect(() => controller.subscribe((next) => (snapshot = next)));
</script>

<!-- The region stays mounted, so a screen reader hears the text appear and
  the end of the sweep. The end is spoken only (sr-only): nothing shows when
  no sweep runs. -->
<div role="status" class={snapshot.phase === "running" ? "" : "sr-only"}>
  {#key text}<span class="text-sm font-medium text-neutral-900">{text}</span
    >{/key}
</div>
