<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { ConfirmDialog, type Agent } from "../../agent";
  import {
    draftOf,
    dueText,
    emptyDraft,
    lastRunText,
    NO_SCHEDULES_TEXT,
    offReasonText,
    repeatWords,
    rowDescription,
    rowName,
    switchText,
    type Schedule,
    type ScheduleDraft,
  } from "../models/schedule";
  import {
    INITIAL_SCHEDULES,
    type HistorySnapshot,
    type SaveOutcome,
    type SchedulesController,
  } from "../state/schedules-controller";
  import ScheduleForm from "./schedule-form.svelte";
  import ScheduleHistory from "./schedule-history.svelte";

  interface Props {
    controller: SchedulesController;
    agents: readonly Pick<Agent, "id" | "name">[];
    /** A schedule to bring into view when the list opens, by name. */
    highlightName?: string | null;
    onclose: () => void;
  }

  let { controller, agents, highlightName = null, onclose }: Props = $props();

  const GONE_TARGET_TEXT = "an agent that is gone";
  const DELETE_BODY =
    "Its history goes. The action record of its past runs stays, and a directive that is already waiting for the target stays too.";

  const uid = $props.id();
  const HEADING_ID = `${uid}-schedules`;

  let snapshot = $state(INITIAL_SCHEDULES);
  let history = $state<HistorySnapshot | null>(null);
  let form = $state<{
    title: string;
    scheduleId: string | null;
    draft: ScheduleDraft;
  } | null>(null);
  let deleteTarget = $state<Schedule | null>(null);
  let body = $state<HTMLElement | null>(null);
  let hasScrolledToName = false;

  $effect(() => controller.subscribe((next) => (snapshot = next)));
  $effect(() => controller.subscribeHistory((next) => (history = next)));
  onMount(() => void controller.open());
  onDestroy(() => controller.close());

  // A notification names its schedule: the row scrolls into view, and the
  // focus stays where it is.
  $effect(() => {
    if (snapshot.phase !== "ready" || hasScrolledToName) return;
    hasScrolledToName = true;
    const rows = body?.querySelectorAll<HTMLElement>("article") ?? [];
    for (const row of rows) {
      if (row.dataset.name === highlightName) row.scrollIntoView?.();
    }
  });

  const targetName = (schedule: Schedule): string =>
    agents.find((agent) => agent.id === schedule.target_agent_id)?.name ??
    GONE_TARGET_TEXT;

  function openNew(): void {
    form = {
      title: "New schedule",
      scheduleId: null,
      draft: emptyDraft(agents[0]?.id ?? ""),
    };
  }

  function openEdit(schedule: Schedule): void {
    form = {
      title: `Edit ${schedule.name}`,
      scheduleId: schedule.id,
      draft: draftOf(schedule),
    };
  }

  async function save(draft: ScheduleDraft): Promise<SaveOutcome> {
    const outcome = await controller.save(draft, form?.scheduleId ?? null);
    if (outcome.saved) form = null;
    return outcome;
  }

  async function confirmDelete(): Promise<void> {
    if (deleteTarget === null) return;
    const { id } = deleteTarget;
    deleteTarget = null;
    await controller.remove(id);
    await tick();
    document.getElementById(HEADING_ID)?.focus();
  }

  const historySchedule = $derived(
    snapshot.schedules.find((item) => item.id === history?.scheduleId) ?? null,
  );

  const HEADING_CLASS =
    "text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";
  const FIELDS_CLASS =
    "grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm text-neutral-900";
</script>

