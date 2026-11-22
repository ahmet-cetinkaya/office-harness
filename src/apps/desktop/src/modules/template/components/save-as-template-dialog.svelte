<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { draftOfAgentBody, type FormStart } from "../models/template-form";
  import type { TemplateController } from "../state/template-controller";
  import TemplateForm from "./template-form.svelte";

  interface Props {
    controller: TemplateController;
    agentId: string;
    agentName: string;
    onclose: () => void;
  }

  let { controller, agentId, agentName, onclose }: Props = $props();

  type Phase =
    | { kind: "loading" }
    | { kind: "failed"; message: string }
    | { kind: "ready"; start: FormStart };

  let phase = $state<Phase>({ kind: "loading" });

  // The agent's definition comes from the server; it saves nothing and
  // leaves the agent as it is (FR-001).
  onMount(() => {
    void controller.fromAgent(agentId).then(
      (body) => {
        phase = {
          kind: "ready",
          start: { name: agentName, agent: draftOfAgentBody(body) },
        };
      },
      (rejection) => {
        phase = {
          kind: "failed",
          message: controller.describeFailure(rejection),
        };
      },
    );
  });
</script>

<Dialog title={`Save ${agentName} as a template`} {onclose}>
  {#if phase.kind === "loading"}
    <p role="status" class="text-neutral-700">Reading the agent.</p>
  {:else if phase.kind === "failed"}
    <FailureMessage
      lead="The agent could not be read."
      message={phase.message}
    />
    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  {:else}
    <TemplateForm
      start={phase.start}
      isBusy={false}
      onsave={controller.save}
      {onclose}
    />
  {/if}
</Dialog>
