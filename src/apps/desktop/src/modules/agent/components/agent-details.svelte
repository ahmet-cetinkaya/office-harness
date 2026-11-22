<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import type { CoreClient } from "../../../core/api";
  import { ActionRecord } from "../../audit";
  import { AGENT_ON_HOLD_TEXT } from "../../hold";
  import { TerminalView } from "../../terminal";
  import {
    isActive,
    shownStatusText,
    type Additions,
    type Agent,
  } from "../models/agent";
  import {
    confirmationText,
    needsConfirmation,
    type Confirmation,
  } from "../services/confirmations";
  import type { AgentDetails, FailureText } from "../state/office-controller";
  import AdditionsEditor from "./additions-editor.svelte";
  import TaskEditor from "./task-editor.svelte";
  import ConfirmDialog from "./confirm-dialog.svelte";
  import SessionList from "./session-list.svelte";

  interface Props {
    client: CoreClient;
    agent: Agent;
    details: AgentDetails;
    /** The text that came with the status, for example why the agent failed. */
    statusDetail: string | null;
    projectName: string | null;
    isBusy: boolean;
    /** True while the stop request of this agent is open. */
    isStopping: boolean;
    /** True while the agent is on hold (0007). */
    isOnHold: boolean;
    /** Why the office held the agent by itself, or null for a hold by the user. */
    holdNote: string | null;
    /** The server's refusal of the last hold or release, or null. */
    holdFailure: string | null;
    onhold: () => void;
    onrelease: () => void;
    onstart: () => void;
    onstop: () => void;
    onnewtask: () => void;
    oncontinue: (sessionId: string) => void;
    ondelete: () => void;
    onsaveadditions: (additions: Additions) => Promise<FailureText>;
    onsavetask: (text: string) => Promise<FailureText>;
    /** Opens the "save as template" dialog for this agent (0009). */
    onsaveastemplate: () => void;
    onretry: () => void;
  }

  let {
    client,
    agent,
    details,
    statusDetail,
    projectName,
    isBusy,
    isStopping,
    isOnHold,
    holdNote,
    holdFailure,
    onhold,
    onrelease,
    onstart,
    onstop,
    onnewtask,
    oncontinue,
    ondelete,
    onsaveadditions,
    onsavetask,
    onsaveastemplate,
    onretry,
  }: Props = $props();

  let confirmation = $state<Confirmation | null>(null);
  const isRunning = $derived(isActive(agent.status));

  function ask(next: Confirmation): void {
    // The delete of an agent with a worktree asks its own question, which
    // is also the confirmation (0010, FR-005).
    const asksAboutWorktree = next.kind === "delete" && agent.worktree != null;
    if (needsConfirmation(next) && !asksAboutWorktree) confirmation = next;
    else run(next);
  }

  function confirm(): void {
    if (confirmation === null) return;
    const confirmed = confirmation;
    confirmation = null;
    run(confirmed);
  }

  function run(chosen: Confirmation): void {
    if (chosen.kind === "stop") onstop();
    else if (chosen.kind === "delete") ondelete();
    else if (chosen.kind === "new_task") onnewtask();
    else oncontinue(chosen.sessionId);
  }
</script>

<section aria-labelledby="details-heading" class="flex flex-col gap-6">
  <header class="flex flex-col gap-2">
    <h2 id="details-heading" class="text-2xl font-semibold text-neutral-900">
      {agent.name}
    </h2>
    <dl
      class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm text-neutral-900"
    >
      <dt class="font-medium">Status</dt>
      <dd>
        <!-- A live region, so a screen reader hears "Stopping" at once. -->
        <span role="status">{shownStatusText(agent, isStopping)}</span>
      </dd>
      {#if isOnHold}
        <dt class="font-medium">Hold</dt>
        <dd>{AGENT_ON_HOLD_TEXT}{holdNote === null ? "" : `. ${holdNote}`}</dd>
      {/if}
      <dt class="font-medium">Command</dt>
      <dd class="break-all">{agent.command}</dd>
      <dt class="font-medium">Working folder</dt>
      <dd class="break-all">{agent.working_folder ?? "The project folder"}</dd>
      {#if projectName !== null}
        <dt class="font-medium">Project</dt>
        <dd>{projectName}</dd>
      {/if}
      {#if agent.worktree}
        <dt class="font-medium">Branch</dt>
        <dd class="break-all">{agent.worktree.branch}</dd>
        <dt class="font-medium">Base</dt>
        <dd class="break-all">{agent.worktree.base}</dd>
        <dt class="font-medium">Worktree folder</dt>
        <dd class="break-all">{agent.worktree.folder}</dd>
      {/if}
    </dl>
    <TaskEditor task={agent.task ?? null} {isBusy} onsave={onsavetask} />
    {#if statusDetail !== null}
      <p
        class="rounded-md border border-neutral-500 bg-neutral-50 p-3 text-sm text-neutral-900"
      >
        <strong>Last message:</strong>
        {statusDetail}
      </p>
    {/if}
    <div class="flex flex-wrap gap-3">
      {#if isRunning}
        <Button
          disabled={isBusy || isStopping}
          onclick={() => ask({ kind: "stop" })}
        >
          Stop agent
        </Button>
      {:else}
        <Button disabled={isBusy} onclick={onstart}>Start agent</Button>
      {/if}
      <Button
        variant="secondary"
        disabled={isBusy}
        onclick={() => ask({ kind: "new_task", isRunning })}
      >
        Start new task
      </Button>
      <Button variant="secondary" onclick={isOnHold ? onrelease : onhold}>
        {isOnHold ? "Release agent" : "Hold agent"}
      </Button>
      <Button variant="secondary" onclick={onsaveastemplate}>
        Save as template
      </Button>
      <Button
        variant="secondary"
        disabled={isBusy}
        onclick={() => ask({ kind: "delete" })}
      >
        Delete agent
      </Button>
    </div>
    {#if holdFailure !== null}
      <FailureMessage lead="The hold did not change." message={holdFailure} />
    {/if}
  </header>

  {#key isRunning}
    <TerminalView
      {client}
      agentId={agent.id}
      agentName={agent.name}
      {isRunning}
    />
  {/key}

  {#if details.phase === "loading"}
    <p role="status" class="text-neutral-700">Loading the agent's details.</p>
  {:else if details.phase === "failed"}
    <FailureMessage
      lead="The details did not load."
      message={details.failure ?? ""}
    />
    <div><Button variant="secondary" onclick={onretry}>Try again</Button></div>
  {:else}
    {#if details.additions !== null}
      <AdditionsEditor
        additions={details.additions}
        {isRunning}
        {isBusy}
        onsave={onsaveadditions}
      />
    {/if}
    <SessionList
      sessions={details.sessions}
      {isBusy}
      oncontinue={(sessionId) =>
        ask({ kind: "continue", sessionId, isRunning })}
    />
    <ActionRecord entries={details.actions} />
  {/if}
</section>

{#if confirmation !== null}
  <ConfirmDialog
    text={confirmationText(confirmation, agent.name)}
    onconfirm={confirm}
    oncancel={() => (confirmation = null)}
  />
{/if}
