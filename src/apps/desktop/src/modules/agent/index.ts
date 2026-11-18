// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createOfficeStore,
  type OfficeStore,
} from "./state/office.store.svelte";
export { default as OfficeView } from "./views/office-view.svelte";
export type { Agent, Project } from "./models/agent";
export { shownStatusText } from "./models/agent";
