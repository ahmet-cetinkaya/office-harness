<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import {
    changeKindText,
    formatCommitTime,
    NO_COMMITS_TEXT,
    NOT_REPOSITORY_TEXT,
    visibleBlock,
    visibleLine,
  } from "../models/file-view";
  import type { FileViewState } from "../state/file-view.svelte";
  import DiffView from "./diff-view.svelte";
  import PartMessage from "./part-message.svelte";

  interface Props {
    fileView: FileViewState;
  }

  let { fileView }: Props = $props();

  const history = $derived(fileView.view.history);
  const opened = $derived(history.opened);
  const detail = $derived(opened?.detail ?? null);
</script>

{#if history.phase === "ready"}
  <div
    class="grid min-h-0 flex-1 grid-rows-[minmax(8rem,40%)_minmax(0,1fr)] gap-4 md:grid-cols-[minmax(14rem,24rem)_minmax(0,1fr)] md:grid-rows-1"
  >
    <div class="flex min-h-0 flex-col gap-2">
      <ul
        aria-label="Commits"
        class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-400 bg-white"
      >
        {#each history.commits as commit (commit.id)}
          <li>
            <button
              type="button"
              aria-current={opened?.id === commit.id}
              class="flex min-h-11 w-full flex-col items-start justify-center px-3 py-1 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-700 aria-[current=true]:bg-blue-50"
              onclick={() => void fileView.openCommit(commit.id)}
            >
              <span class="break-words">
                <span class="font-mono">{commit.short_id}</span>
                {visibleLine(commit.subject)}
              </span>
              <span class="text-xs break-words text-neutral-700">
                {visibleLine(commit.author)}, {formatCommitTime(commit.time)}
              </span>
            </button>
          </li>
        {/each}
      </ul>
      {#if history.moreFailure !== null}
        <FailureMessage
          lead="More commits did not load."
          message={history.moreFailure}
        />
      {/if}
      {#if history.hasMore}
        <div>
          <Button
            variant="secondary"
            disabled={history.isLoadingMore}
            onclick={() => void fileView.showMoreCommits()}
          >
            Show more
          </Button>
        </div>
      {/if}
    </div>
    <div class="flex min-h-0 min-w-0 flex-col gap-4">
      {#if opened === null}
        <p class="text-neutral-700">Choose a commit to see its files.</p>
      {:else if opened.phase === "loading"}
        <p role="status" class="text-neutral-700">Loading the commit.</p>
      {:else if opened.phase === "failed"}
        <FailureMessage
          lead="The commit did not load."
          message={opened.failure ?? ""}
        />
      {:else if detail !== null}
        <!-- Focusable, so the keyboard can scroll it. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          role="region"
          aria-label="Commit details"
          tabindex="0"
          class="max-h-[45%] min-h-24 overflow-auto rounded-md border border-neutral-400 bg-white p-3 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
        >
          <dl
            class="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-1 text-sm text-neutral-900"
          >
            <dt class="font-medium">Commit</dt>
            <dd class="font-mono break-all">{detail.id}</dd>
            <dt class="font-medium">Author</dt>
            <dd class="break-words">
              {visibleLine(detail.author)} ({visibleLine(detail.email)})
            </dd>
            <dt class="font-medium">Time</dt>
            <dd>{formatCommitTime(detail.time)}</dd>
          </dl>
          <pre
            class="mt-3 font-sans text-sm break-words whitespace-pre-wrap text-neutral-900">{visibleBlock(
              detail.message,
            )}</pre>
          <ul aria-label="Files of the commit" class="mt-3 flex flex-col">
            {#each detail.files as file (file.path)}
              <li>
                <button
                  type="button"
                  aria-current={opened.selectedPath === file.path}
                  class="flex min-h-11 w-full items-center gap-2 px-2 text-left text-sm text-neutral-900 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-700 aria-[current=true]:bg-blue-50"
                  onclick={() => fileView.openCommitFile(file)}
                >
                  <span class="font-medium">{changeKindText(file.kind)}</span>
                  <span class="break-all">
                    {visibleLine(file.path)}
                    {#if file.old_path !== null}
                      (from {visibleLine(file.old_path)})
                    {/if}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
          {#if detail.more > 0}
            <p class="text-sm text-neutral-700">{detail.more} more not shown</p>
          {/if}
        </div>
        <DiffView
          viewed={history.viewedDiff}
          idleText="Choose a file of the commit to see its diff."
          onreveal={fileView.revealDiff}
        />
      {/if}
    </div>
  </div>
{:else}
  <PartMessage
    phase={history.phase}
    failure={history.failure}
    loadingText="Loading the history."
    failureLead="The history did not load."
    emptyText={history.repository ? NO_COMMITS_TEXT : NOT_REPOSITORY_TEXT}
    onrefresh={fileView.refresh}
  />
{/if}
