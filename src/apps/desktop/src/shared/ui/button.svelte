<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "../utils";

  interface Props {
    children: Snippet;
    onclick: () => void;
    disabled?: boolean;
    variant?: "primary" | "secondary";
    element?: HTMLButtonElement | null;
  }

  let {
    children,
    onclick,
    disabled = false,
    variant = "primary",
    element = $bindable(null),
  }: Props = $props();

  // One accent (blue-700, 6.7:1 against white) marks the main action; the
  // 2.75rem height keeps the target above the 44px minimum.
  const base =
    "min-h-11 rounded-md px-4 text-sm font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700 disabled:opacity-60";
  const variants = {
    primary: "bg-blue-700 text-white hover:bg-blue-800",
    secondary:
      "border border-neutral-400 bg-white text-neutral-900 hover:bg-neutral-100",
  };
</script>

<button
  bind:this={element}
  type="button"
  {onclick}
  {disabled}
  class={cn(base, variants[variant])}
>
  {@render children()}
</button>
