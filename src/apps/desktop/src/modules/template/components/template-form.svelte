<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { TemplateKind } from "../models/template";
  import {
    agentBodyOfDraft,
    formValuesOf,
    KIND_OPTIONS,
    newMemberDraft,
    teamBodyOfDraft,
    type AgentDraft,
    type FormStart,
    type TeamDraft,
  } from "../models/template-form";
  import type { FailureText } from "../state/template-controller";
  import type { TemplateToSave } from "../services/template-service";
  import AgentFields from "./agent-fields.svelte";

  interface Props {
    start: FormStart;
    isBusy: boolean;
    onsave: (template: TemplateToSave) => Promise<FailureText>;
    onclose: () => void;
  }

  let { start, isBusy, onsave, onclose }: Props = $props();

  // The form mounts for one open, so the start values are only first values.
  // svelte-ignore state_referenced_locally
  const initial = formValuesOf(start);
  const existing = initial.existing;
  let name = $state(initial.name);
  let kind = $state<TemplateKind>(initial.kind);
  let agentDraft = $state<AgentDraft>(initial.agent);
  let teamDraft = $state<TeamDraft>(initial.team);
  let failure = $state<FailureText>(null);

  function removeMember(index: number): void {
    teamDraft.members.splice(index, 1);
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const result =
      kind === "agent"
        ? agentBodyOfDraft(agentDraft, "body")
        : teamBodyOfDraft(teamDraft);
    if (!result.ok) {
      failure = result.message;
      return;
    }
    failure = await onsave({
      id: existing?.id,
      name: name.trim(),
      kind,
      body: result.body,
    });
    if (failure === null) onclose();
  }
</script>

<form class="flex flex-col gap-4" onsubmit={submit}>
  <TextField
    label="Template name"
    hint="Up to 100 characters, unique among the templates."
    bind:value={name}
  />
  {#if existing === null}
    <SelectField label="Kind" options={KIND_OPTIONS} bind:value={kind} />
  {/if}
  {#if kind === "agent"}
    <AgentFields title="Agent" hasRole bind:draft={agentDraft} />
  {:else}
    <AgentFields title="Lead" hasRole={false} bind:draft={teamDraft.lead} />
    {#each teamDraft.members as member, index (index)}
      <div class="flex flex-col gap-3 border-t border-neutral-400 pt-4">
        <AgentFields
          title={`Member entry ${index + 1}`}
          hasRole={false}
          bind:draft={member.entry}
        />
        <TextField
          label={`Member entry ${index + 1}: count`}
          hint="How many agents this entry creates."
          bind:value={member.count}
        />
        <div>
          <Button variant="secondary" onclick={() => removeMember(index)}>
            Remove member entry {index + 1}
          </Button>
        </div>
      </div>
    {/each}
    <div>
      <Button
        variant="secondary"
        onclick={() => teamDraft.members.push(newMemberDraft())}
      >
        Add a member entry
      </Button>
    </div>
  {/if}
  {#if failure !== null}
    <FailureMessage lead="The template was not saved." message={failure} />
  {/if}
  <div class="flex flex-wrap gap-3">
    <Button type="submit" disabled={isBusy}>Save template</Button>
    <Button variant="secondary" onclick={onclose}>Cancel</Button>
  </div>
</form>
