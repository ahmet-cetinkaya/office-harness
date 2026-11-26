<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import SelectField from "$shared/ui/select-field.svelte";
  import TextField from "$shared/ui/text-field.svelte";
  import type { Agent } from "../../agent";
  import {
    draftProblems,
    FIELD_LABELS,
    MAX_HOURS,
    MIN_HOURS,
    problemsOfField,
    problemText,
    REPEAT_KINDS,
    WEEKDAYS,
    type FieldName,
    type FieldProblem,
    type RepeatKind,
    type ScheduleDraft,
    type WeekdayCode,
  } from "../models/schedule";
  import type { SaveOutcome } from "../state/schedules-controller";

  interface Props {
    /** The schedule's text on top of the form, "New schedule" or "Edit ...". */
    title: string;
    draft: ScheduleDraft;
    agents: readonly Pick<Agent, "id" | "name">[];
    onsave: (draft: ScheduleDraft) => Promise<SaveOutcome>;
    onclose: () => void;
  }

  let { title, draft, agents, onsave, onclose }: Props = $props();

  const uid = $props.id();
  const FIELD_ORDER: FieldName[] = ["name", "text", "target", "repeat"];

  // The form edits its own copy; Escape leaves the schedule as it was.
  // svelte-ignore state_referenced_locally
  let values = $state<ScheduleDraft>({ ...draft, days: [...draft.days] });
  let problems = $state<FieldProblem[]>([]);
  let message = $state<string | null>(null);
  let isSaving = $state(false);

  const targetOptions = $derived(
    agents.map((agent) => ({ value: agent.id, label: agent.name })),
  );
  const refusedFields = $derived(
    FIELD_ORDER.filter((field) => problemsOfField(problems, field).length > 0),
  );

  const errorId = (field: FieldName): string | undefined =>
    problemsOfField(problems, field).length === 0
      ? undefined
      : `${uid}-error-${field}`;

  function toggleDay(code: WeekdayCode, isChecked: boolean): void {
    values.days = isChecked
      ? [...values.days, code]
      : values.days.filter((day) => day !== code);
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    message = null;
    problems = draftProblems(values);
    if (problems.length > 0) return;
    isSaving = true;
    const outcome = await onsave(values);
    isSaving = false;
    if (outcome.saved) return;
    problems = outcome.problems;
    message = outcome.problems.length === 0 ? outcome.message : null;
  }
</script>

<Dialog {title} {onclose}>
  <form class="flex flex-col gap-4" onsubmit={submit} novalidate>
    {#if refusedFields.length > 0}
      <div
        role="alert"
        class="rounded-md border border-red-700 bg-red-50 p-3 text-sm text-red-900"
      >
        <strong>The schedule was not saved.</strong>
        <ul class="mt-1 list-disc pl-5">
          {#each problems as item (item.field + item.rule)}
            <li id={`${uid}-error-${item.field}`}>{problemText(item)}</li>
          {/each}
        </ul>
      </div>
    {/if}
    {#if message !== null}
      <FailureMessage lead="The schedule was not saved." {message} />
    {/if}

    <TextField
      label={FIELD_LABELS.name}
      errorId={errorId("name")}
      bind:value={values.name}
    />
    <TextField
      label="Text sent to the target"
      errorId={errorId("text")}
      bind:value={values.text}
    />
    <SelectField
      label={FIELD_LABELS.target}
      options={targetOptions}
      bind:value={values.targetAgentId}
    />
    <SelectField
      label={FIELD_LABELS.repeat}
      options={[...REPEAT_KINDS]}
      value={values.kind}
      onchange={(kind) => (values.kind = kind as RepeatKind)}
    />

    {#if values.kind === "once"}
      <TextField
        label="Date"
        hint="Year, month, day, like 2026-03-15."
        errorId={errorId("repeat")}
        bind:value={values.date}
      />
    {/if}
    {#if values.kind === "weekly"}
      <fieldset class="flex flex-col gap-1">
        <legend class="text-sm font-medium text-neutral-900">Days</legend>
        {#each WEEKDAYS as day (day.code)}
          <label
            class="flex min-h-11 items-center gap-2 text-sm text-neutral-900"
          >
            <input
              type="checkbox"
              class="size-5"
              checked={values.days.includes(day.code)}
              onchange={(event) =>
                toggleDay(day.code, event.currentTarget.checked)}
            />
            {day.label}
          </label>
        {/each}
      </fieldset>
    {/if}
    {#if values.kind !== "every_hours"}
      <TextField
        label="Time"
        hint="Hours and minutes in the office zone, like 09:30."
        errorId={errorId("repeat")}
        bind:value={values.time}
      />
    {:else}
      <TextField
        label="Hours between runs"
        hint={`A whole number from ${MIN_HOURS} to ${MAX_HOURS}.`}
        errorId={errorId("repeat")}
        bind:value={values.hours}
      />
    {/if}

    <label class="flex min-h-11 items-center gap-2 text-sm text-neutral-900">
      <input type="checkbox" class="size-5" bind:checked={values.on} />
      Switched on
    </label>

    <div class="flex flex-wrap gap-3">
      <Button type="submit" disabled={isSaving}>Save schedule</Button>
      <Button variant="secondary" onclick={onclose}>Cancel</Button>
    </div>
  </form>
</Dialog>
