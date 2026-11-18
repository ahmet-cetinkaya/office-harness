<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import SelectField from "$shared/ui/select-field.svelte";
  import type { Project } from "../../agent";

  interface Props {
    projects: Project[];
    /** The chosen project id, or null for "All projects". */
    value: string | null;
    onchange: (choice: string | null) => void;
  }

  let { projects, value, onchange }: Props = $props();

  const ALL_VALUE = "";
  const options = $derived([
    { value: ALL_VALUE, label: "All projects" },
    ...projects.map((project) => ({
      value: project.id,
      label: project.name,
    })),
  ]);

  function choose(next: string): void {
    onchange(next === ALL_VALUE ? null : next);
  }
</script>

<SelectField
  label="Project"
  value={value ?? ALL_VALUE}
  {options}
  onchange={choose}
/>
