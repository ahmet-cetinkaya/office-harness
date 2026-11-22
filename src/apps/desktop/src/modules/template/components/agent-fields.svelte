<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import { ROLE_OPTIONS, type AgentDraft } from "../models/template-form";
  import type { TemplateRole } from "../models/template";

  interface Props {
    draft: AgentDraft;
    /** The role applies to a direct creation only (A4), so a team entry hides it. */
    hasRole: boolean;
    /** Names the group for assistive technology, for example "Lead". */
    title: string;
  }

  let { draft = $bindable(), hasRole, title }: Props = $props();

  // The select works on text; the empty text is "no role".
  function roleValue(): string {
    return draft.role ?? "";
  }
  function setRole(value: string): void {
    draft.role = value === "" ? null : (value as TemplateRole);
  }
</script>

<fieldset class="flex flex-col gap-4">
  <legend class="sr-only">{title}</legend>
  <TextField
    label={`${title}: name pattern`}
    hint="{'{n}'} is the lowest free number, {'{team}'} is the team name."
    bind:value={draft.namePattern}
  />
  <TextField
    label={`${title}: command`}
    hint="The CLI program and its fixed arguments, for example claude."
    bind:value={draft.command}
  />
  <TextField
    label={`${title}: working folder`}
    hint="An absolute path. Leave it empty to use the project folder."
    bind:value={draft.workingFolder}
  />
  <TextField
    label={`${title}: flags`}
    hint="One argument for each line."
    isMultiline
    bind:value={draft.flagsText}
  />
  <TextField
    label={`${title}: environment variables`}
    hint="One NAME=value for each line. A bare NAME has no value."
    isMultiline
    bind:value={draft.environmentText}
  />
  <TextField
    label={`${title}: startup prompt`}
    isMultiline
    bind:value={draft.promptText}
  />
  <label class="flex items-center gap-2 text-sm font-medium text-neutral-900">
    <input type="checkbox" class="size-5" bind:checked={draft.isOwnWorktree} />
    {title}: own worktree
  </label>
  <p class="-mt-2 text-sm text-neutral-700">
    Each agent made from this entry gets its own git branch and worktree. The
    working folder must be in a git repository when the agent is created.
  </p>
  {#if hasRole}
    <SelectField
      label={`${title}: role`}
      options={ROLE_OPTIONS}
      bind:value={roleValue, setRole}
    />
  {/if}
</fieldset>
