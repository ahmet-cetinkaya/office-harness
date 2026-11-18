// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { createOfficeStore } from "../../agent";

/** The project choice of the panel, over the office store's snapshot. */
export function createPanelStore(client: CoreClient) {
  const office = createOfficeStore(client);
  let projectChoice = $state<string | null>(null);

  return {
    /** The project id, or null for "All projects". */
    get projectChoice(): string | null {
      return projectChoice;
    },
    setProjectChoice(choice: string | null): void {
      projectChoice = choice;
    },
    office,
    client,
  };
}

export type PanelStore = ReturnType<typeof createPanelStore>;
