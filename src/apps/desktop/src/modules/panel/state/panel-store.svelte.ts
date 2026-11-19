// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  createNotificationController,
  createOsNotificationService,
  readLastView,
} from "../../notification";
import { createOfficeStore } from "../../agent";
import { createHoldController } from "../../hold";

export type OpenView = "panel" | "scene";

/**
 * The panel's stores over one client: the office, the project choice, the
 * notification list with its OS notifications, the holds (0007), and the
 * last view (0003).
 */
export function createPanelStore(client: CoreClient) {
  const office = createOfficeStore(client, () => void notifications.load());
  const notifications = createNotificationController(
    client,
    createOsNotificationService(),
    () => office.snapshot.selectedAgentId,
    () => document.hasFocus(),
  );
  const hold = createHoldController(
    client,
    (agentId) =>
      office.snapshot.agents.find((agent) => agent.id === agentId)?.name ??
      null,
  );
  // The restore is the one reader of `last_view` and the only place the
  // stored view word is applied. The shell writes the last view only after
  // the restore has settled, or an eager write would clobber the stored
  // value. `isRestoring` is set before the read starts, so a second mount
  // of the panel (a scene round trip) or a reconnect during the read cannot
  // start a second restore.
  let isLastViewRestoring = $state(false);
  let isLastViewRestored = $state(false);
  const lastView = {
    read: readLastView,
    markRestoring(): void {
      isLastViewRestoring = true;
    },
    markRestored(): void {
      isLastViewRestored = true;
    },
    get isRestoring(): boolean {
      return isLastViewRestoring;
    },
    get isRestored(): boolean {
      return isLastViewRestored;
    },
  };
  // The shell's open view. A switch the user makes before the restore has
  // settled wins: the stored view is the last session's place, not today's
  // instruction.
  let openView = $state<OpenView>("panel");
  let hasUserSwitched = false;
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
    /** The open view: the panel, or the office scene (0005 D6). */
    get openView(): OpenView {
      return openView;
    },
    /** The user's own switch of the view. */
    switchView(view: OpenView): void {
      hasUserSwitched = true;
      openView = view;
    },
    /** The restore's view, applied only if the user has not switched. */
    restoreView(view: OpenView): void {
      if (!hasUserSwitched) openView = view;
    },
    office: {
      get snapshot() {
        return office.snapshot;
      },
      controller,
      client,
    },
    notifications,
    hold,
    lastView,
    client,
  };
}

export type PanelStore = ReturnType<typeof createPanelStore>;
