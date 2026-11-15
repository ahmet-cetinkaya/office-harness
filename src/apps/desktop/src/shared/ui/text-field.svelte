<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { FIELD_CLASS } from "./field-class";

  interface Props {
    label: string;
    value: string;
    hint?: string;
    /** More than one line of text, for example a prompt. */
    isMultiline?: boolean;
  }

  let {
    label,
    value = $bindable(),
    hint = "",
    isMultiline = false,
  }: Props = $props();

  const id = $props.id();
  const hintId = `${id}-hint`;
</script>

<div class="flex flex-col gap-1">
  <label for={id} class="text-sm font-medium text-neutral-900">{label}</label>
  {#if isMultiline}
    <textarea
      {id}
      bind:value
      rows="4"
      aria-describedby={hint === "" ? undefined : hintId}
      class={FIELD_CLASS}></textarea>
  {:else}
    <input
      {id}
      bind:value
      type="text"
      autocomplete="off"
      spellcheck="false"
      aria-describedby={hint === "" ? undefined : hintId}
      class={FIELD_CLASS}
    />
  {/if}
  {#if hint !== ""}
    <p id={hintId} class="text-sm text-neutral-700">{hint}</p>
  {/if}
</div>
