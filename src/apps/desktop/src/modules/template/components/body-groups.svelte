<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import type { TextGroup } from "../models/template-text";

  interface Props {
    groups: TextGroup[];
    /** The heading level of each group's title, under the caller's heading. */
    level: "h3" | "h4";
  }

  let { groups, level }: Props = $props();
</script>

{#each groups as group (group.title)}
  <section class="flex flex-col gap-1">
    <svelte:element this={level} class="text-sm font-semibold text-neutral-900">
      {group.title}
    </svelte:element>
    <dl
      class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm text-neutral-900"
    >
      {#each group.rows as row (row.label)}
        <dt class="font-medium">{row.label}</dt>
        <dd class="break-all whitespace-pre-wrap">{row.value}</dd>
      {/each}
    </dl>
  </section>
{/each}
