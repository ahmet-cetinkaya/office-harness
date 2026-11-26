// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createOfficeStore,
  type OfficeStore,
} from "./state/office.store.svelte";
export { default as AgentFormDialog } from "./components/agent-form-dialog.svelte";
export { default as AgentDetails } from "./components/agent-details.svelte";
export { default as WorktreeDeleteDialog } from "./components/worktree-delete-dialog.svelte";
export { default as WorktreeField } from "./components/worktree-field.svelte";
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
export {
  creatorText,
  isActive,
  listStatusText,
  shownStatusText,
  statusSourceText,
  statusText,
} from "./models/agent";
export { confirmationText } from "./services/confirmations";
export { taskRowText, visibleText } from "./models/task-text";
export {
  environmentLineText,
  formatEnvironment,
  formatFlags,
  parseEnvironment,
  parseFlags,
  parseStartupPrompt,
} from "./services/additions-text";
export {
  KEEP_WORKTREES,
  deleteResultText,
  failedOutcome,
  readDeleteResult,
  type DeleteOutcome,
  type DeleteResult,
  type WorktreeChoice,
  type WorktreeInfo,
  type WorktreeSelection,
  type WorktreeQuery,
  type WorktreeStatus,
} from "./models/worktree";
export { createAgentService } from "./services/agent-service";
export {
  LAST_AGENT_GONE_LEAD,
  LAST_AGENT_GONE_MESSAGE,
} from "./state/office-controller";