<Dialog title="Schedules" size="wide" {onclose}>
  <div
    bind:this={body}
    class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto"
  >
    {#if snapshot.phase === "loading"}
      <p role="status" class="text-neutral-700">Loading the schedules.</p>
    {/if}
    {#if snapshot.failure !== null}
      <FailureMessage
        lead="The schedules did not load."
        message={snapshot.failure}
      />
    {/if}
    {#if snapshot.actionFailure !== null}
      <FailureMessage
        lead="The request was refused."
        message={snapshot.actionFailure}
      />
    {/if}

    <section aria-labelledby={HEADING_ID} class="flex flex-col gap-2">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h3 id={HEADING_ID} tabindex="-1" class={HEADING_CLASS}>Schedules</h3>
        <Button onclick={openNew}>New schedule</Button>
      </div>
      {#if snapshot.phase === "ready" && snapshot.schedules.length === 0}
        <p class="text-sm text-neutral-700">{NO_SCHEDULES_TEXT}</p>
      {/if}
      <ul class="flex flex-col gap-2">
        {#each snapshot.schedules as schedule (schedule.id)}
          {@const rowId = `${uid}-row-${schedule.id}`}
          {@const last = snapshot.lastRuns[schedule.id] ?? null}
          {@const reason = offReasonText(schedule)}
          <li>
            <article
              class="rounded-md border border-neutral-400 p-3"
              data-name={schedule.name}
              aria-label={rowName(schedule)}
              aria-describedby={`${rowId}-description`}
            >
              <h4 class="font-semibold break-words text-neutral-900">
                {schedule.name}
              </h4>
              <p id={`${rowId}-description`} class="sr-only">
                {rowDescription(schedule, targetName(schedule), last)}
              </p>
              <dl class={FIELDS_CLASS}>
                <dt class="font-medium">Target</dt>
                <dd class="break-words">{targetName(schedule)}</dd>
                <dt class="font-medium">Action</dt>
                <dd class="break-words">{schedule.text}</dd>
                <dt class="font-medium">Repeat</dt>
                <dd>{repeatWords(schedule.repeat)}</dd>
                <dt class="font-medium">Switch</dt>
                <dd>{switchText(schedule)}</dd>
                <dt class="font-medium">Next run</dt>
                <dd>{dueText(schedule.next_due)}</dd>
                <dt class="font-medium">Last run</dt>
                <dd>{lastRunText(last)}</dd>
                {#if schedule.last_missed !== null}
                  <dt class="font-medium">Last missed</dt>
                  <dd>{dueText(schedule.last_missed)}</dd>
                {/if}
              </dl>
              {#if reason !== null}
                <p class="mt-1 text-sm font-medium text-red-900">{reason}</p>
              {/if}
              <div class="mt-2 flex flex-wrap gap-2">
                <Button variant="secondary" onclick={() => openEdit(schedule)}>
                  Edit {schedule.name}
                </Button>
                <Button
                  variant="secondary"
                  onclick={() =>
                    void controller.switchTo(schedule.id, !schedule.on)}
                >
                  Switch {schedule.on ? "off" : "on"}
                  {schedule.name}
                </Button>
                <Button
                  variant="secondary"
                  onclick={() => void controller.runNow(schedule.id)}
                >
                  Run now {schedule.name}
                </Button>
                <Button
                  variant="secondary"
                  onclick={() => void controller.openHistory(schedule.id)}
                >
                  History of {schedule.name}
                </Button>
                <Button
                  variant="danger"
                  onclick={() => (deleteTarget = schedule)}
                >
                  Delete {schedule.name}
                </Button>
              </div>
            </article>
          </li>
        {/each}
      </ul>
    </section>

    <div><Button variant="secondary" onclick={onclose}>Close</Button></div>
  </div>
</Dialog>

{#if form !== null}
  <ScheduleForm
    title={form.title}
    draft={form.draft}
    {agents}
    onsave={save}
    onclose={() => (form = null)}
  />
{/if}

{#if history !== null && historySchedule !== null}
  <ScheduleHistory
    schedule={historySchedule}
    {history}
    onclose={() => controller.closeHistory()}
  />
{/if}

{#if deleteTarget !== null}
  <ConfirmDialog
    text={{
      title: `Delete ${deleteTarget.name}?`,
      body: DELETE_BODY,
      confirmLabel: "Delete schedule",
      isDestructive: true,
    }}
    onconfirm={() => void confirmDelete()}
    oncancel={() => (deleteTarget = null)}
  />
{/if}
