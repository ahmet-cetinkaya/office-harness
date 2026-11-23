<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { ConfirmDialog, confirmationText, statusText } from "../../agent";
  import {
    creatorNameText,
    durationText,
    endedText,
    keptText,
    NO_FINISHED_TEMPS_TEXT,
    NO_TEMPS_TEXT,
    quietText,
    reasonText,
    reportLine,
    rowName,
  } from "../models/temps";
  import {
    INITIAL_TEMPS,
    type TempsController,
  } from "../state/temps-controller";

  interface Props {
    controller: TempsController;
    onclose: () => void;
  }

  let { controller, onclose }: Props = $props();

  const POLL_INTERVAL_MS = 10_000;
  const uid = $props.id();
  const TEMPS_HEADING_ID = `${uid}-temps`;
  const FINISHED_HEADING_ID = `${uid}-finished`;

  let snapshot = $state(INITIAL_TEMPS);
  let endTarget = $state<{ id: string; name: string } | null>(null);
  let body = $state<HTMLElement | null>(null);
  let hasFocusedFirstAction = false;

  // The list loads after the dialog opens, so the first End (or else the
  // first Dismiss) takes focus once, when the rows are there.
  $effect(() => {
    if (snapshot.phase !== "ready" || hasFocusedFirstAction) return;
    hasFocusedFirstAction = true;
    body?.querySelector<HTMLElement>("article button")?.focus();
  });

  $effect(() => controller.subscribe((next) => (snapshot = next)));
  onMount(() => void controller.open());
  onDestroy(() => controller.close());
  $effect(() => {
    const timer = setInterval(
      () => void controller.refresh(),
      POLL_INTERVAL_MS,
    );
    return () => clearInterval(timer);
  });

  // The acted-on row leaves the list, so focus moves to its heading.
  async function focusHeading(headingId: string): Promise<void> {
    await tick();
    document.getElementById(headingId)?.focus();
  }

  async function confirmEnd(): Promise<void> {
    if (endTarget === null) return;
    const { id } = endTarget;
    endTarget = null;
    await controller.end(id);
    await focusHeading(TEMPS_HEADING_ID);
  }

  async function dismiss(entryId: string): Promise<void> {
    await controller.dismiss(entryId);
    await focusHeading(FINISHED_HEADING_ID);
  }

  const HEADING_CLASS =
    "text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";
  const FIELDS_CLASS =
    "grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm text-neutral-900";
  const ROW_CLASS = "rounded-md border border-neutral-400 p-3";
</script>

<Dialog title="Temps" size="wide" {onclose}>
  <div
    bind:this={body}
    class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto"
  >
    {#if snapshot.phase === "loading"}
      <p role="status" class="text-neutral-700">Loading the temps.</p>
    {/if}
    {#if snapshot.failure !== null}
      <FailureMessage
        lead="The temps did not load."
        message={snapshot.failure}
      />
    {/if}
    {#if snapshot.actionFailure !== null}
      <FailureMessage
        lead="The request was refused."
        message={snapshot.actionFailure}
      />
    {/if}

    <section aria-labelledby={TEMPS_HEADING_ID} class="flex flex-col gap-2">
      <h3 id={TEMPS_HEADING_ID} tabindex="-1" class={HEADING_CLASS}>Temps</h3>
      {#if snapshot.temps.length === 0}
        <p class="text-sm text-neutral-700">{NO_TEMPS_TEXT}</p>
      {:else}
        <ul class="flex flex-col gap-2">
          {#each snapshot.temps as row (row.agent.id)}
            {@const rowId = `${uid}-row-${row.agent.id}`}
            <li>
              <article
                class={ROW_CLASS}
                aria-label={rowName(row, statusText(row.agent.status))}
                aria-describedby={`${rowId}-objective`}
              >
                <h4 class="font-semibold break-words text-neutral-900">
                  {row.agent.name}
                </h4>
                <dl class={FIELDS_CLASS}>
                  <dt class="font-medium">Creator</dt>
                  <dd class="break-words">
                    {creatorNameText(row.creator_name)}
                  </dd>
                  <dt class="font-medium">Parent</dt>
                  <dd class="break-words">{row.parent_name ?? "none"}</dd>
                  <dt class="font-medium">Objective</dt>
                  <dd id={`${rowId}-objective`} class="break-words">
                    {row.agent.temp?.objective ?? ""}
                  </dd>
                  <dt class="font-medium">Status</dt>
                  <dd>{statusText(row.agent.status)}</dd>
                  <dt class="font-medium">Run time</dt>
                  <dd>{durationText(row.run_seconds)}</dd>
                  <dt class="font-medium">Quiet time</dt>
                  <dd>{quietText(row.quiet_seconds)}</dd>
                </dl>
                <div class="mt-2">
                  <Button
                    variant="secondary"
                    onclick={() =>
                      (endTarget = { id: row.agent.id, name: row.agent.name })}
                  >
                    End {row.agent.name}
                  </Button>
                </div>
              </article>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section aria-labelledby={FINISHED_HEADING_ID} class="flex flex-col gap-2">
      <h3 id={FINISHED_HEADING_ID} tabindex="-1" class={HEADING_CLASS}>
        Finished
      </h3>
      {#if snapshot.finished.length === 0}
        <p class="text-sm text-neutral-700">{NO_FINISHED_TEMPS_TEXT}</p>
      {:else}
        <ul class="flex flex-col gap-2">
          {#each snapshot.finished as entry (entry.id)}
            {@const entryId = `${uid}-entry-${entry.id}`}
            {@const report = reportLine(entry)}
            <li>
              <article
                class={ROW_CLASS}
                aria-label={`${entry.name}, finished, created by ${entry.creator_name}`}
                aria-describedby={report === null
                  ? `${entryId}-objective`
                  : `${entryId}-objective ${entryId}-report`}
              >
                <h4 class="font-semibold break-words text-neutral-900">
                  {entry.name}
                </h4>
                <dl class={FIELDS_CLASS}>
                  <dt class="font-medium">Objective</dt>
                  <dd id={`${entryId}-objective`} class="break-words">
                    {entry.objective}
                  </dd>
                  <dt class="font-medium">Reason</dt>
                  <dd class="break-words">{reasonText(entry.reason)}</dd>
                  {#if report !== null}
                    <dt class="font-medium">Report</dt>
                    <dd id={`${entryId}-report`} class="break-words">
                      {report}
                    </dd>
                  {/if}
                  <dt class="font-medium">Kept</dt>
                  <dd class="break-words">{keptText(entry)}</dd>
                  <dt class="font-medium">Ended</dt>
                  <dd>{endedText(entry.ended_at)}</dd>
                </dl>
                <div class="mt-2">
                  <Button
                    variant="secondary"
                    onclick={() => void dismiss(entry.id)}
                  >
                    Dismiss {entry.name}
                  </Button>
                </div>
              </article>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  </div>
</Dialog>

{#if endTarget !== null}
  <ConfirmDialog
    text={confirmationText({ kind: "end_temp" }, endTarget.name)}
    onconfirm={() => void confirmEnd()}
    oncancel={() => (endTarget = null)}
  />
{/if}
