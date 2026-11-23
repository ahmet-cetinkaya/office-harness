<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import {
    grantStatus,
    MAX_SECRETS,
    resetQuestion,
    storeLine,
    type Secret,
  } from "../models/secrets";
  import {
    INITIAL_SECRETS,
    type SecretsController,
    type SecretsSnapshot,
  } from "../state/secrets-controller";

  interface Props {
    controller: SecretsController;
    /** The agents that a grant can name. */
    agents: { id: string; name: string }[];
    onclose: () => void;
  }

  let { controller, agents, onclose }: Props = $props();

  const ERROR_ID = "secrets-error";

  let snapshot = $state<SecretsSnapshot>(INITIAL_SECRETS);
  let failure = $state<string | null>(null);
  let newName = $state("");
  let newValue = $state("");
  /** The secret whose value is being replaced, and its typed value. */
  let replacing = $state<string | null>(null);
  let replacementValue = $state("");
  let deleting = $state<string | null>(null);
  let isResetting = $state(false);
  /** The agent picked for a grant, by secret name. */
  let picked = $state<Record<string, string>>({});
  let isBusy = $state(false);
  // Focus follows the person into a form or a question that replaces the
  // button they pressed, so the keyboard never lands on the page behind.
  let replaceForm = $state<HTMLFormElement | null>(null);
  let confirmDeleteButton = $state<HTMLButtonElement | null>(null);
  let confirmResetButton = $state<HTMLButtonElement | null>(null);

  const view = $derived(snapshot.view);
  const isLocked = $derived(view.store.kind === "locked");
  const nameOfAgent = (agentId: string): string =>
    agents.find((agent) => agent.id === agentId)?.name ?? "deleted agent";

  onMount(() => {
    const unsubscribe = controller.subscribe((next) => (snapshot = next));
    void controller.load();
    return unsubscribe;
  });

  $effect(() => {
    replaceForm?.querySelector<HTMLElement>("input")?.focus();
  });
  $effect(() => {
    confirmDeleteButton?.focus();
  });
  $effect(() => {
    confirmResetButton?.focus();
  });

  // A typed value never outlives the dialog (FR-018).
  onDestroy(clearTypedValues);

  function clearTypedValues(): void {
    newValue = "";
    replacementValue = "";
  }

  async function act(work: () => Promise<string | null>): Promise<boolean> {
    isBusy = true;
    failure = await work();
    isBusy = false;
    return failure === null;
  }

  async function add(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const name = newName;
    const value = newValue;
    // The field is empty from the moment of the save, saved or refused.
    newValue = "";
    if (await act(() => controller.save(name, value))) newName = "";
  }

  async function replace(event: SubmitEvent, name: string): Promise<void> {
    event.preventDefault();
    const value = replacementValue;
    replacementValue = "";
    if (await act(() => controller.save(name, value))) replacing = null;
  }

  function stopReplacing(): void {
    replacing = null;
    replacementValue = "";
  }

  async function remove(name: string): Promise<void> {
    if (await act(() => controller.remove(name))) deleting = null;
  }

  function ungranted(secret: Secret): { value: string; label: string }[] {
    const held = new Set(secret.grants.map((grant) => grant.agentId));
    return agents
      .filter((agent) => !held.has(agent.id))
      .map((agent) => ({ value: agent.id, label: agent.name }));
  }

  async function grant(secret: Secret): Promise<void> {
    const agentId = picked[secret.name] ?? ungranted(secret)[0]?.value;
    if (agentId === undefined) return;
    await act(() => controller.grant(secret.name, agentId));
  }

  async function reset(): Promise<void> {
    if (await act(() => controller.reset(view.secrets.length))) {
      isResetting = false;
    }
  }
</script>

