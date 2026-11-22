<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import Dialog from "$shared/ui/dialog.svelte";
  import type { FormStart } from "../models/template-form";
  import type { FailureText } from "../state/template-controller";
  import type { TemplateToSave } from "../services/template-service";
  import TemplateForm from "./template-form.svelte";

  interface Props {
    /** The template to edit, or null for a new one. */
    start: FormStart;
    isBusy: boolean;
    onsave: (template: TemplateToSave) => Promise<FailureText>;
    onclose: () => void;
  }

  let { start, isBusy, onsave, onclose }: Props = $props();
</script>

<Dialog
  title={start !== null && "template" in start
    ? `Edit ${start.template.name}`
    : "New template"}
  {onclose}
>
  <TemplateForm {start} {isBusy} {onsave} {onclose} />
</Dialog>
