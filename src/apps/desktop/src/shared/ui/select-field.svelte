<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { FIELD_CLASS } from "./field-class";

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    label: string;
    value: string;
    options: Option[];
  }

  let { label, value = $bindable(), options }: Props = $props();

  const id = $props.id();
</script>

<div class="flex flex-col gap-1">
  <label for={id} class="text-sm font-medium text-neutral-900">{label}</label>
  <select {id} bind:value class={FIELD_CLASS}>
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</div>
