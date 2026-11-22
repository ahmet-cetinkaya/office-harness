<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import Dialog from "$shared/ui/dialog.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { visibleText } from "../../agent";
  import type { ImportPreviewEntry } from "../models/template";
  import { bodyGroups, kindText } from "../models/template-text";
  import BodyGroups from "./body-groups.svelte";

  interface Props {
    entries: ImportPreviewEntry[];
    /** The reason the confirm was refused, or null. */
    failure: string | null;
    isBusy: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { entries, failure, isBusy, onconfirm, oncancel }: Props = $props();
</script>

<Dialog title="Import preview" onclose={oncancel}>
  <p class="text-neutral-700">
    Nothing is saved yet. Values in the file are not imported: every environment
    variable is saved with no value. Control characters are shown as symbols.
    Press Escape to cancel.
  </p>
  <ul class="flex flex-col gap-4">
    {#each entries as entry (entry.save_name)}
      <li class="flex flex-col gap-2 rounded-md border border-neutral-400 p-3">
        <h3 class="text-base font-semibold text-neutral-900">
          {visibleText(entry.save_name)}
        </h3>
        <dl
          class="grid grid-cols-[max-content_1fr] gap-x-4 text-sm text-neutral-900"
        >
          <dt class="font-medium">Name in the file</dt>
          <dd class="break-all">{visibleText(entry.name)}</dd>
          <dt class="font-medium">Kind</dt>
          <dd>{kindText(entry.kind)}</dd>
        </dl>
        <BodyGroups groups={bodyGroups(entry.body)} level="h4" />
      </li>
    {/each}
  </ul>
  {#if failure !== null}
    <FailureMessage lead="The templates were not saved." message={failure} />
  {/if}
  <div class="flex flex-wrap gap-3">
    <Button disabled={isBusy} onclick={onconfirm}>
      Save {entries.length === 1
        ? "this template"
        : `these ${entries.length} templates`}
    </Button>
    <Button variant="secondary" onclick={oncancel}>Cancel</Button>
  </div>
</Dialog>
