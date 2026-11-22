<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { Additions, AdditionsView } from "../models/agent";
  import {
    formatEnvironment,
    formatFlags,
    parseEnvironment,
    parseFlags,
    parseStartupPrompt,
  } from "../services/additions-text";
  import type { FailureText } from "../state/office-controller";

  interface Props {
    additions: AdditionsView;
    isRunning: boolean;
    isBusy: boolean;
    onsave: (additions: Additions) => Promise<FailureText>;
  }

  let { additions, isRunning, isBusy, onsave }: Props = $props();

  let flagsText = $state("");
  let environmentText = $state("");
  let promptText = $state("");
  let failure = $state<FailureText>(null);
  let isSaved = $state(false);

  // The saved values replace the draft: after this client's own save, and when
  // another client saves (the additions-changed event), so that the last save
  // wins and the editor shows what is stored.
  $effect(() => {
    flagsText = formatFlags(additions.flags);
    environmentText = formatEnvironment(additions.env);
    promptText = additions.startup_prompt ?? "";
  });

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    isSaved = false;
    const environment = parseEnvironment(environmentText);
    if (!environment.ok) {
      failure = environment.message;
      return;
    }
    failure = await onsave({
      flags: parseFlags(flagsText),
      env: environment.entries,
      startup_prompt: parseStartupPrompt(promptText),
    });
    isSaved = failure === null;
  }
</script>

<form
  aria-labelledby="additions-heading"
  class="flex flex-col gap-4"
  onsubmit={save}
>
  <h3 id="additions-heading" class="text-lg font-semibold text-neutral-900">
    Office additions
  </h3>
  <p class="text-sm text-neutral-700">
    office-harness adds these when it starts this agent. It never writes the
    CLI's own configuration files.
  </p>
  {#if isRunning}
    <p
      role="status"
      class="rounded-md border border-neutral-500 bg-neutral-50 p-3 text-sm text-neutral-900"
    >
      <strong>Applies to the next session.</strong>
      The running session keeps the values it started with. A change that you save
      takes effect when the next session starts.
    </p>
  {/if}
  <TextField
    label="Flags"
    hint="One argument for each line. A flag and its value are two lines."
    isMultiline
    bind:value={flagsText}
  />
  <TextField
    label="Environment variables"
    hint="One NAME=value for each line. A bare NAME has no value: the CLI does not get it, so the login environment's value stays."
    isMultiline
    bind:value={environmentText}
  />
  <TextField
    label="Startup prompt"
    hint="Pasted into the terminal after the first output of a new session. It is never submitted for you."
    isMultiline
    bind:value={promptText}
  />
  {#if failure !== null}
    <FailureMessage lead="The additions were not saved." message={failure} />
  {/if}
  <div class="flex flex-wrap items-center gap-3">
    <Button type="submit" disabled={isBusy}>Save additions</Button>
    {#if isSaved}
      <p role="status" class="text-sm text-neutral-900">Additions saved.</p>
    {/if}
  </div>
</form>
