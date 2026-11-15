// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { createActionRecordSource } from "../../audit";
import {
  createOfficeController,
  INITIAL_OFFICE,
  type OfficeSnapshot,
} from "./office-controller";

/** The office controller's snapshot as reactive state for the views. */
export function createOfficeStore(client: CoreClient) {
  let snapshot = $state<OfficeSnapshot>(INITIAL_OFFICE);
  const controller = createOfficeController(
    client,
    (next) => {
      snapshot = next;
    },
    createActionRecordSource(client),
  );

  return {
    get snapshot(): OfficeSnapshot {
      return snapshot;
    },
    controller,
  };
}

export type OfficeStore = ReturnType<typeof createOfficeStore>;
