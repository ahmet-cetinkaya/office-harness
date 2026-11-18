// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createOfficeStore,
  type OfficeStore,
} from "./state/office.store.svelte";
export { default as AgentFormDialog } from "./components/agent-form-dialog.svelte";
export { default as AgentDetails } from "./components/agent-details.svelte";
export { default as ConfirmDialog } from "./components/confirm-dialog.svelte";
export { default as ProjectFormDialog } from "./components/project-form-dialog.svelte";
export { default as ProjectList } from "./components/project-list.svelte";
export type { Agent, NewAgent, Project } from "./models/agent";
export { isActive, shownStatusText } from "./models/agent";
