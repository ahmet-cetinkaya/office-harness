// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createHoldController,
  INITIAL_HOLD,
  type FailureText,
  type HoldController,
  type HoldSnapshot,
} from "./state/hold-controller";
export {
  AGENT_ON_HOLD_TEXT,
  automaticHoldText,
  holdOfAgent,
  NO_HOLDS,
  OFFICE_ON_HOLD_TEXT,
  type Hold,
  type HoldState,
} from "./models/hold";
export { default as OfficeHoldBar } from "./components/office-hold-bar.svelte";
