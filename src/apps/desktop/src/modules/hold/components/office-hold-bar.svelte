<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { OFFICE_ON_HOLD_TEXT } from "../models/hold";
  import type { FailureText, HoldController } from "../state/hold-controller";
  import { SecretsDialog, type SecretsController } from "../../secrets";
  import LimitsDialog from "./limits-dialog.svelte";
  import OfficeRecordDialog from "./office-record-dialog.svelte";

  interface Props {
    controller: HoldController;
    secrets: SecretsController;
    /** The agents that a secret grant can name. */
    agents: { id: string; name: string }[];
    isOfficeHeld: boolean;
    /** The server's refusal of the last hold or release, or null. */
    failure: FailureText;
    /** True while a hold or release waits for the server. */
    isPending: boolean;
    ontoggle: () => void;
  }

  let {
    controller,
    secrets,
    agents,
    isOfficeHeld,
    failure,
    isPending,
    ontoggle,
  }: Props = $props();

  let isShowingRecord = $state(false);
  let isEditingLimits = $state(false);
  let isEditingSecrets = $state(false);
</script>

<section aria-label="Office hold" class="flex flex-col gap-2">
  <div class="flex flex-wrap items-center gap-3">
    {#if isOfficeHeld}
      <p class="text-sm font-semibold text-neutral-900">
        {OFFICE_ON_HOLD_TEXT}
      </p>
    {/if}
    <Button variant="secondary" disabled={isPending} onclick={ontoggle}>
      {isOfficeHeld ? "Release office" : "Hold office"}
    </Button>
    <Button variant="secondary" onclick={() => (isShowingRecord = true)}>
      Office record
    </Button>
    <Button variant="secondary" onclick={() => (isEditingLimits = true)}>
      Limits
    </Button>
    <Button variant="secondary" onclick={() => (isEditingSecrets = true)}>
      Secrets
    </Button>
  </div>
  {#if failure !== null}
    <FailureMessage lead="The hold did not change." message={failure} />
  {/if}
</section>

{#if isShowingRecord}
  <OfficeRecordDialog {controller} onclose={() => (isShowingRecord = false)} />
{/if}
{#if isEditingSecrets}
  <SecretsDialog
    controller={secrets}
    {agents}
    onclose={() => (isEditingSecrets = false)}
  />
{/if}
{#if isEditingLimits}
  <LimitsDialog {controller} onclose={() => (isEditingLimits = false)} />
{/if}
