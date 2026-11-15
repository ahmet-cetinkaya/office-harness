<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    children: Snippet;
  }

  let { title, children }: Props = $props();
</script>

<section class="mx-auto flex max-w-xl flex-col gap-4 p-6">
  <h1 class="text-2xl font-semibold text-neutral-900">{title}</h1>
  {@render children()}
</section>
