<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Button from "$shared/ui/button.svelte";
  import type { Project } from "../models/agent";

  interface Props {
    projects: Project[];
    isBusy: boolean;
    ondelete: (project: Project) => void;
  }

  let { projects, isBusy, ondelete }: Props = $props();
</script>

<section aria-labelledby="projects-heading" class="flex flex-col gap-2">
  <h2 id="projects-heading" class="text-lg font-semibold text-neutral-900">
    Projects
  </h2>
  {#if projects.length === 0}
    <p class="text-sm text-neutral-700">No project exists yet.</p>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each projects as project (project.id)}
        <li class="flex flex-wrap items-center gap-3 text-sm text-neutral-900">
          <span>
            <span class="font-medium">{project.name}</span>
            <span class="break-all text-neutral-700">{project.folder}</span>
          </span>
          <Button
            variant="secondary"
            disabled={isBusy}
            onclick={() => ondelete(project)}
          >
            Delete project {project.name}
          </Button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
