// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  createNotificationController,
  createOsNotificationService,
  readLastView,
} from "../../notification";
import { createOfficeStore } from "../../agent";

/**
 * The panel's stores over one client: the office, the project choice, the
 * notification list with its OS notifications, and the last view (0003).
 */
export function createPanelStore(client: CoreClient) {
  const office = createOfficeStore(client, () => void notifications.load());
  const notifications = createNotificationController(
    client,
    createOsNotificationService(),
    () => office.snapshot.selectedAgentId,
    () => document.hasFocus(),
  );
  // The shell writes the last view (0005 D6); recording must not begin until
  // the restore has read the stored value, or an eager write would clobber it.
  let isLastViewRestored = $state(false);
  const lastView = {
    read: readLastView,
    markRestored(): void {
      isLastViewRestored = true;
    },
    get isRestored(): boolean {
      return isLastViewRestored;
    },
  };
  let projectChoice = $state<string | null>(null);

  // Opening an agent's terminal reads its notifications (FR-009). The select
  // sits here, so every route through the panel reads, not just the rows.
  const controller = {
    ...office.controller,
    select(agentId: string): void {
      office.controller.select(agentId);
      void notifications.readAgent(agentId);
    },
  };

  return {
    /** The project id, or null for "All projects". */
    get projectChoice(): string | null {
      return projectChoice;
    },
    setProjectChoice(choice: string | null): void {
      projectChoice = choice;
    },
    office: {
      get snapshot() {
        return office.snapshot;
      },
      controller,
      client,
    },
    notifications,
    lastView,
    client,
  };
}

export type PanelStore = ReturnType<typeof createPanelStore>;
