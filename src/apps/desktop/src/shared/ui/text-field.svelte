<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { FIELD_CLASS } from "./field-class";

  interface Props {
    label: string;
    value: string;
    hint?: string;
    /** The id of the message that refuses this field's value, when it does. */
    errorId?: string;
    /** More than one line of text, for example a prompt. */
    isMultiline?: boolean;
  }

  let {
    label,
    value = $bindable(),
    hint = "",
    errorId,
    isMultiline = false,
  }: Props = $props();

  const id = $props.id();
  const hintId = `${id}-hint`;
  const describedBy = $derived(
    [hint === "" ? null : hintId, errorId ?? null]
      .filter((reference) => reference !== null)
      .join(" ") || undefined,
  );
</script>

<div class="flex flex-col gap-1">
  <label for={id} class="text-sm font-medium text-neutral-900">{label}</label>
  {#if isMultiline}
    <textarea
      {id}
      bind:value
      rows="4"
      aria-describedby={describedBy}
      aria-invalid={errorId === undefined ? undefined : "true"}
      class={FIELD_CLASS}></textarea>
  {:else}
    <input
      {id}
      bind:value
      type="text"
      autocomplete="off"
      spellcheck="false"
      aria-describedby={describedBy}
      aria-invalid={errorId === undefined ? undefined : "true"}
      class={FIELD_CLASS}
    />
  {/if}
  {#if hint !== ""}
    <p id={hintId} class="text-sm text-neutral-700">{hint}</p>
  {/if}
</div>
