<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    BINARY_FILE_TEXT,
    NO_DIFF_TEXT,
    SECRET_DIFF_TEXT,
    tooLargeText,
    visibleLine,
    type DiffLineKind,
  } from "../models/file-view";
  import type { ViewedDiff } from "../state/file-view.svelte";

  interface Props {
    viewed: ViewedDiff | null;
    /** What shows while no file is chosen. */
    idleText: string;
    onreveal: () => void;
  }

  let { viewed, idleText, onreveal }: Props = $props();

  // The sign and the words carry the kind; the tint only helps the eye.
  const LINE_CLASS: Record<DiffLineKind, string> = {
    hunk: "bg-neutral-100 text-neutral-700",
    added: "bg-green-50 text-green-900",
    removed: "bg-red-50 text-red-900",
    context: "",
    note: "text-neutral-700",
  };
  const SIGN: Partial<Record<DiffLineKind, string>> = {
    added: "+",
    removed: "-",
  };
  const SIGN_TEXT: Partial<Record<DiffLineKind, string>> = {
    added: "added",
    removed: "removed",
  };
</script>

<div class="flex min-h-0 min-w-0 flex-col gap-3">
  {#if viewed === null}
    <p class="text-neutral-700">{idleText}</p>
  {:else}
    <h3 class="font-mono text-sm break-all text-neutral-900">
      {visibleLine(viewed.target.path)}
    </h3>
    {#if viewed.phase === "loading"}
      <p role="status" class="text-neutral-700">Loading the diff.</p>
    {:else if viewed.phase === "failed"}
      <FailureMessage
        lead="The diff did not load."
        message={viewed.failure ?? ""}
      />
    {:else if viewed.diff?.state === "secret"}
      <p class="text-neutral-900">{SECRET_DIFF_TEXT}</p>
      <div><Button onclick={onreveal}>Reveal</Button></div>
    {:else if viewed.diff?.state === "too_large"}
      <p class="text-neutral-900">
        {tooLargeText("This change", viewed.diff.size)}
      </p>
    {:else if viewed.diff?.state === "binary"}
      <p class="text-neutral-900">{BINARY_FILE_TEXT}</p>
    {:else if viewed.diff?.state === "none" || viewed.lines.length === 0}
      <p role="status" class="text-neutral-700">{NO_DIFF_TEXT}</p>
    {:else}
      <!-- Focusable, so the keyboard can scroll it. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        role="region"
        aria-label="Diff"
        tabindex="0"
        class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-400 bg-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
      >
        <table class="border-collapse font-mono text-sm text-neutral-900">
          <tbody>
            {#each viewed.lines as line, index (index)}
              <tr class={LINE_CLASS[line.kind]}>
                {#if line.kind === "hunk"}
                  <td colspan="4" class="px-3 whitespace-pre">{line.text}</td>
                {:else}
                  <th
                    scope="row"
                    class="px-2 text-right align-top font-normal text-neutral-700 select-none"
                  >
                    {line.oldNumber ?? ""}
                  </th>
                  <td
                    class="px-2 text-right align-top text-neutral-700 select-none"
                  >
                    {line.newNumber ?? ""}
                  </td>
                  <td class="w-6 text-center align-top">
                    {#if SIGN[line.kind]}
                      <span aria-hidden="true">{SIGN[line.kind]}</span>
                      <span class="sr-only">{SIGN_TEXT[line.kind]}</span>
                    {/if}
                  </td>
                  <td class="pr-3 whitespace-pre">{line.text}</td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>
