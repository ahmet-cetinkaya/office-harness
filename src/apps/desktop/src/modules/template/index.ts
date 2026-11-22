// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createTemplateController,
  INITIAL_TEMPLATES,
  type TemplateController,
} from "./state/template-controller";
export { default as TemplatesView } from "./views/templates-view.svelte";
export { default as CreateFromTemplateDialog } from "./components/create-from-template-dialog.svelte";
export { default as SaveAsTemplateDialog } from "./components/save-as-template-dialog.svelte";
