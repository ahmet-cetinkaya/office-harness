<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import SelectField from "$shared/ui/select-field.svelte";
  import {
    baseOptions,
    type RepositoryInfo,
    type WorktreeInfo,
    type WorktreeQuery,
    type WorktreeSelection,
  } from "../models/worktree";

  interface Props {
    /** The chosen folder; the project's folder is used when it is empty. */
    folder: string;
    projectId?: string | null;
    name: string;
    /** Changes after a refused create, so the branch name is read again. */
    refreshKey?: number;
    lookup: (query: WorktreeQuery) => Promise<WorktreeInfo>;
    /** The create fields, or null while the box is off or hidden. */
    selection: WorktreeSelection | null;
  }

  let {
    folder,
    projectId = null,
    name,
    refreshKey = 0,
    lookup,
    selection = $bindable(null),
  }: Props = $props();

  const LOOKUP_DELAY_MS = 300;
  let info = $state<RepositoryInfo | null>(null);
  let isOwn = $state(false);
  let base = $state("");

  function readQuery(): WorktreeQuery | null {
    const trimmed = folder.trim();
    if (trimmed === "" && projectId === null) return null;
    const query: WorktreeQuery = { name: name.trim() };
    if (trimmed !== "") query.folder = trimmed;
    else if (projectId !== null) query.project_id = projectId;
    return query;
  }

  function show(answer: WorktreeInfo): void {
    info = answer.in_repository ? answer : null;
    if (info === null) return;
    const isKnown = baseOptions(info).some((option) => option.value === base);
    if (!isKnown) base = info.base;
  }

  // The form shows the choice only when the answer says so; a folder the
  // server cannot read leaves it hidden and the create reports the reason.
  $effect(() => {
    void refreshKey;
    const query = readQuery();
    if (query === null) {
      info = null;
      return;
    }
    let isStale = false;
    const timer = setTimeout(() => {
      lookup(query).then(
        (answer) => !isStale && show(answer),
        () => !isStale && (info = null),
      );
    }, LOOKUP_DELAY_MS);
    return () => {
      isStale = true;
      clearTimeout(timer);
    };
  });

  $effect(() => {
    selection =
      info !== null && isOwn
        ? { own_worktree: true, base, branch_name: info.branch_name }
        : null;
  });
</script>

{#if info !== null}
  <div class="flex flex-col gap-3">
    <label class="flex items-center gap-2 text-sm font-medium text-neutral-900">
      <input type="checkbox" bind:checked={isOwn} />
      Own worktree
    </label>
    <SelectField label="Base" options={baseOptions(info)} bind:value={base} />
    <p class="text-sm text-neutral-900">
      Branch to create: <strong class="break-all">{info.branch_name}</strong>
    </p>
    <p class="text-sm text-neutral-700">
      Untracked files such as local environment files are not copied.
    </p>
  </div>
{/if}
