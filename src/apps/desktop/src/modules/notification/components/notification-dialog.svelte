<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Dialog from "$shared/ui/dialog.svelte";
  type NotificationRow = {
    id: string;
    text: string;
  };

  interface Props {
    /** The rows to show, in the list order (D4). */
    rows: NotificationRow[];
    onopen: (id: string) => void;
    onclose: () => void;
  }

  let { rows, onopen, onclose }: Props = $props();

  let list = $state<HTMLElement | null>(null);

  const OPTION_CLASS =
    "min-h-11 w-full rounded-md px-3 py-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700";

  // The rows render after the dialog mounts, so the first focus waits for
  // them; an empty list leaves the focus on the dialog itself.
  $effect(() => {
    list?.querySelector<HTMLElement>("button")?.focus();
  });

  function focusOption(step: number, from: HTMLElement): void {
    const buttons = [...(list?.querySelectorAll<HTMLElement>("button") ?? [])];
    buttons[buttons.indexOf(from) + step]?.focus();
  }

  function handleOptionKeys(event: KeyboardEvent): void {
    const step =
      event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    focusOption(step, event.currentTarget as HTMLElement);
  }
</script>

<Dialog title="Notifications" {onclose} initialFocus={list}>
  <div class="flex flex-col gap-3">
    {#if rows.length === 0}
      <p class="text-sm text-neutral-700">No notification has arrived.</p>
    {:else}
      <ul bind:this={list} class="flex max-h-80 flex-col gap-1 overflow-y-auto">
        {#each rows as row (row.id)}
          <li>
            <button
              type="button"
              class={OPTION_CLASS}
              onclick={() => onopen(row.id)}
              onkeydown={handleOptionKeys}
            >
              {row.text}
            </button>
          </li>
        {/each}
      </ul>
      <p class="text-sm text-neutral-700">
        Enter or Space opens the focused notification. Escape closes the list.
      </p>
    {/if}
  </div>
</Dialog>
