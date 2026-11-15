<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  interface Props {
    title: string;
    onclose: () => void;
    /** Takes focus when the dialog opens. Without it the first control does. */
    initialFocus?: HTMLElement | null;
    children: Snippet;
  }

  let { title, onclose, initialFocus = null, children }: Props = $props();

  const titleId = `${$props.id()}-title`;
  let dialog = $state<HTMLDialogElement | null>(null);

  // showModal makes the page behind the dialog inert, so Tab stays inside it.
  // The component is mounted only while the dialog is open, so its removal
  // is the close, and focus goes back to the control that opened it.
  onMount(() => {
    const opener = document.activeElement;
    dialog?.showModal();
    return () => {
      // Later, so that the dialog has left the page by the time focus moves.
      queueMicrotask(() => {
        if (opener instanceof HTMLElement) opener.focus();
      });
    };
  });

  $effect(() => {
    initialFocus?.focus();
  });

  function closeOnEscape(event: Event): void {
    event.preventDefault();
    onclose();
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby={titleId}
  oncancel={closeOnEscape}
  class="m-auto w-[min(32rem,calc(100vw-2rem))] rounded-md border border-neutral-400 bg-white p-6 text-neutral-900 backdrop:bg-neutral-900/60"
>
  <div class="flex flex-col gap-4">
    <h2 id={titleId} class="text-xl font-semibold">{title}</h2>
    {@render children()}
  </div>
</dialog>
