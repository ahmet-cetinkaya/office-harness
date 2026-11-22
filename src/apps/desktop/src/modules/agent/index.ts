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
export type {
  Additions,
  Agent,
  EnvironmentEntry,
  NewAgent,
  Project,
} from "./models/agent";
export { isActive, shownStatusText } from "./models/agent";
export { taskRowText, visibleText } from "./models/task-text";
export {
  formatEnvironment,
  formatFlags,
  parseEnvironment,
  parseFlags,
  parseStartupPrompt,
} from "./services/additions-text";
export {
  LAST_AGENT_GONE_LEAD,
  LAST_AGENT_GONE_MESSAGE,
} from "./state/office-controller";