<Dialog title="Secrets" {onclose}>
  <p role="status" class="text-sm font-medium text-neutral-900">
    {#if !snapshot.isLoaded && snapshot.loadFailure === null}
      Loading the secrets.
    {:else}
      {storeLine(view.store)}
    {/if}
  </p>
  {#if snapshot.loadFailure !== null}
    <FailureMessage
      lead="The secrets did not load."
      message={snapshot.loadFailure}
    />
  {/if}
  <p class="text-sm text-neutral-700">
    A saved value is never shown again; replace it to change it. An agent reads
    a secret it holds a grant for, with
    <code>office-harnessd agent-call secret.get</code>, and must hand it to a
    tool through standard input or a file descriptor, for example
    <code>... --value-only | tool --password-stdin</code>. Never put it in an
    argument: a command line is stored and shown in the CLI's own transcript.
    The office replaces a stored value as a second line of defense.
  </p>

  {#if isLocked}
    <section aria-label="Reset the store" class="flex flex-col gap-2">
      <p class="text-sm text-neutral-900">
        Reads and saves are refused while the store is locked. If the key is
        gone, reset the store and save the secrets again.
      </p>
      {#if isResetting}
        <p class="text-sm font-medium text-neutral-900">
          {resetQuestion(view.secrets.length)}
        </p>
        <div class="flex flex-wrap gap-3">
          <Button
            variant="danger"
            disabled={isBusy}
            bind:element={confirmResetButton}
            onclick={() => void reset()}
          >
            Reset the store
          </Button>
          <Button variant="secondary" onclick={() => (isResetting = false)}>
            Cancel
          </Button>
        </div>
      {:else}
        <div>
          <Button variant="secondary" onclick={() => (isResetting = true)}>
            Reset…
          </Button>
        </div>
      {/if}
    </section>
  {/if}

  {#if snapshot.isLoaded && view.secrets.length === 0}
    <p class="text-sm text-neutral-700">
      No secret is saved. Add one below to give an agent a key.
    </p>
  {/if}

  <ul class="flex flex-col gap-4">
    {#each view.secrets as secret (secret.name)}
      <li>
        <section
          aria-label={secret.name}
          class="flex flex-col gap-2 rounded-md border border-neutral-400 p-3"
        >
          <h3 class="text-base font-semibold text-neutral-900">
            {secret.name}
            <span class="text-sm font-normal text-neutral-700">
              version {secret.version}
            </span>
          </h3>
          {#if secret.isUnreadable}
            <p class="text-sm font-medium text-red-900">
              This secret cannot be read. Replace its value.
            </p>
          {/if}
          {#if secret.grants.length === 0}
            <p class="text-sm text-neutral-700">No agent holds a grant.</p>
          {:else}
            <ul class="flex flex-col gap-1">
              {#each secret.grants as held (held.agentId)}
                <li class="flex flex-wrap items-center gap-2 text-sm">
                  <span class="font-medium text-neutral-900">
                    {nameOfAgent(held.agentId)}
                  </span>
                  <span class="text-neutral-700">
                    {grantStatus(secret, held)}
                  </span>
                  <Button
                    variant="secondary"
                    disabled={isBusy}
                    onclick={() =>
                      void act(() =>
                        controller.revoke(secret.name, held.agentId),
                      )}
                  >
                    Revoke {nameOfAgent(held.agentId)} from {secret.name}
                  </Button>
                </li>
              {/each}
            </ul>
          {/if}
          {#if ungranted(secret).length > 0}
            <div class="flex flex-wrap items-end gap-2">
              <SelectField
                label={`Grant ${secret.name} to`}
                options={ungranted(secret)}
                value={picked[secret.name] ?? ungranted(secret)[0].value}
                onchange={(agentId) => (picked[secret.name] = agentId)}
              />
              <Button
                variant="secondary"
                disabled={isBusy}
                onclick={() => void grant(secret)}
              >
                Grant {secret.name}
              </Button>
            </div>
          {/if}
          {#if replacing === secret.name}
            <form
              bind:this={replaceForm}
              class="flex flex-col gap-2"
              onsubmit={(event) => void replace(event, secret.name)}
            >
              <TextField
                label={`New value of ${secret.name}`}
                hint="At least 8 bytes. It is shown as dots and cleared when you save."
                isSecret
                errorId={failure === null ? undefined : ERROR_ID}
                bind:value={replacementValue}
              />
              <div class="flex flex-wrap gap-3">
                <Button type="submit" disabled={isBusy}>
                  Save new value of {secret.name}
                </Button>
                <Button variant="secondary" onclick={stopReplacing}>
                  Cancel
                </Button>
              </div>
            </form>
          {:else if deleting === secret.name}
            <p class="text-sm font-medium text-neutral-900">
              Delete {secret.name} and its {secret.grants.length} grants?
            </p>
            <div class="flex flex-wrap gap-3">
              <Button
                variant="danger"
                disabled={isBusy}
                bind:element={confirmDeleteButton}
                onclick={() => void remove(secret.name)}
              >
                Delete {secret.name}
              </Button>
              <Button variant="secondary" onclick={() => (deleting = null)}>
                Cancel
              </Button>
            </div>
          {:else}
            <div class="flex flex-wrap gap-3">
              <Button
                variant="secondary"
                disabled={isLocked}
                onclick={() => (replacing = secret.name)}
              >
                Replace value of {secret.name}
              </Button>
              <Button
                variant="secondary"
                onclick={() => (deleting = secret.name)}
              >
                Delete {secret.name}…
              </Button>
            </div>
          {/if}
        </section>
      </li>
    {/each}
  </ul>

  <form class="flex flex-col gap-3" onsubmit={add}>
    <h3 class="text-base font-semibold text-neutral-900">Add a secret</h3>
    <TextField
      label="Name"
      hint="Letters A to Z, digits, and underscore, for example OPENAI_API_KEY."
      bind:value={newName}
    />
    <TextField
      label="Value"
      hint={`At least 8 bytes. It is shown as dots and cleared when you save. At most ${MAX_SECRETS} secrets.`}
      isSecret
      errorId={failure === null ? undefined : ERROR_ID}
      bind:value={newValue}
    />
    <div>
      <Button type="submit" disabled={isBusy || isLocked}>Save secret</Button>
    </div>
  </form>

  {#if failure !== null}
    <FailureMessage
      id={ERROR_ID}
      lead="The change was not made."
      message={failure}
    />
  {/if}
  <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
</Dialog>
